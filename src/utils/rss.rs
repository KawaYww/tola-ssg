#![allow(unused)]

use crate::{
    config::SiteConfig,
    log, run_command,
    utils::{self, build::collect_files, slug::slugify_path},
};
use anyhow::{Context, Ok, Result, anyhow};
use chrono::{DateTime, Utc};
use crossterm::style::Stylize;
use rayon::prelude::*;
use rss::{
    ChannelBuilder, GuidBuilder, ItemBuilder,
    extension::atom::{self, AtomExtension, AtomExtensionBuilder, Link},
    validation::Validate,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

const META_TAG_NAME: &str = "<tola-meta>";

pub struct RSSChannel {
    title: String,
    description: String,
    base_url: String,
    language: String,
    generator: String,
    items: Vec<PostMeta>,
}

#[derive(Serialize, Deserialize, Debug)]
struct PostMeta {
    title: Option<String>,
    summary: Option<String>,
    date: Option<String>,
    update: Option<String>,

    #[serde(default)]
    link: Option<String>,
    author: Option<String>,
}

#[rustfmt::skip]
#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "func", rename_all = "lowercase")]
enum TypstElement {
    Space,
    Linebreak,
    Text { text: String },
    Strike { text: String },
    Link { dest: String, body: Box<TypstElement> },
    Sequence { children: Vec<TypstElement> },
    #[serde(other)]
    OtherIgnored,
}

impl TypstElement {
    fn into_html_tag(self, config: &'static SiteConfig) -> String {
        match self {
            TypstElement::Space => " ".to_string(),
            TypstElement::Linebreak => "<br/>".to_string(),
            TypstElement::Text { text } => text,
            TypstElement::Strike { text } => format!("<strike>{text}</strike>"),
            TypstElement::Link { dest, body } => {
                let dest = if dest.starts_with(['.', '/']) {
                    let dest = dest.trim_start_matches('.').trim_start_matches('/');
                    format!("{}/{}", config.base.url.clone().unwrap(), dest)
                } else {
                    dest
                };
                format!("<a href=\"{dest}\">{}</a>", body.into_html_tag(config))
            }
            TypstElement::Sequence { children } => children
                .into_iter()
                .map(|child| child.into_html_tag(config))
                .collect(),
            TypstElement::OtherIgnored => "".to_string(),
        }
    }
}
pub fn get_guid_from_content_output_path(
    content_path: &Path,
    config: &'static SiteConfig,
) -> Result<String> {
    let root = config.get_root();
    let content = &config.build.content;
    let base_url = config.base.url.clone().unwrap_or_default();

    // println!("{:?}, {:?}, {:?}, {:?}", root, content, output, content_path);
    let relative_post_path = content_path
        .strip_prefix(content)?
        .to_str()
        .ok_or(anyhow!("Invalid path"))?
        .strip_suffix(".typ")
        .ok_or(anyhow!("Not a .typ file"))?;

    let guid_path = if content_path.file_name().is_some_and(|p| p == "index.typ") {
        PathBuf::from("index.html")
    } else {
        PathBuf::from(relative_post_path).join("index.html")
    };

    let guid_path = slugify_path(&guid_path, config);
    let guid_path = guid_path.to_str().unwrap();
    let guid_path = urlencoding::encode(guid_path).into_owned();
    let guid_path = format!("{}/{}", base_url.trim_end_matches("/"), guid_path);

    Ok(guid_path)
}

impl RSSChannel {
    pub fn new(config: &'static SiteConfig) -> Result<Self> {
        log!(true; "rss"; "generating rss feed started");
        let posts_path = collect_files(&config.build.content, &|path| {
            path.extension().is_some_and(|ext| ext == "typ")
        })?;
        let posts_meta = posts_path
            .par_iter()
            .map(|path| query_meta(path, config))
            .collect::<Result<Vec<_>>>()?;
        let rss = Self {
            title: config.base.title.clone(),
            description: config.base.description.clone(),
            base_url: config.base.url.clone().unwrap_or_default(),
            language: config.base.language.clone(),
            generator: "tola-ssg".to_string(),
            items: posts_meta,
        };

        Ok(rss)
    }

    pub fn into_rss_xml(self, config: &'static SiteConfig) -> Result<String> {
        let items = self
            .items
            .into_par_iter()
            .map(|item| {
                ItemBuilder::default()
                    .title(item.title.clone())
                    .link(item.link.clone())
                    .guid(
                        GuidBuilder::default()
                            .permalink(true)
                            .value(item.link.unwrap())
                            .build(),
                    )
                    .description(item.summary.clone())
                    // .pub_date(item.date.clone())
                    .author(item.author.clone())
                    .build()
            })
            .collect::<Vec<_>>();

        let atom_href = format!(
            "{}/{}",
            config.base.url.clone().unwrap().trim_end_matches("/"),
            config
                .build
                .rss
                .path
                .strip_prefix(&config.build.output)?
                .to_str()
                .unwrap()
        );
        let channel = ChannelBuilder::default()
            .atom_ext(
                AtomExtensionBuilder::default()
                    .link(Link {
                        href: atom_href,
                        rel: "self".to_string(),
                        mime_type: Some("application/atom+xml".to_string()),
                        ..Default::default()
                    })
                    .build(),
            )
            .title(self.title.clone())
            .link(self.base_url.clone())
            .description(self.description.clone())
            .language(self.language.clone())
            .generator(Some("tola-ssg".to_string()))
            .items(items)
            .build();
        channel.validate()?;
        Ok(channel.to_string())
    }

    pub fn write_to_file(self, config: &'static SiteConfig) -> Result<()> {
        let xml = self.into_rss_xml(config)?;
        let rss_path = config.build.rss.path.as_path();
        fs::create_dir_all(rss_path.parent().unwrap())?;
        std::fs::write(rss_path, xml)?;

        log!(true; "rss"; "rss feed written successfully");
        Ok(())
    }
}

fn query_meta(post_path: &Path, config: &'static SiteConfig) -> Result<PostMeta> {
    let root = config.get_root();
    let guid = get_guid_from_content_output_path(post_path, config)?;

    // println!("{guid:?}");

    let output = run_command!(&config.build.typst.command;
        "query", "--features", "html", "--format", "json",
        "--font-path", root, "--root", root,
        post_path,
        META_TAG_NAME, "--field", "value", "--one"
    )?;

    let queried_meta = str::from_utf8(output.stdout.as_slice()).unwrap();
    let meta = extract_metadata(guid, queried_meta, config)?;
    // println!("{:?}", queried_meta);

    Ok(meta)
    // todo!()
}

// Helper function used for extracting metadata from typst post
// e.g.:
// -----------------------------------
// author: "John Doe"
// title: "My Post"
// summary: [This post is translated from #link("https://example.com")[original post]]
// date: "2023-01-01"
// -----------------------------------
// The `summary` here is a `content`, which we wanted to eval it into html string but not possible
// `typst query` command will get `{"children":[{"func":"text", "text": "This post is translated from "},{"func":"link","dest":"https://example.com","text":"original post"}]}`
fn extract_metadata(
    guid: String,
    queried_meta: &str,
    config: &'static SiteConfig,
) -> Result<PostMeta> {
    let json: serde_json::Value = serde_json::from_str(queried_meta).with_context(|| {
        format!("Failed to extract post meta. It may be a inner bug: \n {queried_meta}",)
    })?;

    let get_elem_string =
        |json_value: &serde_json::Value, key: &str| json_value.get(key).map(|v| v.to_string());

    let summary = get_elem_string(&json, "summary")
        .context("")
        .and_then(|summary| {
            let summary = parse_element_from_typst_sequence(&summary)?.into_html_tag(config);
            Ok(summary)
        })
        .ok();
    let meta = PostMeta {
        author: Some("email@kawayww.com (柳上川)".into()),
        title: get_elem_string(&json, "title"),
        summary,
        date: get_elem_string(&json, "date"),
        update: get_elem_string(&json, "update"),
        link: Some(guid),
    };

    Ok(meta)
}

fn parse_element_from_typst_sequence(content: &str) -> Result<TypstElement> {
    let parsed_element: TypstElement = serde_json::from_str(content)?;
    Ok(parsed_element)
}

#[test]
fn test_parse_element_from_typst_sequence() {
    let json_str = r#"
    {
        "func": "sequence",
        "children": [
            { "func": "space" },
            { "func": "text", "text": "小鹤双拼是一个简洁, 流畅, 自由的双拼输入法方案" },
            { "func": "space" },
            { "func": "linebreak" },
            { "func": "space" },
            { "func": "link", "dest": "https://example.com", "body": { func: "text", "text": "小鹤双拼" } },
            { "func": "text", "text": "适合想提高打字速度, 但又不想投入巨量精力进行记忆, 追求高性价比的同学" },
            { "func": "space" },
            { "func": "unknown_func" }
        ]
    }
    "#;

    let result = parse_element_from_typst_sequence(json_str).unwrap();
    assert_eq!(
        result,
        TypstElement::Sequence {
            children: vec![
                TypstElement::Space,
                TypstElement::Text {
                    text: "小鹤双拼是一个简洁, 流畅, 自由的双拼输入法方案".to_string()
                },
                TypstElement::Space,
                TypstElement::Linebreak,
                TypstElement::Space,
                TypstElement::Link {
                    dest: "https://example.com".to_string(),
                    body: Box::new(TypstElement::Sequence {
                        children: vec![TypstElement::Text {
                            text: "小鹤双拼".to_string()
                        }]
                    }),
                },
                TypstElement::Space,
                TypstElement::Text {
                    text: "适合想提高打字速度, 但又不想投入巨量精力进行记忆, 追求高性价比的同学"
                        .to_string()
                },
                TypstElement::Space,
                TypstElement::OtherIgnored,
            ]
        }
    );
}

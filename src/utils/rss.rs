#![allow(unused)]

use crate::{
    config::SiteConfig,
    run_command,
    utils::{self, build::collect_files},
};
use anyhow::{Context, Ok, Result};
use chrono::{DateTime, Utc};
use rayon::prelude::*;
use rss::{ChannelBuilder, ItemBuilder};
use serde::{Deserialize, Serialize};
use std::path::Path;

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
    // date: Option<DateTime<Utc>>,
    update: Option<String>,

    #[serde(default)]
    link: Option<String>,
    // author: Option<String>,
}

impl RSSChannel {
    pub fn new(config: &'static SiteConfig) -> Result<Self> {
        let generated_posts_path = config.build.output.join(
            config
                .build
                .content
                .strip_prefix(config.get_root())
                .unwrap(),
        );
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

    pub fn to_rss_xml(&self) -> String {
        let items = self
            .items
            .par_iter()
            .map(|item| {
                ItemBuilder::default()
                    .title(item.title.clone())
                    .link(item.link.clone())
                    .description(item.summary.clone())
                    // .pub_date(item.date.unwrap_or_default().to_rfc2822())
                    // .author(item.author.clone())
                    .build()
            })
            .collect::<Vec<_>>();

        ChannelBuilder::default()
            .title(self.title.clone())
            .link(self.base_url.clone())
            .description(self.description.clone())
            .language(self.language.clone())
            .generator(Some("tola-ssg".to_string()))
            .items(items)
            .build()
            .to_string()
    }

    pub fn write_to_file(&self, path: &Path) -> Result<()> {
        let xml = self.to_rss_xml();
        std::fs::write(path, xml)?;
        Ok(())
    }
}

fn query_meta(post_path: &Path, config: &'static SiteConfig) -> Result<PostMeta> {
    let root = config.get_root();

    // print!("{post_path:?}");

    let output = run_command!(&config.build.typst.command;
        "query", "--features", "html", "--format", "json",
        "--font-path", root, "--root", root,
        post_path,
        META_TAG_NAME, "--field", "value", "--one"
    )?;

    let output = str::from_utf8(output.stdout.as_slice()).unwrap().trim();
    let meta_json: serde_json::Value = serde_json::from_str(output).with_context(|| {
        format!("Failed to extract post meta. It may be a inner bug: \n {output}",)
    })?;
    // print!(" {meta_json:?}\n");
    println!("{}", meta_json["author"]);
    println!("{}", meta_json["title"]);
    println!("{}", meta_json["summary"]);
    println!("{}", meta_json["date"]);

    // Ok(meta_json)
    todo!()
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
fn extract_text_from_typst_content(content: &str) -> String {
    let mut text = String::new();
    let mut in_code_block = false;

    for line in content.lines() {
        if line.starts_with("```") {
            in_code_block = !in_code_block;
        } else if !in_code_block {
            text.push_str(line);
            text.push('\n');
        }
    }

    text.trim().to_string()
}

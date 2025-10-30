// #![allow(unused)]

use crate::{
    config::SiteConfig,
    log, run_command,
    utils::{build::collect_files, slug::slugify_path},
};
use anyhow::{Context, Ok, Result, anyhow, bail};
use rayon::prelude::*;
use regex::Regex;
use rss::{
    ChannelBuilder, GuidBuilder, ItemBuilder,
    // extension::atom::{self, AtomExtension, AtomExtensionBuilder, Link},
    validation::Validate,
};
use serde::{Deserialize, Serialize};
use std::{
    array, fs, path::{Path, PathBuf}, sync::LazyLock
};

// for quering typst metadata
const META_TAG_NAME: &str = "<tola-meta>";

#[derive(Debug)]
pub struct DateTimeUtc {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

#[allow(unused)]
#[rustfmt::skip]
impl DateTimeUtc {
    pub fn new(year: u16, month: u8, day: u8, hour: u8, minute: u8, second: u8) -> Self {
        Self { year, month, day, hour, minute, second }
    }
    
    pub fn validate(&self) -> Result<()> {
        let (year, month, day, hour, minute, second) = (self.year, self.month, self.day, self.hour, self.minute, self.second);
        let is_leap_year = |year: u16| (year.is_multiple_of(4) && year.is_multiple_of(100)) || year.is_multiple_of(400);
        let max_days = match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if is_leap_year(year) => 29,
            2 => 28,
            _ => unreachable!()
        };

        if month == 0 || month > 12 { bail!("month is invalid") }
        if day == 0 || day > max_days{ bail!("day is invalid") }
        if hour > 60 { bail!("hour is invalid") } 
        if minute > 60 { bail!("minute is invalid") } 
        if second > 60 { bail!("second is invalid") } 

        Ok(())
    }

    
    #[rustfmt::skip]
    pub fn to_rfc2822(&self) -> String {
        // Algorithm: Zeller's congruence
        fn calculate_weekday(year: u16, month: u8, day: u8) -> &'static str {
            let (y, m, d) = (year as i32, month as i32, day as i32);
            let (y, m) = if m < 3 { (y - 1, m + 12) } else { (y, m) };

            let weekday = (d + (13*(m+1))/5 + y + y/4 - y/100 + y/400) % 7;
            match weekday {
                0 => "Sat", 1 => "Sun", 2 => "Mon", 3 => "Tue",
                4 => "Wed", 5 => "Thu", 6 => "Fri",
                _ => unreachable!(),
            }
        }

        let month_name = match self.month {
            1 => "Jan", 2 => "Feb", 3 => "Mar", 4 => "Apr",
            5 => "May", 6 => "Jun", 7 => "Jul", 8 => "Aug",
            9 => "Sep", 10 => "Oct", 11 => "Nov", 12 => "Dec",
            _ => unreachable!(),
        };
        
        let weekday = calculate_weekday(self.year, self.month, self.day);
        
        format!("{}, {:02} {} {:04} {:02}:{:02}:{:02} GMT",
            weekday, self.day, month_name, self.year, self.hour, self.minute, self.second
        )
    }
    
    #[rustfmt::skip]
    pub fn to_rfc3339(&self) -> String {
        format!("{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }
    
    #[rustfmt::skip]
    pub fn to_yyyy_mm_dd(&self) -> String {
        format!("{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
    
}

pub struct RSSFeed {
    title: String,
    description: String,
    base_url: String,
    language: String,
    generator: String,
    posts_meta: Vec<PostMeta>,
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

pub fn build_rss(config: &'static SiteConfig) -> Result<()> {
    if config.build.rss.enable {
        let rss_xml = RSSFeed::new(config)?;
        rss_xml.write_to_file(config)?;
    }
    Ok(())
}

impl TypstElement {
    #[rustfmt::skip]
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
                } else { dest };
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
    // let root = config.get_root();
    let content = &config.build.content;
    let base_url = config.base.url.clone().unwrap_or_default();

    // println!("{:?}, {:?}, {:?}, {:?}", root, content, output, content_path);
    let relative_post_path = content_path
        .strip_prefix(content)?
        .to_str()
        .ok_or(anyhow!("Invalid path"))?
        .strip_suffix(".typ")
        .ok_or(anyhow!("Not a .typ file"))
        .with_context(|| format!("building rss: {:?}", content_path))?;

    let guid_path = if content_path.file_name().is_some_and(|p| p == "index.typ") {
        PathBuf::from("index.html")
    } else {
        PathBuf::from(relative_post_path).join("index.html")
    };

    let guid_path = slugify_path(&guid_path, config);
    let guid_path = guid_path.to_str().unwrap();
    let guid_path = urlencoding::encode(guid_path).into_owned();
    let guid_path = guid_path.replace("%2F", "/");
    // println!("{}", guid_path);
    let guid_path = format!("{}/{}", base_url.trim_end_matches("/"), guid_path);

    Ok(guid_path)
}

impl RSSFeed {
    pub fn new(config: &'static SiteConfig) -> Result<Self> {
        log!(true; "rss"; "generating rss feed started");
        let posts_path = collect_files(
            &crate::utils::build::CONTENT_CACHE,
            &config.build.content,
            &|path| path.extension().is_some_and(|ext| ext == "typ"),
        )?;
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
            posts_meta,
        };

        Ok(rss)
    }

    fn into_rss_xml(self, _config: &'static SiteConfig) -> Result<String> {
        let items = self
            .posts_meta
            .into_iter()
            .filter_map(|post_meta| {
                parse_date(post_meta.date).map(|date_rfc2822| {
                    // println!("{}", post_meta.title.clone().unwrap());
                    ItemBuilder::default()
                        .title(post_meta.title.unwrap())
                        .link(post_meta.link.clone())
                        .guid(
                            GuidBuilder::default()
                                .permalink(true)
                                .value(post_meta.link.unwrap())
                                .build(),
                        )
                        .description(post_meta.summary)
                        // .pub_date(item.date.clone())
                        .pub_date(date_rfc2822)
                        .author(post_meta.author)
                        .build()
                })
            })
            .collect::<Vec<_>>();

        // let atom_href = format!(
        //     "{}/{}",
        //     config.base.url.clone().unwrap().trim_end_matches("/"),
        //     config
        //         .build
        //         .rss
        //         .path
        //         .strip_prefix(&config.build.output)?
        //         .to_str()
        //         .unwrap()
        // );
        let channel = ChannelBuilder::default()
            // .atom_ext(
            //     AtomExtensionBuilder::default()
            //         .link(Link {
            //             href: atom_href,
            //             rel: "self".to_string(),
            //             mime_type: Some("application/atom+xml".to_string()),
            //             ..Default::default()
            //         })
            //         .build(),
            // )
            .title(self.title)
            .link(self.base_url)
            .description(self.description)
            .language(self.language)
            .generator(self.generator)
            .items(items)
            .build();
        channel.validate().map_err(|e| anyhow::anyhow!(format!("rss validate: {e}")))?;
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

#[allow(non_camel_case_types)]
#[rustfmt::skip]
fn parse_date(date: Option<String>) -> Option<String> {
    type RE_DATE = LazyLock<Regex>;

    static RE_YYYY_MM_DD: RE_DATE = LazyLock::new(|| Regex::new(r"(?x)(?P<year>\d{4})-(?P<month>\d{2})-(?P<day>\d{2})").unwrap());
    static RE_RFC3339: RE_DATE = LazyLock::new(|| {Regex::new(r"^(?P<year>\d{4})-(?P<month>\d{2})-(?P<day>\d{2})T(?P<hour>\d{2}):(?P<minute>\d{2}):(?P<second>\d{2})Z$").unwrap()});
    let date = date?;

    type FORMAT_YYYY_MM_DD = (u16, u8, u8);
    type FORMAT_RFC3339 = (u16, u8, u8, u8, u8, u8);

    #[allow(unused)]
    enum Choice<A, B, C>{
        A(A),
        B(B),
        C(C)
    }


    #[allow(unused)]
    #[rustfmt::skip]
    impl<A, B, C> Choice<A, B, C> {
        fn get_a(self) -> A { if let Choice::A(a) = self { a } else { unreachable!() } }
        fn get_b(self) -> B { if let Choice::B(b) = self { b } else { unreachable!() } }
        fn get_c(self) -> C { if let Choice::C(c) = self { c } else { unreachable!() } }
        fn a(a: A) -> Self {Choice::A(a)}
        fn b(b: B) -> Self {Choice::B(b)}
        fn c(c: C) -> Self {Choice::C(c)}
    }

    #[rustfmt::skip]
    fn extract_date_from_re(date: &str, format: &'static str) -> Option<Choice<FORMAT_YYYY_MM_DD, FORMAT_RFC3339, ()>> {
        let keys = ["year", "month", "day", "hour", "minute", "second"];
        match format.to_uppercase().as_str() {
            "YYYY-MM-DD" => {if let Some(caps) = RE_YYYY_MM_DD.captures(date) {
                let year: u16 = caps[keys[0]].parse().unwrap();
                let [month, day] = array::from_fn(|i| caps[keys[i + 1]].parse::<u8>().unwrap());
                Some(Choice::a((year, month, day)))
            } else { None }},
            "RFC3339" => {if let Some(caps) = RE_RFC3339.captures(date) {
                let year: u16 = caps[keys[0]].parse().unwrap();
                let [month, day, hour, minute, second] = array::from_fn(|i| caps[keys[i + 1]].parse::<u8>().unwrap());
                Some(Choice::b((year, month, day, hour, minute, second)))
            } else { None }},
            _ => unreachable!()
        }        
    }

    let date = if let Some(date) = extract_date_from_re(&date, "YYYY-MM-DD") {
        let (year, month, day) = date.get_a();
        DateTimeUtc::new(year, month, day, 0, 0,0)
    } else if let Some(date) = extract_date_from_re(&date, "RFC3339") {
        let (year, month, day, hour, minute, second) = date.get_b();
        DateTimeUtc::new(year, month, day, hour, minute, second)
    } else {
        // log!("date"; "Date format is not YYYY-MM-DD or RFC3339");
        return None
    };
    if let Result::Err(e) = date.validate() {
        log!("date"; "{e}");
        return None
    };
    Some(date.to_rfc2822())

}

#[rustfmt::skip]
fn query_meta(post_path: &Path, config: &'static SiteConfig) -> Result<PostMeta> {
    let root = config.get_root();
    let guid = get_guid_from_content_output_path(post_path, config)?;

    // println!("{guid:?}");

    let output = run_command!(&config.build.typst.command;
        "query", "--features", "html", "--format", "json",
        "--font-path", root, "--root", root,
        post_path,
        META_TAG_NAME, "--field", "value", "--one"
    )
    .with_context(|| {
        format!("Failed to query metadata for rss in post path: {}\nMake sure your tag name is correct(\"{}\")",
            post_path.display(), META_TAG_NAME
        )
    })?;

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

    let get_elem = |json: &serde_json::Value, key: &str| json.get(key).map(|v| v.as_str().unwrap_or_default().to_string());

    let summary = get_elem(&json, "summary")
        .context("Failed to get summary metadata")
        .and_then(|summary| {
            let summary = parse_element_from_typst_sequence(&summary)?.into_html_tag(config);
            Ok(summary)
        })
        .ok();
    let author = get_elem(&json, "author");
    let author = correct_rss_author(author.as_ref(), config);

    let meta = PostMeta {
        summary,
        author,
        title: get_elem(&json, "title"),
        date: get_elem(&json, "date"),
        update: get_elem(&json, "update"),
        link: Some(guid),
    };

    Ok(meta)
}

// Example for valid author(for rss): "bob@xxx.com (Bob)"
// Priority for looking up author and email:
// 1. `author` in user's post meta (`<tola-ssg-meta>`)
// 2. `author` in user's site config (`tola.toml`)
// 3. Try to combine `author` and `email` (`tola.toml`)
fn correct_rss_author(author: Option<&String>, config: &'static SiteConfig) -> Option<String> {
    static RE_VALID_AUTHOR: LazyLock<Regex> = LazyLock::new(||
        Regex::new(r"^[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\s*\([^)]+\)$").unwrap()
    );

    let author = author?;
    let author = match RE_VALID_AUTHOR.is_match(author) {
        true => author.to_owned(),
        false => config.base.author.clone(),
    };
    let author = match RE_VALID_AUTHOR.is_match(&author) {
        true => author.to_owned(),
        false => format!("{} ({})", config.base.email.clone(), author),
    };
    Some(author)    
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

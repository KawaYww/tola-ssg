use std::{ffi::OsString, fs, io::Cursor, path::{Path, PathBuf}, sync::OnceLock};
use anyhow::{anyhow, Context, Result};
use quick_xml::{events::{attributes::Attribute, BytesEnd, BytesStart, BytesText, Event}, Reader, Writer};
use crate::{config::SiteConfig, log, run_command, utils::slug::{slugify_fragment, slugify_path}};
use crate::utils::watch::wait_until_stable;
use rayon::prelude::*;

static ASSET_TOP_LEVELS: OnceLock<Vec<OsString>> = OnceLock::new();

pub fn _copy_dir_recursively(src: &Path, dst: &Path) -> Result<()> {
    if !dst.exists() {
        fs::create_dir_all(dst).context("[Utils] Failed to create destination directory")?;
    }

    for entry in fs::read_dir(src).context("[Utils] Failed to read source directory")? {
        let entry = entry.context("[Utils] Invalid directory entry")?;
        let entry_path = entry.path();
        let dest_path = dst.join(entry.file_name());

        if entry_path.is_dir() {
            _copy_dir_recursively(&entry_path, &dest_path)?;
        } else {
            fs::copy(&entry_path, &dest_path).with_context(|| {
                format!("[Utils] Failed to copy {entry_path:?} to {dest_path:?}")
            })?;
            log!("assets", "{}", dest_path.display());
        }
    }

    Ok(())
}

fn collect_files<P>(dir: &Path, p: &P) -> Result<Vec<PathBuf>>
where
    P: Fn(&PathBuf) -> bool,
{
    let mut files = Vec::new();

    for entry in fs::read_dir(dir)?.flatten() {
        let path = entry.path();
        if path.is_dir() {
            files.extend(collect_files(&path, p)?);
        } else if path.is_file() && p(&path) {
            files.push(path);
        }
    }

    Ok(files)
}

pub fn process_files<P, F>(dir: &Path, config: &SiteConfig, p: &P, f: &F) -> Result<()>
where
    P: Fn(&PathBuf) -> bool + Sync,
    F: Fn(&Path, &SiteConfig) -> Result<()> + Sync,
{
    let files = collect_files(dir, p)?;

    files.par_iter()
        .try_for_each(|path| f(path, config))
}

pub fn process_post(post_path: &Path, config: &SiteConfig) -> Result<()> {
    let root = config.get_root();   
    let content = &config.build.content;
    let output = &config.build.output.join(&config.build.base_path);

    // println!("{:?}, {:?}, {:?}, {:?}", root, content, output, post_path);
    let relative_post_path = post_path
        .strip_prefix(content).context("AAA")?
        .to_str()
        .ok_or(anyhow!("Invalid path"))?
        .strip_suffix(".typ")
        .ok_or(anyhow!("Not a .typ file"))?;

    let output = output.join(relative_post_path);
    fs::create_dir_all(&output)?;

    let html_path_1 = if post_path.file_name().is_some_and(|p| p == "home.typ") {
        config.build.output.join("index.html")
    } else {
        output.join("index.html")
    };
    let html_path = slugify_path(&html_path_1, config);
    if html_path != html_path_1 {
        println!("{:?}", html_path_1);
        println!("{:?}", html_path);
    }

    let output = run_command!(&config.build.typst.command;
        "compile", "--features", "html", "--format", "html",
        "--font-path", root, "--root", root,
        post_path, "-"
    )?;

    let html_content = output.stdout;
    let html_content = process_html(&html_content, config);
    
    let html_content = if config.build.minify {
        minify_html::minify(html_content.as_slice(), &minify_html::Cfg::new())
    } else {
        html_content
    };

    fs::write(&html_path, html_content)?;

    log!("content", "{}", relative_post_path);

    Ok(())
}

pub fn process_asset(asset_path: &Path, config: &SiteConfig, should_wait_until_stable: bool) -> Result<()> {   
    let assets = &config.build.assets;
    let output = &config.build.output.join(&config.build.base_path);

    let asset_extension = asset_path.extension().unwrap_or_default().to_str().unwrap_or_default();
    let relative_asset_path = asset_path
        .strip_prefix(assets)?
        .to_str()
        .ok_or(anyhow!("Invalid path"))?;

    let output_path = output.join(relative_asset_path);

    if let Some(parent) = output_path.parent() {
        fs::create_dir_all(parent)?;
    }

    if output_path.exists() {
        fs::remove_file(&output_path)?;
    }

    if should_wait_until_stable {
        wait_until_stable(asset_path, 5)?;
    }

    match asset_extension {
        "css" if config.build.tailwind.enable => {
            let input = config.build.tailwind.input.as_ref().unwrap();
            let input = input.canonicalize().unwrap();
            let asset_path = asset_path.canonicalize().unwrap();
            if input == asset_path {
                let output_path = output.canonicalize().unwrap().join(relative_asset_path);
                run_command!(config.get_root(); &config.build.tailwind.command;
                    "-i", input, "-o", output_path, if config.build.minify { "--minify" } else { "" }
                )?;
            } else {
                fs::copy(asset_path, &output_path)?;
            }
        },
        _ => {
            fs::copy(asset_path, &output_path)?;
        },
    }

    log!("assets", "{}", relative_asset_path);

    Ok(())
}

#[rustfmt::skip]
fn process_html(content: &[u8], config: &SiteConfig) -> Vec<u8> {
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    let mut reader = Reader::from_reader(content);
    reader.config_mut().trim_text(false);
    reader.config_mut().enable_all_checks(false);

    loop { match reader.read_event() {
        Ok(Event::Start(elem)) => match elem.name().as_ref() {
            b"html" => {
                let mut elem = elem.into_owned();
                elem.push_attribute(("lang", config.base.language.as_str()));
                assert!(writer.write_event(Event::Start(elem)).is_ok());
            },
            b"h1" | b"h2" | b"h3" | b"h4" | b"h5" | b"h6" => {
                let attrs: Vec<Attribute> = elem.attributes().filter_map(|s| s.ok()).map(|attr| {
                    let key = attr.key;
                    let value = attr.value;
                    if key.as_ref() == b"id" {
                        let value = str::from_utf8(value.as_ref()).unwrap();
                        let value = slugify_fragment(value, config);
                        let value = value.as_bytes().to_vec().into();
                        Attribute { key, value }
                    } else {
                        Attribute { key, value }
                    }
                }).collect();
                let elem = elem.to_owned().with_attributes(attrs);
                assert!(writer.write_event(Event::Start(elem)).is_ok());
            },
            _ => {
                let attrs: Vec<Attribute> = elem.attributes().par_bridge().filter_map(|s| s.ok()).map(|attr| {
                    let key = attr.key;
                    let value = attr.value;
                    if key.as_ref() == b"href" || key.as_ref() == b"src" {
                        let value = {
                            let value = str::from_utf8(value.as_ref()).unwrap();
                            let value = match &value[0..=0] {
                                "/" => {
                                    let base_path = PathBuf::from("/").join(config.build.base_path.as_path());
                                    if is_asset_link(value, config) {
                                        base_path.join(value)
                                    } else {
                                        let (path, fragment) = value.split_once('#').unwrap_or((value, ""));
                                        let mut path = {
                                            let path = slugify_path(path, config);
                                            path.into_os_string()
                                        };
                                        let fragment = if !fragment.is_empty() {
                                            &(String::from("#") + &slugify_fragment(fragment, config))
                                        } else {
                                            ""
                                        };
                                        path.push(fragment);
                                        base_path.join(path)
                                    }
                                },
                                "#" => {
                                    let fragment = &value[1..];
                                    let fragment = String::from("#") + &slugify_fragment(fragment, config);
                                    PathBuf::from(fragment)
                                },
                                _ => if is_external_link(value) { // TODO, or not?
                                    PathBuf::from(value)
                                } else {
                                    PathBuf::from(value)
                                }
                            };
                            let value = value.to_str().unwrap();
                            value.as_bytes().to_vec()
                        }.into();
                        Attribute { key, value }
                    } else {
                        Attribute { key, value }
                    }
                }).collect();

                let elem = elem.to_owned().with_attributes(attrs);
                assert!(writer.write_event(Event::Start(elem)).is_ok());
            }
        },
        Ok(Event::End(elem)) => match elem.name().as_ref() {
            b"head" => {
                let title = config.base.title.as_str();
                let description = config.base.description.as_str();

                if !title.is_empty() {
                    assert!(writer.write_event(Event::Start(BytesStart::new("title"))).is_ok());
                    assert!(writer.write_event(Event::Text(BytesText::new(title))).is_ok());
                    assert!(writer.write_event(Event::End(BytesEnd::new("title"))).is_ok());
                }
                
                if !description.is_empty() {
                    let mut elem = BytesStart::new("meta");
                    elem.push_attribute(("name", "description"));
                    elem.push_attribute(("content", description));
                    assert!(writer.write_event(Event::Start(elem)).is_ok());
                    assert!(writer.write_event(Event::End(BytesEnd::new("meta"))).is_ok());
                }

                if config.build.tailwind.enable && let Some(input) = &config.build.tailwind.input {
                    let input = {
                        let base_path = &config.build.base_path;
                        let assets = config.build.assets.as_path().canonicalize().unwrap();
                        let input = input.canonicalize().unwrap();
                        let input = input.strip_prefix(assets).unwrap();
                        // println!("{assets:?}, {input:?}");
                        let input = base_path.join(input);
                        PathBuf::from("/").join(input)
                    };
                    let input = input.to_string_lossy();
                    let mut elem = BytesStart::new("link");
                    elem.push_attribute(("rel", "stylesheet"));
                    elem.push_attribute(("href", input));
                    assert!(writer.write_event(Event::Start(elem)).is_ok());
                }

                assert!(writer.write_event(Event::End(BytesEnd::new("head"))).is_ok());
            },
            _ => assert!(writer.write_event(Event::End(elem)).is_ok()),
        },
        Ok(Event::Eof) => break,
        Ok(elem) => assert!(writer.write_event(elem).is_ok()),
        Err(elem) => panic!("Error at position {}: {:?}", reader.error_position(), elem),
    }}

    // let a = writer.into_inner().into_inner();
    // println!("{}", String::from_utf8_lossy(&a));
    // a
    writer.into_inner().into_inner()
}

fn get_asset_top_levels(assets_dir: &Path) -> &'static [OsString] {
    ASSET_TOP_LEVELS.get_or_init(|| {
        fs::read_dir(assets_dir)
            .map(|dir|
                dir.filter_map(|e| e.ok())
                    .map(|entry| entry.file_name())
                    .collect()
            )
            .unwrap_or_default()
    })
}

fn is_asset_link(path: impl AsRef<Path>, config: &SiteConfig) -> bool {
    let path = path.as_ref();
    let asset_top_levels = get_asset_top_levels(&config.build.assets);

    // println!("{:?}, {:?}", path, asset_top_levels);
    match path.components().nth(1) {
        Some(std::path::Component::Normal(first)) => {
            asset_top_levels.par_iter().any(|name| name == first)
        }
        _ => false,
    }
}

fn is_external_link(link: &str) -> bool {
    match link.find(':') {
        Some(colon_pos) => {
            let scheme = &link[..colon_pos];
            // scheme must be ASCII letters + digits + `+` / `-` / `.`
            // and must not contain `/` before the colon
            scheme.chars().all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.')
        }
        None => false,
    }
}

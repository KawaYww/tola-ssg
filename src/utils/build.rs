use std::{fs, io::Cursor, path::{Path, PathBuf}};
use anyhow::{anyhow, Context, Result};
use quick_xml::{events::{attributes::Attribute, BytesEnd, BytesStart, BytesText, Event}, Reader, Writer};
use crate::{config::SiteConfig, log, run_command};
use crate::utils::watch::wait_until_stable;
use rayon::prelude::*;

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

pub fn process_files<P, F>(dir: &Path, config: &SiteConfig, p: &P, f: &F) -> Result<()>
where
    P: Fn(&PathBuf) -> bool + Sync,
    F: Fn(&Path, &SiteConfig) -> Result<()> + Sync,
{   
    fs::read_dir(dir)?
        .collect::<Vec<_>>()
        .par_iter()
        .flatten()
        .map(|entry| entry.path())
        .try_for_each(|path| {
            if path.is_dir() {
                process_files(&path, config, p, f)
            } else if path.is_file() && p(&path) {
                f(&path, config)
            } else {
                Ok(())
            }
        })
}

pub fn process_post(post_path: &Path, config: &SiteConfig) -> Result<()> {
    let root = config.get_root();   
    let content = &config.build.content;
    let output = &config.build.output.join(&config.base.path);

    // println!("{:?}, {:?}, {:?}, {:?}", root, content, output, post_path);
    let relative_post_path = post_path
        .strip_prefix(content).context("AAA")?
        .to_str()
        .ok_or(anyhow!("Invalid path"))?
        .strip_suffix(".typ")
        .ok_or(anyhow!("Not a .typ file"))?;

    let output = output.join(relative_post_path);
    fs::create_dir_all(&output)?;

    let html_path = if post_path.file_name().is_some_and(|p| p == "home.typ") {
        config.build.output.join("index.html")
    } else {
        output.join("index.html")
    };

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

#[rustfmt::skip]
fn process_html(content: &[u8], config: &SiteConfig) -> Vec<u8> {
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    let mut reader = Reader::from_reader(content);
    reader.config_mut().trim_text(true);
    reader.config_mut().enable_all_checks(false);

    loop { match reader.read_event() {
        Ok(Event::Start(e)) => match e.name().as_ref() {
            b"html" => {
                let mut elem = BytesStart::new("html");
                elem.push_attribute(("lang", config.base.language.as_str()));
                assert!(writer.write_event(Event::Start(elem)).is_ok());
            },
            _ => {
                let attrs: Vec<Attribute> = e.attributes().filter_map(|s| s.ok()).map(|attr| {
                    let key = attr.key;
                    let value = attr.value;
                    if key.as_ref() == b"href" || key.as_ref() == b"src" {
                        let value = str::from_utf8(value.as_ref()).unwrap();
                        let value = if value.starts_with("/") {
                            let base_path = PathBuf::from("/").join(config.base.path.as_path());
                            let value = value.strip_prefix("/").unwrap();
                            // println!("{:?} {:?}", base_path, value);
                            base_path.join(value)
                        } else {
                            PathBuf::from(value)
                        };
                        let value = value.to_str().unwrap().as_bytes().to_vec();
                        Attribute { key, value: value.into() }
                    } else {
                        Attribute { key, value }
                    }
                }).collect();

                let mut e = e.to_owned();
                e.clear_attributes();
                e.extend_attributes(attrs);
                assert!(writer.write_event(Event::Start(e)).is_ok());
            }
        },
        Ok(Event::End(e)) => match e.name().as_ref() {
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
                        let base_path = &config.base.path;
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
            _ => assert!(writer.write_event(Event::End(e)).is_ok()),
        },
        Ok(Event::Eof) => break,
        Ok(e) => assert!(writer.write_event(e).is_ok()),
        Err(e) => panic!("Error at position {}: {:?}", reader.error_position(), e),
    }}

    // let a = writer.into_inner().into_inner();
    // println!("{}", String::from_utf8_lossy(&a));
    // a
    writer.into_inner().into_inner()
}

pub fn process_asset(asset_path: &Path, config: &SiteConfig, should_wait_until_stable: bool) -> Result<()> {   
    let assets = &config.build.assets;
    let output = &config.build.output.join(&config.base.path);

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

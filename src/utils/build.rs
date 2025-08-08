use crate::utils::watch::wait_until_stable;
use crate::{
    config::{ExtractSvgType, SiteConfig},
    log, run_command, run_command_with_stdin,
    utils::slug::{slugify_fragment, slugify_path},
};
use anyhow::{Context, Result, anyhow};
use quick_xml::{
    Reader, Writer,
    events::{BytesEnd, BytesStart, BytesText, Event, attributes::Attribute},
};
use rayon::prelude::*;
use std::{
    ffi::OsString,
    fs,
    io::{Cursor, Write},
    path::{Path, PathBuf},
    sync::OnceLock,
    thread::JoinHandle,
};

const PADDING_TOP: f32 = 5.0;
const PADDING_BOTTOM: f32 = 4.0;

struct Svg {
    data: Vec<u8>,
    size: (f32, f32),
}

impl Svg {
    pub fn new(data: Vec<u8>, width: f32, height: f32) -> Self {
        Self {
            data,
            size: (width, height),
        }
    }
}

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
            log!("assets"; "{}", dest_path.display());
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

pub fn process_files<P, F>(dir: &Path, config: &'static SiteConfig, p: &P, f: &F) -> Result<()>
where
    P: Fn(&PathBuf) -> bool + Sync,
    F: Fn(&Path, &'static SiteConfig) -> Result<Option<JoinHandle<()>>> + Sync,
{
    let files = collect_files(dir, p)?;

    let handles: Vec<_> = files
        .par_iter()
        .map(|path| f(path, config))
        .collect::<Result<_>>()?;

    for handle in handles.into_iter().flatten() {
        handle.join().ok();
    }

    Ok(())
}

pub fn process_content(
    content_path: &Path,
    config: &'static SiteConfig,
    should_log_newline: bool,
) -> Result<Option<JoinHandle<()>>> {
    let root = config.get_root();
    let content = &config.build.content;
    let output = &config.build.output.join(&config.build.base_path);

    let is_relative_asset = content_path.extension().is_some_and(|ext| ext != "typ");

    if is_relative_asset {
        let relative_asset_path = content_path
            .strip_prefix(content)?
            .to_str()
            .ok_or(anyhow!("Invalid path"))?;

        log!(should_log_newline; "content"; "{}", relative_asset_path);

        let output = output.join(relative_asset_path);
        fs::create_dir_all(output.parent().unwrap()).unwrap();
        fs::copy(content_path, output)?;

        return Ok(None);
    }

    // println!("{:?}, {:?}, {:?}, {:?}", root, content, output, content_path);
    let relative_post_path = content_path
        .strip_prefix(content)?
        .to_str()
        .ok_or(anyhow!("Invalid path"))?
        .strip_suffix(".typ")
        .ok_or(anyhow!("Not a .typ file"))?;

    log!(should_log_newline; "content"; "{}", relative_post_path);

    let output = output.join(relative_post_path);
    fs::create_dir_all(&output).unwrap();

    let html_path = if content_path.file_name().is_some_and(|p| p == "index.typ") {
        config.build.output.join("index.html")
    } else {
        output.join("index.html")
    };
    let html_path = slugify_path(&html_path, config);

    let output = run_command!(&config.build.typst.command;
        "compile", "--features", "html", "--format", "html",
        "--font-path", root, "--root", root,
        content_path, "-"
    )?;

    let html_content = output.stdout;
    // println!("{}", str::from_utf8(&html_content).unwrap());
    let (handle, html_content) = process_html(&html_path, &html_content, config);

    let html_content = if config.build.minify {
        minify_html::minify(html_content.as_slice(), &minify_html::Cfg::new())
    } else {
        html_content
    };

    fs::write(&html_path, html_content)?;
    Ok(Some(handle))
}

pub fn process_asset(
    asset_path: &Path,
    config: &'static SiteConfig,
    should_wait_until_stable: bool,
    should_log_newline: bool,
) -> Result<Option<JoinHandle<()>>> {
    let assets = &config.build.assets;
    let output = &config.build.output.join(&config.build.base_path);

    let asset_extension = asset_path
        .extension()
        .unwrap_or_default()
        .to_str()
        .unwrap_or_default();
    let relative_asset_path = asset_path
        .strip_prefix(assets)?
        .to_str()
        .ok_or(anyhow!("Invalid path"))?;

    log!(should_log_newline; "assets"; "{}", relative_asset_path);

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
            match input == asset_path {
                true => {
                    let output_path = output.canonicalize().unwrap().join(relative_asset_path);
                    run_command!(config.get_root(); &config.build.tailwind.command;
                        "-i", input, "-o", output_path, if config.build.minify { "--minify" } else { "" }
                    )?;
                }
                false => {
                    fs::copy(asset_path, &output_path)?;
                }
            }
        }
        _ => {
            fs::copy(asset_path, &output_path)?;
        }
    }

    Ok(None)
}

#[rustfmt::skip]
fn process_html(html_path: &Path, content: &[u8], config: &'static SiteConfig) -> (JoinHandle<()>, Vec<u8>) {
    let mut svg_cnt = 0;
    let mut writer = Writer::new(Cursor::new(Vec::new()));
    let mut reader = {
        let mut reader = Reader::from_reader(content);
        reader.config_mut().trim_text(false);
        reader.config_mut().enable_all_checks(false);
        reader
    };

    let mut svgs = vec![];

    loop { match reader.read_event() {
        Ok(Event::Start(elem)) => match elem.name().as_ref() {
            b"html" => {
                let mut elem = elem.into_owned();
                elem.push_attribute(("lang", config.base.language.as_str()));
                writer.write_event(Event::Start(elem)).unwrap();
            },
            b"h1" | b"h2" | b"h3" | b"h4" | b"h5" | b"h6" => {
                let attrs: Vec<Attribute> = elem.attributes().flatten().map(|attr| {
                    let key = attr.key;
                    let value = attr.value;
                    let value = if key.as_ref() == b"id" {
                        let value = str::from_utf8(value.as_ref()).unwrap();
                        let value = slugify_fragment(value, config);
                        value.as_bytes().to_vec().into()
                    } else {
                        value
                    };
                    Attribute { key, value }
                }).collect();
                let elem = elem.to_owned().with_attributes(attrs);
                writer.write_event(Event::Start(elem)).unwrap();
            },
            b"svg" => {
                let svg = process_svg_in_html(html_path, &mut svg_cnt, &mut reader, &mut writer, elem, config);
                svgs.push(svg);
            },
            _ => process_link_in_html(&mut writer, elem, config),
        },
        Ok(Event::End(elem)) => match elem.name().as_ref() {
            b"head" => process_head_in_html(&mut writer, config),
            _ => writer.write_event(Event::End(elem)).unwrap(),
        },
        Ok(Event::Eof) => break,
        Ok(elem) => writer.write_event(elem).unwrap(),
        Err(elem) => panic!("Error at position {}: {:?}", reader.error_position(), elem),
    }}

    let handle = std::thread::spawn({
        let html_path = html_path.to_path_buf();
        let svgs = svgs.into_iter().flatten().collect();
        move || compress_svgs(svgs, &html_path, config)
    });

    (handle, writer.into_inner().into_inner())
}

fn process_svg_in_html(
    html_path: &Path,
    cnt: &mut i32,
    reader: &mut Reader<&[u8]>,
    writer: &mut Writer<Cursor<Vec<u8>>>,
    elem: BytesStart<'_>,
    config: &'static SiteConfig,
) -> Option<Svg> {
    if let ExtractSvgType::Embedded = config.build.typst.svg.extract_type {
        writer.write_event(Event::Start(elem)).unwrap();
        return None;
    }

    let attrs: Vec<_> = elem
        .attributes()
        .flatten()
        .map(|attr| {
            let key = attr.key.as_ref();
            let value = attr.value.as_ref();
            match key {
                // b"width" | b"height" => None,
                b"height" => {
                    let height = str::from_utf8(attr.value.as_ref())
                        .unwrap()
                        .trim_end_matches("pt");
                    let height = height.parse::<f32>().unwrap();
                    let height = format!("{}pt", height + PADDING_TOP);
                    let height = height.as_bytes().to_vec().into();
                    Attribute {
                        key: attr.key,
                        value: height,
                    }
                }
                b"viewBox" => {
                    let viewbox_inner: Vec<_> = str::from_utf8(value)
                        .unwrap()
                        .split_whitespace()
                        .map(|x| x.parse::<f32>().unwrap())
                        .collect();
                    let viewbox = format!(
                        "{} {} {} {}",
                        viewbox_inner[0],
                        viewbox_inner[1] - PADDING_TOP,
                        viewbox_inner[2],
                        viewbox_inner[3] + PADDING_BOTTOM + PADDING_TOP
                    );
                    Attribute {
                        key: attr.key,
                        value: viewbox.as_bytes().to_vec().into(),
                    }
                }
                _ => attr,
            }
        })
        .collect();

    let mut svg_writer = Writer::new(Cursor::new(Vec::new()));
    svg_writer
        .write_event(Event::Start(BytesStart::new("svg").with_attributes(attrs)))
        .unwrap();
    while let Ok(event) = reader.read_event() {
        let should_break = matches!(&event, Event::End(e) if e.name().as_ref() == b"svg");
        svg_writer.write_event(event).unwrap();

        if should_break {
            break;
        }
    }
    let svg_data = svg_writer.into_inner().into_inner();

    let inline_max_size = config.get_inline_max_size();
    // println!("{} {cnt} {} {}", html_path.display(), svg_data.len(), inline_max_size);
    let svg_filename = match (&config.build.typst.svg.extract_type, svg_data.len()) {
        (ExtractSvgType::JustSvg, _) => format!("svg-{cnt}.svg"),
        (_, size) if size < inline_max_size => format!("svg-{cnt}.svg"),
        _ => format!("svg-{cnt}.avif"),
    };
    let svg_path = html_path.parent().unwrap().join(svg_filename.as_str());
    *cnt += 1;

    let dpi = config.build.typst.svg.dpi;
    let opt = usvg::Options {
        dpi,
        ..Default::default()
    };
    let usvg_tree = usvg::Tree::from_data(&svg_data, &opt).unwrap();
    let write_opt = usvg::WriteOptions {
        indent: usvg::Indent::None,
        ..Default::default()
    };
    let usvg = usvg_tree.to_string(&write_opt);

    let (width, height) = extract_svg_size(&usvg).unwrap();
    let img_elem = {
        let svg_path = svg_path.strip_prefix(&config.build.output).unwrap();
        let svg_path = PathBuf::from("/").join(svg_path);
        let svg_path = svg_path.to_str().unwrap();
        let scale = config.get_scale();
        let attrs = [
            ("src", svg_path),
            (
                "style",
                &format!("width:{}px;height:{}px;", (width / scale), (height / scale)),
            ),
            // ("style", &format!("width:{}pt;height:{}pt", width, (height + PADDING_BOTTOM + PADDING_TOP)))
        ];
        BytesStart::new("img").with_attributes(attrs)
    };
    writer.write_event(Event::Start(img_elem)).unwrap();

    Some(Svg::new(usvg.into_bytes(), width, height))
}

fn extract_svg_size(svg_data: &str) -> Option<(f32, f32)> {
    let width_start = svg_data.find("width=\"")? + "width=\"".len();
    let width_end = svg_data[width_start..].find('"')? + width_start;
    let width_str = &svg_data[width_start..width_end];

    let height_start = svg_data[width_end..].find("height=\"")? + width_end + "height=\"".len();
    let height_end = svg_data[height_start..].find('"')? + height_start;
    let height_str = &svg_data[height_start..height_end];

    let width = width_str.parse::<f32>().unwrap();
    let height = height_str.parse::<f32>().unwrap();

    Some((width, height))
}

// FUCK the size of generated `.avif` is so big, FUCKING pure rust avif library
fn compress_svgs(svgs: Vec<Svg>, html_path: &Path, config: &'static SiteConfig) {
    let scale = config.get_scale();
    // let opt = usvg::Options::default();
    let parent = html_path.parent().unwrap();
    let inline_max_size = config.get_inline_max_size();

    svgs.par_iter().enumerate().for_each(move |(cnt, svg)| {
        let relative_path = html_path.strip_prefix(&config.build.output).unwrap().to_string_lossy();
        let relative_path = relative_path.trim_end_matches("index.html");
        log!("svg"; "in {relative_path}: compress svg-{cnt}");

        let svg_data = svg.data.as_slice();

        let svg_filename = match (&config.build.typst.svg.extract_type, svg_data.len()) {
            (ExtractSvgType::JustSvg, _) => format!("svg-{cnt}.svg"),
            (_, size) if size < inline_max_size => format!("svg-{cnt}.svg"),
            _ => format!("svg-{cnt}.avif"),
        };
        let svg_path = parent.join(svg_filename.as_str());

        let extract_type = match &config.build.typst.svg.extract_type {
            ExtractSvgType::Embedded => return,
            ExtractSvgType::Builtin | ExtractSvgType::Magick | ExtractSvgType::Ffmpeg if svg_data.len() < inline_max_size => ExtractSvgType::JustSvg,
            e => e.clone(),
        };
        match extract_type {
            ExtractSvgType::Embedded => unreachable!(),
            ExtractSvgType::Magick => {
                let mut child_stdin = run_command_with_stdin!(["magick"];
                    "-background", "none", "-density", (scale * 96.).to_string(), "-", &svg_path
                ).unwrap();
                child_stdin.write_all(svg_data).unwrap();
            },
            ExtractSvgType::Ffmpeg => {
                let mut child_stdin = run_command_with_stdin!(["ffmpeg"];
                    "-f", "svg_pipe", "-frame_size", "1000000000", "-i", "pipe:",
                    "-filter_complex", "[0:v]split[color][alpha];[alpha]alphaextract[alpha];[color]format=yuv420p[color]",
                    "-map", "[color]",
                    "-c:v:0", "libsvtav1", "-pix_fmt", "yuv420p",
                    "-svtav1-params", "preset=4:still-picture=1",
                    "-map", "[alpha]",
                    "-c:v:1", "libaom-av1", "-pix_fmt", "gray",
                    "-still-picture", "1",
                    "-strict", "experimental",
                    "-c:v", "libaom-av1",
                    "-y", &svg_path
                ).unwrap();
                child_stdin.write_all(svg_data).unwrap();
            },
            ExtractSvgType::JustSvg => {
                fs::write(&svg_path, svg_data).unwrap();
            },
            ExtractSvgType::Builtin => {
                let size = svg.size;
                let (width, height) = (size.0 * scale, size.1 * scale);

                let pixmap: Vec<_> = svg_data.to_vec()
                    .into_par_iter()
                    .chunks(4)
                    .map(|chunk| ravif::RGBA8::new(chunk[0], chunk[1], chunk[2], chunk[3]))
                    .collect();

                let img = ravif::Encoder::new()
                    .with_quality(90.)
                    .with_speed(4)
                    .encode_rgba(ravif::Img::new(&pixmap, width as usize, height as usize))
                    .unwrap();

                fs::write(&svg_path, img.avif_file).unwrap();
            }
        }
        log!("svg"; "in {relative_path}: finish compressing svg-{cnt}");
    });
}

fn process_link_in_html(
    writer: &mut Writer<Cursor<Vec<u8>>>,
    elem: BytesStart<'_>,
    config: &'static SiteConfig,
) {
    let attrs: Vec<Attribute> = elem
        .attributes()
        .par_bridge()
        .flatten()
        .map(|attr| {
            let key = attr.key;
            let value = attr.value;
            if key.as_ref() == b"href" || key.as_ref() == b"src" {
                let value = {
                    let value = str::from_utf8(value.as_ref())
                        .unwrap_or_else(|_| panic!("The Link is empty"));
                    let value = match &value[0..=0] {
                        "/" => {
                            let base_path =
                                PathBuf::from("/").join(config.build.base_path.as_path());
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
                        }
                        "#" => {
                            // It is fragment
                            let fragment = &value[1..];
                            let fragment = String::from("#") + &slugify_fragment(fragment, config);
                            PathBuf::from(fragment)
                        }
                        _ => {
                            if is_external_link(value) {
                                // don't modify external link
                                PathBuf::from(value)
                            } else {
                                // inner and relative link
                                // e.g. "./bbb.png" -> "../bbb.png"
                                // because the post url: "aaa.typ" -> "aaa/index.html"
                                PathBuf::from("../").join(value)
                            }
                        }
                    };
                    let value = value.to_str().unwrap();
                    value.as_bytes().to_vec()
                }
                .into();
                Attribute { key, value }
            } else {
                Attribute { key, value }
            }
        })
        .collect();

    let elem = elem.to_owned().with_attributes(attrs);
    writer.write_event(Event::Start(elem)).unwrap()
}

fn process_head_in_html(writer: &mut Writer<Cursor<Vec<u8>>>, config: &'static SiteConfig) {
    let title = config.base.title.as_str();
    let description = config.base.description.as_str();

    if !title.is_empty() {
        writer
            .write_event(Event::Start(BytesStart::new("title")))
            .unwrap();
        writer
            .write_event(Event::Text(BytesText::new(title)))
            .unwrap();
        writer
            .write_event(Event::End(BytesEnd::new("title")))
            .unwrap();
    }

    if !description.is_empty() {
        let mut elem = BytesStart::new("meta");
        elem.push_attribute(("name", "description"));
        elem.push_attribute(("content", description));
        writer.write_event(Event::Start(elem)).unwrap();
        writer
            .write_event(Event::End(BytesEnd::new("meta")))
            .unwrap();
    }

    if config.build.tailwind.enable
        && let Some(input) = &config.build.tailwind.input
    {
        let input = {
            let base_path = &config.build.base_path;
            let assets = config.build.assets.as_path().canonicalize().unwrap();
            let input = input.canonicalize().unwrap();
            let input = input.strip_prefix(assets).unwrap();
            let input = base_path.join(input);
            PathBuf::from("/").join(input)
        };
        let input = input.to_string_lossy();
        let mut elem = BytesStart::new("link");
        elem.push_attribute(("rel", "stylesheet"));
        elem.push_attribute(("href", input));
        writer.write_event(Event::Start(elem)).unwrap();
    }

    writer
        .write_event(Event::End(BytesEnd::new("head")))
        .unwrap();
}

fn get_asset_top_levels(assets_dir: &Path) -> &'static [OsString] {
    ASSET_TOP_LEVELS.get_or_init(|| {
        fs::read_dir(assets_dir)
            .map(|dir| dir.flatten().map(|entry| entry.file_name()).collect())
            .unwrap_or_default()
    })
}

fn is_asset_link(path: impl AsRef<Path>, config: &'static SiteConfig) -> bool {
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
            scheme
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '-' || c == '.')
        }
        None => false,
    }
}

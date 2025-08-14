use crate::cli::{Cli, Commands};
use anyhow::{Context, Result, bail};
use educe::Educe;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("IO error when reading `{0}`")]
    Io(PathBuf, #[source] std::io::Error),

    #[error("config file parsing error")]
    Toml(#[from] toml::de::Error),

    #[error("config file validation error: {0}")]
    Validation(String),
}

// for default value in serde
pub mod serde_defaults {
    pub fn r#true() -> bool {
        true
    }

    #[allow(unused)]
    pub fn r#false() -> bool {
        false
    }

    pub mod base {
        pub fn url() -> Option<String> {
            None
        }
    }

    pub mod build {
        use std::path::PathBuf;

        pub fn root() -> Option<PathBuf> {
            None
        }
        pub fn base_path() -> PathBuf {
            "".into()
        }
        pub fn language() -> String {
            "zh-Hans".into()
        }
        pub fn content() -> PathBuf {
            "content".into()
        }
        pub fn output() -> PathBuf {
            "public".into()
        }
        pub fn assets() -> PathBuf {
            "assets".into()
        }

        pub mod rss {
            use std::path::PathBuf;

            pub fn path() -> PathBuf {
                "rss.xml".into()
            }
        }

        #[allow(unused)]
        pub mod slug {
            use crate::config::SlugMode;

            pub fn default() -> SlugMode {
                SlugMode::default()
            }
            pub fn no() -> SlugMode {
                SlugMode::No
            }
            pub fn safe() -> SlugMode {
                SlugMode::Safe
            }
            pub fn on() -> SlugMode {
                SlugMode::On
            }
        }

        pub mod typst {
            pub fn command() -> Vec<String> {
                vec!["typst".into()]
            }
            pub mod svg {
                use crate::config::ExtractSvgType;

                pub fn extract_type() -> ExtractSvgType {
                    ExtractSvgType::default()
                }
                pub fn inline_max_size() -> String {
                    "20KB".into()
                }
                pub fn dpi() -> f32 {
                    96.
                }
            }
        }

        pub mod tailwind {
            use std::path::PathBuf;

            pub fn input() -> Option<PathBuf> {
                None
            }
            pub fn command() -> Vec<String> {
                vec!["tailwindcss".into()]
            }
        }
    }

    pub mod serve {
        pub fn interface() -> String {
            "127.0.0.1".into()
        }
        pub fn port() -> u16 {
            5277
        }
    }

    pub mod deploy {
        pub fn provider() -> String {
            "github".into()
        }

        pub mod github {
            use std::path::PathBuf;

            pub fn url() -> String {
                "https://github.com/alice/alice.github.io".into()
            }
            pub fn branch() -> String {
                "main".into()
            }
            pub fn token_path() -> Option<PathBuf> {
                None
            }
        }

        pub mod cloudflare {
            use std::path::PathBuf;

            pub fn _remote() -> String {
                "https://alice.com".into()
            }
            pub fn _branch() -> String {
                "main".into()
            }
            pub fn _token_path() -> PathBuf {
                "~/xxx/xxx/.github-token-in-this-file".into()
            }
        }

        pub mod vercal {
            use std::path::PathBuf;

            pub fn _remote() -> String {
                "https://alice.com".into()
            }
            pub fn _branch() -> String {
                "main".into()
            }
            pub fn _token_path() -> PathBuf {
                "~/xxx/xxx/.github-token-in-this-file".into()
            }
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SlugMode {
    On,

    #[default]
    Safe,

    No,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExtractSvgType {
    Builtin,

    Magick,

    Ffmpeg,

    JustSvg,

    #[default]
    Embedded,
}

// `[base]` in toml
#[derive(Debug, Clone, Educe, Serialize, Deserialize)]
#[educe(Default)]
#[serde(deny_unknown_fields)]
pub struct BaseConfig {
    // title
    pub title: String,

    // author, e.g.: "KawaYww email@kawayww.com"
    pub author: String,

    // description
    pub description: String,

    // e.g., "https://kawayww.com", for generating `rss.xml`/`atom.xl`, `sitemap.xml`
    #[serde(default = "serde_defaults::base::url")]
    #[educe(Default = serde_defaults::base::url())]
    pub url: Option<String>,

    // e.g., "zh-Hans", "zh_CN", "en_US"
    #[serde(default = "serde_defaults::build::language")]
    #[educe(Default = serde_defaults::build::language())]
    pub language: String,

    #[serde(default)]
    pub copyright: String,
}

#[test]
fn validate_base_config() {
    let config = r#"
        [base]
        title = "KawaYww"
        description = "KawaYww's Blog"
        url = "https://kawayww.com"
        language = "zh_Hans"
        copyright = "2025 KawaYww"
    "#;
    let config: SiteConfig = toml::from_str(config).unwrap();

    assert_eq!(config.base.title, "KawaYww");
    assert_eq!(config.base.description, "KawaYww");
    assert_eq!(config.base.title, "KawaYww");
}

// `[build]` in toml
#[derive(Debug, Clone, Educe, Serialize, Deserialize)]
#[educe(Default)]
#[serde(default, deny_unknown_fields)]
pub struct BuildConfig {
    // root directory path
    #[serde(default = "serde_defaults::build::root")]
    #[educe(Default = serde_defaults::build::root())]
    pub root: Option<PathBuf>,

    // e.g., "myblog"
    #[serde(default = "serde_defaults::build::base_path")]
    #[educe(Default = serde_defaults::build::base_path())]
    pub base_path: PathBuf,

    // content directory path related to `root`
    #[serde(default = "serde_defaults::build::content")]
    #[educe(Default = serde_defaults::build::content())]
    pub content: PathBuf,

    // output directory path related to `root`
    #[serde(default = "serde_defaults::build::output")]
    #[educe(Default = serde_defaults::build::output())]
    pub output: PathBuf,

    // assets directory path related to `root`
    #[serde(default = "serde_defaults::build::assets")]
    #[educe(Default = serde_defaults::build::assets())]
    pub assets: PathBuf,

    // minify the html content
    #[serde(default = "serde_defaults::r#true")]
    #[educe(Default = true)]
    pub minify: bool,

    // Whether to clear output dir before generating site
    #[serde(default = "serde_defaults::r#false")]
    #[educe(Default = false)]
    pub clear: bool,

    // rss file path related to `root`
    #[serde(default)]
    pub rss: RssConfig,

    // should slug or not
    #[serde(default)]
    pub slug: SlugConfig,

    // typst config
    #[serde(default)]
    pub typst: TypstConfig,

    // tailwind config
    #[serde(default)]
    pub tailwind: TailwindConfig,
}

// `[build.rss]` in toml
#[derive(Debug, Clone, Educe, Serialize, Deserialize)]
#[educe(Default)]
#[serde(deny_unknown_fields)]
pub struct RssConfig {
    // slugify the path or not
    #[serde(default = "serde_defaults::r#false")]
    #[educe(Default = serde_defaults::r#false())]
    pub enable: bool,

    // slugify the fragment or not
    #[serde(default = "serde_defaults::build::rss::path")]
    #[educe(Default = serde_defaults::build::rss::path())]
    pub path: PathBuf,
}

// `[build.typst]` in toml
#[derive(Debug, Clone, Educe, Serialize, Deserialize)]
#[educe(Default)]
#[serde(deny_unknown_fields)]
pub struct SlugConfig {
    // slugify the path or not
    #[serde(default = "serde_defaults::build::slug::default")]
    #[educe(Default = serde_defaults::build::slug::default())]
    pub path: SlugMode,

    // slugify the fragment or not
    #[serde(default = "serde_defaults::build::slug::on")]
    #[educe(Default = serde_defaults::build::slug::on())]
    pub fragment: SlugMode,
}

// `[build.typst]` in toml
#[derive(Debug, Clone, Educe, Serialize, Deserialize)]
#[educe(Default)]
#[serde(deny_unknown_fields)]
pub struct TypstConfig {
    // The name of typst command
    #[serde(default = "serde_defaults::build::typst::command")]
    #[educe(Default = serde_defaults::build::typst::command())]
    pub command: Vec<String>,

    // `[build.typst.svg]` part
    #[serde(default)]
    pub svg: TypstSvgConfig,
}

// `[build.typst.svg]` in toml
#[derive(Debug, Clone, Educe, Serialize, Deserialize)]
#[educe(Default)]
#[serde(deny_unknown_fields)]
pub struct TypstSvgConfig {
    // whether to extract a embedded svg into separate file, for smaller size && faster loading
    #[serde(default = "serde_defaults::build::typst::svg::extract_type")]
    #[educe(Default = serde_defaults::build::typst::svg::extract_type())]
    pub extract_type: ExtractSvgType,

    // The max size for inlining svg image
    #[serde(default = "serde_defaults::build::typst::svg::inline_max_size")]
    #[educe(Default = serde_defaults::build::typst::svg::inline_max_size())]
    pub inline_max_size: String,

    // The max size for inlining svg image
    #[serde(default = "serde_defaults::build::typst::svg::dpi")]
    #[educe(Default = serde_defaults::build::typst::svg::dpi())]
    pub dpi: f32,
}

// `[build.tailwind]` in toml
#[derive(Debug, Clone, Educe, Serialize, Deserialize)]
#[educe(Default)]
#[serde(deny_unknown_fields)]
pub struct TailwindConfig {
    // whether to enable tailwindcss support
    #[serde(default = "serde_defaults::r#false")]
    #[educe(Default = false)]
    pub enable: bool,

    // whether to enable tailwindcss support
    #[serde(default = "serde_defaults::build::tailwind::input")]
    #[educe(Default = serde_defaults::build::tailwind::input())]
    pub input: Option<PathBuf>,

    // The name of tailwind command
    #[serde(default = "serde_defaults::build::tailwind::command")]
    #[educe(Default = serde_defaults::build::tailwind::command())]
    pub command: Vec<String>,
}

// `[serve]` in toml
#[derive(Debug, Clone, Educe, Serialize, Deserialize)]
#[educe(Default)]
#[serde(deny_unknown_fields)]
pub struct ServeConfig {
    // Interface to bind on
    #[serde(default = "serde_defaults::serve::interface")]
    #[educe(Default = serde_defaults::serve::interface())]
    pub interface: String,

    // The port you should provide
    #[serde(default = "serde_defaults::serve::port")]
    #[educe(Default = serde_defaults::serve::port())]
    pub port: u16,

    // enable watch
    #[serde(default = "serde_defaults::r#true")]
    #[educe(Default = true)]
    pub watch: bool,
}

// `[deploy]` in toml
#[derive(Debug, Clone, Educe, Serialize, Deserialize)]
#[educe(Default)]
#[serde(deny_unknown_fields)]
pub struct DeployConfig {
    // The provider to use for deployment
    #[serde(default = "serde_defaults::deploy::provider")]
    #[educe(Default = serde_defaults::deploy::provider())]
    pub provider: String,

    // The provider to use for deployment
    #[serde(default = "serde_defaults::r#false")]
    #[educe(Default = serde_defaults::r#false())]
    pub force: bool,

    // The git provider for deployment
    #[serde(rename = "github", default)]
    pub github_provider: GithubProvider,

    // The cloudflare provider for deployment
    #[serde(rename = "cloudflare", default)]
    pub cloudflare_provider: CloudflareProvider,

    // The vercal provider for deployment
    #[serde(rename = "vercal", default)]
    pub vercal_provider: VercalProvider,
}

// `[deploy.git]` in toml
#[derive(Debug, Clone, Educe, Serialize, Deserialize)]
#[educe(Default)]
#[serde(deny_unknown_fields)]
pub struct GithubProvider {
    // The remote_url of generated site repo
    #[serde(default = "serde_defaults::deploy::github::url")]
    #[educe(Default = serde_defaults::deploy::github::url())]
    pub url: String,

    // The branch of generated site repo
    #[serde(default = "serde_defaults::deploy::github::branch")]
    #[educe(Default = serde_defaults::deploy::github::branch())]
    pub branch: String,

    // Warning: Be carefully if you enable this option
    // Warning: Not pushing your token into public repo
    // The provider to use for deployment
    #[serde(default = "serde_defaults::deploy::github::token_path")]
    #[educe(Default = serde_defaults::deploy::github::token_path())]
    pub token_path: Option<PathBuf>,
}

// `[deploy.cloudflare]` in toml
#[derive(Debug, Clone, Educe, Serialize, Deserialize)]
#[educe(Default)]
#[serde(deny_unknown_fields)]
pub struct CloudflareProvider {
    // The provider to use for deployment
    #[serde(default = "serde_defaults::deploy::provider")]
    #[educe(Default = serde_defaults::deploy::provider())]
    pub provider: String,
}

// `[deploy.vercal]` in toml
#[derive(Debug, Clone, Educe, Serialize, Deserialize)]
#[educe(Default)]
#[serde(deny_unknown_fields)]
pub struct VercalProvider {
    // The provider to use for deployment
    #[serde(default = "serde_defaults::deploy::provider")]
    #[educe(Default = serde_defaults::deploy::provider())]
    pub provider: String,
}

// top-level toml
#[derive(Debug, Clone, Educe, Serialize, Deserialize)]
#[educe(Default)]
#[serde(deny_unknown_fields)]
pub struct SiteConfig {
    #[serde(skip)]
    pub cli: Option<&'static Cli>,

    #[serde(default)]
    pub base: BaseConfig,

    #[serde(default)]
    pub build: BuildConfig,

    #[serde(default)]
    pub serve: ServeConfig,

    #[serde(default)]
    pub deploy: DeployConfig,

    #[serde(default)]
    pub extra: HashMap<String, toml::Value>,
}

impl SiteConfig {
    pub fn from_str(content: &str) -> Result<Self> {
        let config: SiteConfig = toml::from_str(content)?;
        Ok(config)
    }

    pub fn from_path(path: &Path) -> Result<Self> {
        let content =
            fs::read_to_string(path).map_err(|err| ConfigError::Io(path.to_path_buf(), err))?;
        Self::from_str(&content)
    }

    pub fn get_root(&self) -> &Path {
        self.build.root.as_deref().unwrap_or(Path::new("./"))
    }

    pub fn set_root(&mut self, path: &Path) {
        self.build.root = Some(path.to_path_buf())
    }

    pub fn get_cli(&self) -> &'static Cli {
        self.cli.unwrap()
    }

    pub fn get_inline_max_size(&self) -> usize {
        let inline_max_size = self.build.typst.svg.inline_max_size.as_str();
        let per_size = if inline_max_size.ends_with("MB") {
            1024 * 1024
        } else if inline_max_size.ends_with("KB") {
            1024
        } else if inline_max_size.ends_with("B") {
            1
        } else {
            unreachable!()
        };
        per_size
            * inline_max_size
                .trim_end_matches(|c: char| c.is_ascii_uppercase())
                .parse::<usize>()
                .unwrap()
    }

    pub fn get_scale(&self) -> f32 {
        self.build.typst.svg.dpi / 96.
    }

    #[rustfmt::skip]
    pub fn update_with_cli(&mut self, cli: &'static Cli) {
        self.cli = Some(cli);

        let root = if let Some(root) = &cli.root {
            root.to_owned()
        } else {
            self.get_root().to_owned()
        };
        self.set_root(&root);
        self.update_path_with_root(&root);

        Self::update_option(&mut self.build.minify, cli.minify.as_ref());
        Self::update_option(&mut self.build.tailwind.enable, cli.tailwind.as_ref());

        self.build.typst.svg.inline_max_size = self.build.typst.svg.inline_max_size.to_uppercase();

        match &cli.command {
            Commands::Init { name: Some(name) } => {
                let root = if let Some(root) = &self.build.root {
                    root.join(name)
                } else {
                    name.clone()
                };
                self.update_path_with_root(&root);
            },
            Commands::Serve { interface, port, watch } => {
                Self::update_option(&mut self.serve.interface, interface.as_ref());
                Self::update_option(&mut self.serve.port, port.as_ref());
                Self::update_option(&mut self.serve.watch, watch.clone().as_ref());
                self.base.url = Some(format!("http://{}:{}", self.serve.interface, self.serve.port));
            },
            Commands::Deploy { force } => {
                Self::update_option(&mut self.deploy.force, force.clone().as_ref());
            },
            _ => ()
        }
    }

    fn update_option<T: Clone>(config_option: &mut T, cli_option: Option<&T>) {
        if let Some(option) = cli_option {
            *config_option = option.clone()
        }
    }

    fn update_path_with_root(&mut self, root: &Path) {
        let cli = self.get_cli();

        self.set_root(root);
        Self::update_option(&mut self.build.content, cli.content.as_ref());
        Self::update_option(&mut self.build.assets, cli.assets.as_ref());
        Self::update_option(&mut self.build.output, cli.output.as_ref());

        self.build.content = root.join(&self.build.content);
        self.build.assets = root.join(&self.build.assets);
        self.build.output = root.join(&self.build.output);
        self.build.rss.path = self.build.output.join(&self.build.rss.path);

        if self.build.tailwind.enable
            && let Some(input) = self.build.tailwind.input.as_ref()
        {
            self.build.tailwind.input.replace(root.join(input));
        }

        if let Some(token_path) = &self.deploy.github_provider.token_path {
            let path = shellexpand::tilde(token_path.to_str().unwrap()).into_owned();
            let path = PathBuf::from(path);
            self.deploy.github_provider.token_path = if path.is_relative() {
                Some(root.join(path))
            } else {
                Some(path.to_owned())
            };
        }
    }

    #[rustfmt::skip]
    #[allow(unused)]
    pub fn validate(&self) -> Result<()> {
        let cli = self.get_cli();

        if !self.get_root().join(cli.config.as_path()).exists() {
            bail!("the config file didn't exist");
        }

        #[allow(clippy::collapsible_if)]
        if self.build.rss.enable {
            if self.base.url.is_none() {
                bail!("the [base.url] is required for generating RSS");
            }
        }

        Self::check_command_installed("[build.typst.command]", &self.build.typst.command);

        let root = self.get_root();
        let output = self.build.output.as_path();
        let token_path = self.deploy.github_provider.token_path.as_ref();
        let force = self.deploy.force;

        if let Some(base_url) = self.base.url.as_ref() && !base_url.starts_with("http") {
            bail!(ConfigError::Validation(
                "[base.url] should start with `http://` or `https://`".into()
            ))
        }

        if self.build.tailwind.enable {
            Self::check_command_installed("[build.tailwind.command]", &self.build.tailwind.command);

            match &self.build.tailwind.input {
                None => bail!("[build.tailwind.enable] = true, but you didn't specify [build.tailwind.input] for input file"),
                Some(path) => {
                    if !path.exists() { bail!(ConfigError::Validation(
                        "[build.tailwind.input] not exists".into()
                    ))}
                    if !path.is_file() { bail!(ConfigError::Validation(
                        "[build.tailwind.input] is not a file".into()
                    ))}
                }
            }
        }

        let is_valid_size = ["B", "KB", "MB"].iter().any(|s| self.build.typst.svg.inline_max_size.ends_with(s));
        if !is_valid_size {bail!(ConfigError::Validation(
            "The size must end with `B`, `KB`, `MB`".into()
        ))}

        match cli.command {
            Commands::Init { .. } => {
                if root.exists() { bail!("The path already exists") }
            },
            Commands::Deploy { .. } => {
                if let Some(path) =  token_path {
                    if !path.exists() { bail!(ConfigError::Validation(
                        "[deploy.github.token_path] not exists".into()
                    ))}
                    if !path.is_file() { bail!(ConfigError::Validation(
                        "[deploy.github.token_path] is not a file".into()
                    ))}
                }
            },
            _ => ()
        }

        Ok(())
    }

    fn check_command_installed(fields_in_config: &str, command: &[String]) -> Result<()> {
        if command.is_empty() {
            bail!(ConfigError::Validation(format!(
                "{fields_in_config} should have at least one field"
            )))
        }

        let command = command[0].as_str();
        which::which(command).with_context(|| {
            format!("[check] `{command}` not found. Please install `{command}` first")
        })?;

        Ok(())
    }
}

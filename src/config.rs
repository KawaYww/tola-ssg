use anyhow::{Result, Context, bail};
use educe::Educe;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::{Path, PathBuf}};
use thiserror::Error;
use crate::cli::{Cli, Commands};

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("IO error when reading `{0}`")]
    Io(
        PathBuf,
        #[source] std::io::Error,
    ),

    #[error("config file parsing error")]
    Toml(#[from] toml::de::Error),

    #[error("config file validation error: {0}")]
    Validation(String),
}

// for default value in serde
pub mod serde_defaults {
    pub fn r#true() -> bool { true }

    #[allow(unused)]
    pub fn r#false() -> bool { false }

    pub mod base {
        pub fn base_url() -> String { "https://bob-example.com".into() }   
    }
    
    pub mod build {
        use std::path::PathBuf;

        pub fn language() -> String { "zh-Hans".into() }   
        pub fn root_path() -> Option<PathBuf> { None }
        pub fn content_dir() -> PathBuf { "content".into() }
        pub fn output_dir() -> PathBuf { "public".into() }
        pub fn assets_dir() -> PathBuf { "assets".into() }

        pub mod typst {
            pub fn command() -> Vec<String> { vec!["typst".into()] }
        }

        pub mod tailwind {
            use std::path::PathBuf;

            pub fn input() -> Option<PathBuf> { None }
            pub fn command() -> Vec<String> { vec!["tailwindcss".into()] }
        }
    }

    pub mod serve {
        pub fn interface() -> String { "127.0.0.1".into() }
        pub fn port() -> u16 { 5277 }
    }


    pub mod deploy {
        pub fn provider() -> String { "github".into() }

        pub mod github {
            use std::path::PathBuf;

            pub fn url() -> String { "https://github.com/alice/alice.github.io".into() }
            pub fn branch() -> String { "main".into() }
            pub fn token_path() -> Option<PathBuf> { None }
        }

        pub mod cloudflare {
            use std::path::PathBuf;

            pub fn _remote() -> String { "https://alice.com".into() }
            pub fn _branch() -> String { "main".into() }
            pub fn _token_path() -> PathBuf { "~/xxx/xxx/.github-token-in-this-file".into() }
        }

        pub mod vercal {
            use std::path::PathBuf;

            pub fn _remote() -> String { "https://alice.com".into() }
            pub fn _branch() -> String { "main".into() }
            pub fn _token_path() -> PathBuf { "~/xxx/xxx/.github-token-in-this-file".into() }
        }
    }
}

// `[base]` in toml
#[derive(Debug, Clone, Educe, Serialize, Deserialize)]
#[educe(Default)]
#[serde(deny_unknown_fields)]
pub struct BaseConfig {
    // title
    pub title: String,
    
    // description
    pub description: String,

    // e.g., "https://kawayww.com"
    #[serde(default = "serde_defaults::base::base_url")]
    #[educe(Default = serde_defaults::base::base_url())]
    pub base_url: String,

    // e.g., "zh-Hans", "zh_CN", "en_US"
    #[serde(default = "serde_defaults::build::language")]
    #[educe(Default = serde_defaults::build::language())]
    pub default_language: String,

    #[serde(default)]
    pub copyright: String,
}

#[test]
fn validate_base_config() {
    let config = r#"
        [base]
        title = "KawaYww"
        description = "KawaYww's Blog"
        base_url = "https://kawayww.com"
        default_language = "zh_Hans"
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
    #[serde(default = "serde_defaults::build::root_path")]
    #[educe(Default = serde_defaults::build::root_path())]
    pub root_path: Option<PathBuf>,

    // Content directory path related to `root_dor`
    #[serde(default = "serde_defaults::build::content_dir")]
    #[educe(Default = serde_defaults::build::content_dir())]
    pub content_dir: PathBuf,

    // Output directory path related to `root_dor`
    #[serde(default = "serde_defaults::build::output_dir")]
    #[educe(Default = serde_defaults::build::output_dir())]
    pub output_dir: PathBuf,

    // Output directory path related to `root_dor`
    #[serde(default = "serde_defaults::build::assets_dir")]
    #[educe(Default = serde_defaults::build::assets_dir())]
    pub assets_dir: PathBuf,

    // Minify the html content
    #[serde(default = "serde_defaults::r#true")]
    #[educe(Default = true)]
    pub minify: bool,

    // typst config
    #[serde(default)]
    pub typst: TypstConfig,

    // tailwind config
    #[serde(default)]
    pub tailwind: TailwindConfig,
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

// `[build.typst]` in toml
#[derive(Debug, Clone, Educe, Serialize, Deserialize)]
#[educe(Default)]
#[serde(deny_unknown_fields)]
pub struct TypstConfig {
    // The name of typst command
    #[serde(default = "serde_defaults::build::typst::command")]
    #[educe(Default = serde_defaults::build::typst::command())]
    pub command: Vec<String>,
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

    pub fn from_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path).map_err(|err| ConfigError::Io (
            path.to_path_buf(),
            err
        ))?;
        Self::from_str(&content)
    }

    pub fn get_root(&self) -> PathBuf {
        self.build.root_path.clone().unwrap_or_default()
    }

    pub fn set_root(&mut self, path: &Path) {
        self.build.root_path = Some(path.to_path_buf())
    }

    #[rustfmt::skip]
    pub fn update_with_cli(&mut self, cli: &Cli) {      
        if let Some(root) = &cli.root {
            self.update_path_with_root(root.as_path(), cli);
            self.set_root(root);
        }

        Self::update_option(&mut self.build.minify, cli.minify.as_ref());
        Self::update_option(&mut self.build.tailwind.enable, cli.tailwind.as_ref());

        match &cli.command {
            Commands::Init { name: Some(name) } => {
                let root = if let Some(root) = &self.build.root_path {
                    root.join(name)
                } else {
                    name.clone()
                };
                self.update_path_with_root(&root, cli);
            },
            Commands::Serve { interface, port, watch } => {
                self.serve.interface = interface.to_owned();
                self.serve.port = *port;
                Self::update_option(&mut self.serve.watch, watch.clone().as_ref());
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

    fn update_path_with_root(&mut self, root: &Path, cli: &Cli) {
        self.set_root(root);
        self.build.content_dir = root.join(&cli.content);
        self.build.output_dir = root.join(&cli.output);
        self.build.assets_dir = root.join(&cli.assets);

        // if self.build.tailwind.enable {
        //     self.build.tailwind.input 
        // }

        if let Some(token_path) = &self.deploy.github_provider.token_path {
            let path = shellexpand::tilde(token_path.to_str().unwrap());
            let path = PathBuf::from(path.into_owned());
            self.deploy.github_provider.token_path = if path.is_relative() {
                Some(root.join(path))
            } else {
                Some(path.to_owned())
            }
        }

    }
    
    #[rustfmt::skip]
    #[allow(unused)]
    pub fn validate(&self, cli: &Cli) -> Result<()> {
        Self::check_command_installed("[build.typst.command]", &self.build.typst.command);
        
        let root = self.get_root();
        let output_dir = self.build.output_dir.as_path();
        let base_url = self.base.base_url.as_str();
        let token_path = self.deploy.github_provider.token_path.as_ref();
        let force = self.deploy.force;
        
        if !base_url.starts_with("http") { bail!(ConfigError::Validation(
            "[base.base_url] should start with `http://` or `https://`".into()
        ))}

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
        if command.is_empty() { bail!(ConfigError::Validation(
            format!("{fields_in_config} should have at least one field")
        ))}

        let command = command[0].as_str();
        which::which(command).with_context(|| format!("[checker] `{command}` not found. Please install `{command}` first"))?;

        Ok(())
    }

}


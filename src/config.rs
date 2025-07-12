use anyhow::{Result, bail};
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
        pub fn typst_command() -> String { "typst".into() }   
        pub fn root_path() -> PathBuf { "./".into() }
        pub fn content_dir() -> PathBuf { "content".into() }
        pub fn output_dir() -> PathBuf { "public".into() }
        pub fn assets_dir() -> PathBuf { "assets".into() }
    }

    pub mod serve {
        pub fn interface() -> String { "127.0.0.1".into() }
        pub fn port() -> u16 { 5277 }
    }

    pub mod tailwind {
        pub fn command() -> String { "tailwindcss".into() }
    }

    pub mod deploy {
        pub fn provider() -> String { "github".into() }

        pub mod github {
            use std::path::PathBuf;

            pub fn remote_url() -> String { "https://github.com/alice/alice.github.io".into() }
            pub fn branch() -> String { "main".into() }
            pub fn token_path() -> PathBuf { PathBuf::new() }
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

// `[build]` in toml
#[derive(Debug, Clone, Educe, Serialize, Deserialize)]
#[educe(Default)]
#[serde(default, deny_unknown_fields)]
pub struct BuildConfig {
    // root directory path
    #[serde(default = "serde_defaults::build::root_path")]
    #[educe(Default = serde_defaults::build::root_path())]
    pub root_path: PathBuf,

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

    // The name of typst command
    #[serde(default = "serde_defaults::build::typst_command")]
    #[educe(Default = serde_defaults::build::typst_command())]
    pub typst_command: String,

    // Minify the html content
    #[serde(default = "serde_defaults::r#true")]
    #[educe(Default = true)]
    pub minify: bool,
}

// `[server]` in toml
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
// `[tailwind]` in toml
#[derive(Debug, Clone, Educe, Serialize, Deserialize)]
#[educe(Default)]
#[serde(deny_unknown_fields)]
pub struct TailwindConfig {
    // whether to enable tailwindcss support
    #[serde(default = "serde_defaults::r#true")]
    #[educe(Default = true)]
    pub enable: bool,

    // The name of tailwind command
    #[serde(default = "serde_defaults::tailwind::command")]
    #[educe(Default = serde_defaults::tailwind::command())]
    pub command: String,
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
    #[serde(default = "serde_defaults::deploy::github::remote_url")]
    #[educe(Default = serde_defaults::deploy::github::remote_url())]
    pub remote_url: String,

    // The branch of generated site repo
    #[serde(default = "serde_defaults::deploy::github::branch")]
    #[educe(Default = serde_defaults::deploy::github::branch())]
    pub branch: String,

    // Warning: Be carefully if you enable this option
    // Warning: Not pushing your token into public repo
    // The provider to use for deployment
    #[serde(default = "serde_defaults::deploy::github::token_path")]
    #[educe(Default = serde_defaults::deploy::github::token_path())]
    pub token_path: PathBuf,
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
    pub tailwind: TailwindConfig,

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

    #[rustfmt::skip]
    pub fn update_with_cli(&mut self, cli: &Cli) {      
        Self::update_option(&mut self.build.root_path, cli.root.as_ref());
        Self::update_option(&mut self.build.minify, cli.minify.as_ref());
        Self::update_option(&mut self.tailwind.enable, cli.tailwind.as_ref());

        self.update_path_with_root(self.build.root_path.clone().as_path(), cli);

        match &cli.command {
            Commands::Init { name: Some(name) } => {
                self.update_path_with_root(&self.build.root_path.join(name), cli);
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
        self.build.content_dir = root.join(&cli.content);
        self.build.output_dir = root.join(&cli.output);
        self.build.assets_dir = root.join(&cli.assets);
        self.build.root_path = root.to_owned();

        let token_path = {
            let path = &self.deploy.github_provider.token_path;
            let path = shellexpand::tilde(path.to_str().unwrap());
            PathBuf::from(path.into_owned())
        };

        self.deploy.github_provider.token_path = if token_path.is_relative() {
            root.join(token_path)
        } else {
            token_path
        }
    }
    
    #[rustfmt::skip]
    #[allow(unused)]
    pub fn validate(&self, cli: &Cli) -> Result<()> {
        let root = self.build.root_path.as_path();
        let output_dir = self.build.output_dir.as_path();
        let base_url = self.base.base_url.as_str();
        let token_path = self.deploy.github_provider.token_path.as_path();
        let force = self.deploy.force;
        
        if !base_url.starts_with("http") { bail!(ConfigError::Validation(
            "[base.base_url] should start with `http://` or `https://`".into()
        ))}

        match cli.command {
            Commands::Init { .. } => {
                if root.exists() { bail!("The path already exists") }
            },
            Commands::Deploy { .. } => {
                if token_path != Path::new("") {
                    if !token_path.exists() { bail!(ConfigError::Validation(
                        "[deploy.github.token_path] not exists".into()
                    ))}
                    if !token_path.is_file() { bail!(ConfigError::Validation(
                        "[deploy.github.token_path] is not a file".into()
                    ))}
                }
            },
            _ => ()
        }      

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_CONFIG: &str = r#"
        [base]
        title = "我的博客"
        description = "一个关于技术的博客"
        base_url = "https://example.com"
        default_language = "zh-CN"
        copyright = "© 2023 我的博客"

        [build]
        output_dir = "public"
        compile_tailwindcss = true

        [extra]
        author = "张三"
        github = "https://github.com/zhangsan"
    "#;

    #[test]
    fn parse() {
        let config = SiteConfig::from_str(SAMPLE_CONFIG).unwrap();
        
        assert_eq!(config.base.title, "我的博客");
        assert_eq!(config.build.output_dir, PathBuf::from("public"));
        assert_eq!(config.extra["author"].as_str(), Some("张三"));
    }

    #[test]
    fn default_values() {
        let config_str = r#"
            [base]
            title = "默认值测试"
            base_url = "https://example.com"
        "#;
        
        let config = SiteConfig::from_str(config_str).unwrap();
        
        assert_eq!(config.build.output_dir, PathBuf::from("public"));
        assert_eq!(config.build.minify, true);
        assert_eq!(config.tailwind.enable, true);
    }

    #[test]
    fn validation() {
        let invalid_url = r#"
            [base]
            title = "测试"
            base_url = "example.com"
        "#;
        assert!(SiteConfig::from_str(invalid_url).is_err());
    }
}

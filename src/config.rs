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
        pub fn tailwind_command() -> String { "tailwindcss".into() }   
        pub fn root_path() -> PathBuf { "./".into() }
        pub fn content_dir() -> PathBuf { "content".into() }
        pub fn output_dir() -> PathBuf { "public".into() }
        pub fn assets_dir() -> PathBuf { "assets".into() }
    }

    pub mod serve {
        pub fn interface() -> String { "127.0.0.1".into() }
        pub fn port() -> u16 { 5277 }
    }

    pub mod deploy {
        pub fn provider() -> String { "git".into() }

        pub mod git {
            use gix::config::tree::Author;

            pub fn _remote() -> String { Author::NAME.to_string() }
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


    // enable tailwindcss support
    #[serde(default = "serde_defaults::r#true")]
    #[educe(Default = true)]
    pub tailwind_support: bool,

    // enable tailwindcss support
    #[serde(default = "serde_defaults::build::tailwind_command")]
    #[educe(Default = serde_defaults::build::tailwind_command())]
    pub tailwind_command: String,

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
    #[serde(rename = "git", default)]
    pub git_provider: GithubProvider, 

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
    // The provider to use for deployment
    #[serde(default = "serde_defaults::deploy::provider")]
    #[educe(Default = serde_defaults::deploy::provider())]
    pub remote: String,

    // The provider to use for deployment
    #[serde(default = "serde_defaults::deploy::provider")]
    #[educe(Default = serde_defaults::deploy::provider())]
    pub branch: String,

    // The provider to use for deployment
    #[serde(default = "serde_defaults::deploy::provider")]
    #[educe(Default = serde_defaults::deploy::provider())]
    pub commit_message: String,
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
    pub fn from_str(content: &str) -> Result<Self, ConfigError> {
        let config: SiteConfig = toml::from_str(content)?;
        config.validate()?;
        Ok(config)
    }

    pub fn from_file(path: &Path) -> Result<Self, ConfigError> {
        let content = fs::read_to_string(path).map_err(|err| ConfigError::Io (
            path.to_path_buf(),
            err
        ))?;
        Self::from_str(&content)
    }

    #[rustfmt::skip]
    pub fn update_wiht_cli_settings(mut self, cli: &Cli) -> Self {
        fn update_path_with_root(root: &Path, config: &mut SiteConfig, cli: &Cli) {
            config.build.root_path = root.to_owned();
            config.build.content_dir = root.join(&cli.content);
            config.build.output_dir = root.join(&cli.output);
            config.build.assets_dir = root.join(&cli.assets);
        }
        
        self.build.tailwind_support = cli.tailwind_support;
        self.build.tailwind_command = cli.tailwind_command.to_owned();

        update_path_with_root(&self.build.root_path.clone(), &mut self, cli);

        if let Some(subcommand)  = &cli.command { match subcommand {
            Commands::Init { name } => {
                update_path_with_root(name, &mut self, cli);
            },
            Commands::Serve { interface, port, watch } => {
                self.serve.interface = interface.to_owned();
                self.serve.port = *port;
                self.serve.watch = *watch;
            },
            Commands::Deploy { force } => {
                self.deploy.force = *force;
            },
            _ => ()
        }}

        self
    }
    
    fn validate(&self) -> Result<(), ConfigError> {
        if !self.base.base_url.starts_with("http") {
            return Err(ConfigError::Validation(
                "`base_url` should start with `http://` or `https://`".into()
            ));
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
    fn parse_config() {
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
        assert_eq!(config.build.tailwind_support, true);
    }

    #[test]
    fn config_validation() {
        let invalid_url = r#"
            [base]
            title = "测试"
            base_url = "example.com"
        "#;
        assert!(SiteConfig::from_str(invalid_url).is_err());
    }
}

use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about = None, arg_required_else_help = true)]
pub struct Cli {
    /// root directory path
    #[arg(short, long)]
    pub root: Option<PathBuf>,

    /// Output directory path related to `root`
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Content directory path related to `root`
    #[arg(short, long)]
    pub content: Option<PathBuf>,

    /// Assets directory path related to `root`
    #[arg(short, long)]
    pub assets: Option<PathBuf>,

    /// Config file path related to `root`
    #[arg(short = 'C', long, default_value = "tola.toml")]
    pub config: PathBuf,

    /// Minify the html content
    #[arg(short, long, action = clap::ArgAction::Set, num_args = 0..=1, default_missing_value = "true", require_equals = false)]
    pub minify: Option<bool>,

    /// enable tailwindcss support
    #[arg(short, long, action = clap::ArgAction::Set, num_args = 0..=1, default_missing_value = "true", require_equals = false)]
    pub tailwind: Option<bool>,

    /// subcommands
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// Init a template site
    Init {
        /// the name(path) of site directory, related to `root`
        name: Option<PathBuf>,
    },

    /// Deletes the output directory if there is one and rebuilds the site
    Build {},

    /// Serve the site. Rebuild and reload on change automatically
    Serve {
        /// Interface to bind on
        #[arg(short, long)]
        interface: Option<String>,

        /// The port you should provide
        #[arg(short, long)]
        port: Option<u16>,

        /// enable watch
        #[arg(short, long, action = clap::ArgAction::Set, num_args = 0..=1, default_missing_value = "true", require_equals = false)]
        watch: Option<bool>,
    },

    /// Deletes the output directory if there is one and rebuilds the site
    Deploy {
        /// enable watch
        #[arg(short, long, action = clap::ArgAction::Set, num_args = 0..=1, default_missing_value = "true", require_equals = false)]
        force: Option<bool>,
    },
}

#[allow(unused)]
impl Cli {
    pub fn is_init(&self) -> bool {
        matches!(self.command, Commands::Init { .. })
    }
    pub fn is_build(&self) -> bool {
        matches!(self.command, Commands::Build { .. })
    }
    pub fn is_serve(&self) -> bool {
        matches!(self.command, Commands::Serve { .. })
    }
    pub fn is_deploy(&self) -> bool {
        matches!(self.command, Commands::Deploy { .. })
    }
}

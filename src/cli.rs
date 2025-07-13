use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about = None, arg_required_else_help = true)]
pub struct Cli {
    /// root directory path
    #[arg(short, long, default_value = "./")]
    pub root: Option<PathBuf>,

    /// Output directory path related to `root_dor`
    #[arg(short, long, default_value = "public")]
    pub output: PathBuf,

    /// Content directory path related to `root_dor`
    #[arg(short, long, default_value = "content")]
    pub content: PathBuf,

    /// Assets directory path related to `root_dor`
    #[arg(short, long, default_value = "assets")]
    pub assets: PathBuf,

    /// Config file path related to `root_dor`
    #[arg(short = 'C', long, default_value = "tola.toml")]
    pub config: PathBuf,

    /// Minify the html content
    #[arg(short, long)]
    pub minify: Option<bool>,

    /// enable tailwindcss support
    #[arg(short, long)]
    pub tailwind: Option<bool>,

    /// subcommands
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// Init a template site
    Init {
        /// the name(path) of site directory, related to `root_path`
        #[arg()]
        name: Option<PathBuf>,
    },
    
    /// Serve the site. Rebuild and reload on change automatically
    Serve {
        /// Interface to bind on
        #[arg(short, long, default_value = "127.0.0.1")]
        interface: String,

        /// The port you should provide
        #[arg(short, long, default_value_t = 5277)]
        port: u16,

        /// enable watch
        #[arg(short, long)]
        watch: Option<bool>,
    },

    /// Deletes the output directory if there is one and rebuilds the site
    Build {
    },

    /// Deletes the output directory if there is one and rebuilds the site
    Deploy {
        /// enable watch
        #[arg(short, long)]
        force: Option<bool>,
    },
}

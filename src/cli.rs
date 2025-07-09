use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about = None, arg_required_else_help = true)]
pub struct Cli {
    /// root directory path
    #[arg(short, long, default_value = "./")]
    pub root: PathBuf,

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
    #[arg(short, long, default_value_t = true)]
    pub minify: bool,

    // enable tailwindcss support
    #[arg(long, default_value_t = true)]
    pub tailwind_support: bool,

    // enable tailwindcss support
    #[arg(long, default_value = "tailwindcss")]
    pub tailwind_command: String,

    // subcommands
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// Init a template site
    Init {
        /// the name of site directory
        #[arg()]
        name: PathBuf,
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
        #[arg(short, long, default_value_t = true)]
        watch: bool,
    },

    /// Deletes the output directory if there is one and rebuilds the site
    Build {
    },

    /// Deletes the output directory if there is one and rebuilds the site
    Deploy {
        /// enable watch
        #[arg(short, long)]
        force: bool,
    },
}

impl Cli {
    pub fn command_is_serve(&self) -> bool {
        matches!(self.command, Some(Commands::Serve { .. }))
    }

    pub fn command_is_built(&self) -> bool {
        matches!(self.command, Some(Commands::Build { .. }))
    }

    pub fn command_is_init(&self) -> bool {
        matches!(self.command, Some(Commands::Init { .. }))
    }

    pub fn command_is_deploy(&self) -> bool {
        matches!(self.command, Some(Commands::Deploy { .. }))
    }
}

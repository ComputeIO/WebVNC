use clap::Parser;

#[derive(Parser, Debug, Clone)]
#[command(author, version, about = "x11vnc (Rust PoC) - options", long_about = None)]
pub struct Options {
    /// show a quick status and exit
    #[arg(long)]
    pub status: bool,

    /// debug mode
    #[arg(short, long)]
    pub debug: bool,

    /// RFB listen port
    #[arg(long)]
    pub port: Option<u16>,

    /// allow list (comma separated prefixes)
    #[arg(long)]
    pub allow: Option<String>,

    /// one-time allow list
    #[arg(long)]
    pub allow_once: Option<String>,

    /// optional password file
    #[arg(long)]
    pub passwdfile: Option<String>,

    /// view-only mode
    #[arg(long)]
    pub viewonly: bool,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            status: false,
            debug: false,
            port: None,
            allow: None,
            allow_once: None,
            passwdfile: None,
            viewonly: false,
        }
    }
}

impl Options {
    /// Parse options from the CLI (used in `main.rs`).
    pub fn from_args() -> Self {
        Options::parse()
    }
}

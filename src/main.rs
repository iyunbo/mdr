mod error;

use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "mdr", version, about = "A terminal markdown reader")]
struct Cli {
    /// Path to a markdown file or directory to browse
    path: Option<PathBuf>,
}

fn main() {
    let cli = Cli::parse();
    match cli.path {
        Some(p) => println!("Opening: {}", p.display()),
        None => println!("No path given — will open file browser"),
    }
}

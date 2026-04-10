mod error;
mod fs;

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
    if let Some(path) = cli.path {
        match fs::read_file(path.to_str().unwrap_or("")) {
            Ok(content) => println!("{}", content),
            Err(e) => eprintln!("Error: {}", e),
        }
    } else {
        println!("No path given — will open file browser");
    }
}

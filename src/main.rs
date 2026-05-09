use clap::{Parser, Subcommand};
mod commands;
mod error;
use error::CliError;

#[derive(Parser)]
#[command(name = "RsCoreutils", version, about = "Unified POSIX-like CLI toolbox")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Concatenate files and print to stdout
    Cat(commands::cat::CatArgs),
    /// Print lines matching a pattern
    Grep(commands::grep::GrepArgs),
}

fn run(cli: Cli) -> Result<(), CliError> {
    match cli.command {
        Commands::Cat(args) => {
            if commands::cat::run(args) {
                std::process::exit(1);
            }
        }
        Commands::Grep(args) => {
            let exit_code = commands::grep::run(args);
            std::process::exit(exit_code as i32);
        }
    }
    Ok(())
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(cli) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
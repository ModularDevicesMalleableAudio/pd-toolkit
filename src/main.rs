use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "pdtk",
    version,
    about = "Safe parser, editor, and formatter for Pure Data patch files"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Parse a .pd file and print summary statistics
    Parse {
        /// Path to .pd file
        file: String,

        /// Output results as JSON
        #[arg(long)]
        json: bool,
    },

    /// Check patch structure and connection integrity
    Validate {
        /// Path to .pd file
        file: String,

        /// Enable strict validation checks
        #[arg(long)]
        strict: bool,

        /// Output results as JSON
        #[arg(long)]
        json: bool,
    },

    /// List objects with indices and details
    List {
        /// Path to .pd file
        file: String,

        /// Subpatch depth to list (0 = top-level)
        #[arg(long)]
        depth: Option<usize>,

        /// Output results as JSON
        #[arg(long)]
        json: bool,
    },
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Parse { file, .. }) => {
            eprintln!("pdtk parse: not yet implemented (file: {})", file);
            std::process::exit(2);
        }
        Some(Commands::Validate { file, .. }) => {
            eprintln!("pdtk validate: not yet implemented (file: {})", file);
            std::process::exit(2);
        }
        Some(Commands::List { file, .. }) => {
            eprintln!("pdtk list: not yet implemented (file: {})", file);
            std::process::exit(2);
        }
        None => {
            Cli::parse_from(["pdtk", "--help"]);
        }
    }

    Ok(())
}

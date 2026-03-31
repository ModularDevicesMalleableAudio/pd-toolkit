use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "pdtk",
    version,
    about = "Safe parser, editor, and formatter for Pure Data patch files"
)]
pub struct Cli {
    /// Enable verbose output
    #[arg(long, global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Parse a .pd file and print summary statistics
    Parse {
        /// Path to .pd file
        file: String,

        /// Output results as JSON
        #[arg(long)]
        json: bool,

        /// Write re-serialized .pd to this path (proves round-trip fidelity)
        #[arg(long, value_name = "PATH")]
        output: Option<String>,
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

        /// Write output to this file instead of stdout
        #[arg(long, value_name = "PATH")]
        output: Option<String>,
    },

    /// Check patch structure and connection integrity
    Validate {
        /// Path to .pd file
        file: String,

        /// Enable strict checks (warns on duplicate connections, out-of-range outlets)
        #[arg(long)]
        strict: bool,

        /// Output results as JSON
        #[arg(long)]
        json: bool,

        /// Write output to this file instead of stdout
        #[arg(long, value_name = "PATH")]
        output: Option<String>,
    },
}

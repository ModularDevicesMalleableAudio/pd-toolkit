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

    /// Insert an object at a specific index
    Insert {
        /// Path to .pd file
        file: String,

        /// Subpatch depth for insertion (0 = top-level)
        #[arg(long)]
        depth: usize,

        /// Object index where to insert (0-based, before existing object at this index)
        #[arg(long)]
        index: usize,

        /// Raw entry text (e.g. "#X obj 50 50 print;")
        #[arg(long, value_name = "TEXT")]
        entry: String,

        /// Overwrite the original file
        #[arg(long)]
        in_place: bool,

        /// Create a .bak backup before modifying
        #[arg(long)]
        backup: bool,

        /// Write output to this file instead of stdout
        #[arg(long, value_name = "PATH")]
        output: Option<String>,
    },

    /// Delete an object at a specific index
    Delete {
        /// Path to .pd file
        file: String,

        /// Subpatch depth of object to delete (0 = top-level)
        #[arg(long)]
        depth: usize,

        /// Object index to delete (0-based)
        #[arg(long)]
        index: usize,

        /// Overwrite the original file
        #[arg(long)]
        in_place: bool,

        /// Create a .bak backup before modifying
        #[arg(long)]
        backup: bool,

        /// Write output to this file instead of stdout
        #[arg(long, value_name = "PATH")]
        output: Option<String>,
    },

    /// Shift connection indices at a specific depth
    Renumber {
        /// Path to .pd file
        file: String,

        /// Subpatch depth for renumbering (0 = top-level)
        #[arg(long)]
        depth: usize,

        /// Starting index for shift (indices >= this value are shifted)
        #[arg(long)]
        from: usize,

        /// Delta to add to indices (can be negative)
        #[arg(long, allow_hyphen_values = true)]
        delta: i32,

        /// Overwrite the original file
        #[arg(long)]
        in_place: bool,

        /// Create a .bak backup before modifying
        #[arg(long)]
        backup: bool,

        /// Write output to this file instead of stdout
        #[arg(long, value_name = "PATH")]
        output: Option<String>,
    },
}

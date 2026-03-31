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

    /// Find objects with zero connections
    FindOrphans {
        /// File or directory to scan
        target: String,
        #[arg(long)]
        depth: Option<usize>,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        delete: bool,
        #[arg(long)]
        in_place: bool,
        #[arg(long)]
        backup: bool,
        #[arg(long)]
        include_comments: bool,
    },

    /// Find connected display/number widgets
    FindDisplays {
        /// File or directory to scan
        target: String,
        #[arg(long)]
        depth: Option<usize>,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        delete: bool,
        #[arg(long)]
        in_place: bool,
        #[arg(long)]
        backup: bool,
        #[arg(long)]
        include_unconnected: bool,
        #[arg(long)]
        include_labels: bool,
    },

    /// Search objects by type and/or text
    Search {
        /// File or directory to scan
        target: String,
        #[arg(long = "type")]
        obj_type: Option<String>,
        #[arg(long)]
        text: Option<String>,
        #[arg(long)]
        depth: Option<usize>,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        regex: bool,
        #[arg(long)]
        case_sensitive: bool,
    },

    /// List arrays in patches
    Arrays {
        /// File or directory to scan
        target: String,
        #[arg(long)]
        json: bool,
    },

    /// Patch complexity metrics
    Stats {
        /// File or directory to scan
        target: String,
        #[arg(long)]
        json: bool,
    },

    /// Change an object's text in place without affecting index or connections
    Modify {
        file: String,
        #[arg(long)]
        depth: usize,
        #[arg(long)]
        index: usize,
        /// New class and arguments (e.g. "route 1 2 3")
        #[arg(long, value_name = "TEXT")]
        text: String,
        #[arg(long)]
        in_place: bool,
        #[arg(long)]
        backup: bool,
        #[arg(long, value_name = "PATH")]
        output: Option<String>,
    },

    /// Add a patch cord between two objects
    Connect {
        file: String,
        #[arg(long)]
        depth: usize,
        #[arg(long)]
        src: usize,
        #[arg(long)]
        outlet: usize,
        #[arg(long)]
        dst: usize,
        #[arg(long)]
        inlet: usize,
        #[arg(long)]
        in_place: bool,
        #[arg(long)]
        backup: bool,
        #[arg(long, value_name = "PATH")]
        output: Option<String>,
    },

    /// Remove a specific patch cord
    Disconnect {
        file: String,
        #[arg(long)]
        depth: usize,
        #[arg(long)]
        src: usize,
        #[arg(long)]
        outlet: usize,
        #[arg(long)]
        dst: usize,
        #[arg(long)]
        inlet: usize,
        #[arg(long)]
        in_place: bool,
        #[arg(long)]
        backup: bool,
        #[arg(long, value_name = "PATH")]
        output: Option<String>,
    },

    /// List all connections to/from a specific object
    Connections {
        file: String,
        #[arg(long)]
        index: usize,
        #[arg(long, default_value = "0")]
        depth: usize,
        #[arg(long)]
        json: bool,
    },

    /// Rename send/receive pairs atomically
    RenameSend {
        target: String,
        #[arg(long)]
        from: String,
        #[arg(long)]
        to: String,
        #[arg(long)]
        in_place: bool,
        #[arg(long)]
        backup: bool,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        force: bool,
    },
}

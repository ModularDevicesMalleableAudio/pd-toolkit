use clap::{Parser, Subcommand};
use clap_complete::Shell;

// Top-level CLI

const AFTER_HELP: &str = "\
COMMANDS BY CATEGORY

  Inspection:
    parse         Parse a .pd file and print summary statistics
    list          List objects with indices and details
    validate      Check patch structure and connection integrity
    lint          Combined validation and layout style checks
    stats         Patch complexity metrics
    connections   List all connections to/from a specific object
    arrays        List all PD arrays defined in patches

  Search & Analysis:
    search        Find objects by class name or text pattern
    find-orphans  Find objects with zero connections
    find-displays Find connected number/display boxes
    trace         Trace message/signal path between objects
    diff          Structural diff between two patches
    deps          List abstraction dependencies

  Editing:
    insert        Insert an object with automatic renumbering
    delete        Delete an object with automatic renumbering
    modify        Change an object's text in place
    connect       Add a patch cord
    disconnect    Remove a patch cord
    renumber      Manually shift connection indices
    rename-send   Rename send/receive pairs across files

  Layout & Visualization:
    format        Auto-reposition objects (coordinates only)

  Subpatch Operations:
    extract       Extract a subpatch into a standalone abstraction

  Utilities:
    batch         Apply a command recursively across .pd files
    completions   Generate shell completion scripts

Run `pdtk <COMMAND> --help` for detailed usage and examples.";

#[derive(Debug, Parser)]
#[command(
    name = "pdtk",
    version,
    about = "Safe parser, editor, and formatter for Pure Data .pd patch files",
    long_about = "pdtk is a command-line tool for safely parsing, inspecting, editing,\n\
                  validating, and auto-formatting Pure Data (.pd) patch files without\n\
                  breaking connections.\n\n\
                  Every mutating command validates the result before writing, and\n\
                  all commands are safe to run on production patches.",
    after_long_help = AFTER_HELP
)]
pub struct Cli {
    /// Enable verbose output
    #[arg(long, global = true, help = "Enable verbose output")]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

// Subcommands

#[derive(Debug, Subcommand)]
pub enum Commands {
    // Inspection
    /// Parse a .pd file and print summary statistics
    #[command(
        long_about = "Parse a Pure Data patch file into its structural components and report\n\
                      total object count, connection count, subpatch depth, canvas count,\n\
                      and any parse warnings (e.g. unterminated entries).",
        after_long_help = "EXAMPLES:\n    pdtk parse patch.pd\n    \
                           pdtk parse --json patch.pd | jq .objects\n    \
                           pdtk parse --verbose patch.pd"
    )]
    Parse {
        /// Path to the .pd file
        file: String,
        /// Output results as JSON
        #[arg(long, help = "Output results as JSON")]
        json: bool,
        /// Write re-serialised .pd to this path (proves round-trip fidelity)
        #[arg(long, value_name = "PATH", help = "Write re-serialised .pd to file")]
        output: Option<String>,
    },

    /// List objects with indices, class, and coordinates
    #[command(
        long_about = "List every indexed object in the patch — or only those at a given\n\
                      depth — with its [depth:index] address, class name, and arguments.\n\
                      #X restore entries appear at the parent depth as class 'restore'.\n\
                      Standalone #X declare and #X f width-hint entries are not objects\n\
                      and do not appear in the listing.",
        after_long_help = "EXAMPLES:\n    pdtk list patch.pd\n    \
                           pdtk list patch.pd --depth 1\n    \
                           pdtk list patch.pd --json | jq '.[].class'"
    )]
    List {
        /// Path to the .pd file
        file: String,
        /// Only list objects at this depth (0 = top-level)
        #[arg(long, help = "Filter to a specific depth (0 = top-level)")]
        depth: Option<usize>,
        /// Output results as JSON
        #[arg(long, help = "Output results as JSON")]
        json: bool,
        /// Write output to this file instead of stdout
        #[arg(long, value_name = "PATH", help = "Write output to file")]
        output: Option<String>,
    },

    /// Check patch structure and connection integrity
    #[command(
        long_about = "Validate a patch file by checking that:\n\
                      - All #X connect src and dst indices are in range\n\
                      - #N canvas / #X restore pairs are balanced\n\
                      - No malformed connect entries exist\n\
                      With --strict, duplicate connections are also reported as warnings.\n\
                      Exits 0 on success, 1 on validation errors, 2 on parse failures.",
        after_long_help = "EXAMPLES:\n    pdtk validate patch.pd\n    \
                           pdtk validate --strict patch.pd\n    \
                           pdtk validate --json patch.pd | jq .errors"
    )]
    Validate {
        /// Path to the .pd file
        file: String,
        /// Warn on duplicate connections (in addition to core checks)
        #[arg(long, help = "Also warn on duplicate connections")]
        strict: bool,
        /// Output results as JSON
        #[arg(long, help = "Output results as JSON")]
        json: bool,
        /// Write output to this file instead of stdout
        #[arg(long, value_name = "PATH", help = "Write output to file")]
        output: Option<String>,
    },

    /// Combined validate + layout style checks
    #[command(
        long_about = "Run structural validation (same checks as 'validate') plus layout\n\
                      style analysis: detects bounding-box overlaps between objects at\n\
                      the same Y coordinate.\n\
                      Exits 0 if the patch is structurally valid (style issues are\n\
                      informational). Exits 1 on structural errors.",
        after_long_help = "EXAMPLES:\n    pdtk lint patch.pd\n    \
                           pdtk lint --json patch.pd | jq .style"
    )]
    Lint {
        /// Path to the .pd file
        file: String,
        /// Output results as JSON (errors, warnings, and style fields)
        #[arg(long, help = "Output results as JSON")]
        json: bool,
    },

    /// Patch complexity metrics
    #[command(
        long_about = "Report per-file and aggregate metrics:\n\
                      object count, connection count, max subpatch depth,\n\
                      class histogram (most-used objects), max fan-in / fan-out,\n\
                      orphan count, display object count, and array count.\n\
                      In directory mode, aggregates across all .pd files found.",
        after_long_help = "EXAMPLES:\n    pdtk stats patch.pd\n    \
                           pdtk stats src/ --json | jq .total_objects"
    )]
    Stats {
        /// File or directory to scan
        target: String,
        /// Output results as JSON
        #[arg(long, help = "Output results as JSON")]
        json: bool,
    },

    /// List all connections to/from a specific object
    #[command(
        long_about = "Show every patch cord that connects to or from the object at\n\
                      --index I at --depth N. Results are grouped into Inlets\n\
                      (what feeds into this object) and Outlets (what this object\n\
                      feeds into), with the connected object's raw text.",
        after_long_help = "EXAMPLES:\n    pdtk connections patch.pd --index 3\n    \
                           pdtk connections patch.pd --index 3 --depth 1 --json"
    )]
    Connections {
        /// Path to the .pd file
        file: String,
        /// Object index to inspect
        #[arg(long, help = "Object index (0-based)")]
        index: usize,
        /// Subpatch depth (0 = top-level)
        #[arg(long, default_value = "0", help = "Subpatch depth (0 = top-level)")]
        depth: usize,
        /// Output results as JSON
        #[arg(long, help = "Output results as JSON")]
        json: bool,
    },

    /// List all PD arrays defined in patches
    #[command(
        long_about = "Scan one file or an entire directory tree for #X array definitions\n\
                      and report each array's name, size, and source file.\n\
                      In directory mode, also detects duplicate array names across files.",
        after_long_help = "EXAMPLES:\n    pdtk arrays patch.pd\n    \
                           pdtk arrays src/ --json | jq .arrays"
    )]
    Arrays {
        /// File or directory to scan
        target: String,
        /// Output results as JSON
        #[arg(long, help = "Output results as JSON")]
        json: bool,
    },

    // Search & Analysis
    /// Find objects by type and/or text pattern
    #[command(
        long_about = "Search for objects matching a class name (--type) and/or a text\n\
                      pattern (--text). Patterns are glob syntax by default; use --regex\n\
                      for regular expressions. Matching is case-insensitive by default.\n\
                      Works on a single file or recursively across a directory.",
        after_long_help = "EXAMPLES:\n    pdtk search patch.pd --type route\n    \
                           pdtk search src/ --type send --text \"clock_*\"\n    \
                           pdtk search patch.pd --regex --text \"trig_\\\\d+\""
    )]
    Search {
        /// File or directory to scan
        target: String,
        /// Match objects of this class (e.g. route, s, nbx)
        #[arg(long = "type", help = "Filter by object class name")]
        obj_type: Option<String>,
        /// Match objects whose class+args matches this pattern
        #[arg(long, help = "Text pattern (glob by default, see --regex)")]
        text: Option<String>,
        /// Only search at this depth (0 = top-level)
        #[arg(long, help = "Filter to a specific depth")]
        depth: Option<usize>,
        /// Output results as JSON
        #[arg(long, help = "Output results as JSON")]
        json: bool,
        /// Treat --text as a regular expression
        #[arg(long, help = "Treat --text as a regular expression")]
        regex: bool,
        /// Make --type and --text matching case-sensitive
        #[arg(long, help = "Case-sensitive matching (default: case-insensitive)")]
        case_sensitive: bool,
    },

    /// Find objects with zero connections
    #[command(
        long_about = "Report all objects that are not referenced by any #X connect line\n\
                      at their depth. #X text (comment) entries and standalone #X declare\n\
                      entries are excluded by default.\n\
                      With --delete --in-place, orphans are removed and connections\n\
                      renumbered. The result is validated before writing.",
        after_long_help = "EXAMPLES:\n    pdtk find-orphans patch.pd\n    \
                           pdtk find-orphans src/ --json\n    \
                           pdtk find-orphans patch.pd --delete --in-place --backup"
    )]
    FindOrphans {
        /// File or directory to scan
        target: String,
        /// Only report orphans at this depth
        #[arg(long, help = "Filter to a specific depth")]
        depth: Option<usize>,
        /// Output results as JSON
        #[arg(long, help = "Output results as JSON")]
        json: bool,
        /// Remove orphan objects (requires --in-place)
        #[arg(long, help = "Remove orphan objects (requires --in-place)")]
        delete: bool,
        /// Overwrite the original file
        #[arg(long, help = "Write changes back to the original file")]
        in_place: bool,
        /// Create a .bak backup before modifying
        #[arg(long, help = "Create a .bak backup before overwriting")]
        backup: bool,
        /// Also report #X text (comment) entries as orphans
        #[arg(long, help = "Include comment entries in orphan results")]
        include_comments: bool,
    },

    /// Find connected number/display boxes (debug artifacts)
    #[command(
        long_about = "Find floatatom, symbolatom, nbx, and vu objects that have at least\n\
                      one connection — typically leftover debug displays.\n\
                      With --include-unconnected, reports all display objects regardless\n\
                      of connection status.\n\
                      With --delete --in-place, found objects and their connections are\n\
                      removed and renumbered.",
        after_long_help = "EXAMPLES:\n    pdtk find-displays patch.pd\n    \
                           pdtk find-displays patch.pd --include-unconnected\n    \
                           pdtk find-displays patch.pd --delete --in-place"
    )]
    FindDisplays {
        /// File or directory to scan
        target: String,
        /// Only report displays at this depth
        #[arg(long, help = "Filter to a specific depth")]
        depth: Option<usize>,
        /// Output results as JSON
        #[arg(long, help = "Output results as JSON")]
        json: bool,
        /// Remove display objects (requires --in-place)
        #[arg(long, help = "Remove display objects (requires --in-place)")]
        delete: bool,
        /// Overwrite the original file
        #[arg(long, help = "Write changes back to the original file")]
        in_place: bool,
        /// Create a .bak backup before modifying
        #[arg(long, help = "Create a .bak backup before overwriting")]
        backup: bool,
        /// Also report unconnected display objects (default: connected only)
        #[arg(long, help = "Include unconnected display objects")]
        include_unconnected: bool,
        /// Also report cnv (canvas/label) objects
        #[arg(long, help = "Include cnv label objects")]
        include_labels: bool,
    },

    /// Trace message/signal path forward from an object
    #[command(
        long_about = "Follow all outgoing connections from --from I using BFS, reporting\n\
                      every reachable downstream object and the hop count.\n\
                      With --to J, finds the shortest path between two objects.\n\
                      Cycle detection prevents infinite loops on feedback patches.\n\
                      --max-hops limits traversal depth.",
        after_long_help = "EXAMPLES:\n    pdtk trace patch.pd --from 0\n    \
                           pdtk trace patch.pd --from 0 --to 5\n    \
                           pdtk trace patch.pd --from 0 --max-hops 3 --json"
    )]
    Trace {
        /// Path to the .pd file
        file: String,
        /// Start object index
        #[arg(long, help = "Start object index (0-based)")]
        from: usize,
        /// Target object index for path finding
        #[arg(long, help = "Target object index (enables path-finding mode)")]
        to: Option<usize>,
        /// Depth to trace (0 = top-level)
        #[arg(long, default_value = "0", help = "Subpatch depth (0 = top-level)")]
        depth: usize,
        /// Maximum number of hops to follow
        #[arg(long, help = "Stop after this many hops")]
        max_hops: Option<usize>,
        /// Output results as JSON
        #[arg(long, help = "Output results as JSON")]
        json: bool,
    },

    /// Structural diff between two patches
    #[command(
        long_about = "Compare two .pd files structurally, reporting objects added, removed,\n\
                      or modified, and connections added or removed.\n\
                      Object matching uses LCS on class+args (coordinates are always\n\
                      stripped for matching). With --ignore-coords, coordinate-only\n\
                      changes are suppressed — essential when comparing before/after format.",
        after_long_help = "EXAMPLES:\n    pdtk diff old.pd new.pd\n    \
                           pdtk diff old.pd new.pd --ignore-coords\n    \
                           pdtk diff old.pd new.pd --json | jq .objects_added"
    )]
    Diff {
        /// First .pd file (base)
        file_a: String,
        /// Second .pd file (modified)
        file_b: String,
        /// Output results as JSON
        #[arg(long, help = "Output results as JSON")]
        json: bool,
        /// Suppress coordinate-only object changes
        #[arg(long, help = "Treat coordinate-only changes as identical")]
        ignore_coords: bool,
    },

    /// List abstraction dependencies
    #[command(
        long_about = "Scan #X obj entries for non-builtin object class names, which are\n\
                      treated as abstraction references. Searches for <name>.pd in the\n\
                      file's own directory and any paths declared with #X declare -path.\n\
                      --missing shows only abstractions whose file cannot be found.\n\
                      --recursive follows found abstractions (circular refs are handled).",
        after_long_help = "EXAMPLES:\n    pdtk deps patch.pd\n    \
                           pdtk deps patch.pd --missing\n    \
                           pdtk deps src/ --recursive --json"
    )]
    Deps {
        /// File or directory to scan
        target: String,
        /// Follow found abstractions recursively
        #[arg(long, help = "Follow found abstractions recursively")]
        recursive: bool,
        /// Only report abstractions whose .pd file cannot be found
        #[arg(long, help = "Only report missing abstractions")]
        missing: bool,
        /// Output results as JSON
        #[arg(long, help = "Output results as JSON")]
        json: bool,
    },

    // Editing
    /// Insert an object with automatic connection renumbering
    #[command(
        long_about = "Insert a new entry at index I in depth N. All connections at that\n\
                      depth whose src or dst >= I are incremented by 1. The result is\n\
                      validated before writing.\n\
                      --entry must be a complete .pd entry string, e.g. '#X obj 50 50 print;'",
        after_long_help = "EXAMPLES:\n    pdtk insert patch.pd --depth 0 --index 2 --entry '#X obj 50 100 bang;'\n    \
                           pdtk insert patch.pd --depth 0 --index 0 --entry '#X obj 50 50 loadbang;' --in-place"
    )]
    Insert {
        /// Path to the .pd file
        file: String,
        /// Depth to insert at (0 = top-level)
        #[arg(long, help = "Depth to insert at (0 = top-level)")]
        depth: usize,
        /// Index to insert before (0-based)
        #[arg(long, help = "Insert before this index (0-based)")]
        index: usize,
        /// Raw entry text to insert, e.g. '#X obj 50 50 print;'
        #[arg(long, value_name = "TEXT", help = "Raw .pd entry to insert")]
        entry: String,
        /// Overwrite the original file
        #[arg(long, help = "Write changes back to the original file")]
        in_place: bool,
        /// Create a .bak backup before modifying
        #[arg(long, help = "Create a .bak backup before overwriting")]
        backup: bool,
        /// Write output to this file instead of stdout
        #[arg(long, value_name = "PATH", help = "Write output to file")]
        output: Option<String>,
    },

    /// Delete an object with automatic connection renumbering
    #[command(
        long_about = "Remove the object at index I in depth N. All connections that\n\
                      reference index I are deleted. Remaining connections with src/dst > I\n\
                      are decremented by 1. The result is validated before writing.",
        after_long_help = "EXAMPLES:\n    pdtk delete patch.pd --depth 0 --index 3\n    \
                           pdtk delete patch.pd --depth 1 --index 0 --in-place --backup"
    )]
    Delete {
        /// Path to the .pd file
        file: String,
        /// Depth of the object to delete (0 = top-level)
        #[arg(long, help = "Depth of the object (0 = top-level)")]
        depth: usize,
        /// Index of the object to delete (0-based)
        #[arg(long, help = "Object index to delete (0-based)")]
        index: usize,
        /// Overwrite the original file
        #[arg(long, help = "Write changes back to the original file")]
        in_place: bool,
        /// Create a .bak backup before modifying
        #[arg(long, help = "Create a .bak backup before overwriting")]
        backup: bool,
        /// Write output to this file instead of stdout
        #[arg(long, value_name = "PATH", help = "Write output to file")]
        output: Option<String>,
    },

    /// Change an object's text in place without affecting index or connections
    #[command(
        long_about = "Replace the class and arguments of the object at --depth N --index I\n\
                      with --text. The X/Y coordinates and all connections are preserved.\n\
                      A warning is printed to stderr if the new object has fewer outlets\n\
                      than existing connections reference.",
        after_long_help = "EXAMPLES:\n    pdtk modify patch.pd --depth 0 --index 3 --text 'route 1 2 3'\n    \
                           pdtk modify patch.pd --depth 0 --index 0 --text 'metro 250' --in-place"
    )]
    Modify {
        /// Path to the .pd file
        file: String,
        /// Depth of the object (0 = top-level)
        #[arg(long, help = "Depth of the object (0 = top-level)")]
        depth: usize,
        /// Index of the object to modify (0-based)
        #[arg(long, help = "Object index to modify (0-based)")]
        index: usize,
        /// New class and arguments, e.g. 'route 1 2 3'
        #[arg(long, value_name = "TEXT", help = "New class + arguments")]
        text: String,
        /// Overwrite the original file
        #[arg(long, help = "Write changes back to the original file")]
        in_place: bool,
        /// Create a .bak backup before modifying
        #[arg(long, help = "Create a .bak backup before overwriting")]
        backup: bool,
        /// Write output to this file instead of stdout
        #[arg(long, value_name = "PATH", help = "Write output to file")]
        output: Option<String>,
    },

    /// Add a patch cord between two objects
    #[command(
        long_about = "Add a new #X connect entry between src and dst at the given depth.\n\
                      Duplicate connections and out-of-range indices are refused.\n\
                      The new cord is inserted after the last existing connection at\n\
                      that depth.",
        after_long_help = "EXAMPLES:\n    pdtk connect patch.pd --depth 0 --src 0 --outlet 0 --dst 2 --inlet 0\n    \
                           pdtk connect patch.pd --depth 0 --src 1 --outlet 0 --dst 3 --inlet 0 --in-place"
    )]
    Connect {
        /// Path to the .pd file
        file: String,
        /// Depth of the connection (0 = top-level)
        #[arg(long, help = "Depth (0 = top-level)")]
        depth: usize,
        /// Source object index
        #[arg(long, help = "Source object index (0-based)")]
        src: usize,
        /// Source outlet number
        #[arg(long, help = "Source outlet number (0-based)")]
        outlet: usize,
        /// Destination object index
        #[arg(long, help = "Destination object index (0-based)")]
        dst: usize,
        /// Destination inlet number
        #[arg(long, help = "Destination inlet number (0-based)")]
        inlet: usize,
        /// Overwrite the original file
        #[arg(long, help = "Write changes back to the original file")]
        in_place: bool,
        /// Create a .bak backup before modifying
        #[arg(long, help = "Create a .bak backup before overwriting")]
        backup: bool,
        /// Write output to this file instead of stdout
        #[arg(long, value_name = "PATH", help = "Write output to file")]
        output: Option<String>,
    },

    /// Remove a specific patch cord
    #[command(
        long_about = "Remove the exact #X connect entry matching src outlet dst inlet at\n\
                      the given depth. Exits with code 2 if the connection does not exist.",
        after_long_help = "EXAMPLES:\n    pdtk disconnect patch.pd --depth 0 --src 0 --outlet 0 --dst 1 --inlet 0\n    \
                           pdtk disconnect patch.pd --depth 0 --src 0 --outlet 0 --dst 1 --inlet 0 --in-place"
    )]
    Disconnect {
        /// Path to the .pd file
        file: String,
        /// Depth of the connection (0 = top-level)
        #[arg(long, help = "Depth (0 = top-level)")]
        depth: usize,
        /// Source object index
        #[arg(long, help = "Source object index (0-based)")]
        src: usize,
        /// Source outlet number
        #[arg(long, help = "Source outlet number (0-based)")]
        outlet: usize,
        /// Destination object index
        #[arg(long, help = "Destination object index (0-based)")]
        dst: usize,
        /// Destination inlet number
        #[arg(long, help = "Destination inlet number (0-based)")]
        inlet: usize,
        /// Overwrite the original file
        #[arg(long, help = "Write changes back to the original file")]
        in_place: bool,
        /// Create a .bak backup before modifying
        #[arg(long, help = "Create a .bak backup before overwriting")]
        backup: bool,
        /// Write output to this file instead of stdout
        #[arg(long, value_name = "PATH", help = "Write output to file")]
        output: Option<String>,
    },

    /// Manually shift connection indices at a specific depth
    #[command(
        long_about = "Shift all connection src/dst indices >= --from by --delta at the\n\
                      given depth. Negative deltas are allowed. The result is validated\n\
                      before writing — out-of-range results are refused.",
        after_long_help = "EXAMPLES:\n    pdtk renumber patch.pd --depth 0 --from 2 --delta 1\n    \
                           pdtk renumber patch.pd --depth 1 --from 3 --delta -1 --in-place"
    )]
    Renumber {
        /// Path to the .pd file
        file: String,
        /// Depth of connections to renumber (0 = top-level)
        #[arg(long, help = "Depth (0 = top-level)")]
        depth: usize,
        /// Shift indices >= this value
        #[arg(long, help = "Shift indices >= this value")]
        from: usize,
        /// Amount to add (may be negative)
        #[arg(
            long,
            allow_hyphen_values = true,
            help = "Amount to add to matching indices (may be negative)"
        )]
        delta: i32,
        /// Overwrite the original file
        #[arg(long, help = "Write changes back to the original file")]
        in_place: bool,
        /// Create a .bak backup before modifying
        #[arg(long, help = "Create a .bak backup before overwriting")]
        backup: bool,
        /// Write output to this file instead of stdout
        #[arg(long, value_name = "PATH", help = "Write output to file")]
        output: Option<String>,
    },

    /// Rename send/receive pairs atomically across files
    #[command(
        long_about = "Rename all occurrences of a send/receive name across one file or a\n\
                      directory tree. Handles s/r, s~/r~, throw~/catch~, and embedded\n\
                      send/receive fields in GUI objects (tgl, bng, nbx, vsl, hsl, etc.).\n\
                      Files with no matches are not written (byte-identical guarantee).\n\
                      --dry-run shows matches without modifying. --force overrides the\n\
                      safety check that refuses if the target name already exists.",
        after_long_help = "EXAMPLES:\n    pdtk rename-send patch.pd --from clock_main --to clock_renamed --in-place\n    \
                           pdtk rename-send src/ --from audio_bus --to audio_main --in-place --backup\n    \
                           pdtk rename-send patch.pd --from clock --to new_clock --dry-run"
    )]
    RenameSend {
        /// File or directory to scan
        target: String,
        /// Current send/receive name
        #[arg(long, help = "Current send/receive name to replace")]
        from: String,
        /// New send/receive name
        #[arg(long, help = "New send/receive name")]
        to: String,
        /// Overwrite files in place
        #[arg(long, help = "Write changes back to the original files")]
        in_place: bool,
        /// Create .bak backups before modifying
        #[arg(long, help = "Create a .bak backup before overwriting")]
        backup: bool,
        /// Show what would change without writing
        #[arg(long, help = "Show matches without modifying any files")]
        dry_run: bool,
        /// Allow renaming even if the target name already exists
        #[arg(long, help = "Allow renaming even if --to name already exists")]
        force: bool,
    },

    // Layout & Visualization
    /// Auto-reposition objects (coordinates only, connections untouched)
    #[command(
        long_about = "Recompute X/Y coordinates for every object using topological\n\
                      longest-path layering and barycenter crossing minimisation.\n\
                      Only coordinate fields are ever modified — class, arguments, and\n\
                      connection lines are guaranteed byte-identical before and after.\n\
                      --dry-run prints the result without writing any file.",
        after_long_help = "EXAMPLES:\n    pdtk format patch.pd --in-place\n    \
                           pdtk format patch.pd --dry-run\n    \
                           pdtk format patch.pd --depth 0 --grid 20 --output out.pd"
    )]
    Format {
        /// Path to the .pd file
        file: String,
        /// Only reformat this depth (omit to reformat all depths)
        #[arg(long, help = "Only reformat this depth (omit for all)")]
        depth: Option<usize>,
        /// Grid snap interval in pixels
        #[arg(
            long,
            default_value = "30",
            help = "Grid snap interval in pixels (default: 30)"
        )]
        grid: i32,
        /// Extra horizontal gap between boxes
        #[arg(
            long,
            default_value = "10",
            help = "Extra horizontal gap between boxes (default: 10)"
        )]
        hpad: i32,
        /// Left/top margin in pixels
        #[arg(
            long,
            default_value = "20",
            help = "Left and top margin in pixels (default: 20)"
        )]
        margin: i32,
        /// Print result to stdout without writing any file
        #[arg(long, help = "Print result without writing")]
        dry_run: bool,
        /// Overwrite the original file
        #[arg(long, help = "Write changes back to the original file")]
        in_place: bool,
        /// Create a .bak backup before modifying
        #[arg(long, help = "Create a .bak backup before overwriting")]
        backup: bool,
        /// Write output to this file instead of stdout
        #[arg(long, value_name = "PATH", help = "Write output to file")]
        output: Option<String>,
    },

    // Subpatch Operations
    /// Extract a subpatch into a standalone abstraction file
    #[command(
        long_about = "Find the subpatch at --depth N (the first nested canvas at that\n\
                      level) and write its contents as a standalone .pd abstraction.\n\
                      inlet/outlet objects are added to represent connections that crossed\n\
                      the subpatch boundary in the original.\n\
                      With --in-place, the subpatch block in the source is replaced by\n\
                      a single #X obj referencing the new abstraction by filename stem.\n\
                      Both outputs are validated before writing.",
        after_long_help = "EXAMPLES:\n    pdtk extract patch.pd --depth 1 --output my_abs.pd\n    \
                           pdtk extract patch.pd --depth 1 --output my_abs.pd --in-place --backup"
    )]
    Extract {
        /// Path to the .pd file containing the subpatch
        file: String,
        /// Subpatch depth to extract (1 = first nested canvas)
        #[arg(long, help = "Depth of the subpatch to extract (1 = first nested)")]
        depth: usize,
        /// Path to write the extracted abstraction
        #[arg(
            long,
            value_name = "PATH",
            help = "Output path for the extracted abstraction"
        )]
        output: String,
        /// Replace the subpatch in the source with an abstraction reference
        #[arg(
            long,
            help = "Replace subpatch in source with an abstraction reference"
        )]
        in_place: bool,
        /// Create a .bak backup of the source before modifying
        #[arg(long, help = "Create a .bak backup before overwriting")]
        backup: bool,
    },

    // Utilities
    /// Apply a pdtk command recursively across .pd files in a directory
    #[command(
        long_about = "Run any pdtk subcommand against every .pd file found under <dir>.\n\
                      The file path is appended as the last argument to each invocation.\n\
                      --glob restricts which filenames are processed.\n\
                      --dry-run lists files without executing.\n\
                      --continue-on-error processes all files even when some fail.\n\
                      Exits 1 if any file failed.",
        after_long_help = "EXAMPLES:\n    pdtk batch src/ validate\n    \
                           pdtk batch src/ --glob '*.pd' format --in-place\n    \
                           pdtk batch src/ --dry-run validate\n    \
                           pdtk batch src/ --continue-on-error validate --json"
    )]
    Batch {
        /// Directory to scan for .pd files
        dir: String,
        /// pdtk subcommand and flags to apply to each file (file appended last)
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        command: Vec<String>,
        /// Only process files matching this glob pattern
        #[arg(
            long,
            default_value = "**/*.pd",
            help = "Glob filter for filenames (default: **/*.pd)"
        )]
        glob: String,
        /// List what would be done without running
        #[arg(long, help = "Show what would run without executing")]
        dry_run: bool,
        /// Continue processing after errors
        #[arg(long, help = "Continue after errors (default: stop on first failure)")]
        continue_on_error: bool,
        /// Output results as JSON
        #[arg(long, help = "Output results as JSON")]
        json: bool,
    },

    /// Generate shell completion scripts
    #[command(
        long_about = "Generate tab-completion scripts for the given shell and print to\n\
                      stdout. Redirect into the appropriate location for your shell.",
        after_long_help = "EXAMPLES:\n    \
                           # Bash:\n    \
                           pdtk completions bash > ~/.local/share/bash-completion/completions/pdtk\n\n    \
                           # Zsh:\n    \
                           pdtk completions zsh > ~/.zfunc/_pdtk\n\n    \
                           # Fish:\n    \
                           pdtk completions fish > ~/.config/fish/completions/pdtk.fish"
    )]
    Completions {
        /// Shell to generate completions for
        #[arg(
            value_name = "SHELL",
            help = "Shell: bash, zsh, fish, elvish, powershell"
        )]
        shell: Shell,
    },
}

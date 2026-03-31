/// Build script: generate man pages in man/ using clap_mangen.
///
/// Generates:
///   man/pdtk.1         — top-level man page
///   man/pdtk-<cmd>.1   — one per subcommand
///
/// The man/ directory is listed in .gitignore; it is regenerated on every
/// `cargo build`.  Man pages are always in sync with the --help text because
/// they are derived from the same clap Command definition.
fn main() -> std::io::Result<()> {
    // Tell Cargo to re-run this script only when the CLI source changes.
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/cli.rs");

    let man_dir = std::path::PathBuf::from("man");
    std::fs::create_dir_all(&man_dir)?;

    let cmd = build_root_command();
    write_man_page(&cmd, &man_dir, "pdtk")?;

    for sub in cmd.get_subcommands() {
        let name = format!("pdtk-{}", sub.get_name());
        write_man_page(sub, &man_dir, &name)?;
    }

    Ok(())
}

fn write_man_page(cmd: &clap::Command, dir: &std::path::Path, name: &str) -> std::io::Result<()> {
    // clap_mangen::Man renders roff man-page format.
    let man = clap_mangen::Man::new(cmd.clone());
    let mut buf = Vec::new();
    man.render(&mut buf).map_err(std::io::Error::other)?;
    std::fs::write(dir.join(format!("{name}.1")), buf)
}

// Minimal Command tree — mirrors src/cli.rs without the derive macro so that
// build.rs can compile independently of the binary's error/io modules.
fn build_root_command() -> clap::Command {
    use clap::{Arg, Command};

    Command::new("pdtk")
        .version(env!("CARGO_PKG_VERSION"))
        .about("Safe parser, editor, and formatter for Pure Data .pd patch files")
        .long_about(
            "pdtk is a command-line tool for safely parsing, inspecting, editing,\n\
             validating, and auto-formatting Pure Data (.pd) patch files without\n\
             breaking connections.\n\n\
             Every mutating command validates the result before writing.",
        )
        .arg(
            Arg::new("verbose")
                .long("verbose")
                .global(true)
                .num_args(0)
                .help("Enable verbose output"),
        )
        .subcommand(
            Command::new("parse")
                .about("Parse a .pd file and print summary statistics")
                .arg(Arg::new("file").required(true))
                .arg(Arg::new("json").long("json").num_args(0))
                .arg(Arg::new("output").long("output").value_name("PATH")),
        )
        .subcommand(
            Command::new("list")
                .about("List objects with indices and details")
                .arg(Arg::new("file").required(true))
                .arg(Arg::new("depth").long("depth").value_name("N"))
                .arg(Arg::new("json").long("json").num_args(0))
                .arg(Arg::new("output").long("output").value_name("PATH")),
        )
        .subcommand(
            Command::new("validate")
                .about("Check patch structure and connection integrity")
                .arg(Arg::new("file").required(true))
                .arg(Arg::new("strict").long("strict").num_args(0))
                .arg(Arg::new("json").long("json").num_args(0))
                .arg(Arg::new("output").long("output").value_name("PATH")),
        )
        .subcommand(
            Command::new("lint")
                .about("Combined validation and layout style checks")
                .arg(Arg::new("file").required(true))
                .arg(Arg::new("json").long("json").num_args(0)),
        )
        .subcommand(
            Command::new("stats")
                .about("Patch complexity metrics")
                .arg(Arg::new("target").required(true))
                .arg(Arg::new("json").long("json").num_args(0)),
        )
        .subcommand(
            Command::new("connections")
                .about("List all connections to/from a specific object")
                .arg(Arg::new("file").required(true))
                .arg(
                    Arg::new("index")
                        .long("index")
                        .required(true)
                        .value_name("N"),
                )
                .arg(
                    Arg::new("depth")
                        .long("depth")
                        .value_name("N")
                        .default_value("0"),
                )
                .arg(Arg::new("json").long("json").num_args(0)),
        )
        .subcommand(
            Command::new("arrays")
                .about("List all PD arrays defined in patches")
                .arg(Arg::new("target").required(true))
                .arg(Arg::new("json").long("json").num_args(0)),
        )
        .subcommand(
            Command::new("search")
                .about("Find objects by type and/or text pattern")
                .arg(Arg::new("target").required(true))
                .arg(Arg::new("type").long("type").value_name("CLASS"))
                .arg(Arg::new("text").long("text").value_name("PATTERN"))
                .arg(Arg::new("depth").long("depth").value_name("N"))
                .arg(Arg::new("json").long("json").num_args(0))
                .arg(Arg::new("regex").long("regex").num_args(0))
                .arg(
                    Arg::new("case-sensitive")
                        .long("case-sensitive")
                        .num_args(0),
                ),
        )
        .subcommand(
            Command::new("find-orphans")
                .about("Find objects with zero connections")
                .arg(Arg::new("target").required(true))
                .arg(Arg::new("depth").long("depth").value_name("N"))
                .arg(Arg::new("json").long("json").num_args(0))
                .arg(Arg::new("delete").long("delete").num_args(0))
                .arg(Arg::new("in-place").long("in-place").num_args(0))
                .arg(Arg::new("backup").long("backup").num_args(0))
                .arg(
                    Arg::new("include-comments")
                        .long("include-comments")
                        .num_args(0),
                ),
        )
        .subcommand(
            Command::new("find-displays")
                .about("Find connected number/display boxes")
                .arg(Arg::new("target").required(true))
                .arg(Arg::new("depth").long("depth").value_name("N"))
                .arg(Arg::new("json").long("json").num_args(0))
                .arg(Arg::new("delete").long("delete").num_args(0))
                .arg(Arg::new("in-place").long("in-place").num_args(0))
                .arg(Arg::new("backup").long("backup").num_args(0))
                .arg(
                    Arg::new("include-unconnected")
                        .long("include-unconnected")
                        .num_args(0),
                )
                .arg(
                    Arg::new("include-labels")
                        .long("include-labels")
                        .num_args(0),
                ),
        )
        .subcommand(
            Command::new("trace")
                .about("Trace message/signal path forward from an object")
                .arg(Arg::new("file").required(true))
                .arg(Arg::new("from").long("from").required(true).value_name("N"))
                .arg(Arg::new("to").long("to").value_name("N"))
                .arg(
                    Arg::new("depth")
                        .long("depth")
                        .value_name("N")
                        .default_value("0"),
                )
                .arg(Arg::new("max-hops").long("max-hops").value_name("N"))
                .arg(Arg::new("json").long("json").num_args(0)),
        )
        .subcommand(
            Command::new("diff")
                .about("Structural diff between two patches")
                .arg(Arg::new("file_a").required(true))
                .arg(Arg::new("file_b").required(true))
                .arg(Arg::new("json").long("json").num_args(0))
                .arg(Arg::new("ignore-coords").long("ignore-coords").num_args(0)),
        )
        .subcommand(
            Command::new("deps")
                .about("List abstraction dependencies")
                .arg(Arg::new("target").required(true))
                .arg(Arg::new("recursive").long("recursive").num_args(0))
                .arg(Arg::new("missing").long("missing").num_args(0))
                .arg(Arg::new("json").long("json").num_args(0)),
        )
        .subcommand(
            Command::new("insert")
                .about("Insert an object with automatic connection renumbering")
                .arg(Arg::new("file").required(true))
                .arg(
                    Arg::new("depth")
                        .long("depth")
                        .required(true)
                        .value_name("N"),
                )
                .arg(
                    Arg::new("index")
                        .long("index")
                        .required(true)
                        .value_name("N"),
                )
                .arg(
                    Arg::new("entry")
                        .long("entry")
                        .required(true)
                        .value_name("TEXT"),
                )
                .arg(Arg::new("in-place").long("in-place").num_args(0))
                .arg(Arg::new("backup").long("backup").num_args(0))
                .arg(Arg::new("output").long("output").value_name("PATH")),
        )
        .subcommand(
            Command::new("delete")
                .about("Delete an object with automatic connection renumbering")
                .arg(Arg::new("file").required(true))
                .arg(
                    Arg::new("depth")
                        .long("depth")
                        .required(true)
                        .value_name("N"),
                )
                .arg(
                    Arg::new("index")
                        .long("index")
                        .required(true)
                        .value_name("N"),
                )
                .arg(Arg::new("in-place").long("in-place").num_args(0))
                .arg(Arg::new("backup").long("backup").num_args(0))
                .arg(Arg::new("output").long("output").value_name("PATH")),
        )
        .subcommand(
            Command::new("modify")
                .about("Change an object's text in place without affecting connections")
                .arg(Arg::new("file").required(true))
                .arg(
                    Arg::new("depth")
                        .long("depth")
                        .required(true)
                        .value_name("N"),
                )
                .arg(
                    Arg::new("index")
                        .long("index")
                        .required(true)
                        .value_name("N"),
                )
                .arg(
                    Arg::new("text")
                        .long("text")
                        .required(true)
                        .value_name("TEXT"),
                )
                .arg(Arg::new("in-place").long("in-place").num_args(0))
                .arg(Arg::new("backup").long("backup").num_args(0))
                .arg(Arg::new("output").long("output").value_name("PATH")),
        )
        .subcommand(
            Command::new("connect")
                .about("Add a patch cord between two objects")
                .arg(Arg::new("file").required(true))
                .arg(
                    Arg::new("depth")
                        .long("depth")
                        .required(true)
                        .value_name("N"),
                )
                .arg(Arg::new("src").long("src").required(true).value_name("N"))
                .arg(
                    Arg::new("outlet")
                        .long("outlet")
                        .required(true)
                        .value_name("N"),
                )
                .arg(Arg::new("dst").long("dst").required(true).value_name("N"))
                .arg(
                    Arg::new("inlet")
                        .long("inlet")
                        .required(true)
                        .value_name("N"),
                )
                .arg(Arg::new("in-place").long("in-place").num_args(0))
                .arg(Arg::new("backup").long("backup").num_args(0))
                .arg(Arg::new("output").long("output").value_name("PATH")),
        )
        .subcommand(
            Command::new("disconnect")
                .about("Remove a specific patch cord")
                .arg(Arg::new("file").required(true))
                .arg(
                    Arg::new("depth")
                        .long("depth")
                        .required(true)
                        .value_name("N"),
                )
                .arg(Arg::new("src").long("src").required(true).value_name("N"))
                .arg(
                    Arg::new("outlet")
                        .long("outlet")
                        .required(true)
                        .value_name("N"),
                )
                .arg(Arg::new("dst").long("dst").required(true).value_name("N"))
                .arg(
                    Arg::new("inlet")
                        .long("inlet")
                        .required(true)
                        .value_name("N"),
                )
                .arg(Arg::new("in-place").long("in-place").num_args(0))
                .arg(Arg::new("backup").long("backup").num_args(0))
                .arg(Arg::new("output").long("output").value_name("PATH")),
        )
        .subcommand(
            Command::new("renumber")
                .about("Manually shift connection indices at a specific depth")
                .arg(Arg::new("file").required(true))
                .arg(
                    Arg::new("depth")
                        .long("depth")
                        .required(true)
                        .value_name("N"),
                )
                .arg(Arg::new("from").long("from").required(true).value_name("N"))
                .arg(
                    Arg::new("delta")
                        .long("delta")
                        .required(true)
                        .allow_hyphen_values(true)
                        .value_name("D"),
                )
                .arg(Arg::new("in-place").long("in-place").num_args(0))
                .arg(Arg::new("backup").long("backup").num_args(0))
                .arg(Arg::new("output").long("output").value_name("PATH")),
        )
        .subcommand(
            Command::new("rename-send")
                .about("Rename send/receive pairs atomically across files")
                .arg(Arg::new("target").required(true))
                .arg(
                    Arg::new("from")
                        .long("from")
                        .required(true)
                        .value_name("NAME"),
                )
                .arg(Arg::new("to").long("to").required(true).value_name("NAME"))
                .arg(Arg::new("in-place").long("in-place").num_args(0))
                .arg(Arg::new("backup").long("backup").num_args(0))
                .arg(Arg::new("dry-run").long("dry-run").num_args(0))
                .arg(Arg::new("force").long("force").num_args(0)),
        )
        .subcommand(
            Command::new("format")
                .about("Auto-reposition objects (coordinates only, connections untouched)")
                .arg(Arg::new("file").required(true))
                .arg(Arg::new("depth").long("depth").value_name("N"))
                .arg(
                    Arg::new("grid")
                        .long("grid")
                        .value_name("N")
                        .default_value("30"),
                )
                .arg(
                    Arg::new("hpad")
                        .long("hpad")
                        .value_name("N")
                        .default_value("10"),
                )
                .arg(
                    Arg::new("margin")
                        .long("margin")
                        .value_name("N")
                        .default_value("20"),
                )
                .arg(Arg::new("dry-run").long("dry-run").num_args(0))
                .arg(Arg::new("in-place").long("in-place").num_args(0))
                .arg(Arg::new("backup").long("backup").num_args(0))
                .arg(Arg::new("output").long("output").value_name("PATH")),
        )
        .subcommand(
            Command::new("extract")
                .about("Extract a subpatch into a standalone abstraction file")
                .arg(Arg::new("file").required(true))
                .arg(
                    Arg::new("depth")
                        .long("depth")
                        .required(true)
                        .value_name("N"),
                )
                .arg(
                    Arg::new("output")
                        .long("output")
                        .required(true)
                        .value_name("PATH"),
                )
                .arg(Arg::new("in-place").long("in-place").num_args(0))
                .arg(Arg::new("backup").long("backup").num_args(0)),
        )
        .subcommand(
            Command::new("batch")
                .about("Apply a pdtk command recursively across .pd files in a directory")
                .arg(Arg::new("dir").required(true))
                .arg(Arg::new("command").num_args(0..).trailing_var_arg(true))
                .arg(
                    Arg::new("glob")
                        .long("glob")
                        .value_name("PATTERN")
                        .default_value("**/*.pd"),
                )
                .arg(Arg::new("dry-run").long("dry-run").num_args(0))
                .arg(
                    Arg::new("continue-on-error")
                        .long("continue-on-error")
                        .num_args(0),
                )
                .arg(Arg::new("json").long("json").num_args(0)),
        )
        .subcommand(
            Command::new("completions")
                .about("Generate shell completion scripts")
                .arg(
                    Arg::new("shell")
                        .required(true)
                        .value_name("SHELL")
                        .help("Shell: bash, zsh, fish, elvish, powershell"),
                ),
        )
}

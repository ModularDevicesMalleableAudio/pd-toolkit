use crate::model::EntryKind;
use crate::parser::parse;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Source of a builtin classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum BuiltinSource {
    /// Always-loaded vanilla core class.
    Core,
    /// Ships with pd-vanilla in `extra/`; requires `declare -lib` or
    /// `-stdpath extra` at runtime, but reported here as found.
    CoreExtra,
}

/// True if `name` is a known vanilla class (core or extra).
#[must_use]
pub fn is_builtin(name: &str) -> bool {
    builtin_source(name).is_some()
}

/// Classify a known builtin name. Returns `None` for unknown names.
#[must_use]
pub fn builtin_source(name: &str) -> Option<BuiltinSource> {
    if CORE.contains(name) {
        Some(BuiltinSource::Core)
    } else if EXTRA.contains(name) {
        Some(BuiltinSource::CoreExtra)
    } else {
        None
    }
}

const CORE_NAMES: &[&str] = &[
    // Core control
    "bang",
    "b",
    "float",
    "f",
    "symbol",
    "int",
    "i",
    "list",
    "anything",
    "send",
    "s",
    "receive",
    "r",
    "s~",
    "send~",
    "r~",
    "receive~",
    "throw~",
    "catch~",
    "trigger",
    "t",
    "pack",
    "unpack",
    "route",
    "select",
    "sel",
    "moses",
    "spigot",
    "gate",
    "swap",
    "change",
    "until",
    "print",
    "netsend",
    "netreceive",
    // Math
    "+",
    "-",
    "*",
    "/",
    "%",
    "pow",
    "log",
    "exp",
    "sqrt",
    "abs",
    "cos",
    "sin",
    "tan",
    "atan",
    "atan2",
    "max",
    "min",
    "clip",
    "wrap",
    "mod",
    "div",
    "floor",
    "ceil",
    "rint",
    ">",
    "<",
    ">=",
    "<=",
    "==",
    "!=",
    // Sequencing / timing
    "metro",
    "delay",
    "pipe",
    "timer",
    "cputime",
    "realtime",
    "line",
    "vline",
    "random",
    "seed",
    "counter",
    "makesym",
    "makefilename",
    // Data
    "table",
    "array",
    "tabread",
    "tabwrite",
    "tabread4",
    "text",
    "qlist",
    "textfile",
    "openpanel",
    "savepanel",
    "soundfiler",
    "writesf~",
    "readsf~",
    // Structure
    "loadbang",
    "bang~",
    "value",
    "bag",
    "struct",
    "draw",
    "drawpolygon",
    "filledpolygon",
    "drawcurve",
    "filledcurve",
    "plot",
    "pointer",
    "get",
    "set",
    "getsize",
    "setsize",
    "append",
    "element",
    "sublist",
    "listfunnel",
    // DSP core
    "dac~",
    "adc~",
    "osc~",
    "phasor~",
    "noise~",
    "sig~",
    "snapshot~",
    "samphold~",
    "env~",
    "line~",
    "vline~",
    "threshold~",
    "samplerate~",
    "blocksize~",
    "block~",
    "switch~",
    // DSP math
    "+~",
    "-~",
    "*~",
    "/~",
    ">~",
    "<~",
    ">=~",
    "<=~",
    "==~",
    "!=~",
    "abs~",
    "sqrt~",
    "wrap~",
    "clip~",
    "cos~",
    "log~",
    "exp~",
    "pow~",
    "max~",
    "min~",
    // Filters
    "hip~",
    "lop~",
    "bp~",
    "vcf~",
    "rzero~",
    "rzero_rev~",
    "rpole~",
    "czero~",
    "czero_rev~",
    "cpole~",
    // Delay
    "delwrite~",
    "delread~",
    "vd~",
    // FFT
    "fft~",
    "ifft~",
    "rfft~",
    "rifft~",
    "framp~",
    "rmstodb~",
    "dbtorms~",
    "mtof~",
    "ftom~",
    // Conversion
    "mtof",
    "ftom",
    "rmstodb",
    "dbtorms",
    "powtodb",
    "dbtopow",
    // MIDI I/O
    "notein",
    "noteout",
    "ctlin",
    "ctlout",
    "pgmin",
    "pgmout",
    "bendin",
    "bendout",
    "touchin",
    "touchout",
    "polytouchin",
    "polytouchout",
    "midiin",
    "midiout",
    "midiclkin",
    "midirealtimein",
    "sysexin",
    // Input
    "key",
    "keyup",
    "keyname",
    // GUI
    "bng",
    "tgl",
    "nbx",
    "vsl",
    "hsl",
    "vradio",
    "hradio",
    "hdl",
    "vdl",
    "vu",
    "cnv",
    "floatatom",
    "symbolatom",
    "listbox",
    // expr family
    "expr",
    "expr~",
    "fexpr~",
    // compat aliases
    "v",
    // modern additions
    "clone",
    "namecanvas",
    "scalar",
    "oscparse",
    "oscformat",
    // tabread/tabwrite signal-rate family
    "tabread~",
    "tabread4~",
    "tabwrite~",
    "tabosc4~",
    "tabsend~",
    "tabreceive~",
    "tabplay~",
    // missing control
    "makenote",
    "stripnote",
    "poly",
    "bag",
    // Subpatch / abstraction
    "pd",
    "inlet",
    "outlet",
    "inlet~",
    "outlet~",
    "declare",
    "import",
];

/// pd-vanilla classes shipped in `extra/` (require `declare -lib` or
/// `-stdpath extra` at runtime, but treated as known builtins so `deps`
/// doesn't report them as missing abstractions).
const EXTRA_NAMES: &[&str] = &[
    "bob~",
    "slop~",
    "pdcontrol",
    "fudiformat",
    "fudiparse",
    "savestate",
    "bonk~",
    "choice~",
    "fiddle~",
    "loop~",
    "lrshift~",
    "pique~",
    "sigmund~",
    "stdout",
];

static CORE: std::sync::LazyLock<std::collections::HashSet<&'static str>> =
    std::sync::LazyLock::new(|| CORE_NAMES.iter().copied().collect());

static EXTRA: std::sync::LazyLock<std::collections::HashSet<&'static str>> =
    std::sync::LazyLock::new(|| EXTRA_NAMES.iter().copied().collect());

#[derive(Debug, Clone, serde::Serialize)]
pub struct DepEntry {
    pub file: String,
    pub depth: usize,
    pub index: usize,
    pub name: String,
    pub found: bool,
    pub found_at: Option<String>,
    /// `Some(Core)` if vanilla core, `Some(CoreExtra)` if pd-vanilla `extra/`,
    /// `None` for missing or abstraction-resolved entries.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<BuiltinSource>,
    /// Libraries declared in this canvas (or an ancestor) via
    /// `#X declare -lib`/`-stdlib` or an ELSE-style `[import ...]` object,
    /// which may provide this class at runtime from a monolithic binary that
    /// static analysis cannot introspect.
    ///
    /// Non-empty only for an otherwise-unresolved class when at least one
    /// library is declared; always empty for found or builtin classes. Such
    /// an entry is reported as `unresolved (library declared)` rather than
    /// `MISSING`, and is excluded from `--missing`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub declared_libs: Vec<String>,
}

/// Collect the libraries a canvas loads via `#X declare -lib`/`-stdlib` or an
/// ELSE-style `[import ...]` object, in document order (deduplicated).
///
/// A declared library may register object classes that live inside a single
/// (monolithic) binary — e.g. `-lib zexy` provides `[demux]` — which have no
/// per-class file on disk, so `deps` cannot confirm or deny them. Their
/// presence downgrades an unresolved class from `MISSING` to
/// `unresolved (library declared)`.
fn declared_libraries(entries: &[crate::model::Entry]) -> Vec<String> {
    let mut libs: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    let mut push = |name: &str| {
        let n = name.trim_end_matches(';');
        if !n.is_empty() && seen.insert(n.to_string()) {
            libs.push(n.to_string());
        }
    };
    for e in entries {
        match e.kind {
            EntryKind::Declare => {
                let toks: Vec<&str> = e.raw.trim_end_matches(';').split_whitespace().collect();
                let mut i = 2; // skip `#X declare`
                while i < toks.len() {
                    if (toks[i] == "-lib" || toks[i] == "-stdlib") && i + 1 < toks.len() {
                        push(toks[i + 1]);
                        i += 2;
                    } else {
                        i += 1;
                    }
                }
            }
            // ELSE's `[import lib ...]` loads one or more library namespaces.
            EntryKind::Obj if e.class() == "import" => {
                for a in e.args() {
                    push(&a);
                }
            }
            _ => {}
        }
    }
    libs
}

/// Collect abstraction search paths from `#X declare -path dir` entries in a file.
///
/// Paths are resolved relative to the patch file's directory, matching Pd's
/// behavior. The trailing `;` entry terminator is stripped from path tokens.
fn declare_paths(file: &Path, content: &str) -> Vec<PathBuf> {
    let mut paths = Vec::new();
    let base = file.parent().unwrap_or(Path::new("."));
    for line in content.lines() {
        let t = line.trim();
        if t.starts_with("#X declare") {
            let parts: Vec<&str> = t.trim_end_matches(';').split_whitespace().collect();
            let mut i = 2;
            while i + 1 < parts.len() {
                if parts[i] == "-path" {
                    let raw = parts[i + 1].trim_end_matches(';');
                    if !raw.is_empty() {
                        paths.push(base.join(raw));
                    }
                    i += 2;
                } else {
                    i += 1;
                }
            }
        }
    }
    paths
}

/// File extensions Pd resolves for an object class, in resolution order:
/// compiled externals (platform-specific), Lua externals (pdlua loader), then
/// abstractions (`.pd`, `.pat`).  Mirrors `sys_loadlib_iter` (loaders first,
/// then `sys_do_load_abs`).
fn class_extensions() -> Vec<&'static str> {
    let mut exts: Vec<&'static str> = Vec::new();
    if cfg!(target_os = "linux") {
        exts.extend(["pd_linux", "l_amd64", "l_ia64", "so"]);
    } else if cfg!(target_os = "macos") {
        exts.extend(["pd_darwin", "d_amd64", "d_arm64", "d_fat", "so"]);
    } else if cfg!(target_os = "windows") {
        exts.extend(["dll", "m_amd64"]);
    }
    // Loader-based (pdlua) and abstraction extensions are platform-independent.
    exts.extend(["pd_lua", "pd_luax", "pd", "pat"]);
    exts
}

/// True if `path` is a Pd patch (abstraction) we can meaningfully recurse into.
fn is_patch_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("pd" | "pat")
    )
}

/// Resolve an object class to a patch (abstraction) file relative to `file`.
///
/// Searches the patch's own directory and any `#X declare -path` directories
/// (matching Pd's own-canvas resolution order), then filters to patch files
/// (`.pd`/`.pat`) only — compiled and Lua externals cannot be introspected.
/// `content` is the already-read text of `file` (used to find declares).
#[must_use]
pub fn resolve_abstraction(class: &str, file: &Path, content: &str) -> Option<PathBuf> {
    let base = file.parent().unwrap_or(Path::new("."));
    let mut dirs = vec![base.to_path_buf()];
    dirs.extend(declare_paths(file, content));
    let path = locate_abstraction(class, &dirs)?;
    is_patch_file(&path).then_some(path)
}

/// Count an abstraction's top-level inlets and outlets, returning
/// `(inlets, outlets)`.
///
/// An abstraction's control/signal I/O is fixed by the `inlet`/`inlet~` and
/// `outlet`/`outlet~` objects on its top-level canvas (both rates share one
/// numbering space in Pd). Only depth-0 objects count — inlet/outlet objects
/// nested inside the abstraction's own subpatches belong to those subpatches.
/// Returns `None` if the file cannot be read or parsed.
#[must_use]
pub fn abstraction_io_counts(path: &Path) -> Option<(usize, usize)> {
    let content = std::fs::read_to_string(path).ok()?;
    let patch = parse(&content).ok()?;
    let mut inlets = 0usize;
    let mut outlets = 0usize;
    for e in &patch.entries {
        // Top-level objects live at internal depth 1 (user depth 0).
        if e.kind != EntryKind::Obj || e.depth != 1 {
            continue;
        }
        match e.class() {
            "inlet" | "inlet~" => inlets += 1,
            "outlet" | "outlet~" => outlets += 1,
            _ => {}
        }
    }
    Some((inlets, outlets))
}

/// Search for an object class's implementation file across `search_dirs`.
///
/// For each directory, tries `name.<ext>` and the `name/name.<ext>`
/// class-in-folder convention for every known extension (abstractions, Lua
/// externals, and platform compiled externals).
fn locate_abstraction(name: &str, search_dirs: &[PathBuf]) -> Option<PathBuf> {
    let exts = class_extensions();
    for dir in search_dirs {
        for ext in &exts {
            let direct = dir.join(format!("{name}.{ext}"));
            if direct.exists() {
                return Some(direct);
            }
            let in_folder = dir.join(name).join(format!("{name}.{ext}"));
            if in_folder.exists() {
                return Some(in_folder);
            }
        }
    }
    None
}

/// Analyse one file and collect dependency entries.
/// `visited` prevents re-processing the same file in recursive mode.
pub fn analyse_file(file: &Path, recursive: bool, visited: &mut HashSet<PathBuf>) -> Vec<DepEntry> {
    analyse_file_with_ancestors(file, recursive, visited, &[], &[], &[])
}

/// Like `analyse_file`, but also searches `extra_dirs` as a fallback after
/// the patch's own directory and `#X declare -path` entries.
pub fn analyse_file_with_extra(
    file: &Path,
    recursive: bool,
    visited: &mut HashSet<PathBuf>,
    extra_dirs: &[PathBuf],
) -> Vec<DepEntry> {
    analyse_file_with_ancestors(file, recursive, visited, &[], extra_dirs, &[])
}

/// Internal: analyse a file, honoring ancestor canvases' search directories.
///
/// Matches Pd's `canvas_path_iterate` semantics: when resolving abstraction
/// references, Pd walks the owner chain of canvases, trying each ancestor's
/// own directory and its `#X declare -path` entries. Thus a child abstraction
/// with no declares of its own still resolves references via its caller's
/// declares.
///
/// `ancestor_libs` carries `-lib`/`-stdlib`/`import` declarations from the
/// owner chain: Pd loads libraries process-globally, so a class used in a
/// child abstraction may be provided by a library the caller declared.
fn analyse_file_with_ancestors(
    file: &Path,
    recursive: bool,
    visited: &mut HashSet<PathBuf>,
    ancestor_dirs: &[PathBuf],
    extra_dirs: &[PathBuf],
    ancestor_libs: &[String],
) -> Vec<DepEntry> {
    let canon = match file.canonicalize() {
        Ok(p) => p,
        Err(_) => file.to_path_buf(),
    };
    if !visited.insert(canon.clone()) {
        return Vec::new(); // already processed
    }

    let Ok(content) = std::fs::read_to_string(file) else {
        return Vec::new();
    };
    let Ok(patch) = parse(&content) else {
        return Vec::new();
    };

    let file_dir = file.parent().unwrap_or(Path::new("."));
    let mut own_dirs = vec![file_dir.to_path_buf()];
    own_dirs.extend(declare_paths(file, &content));

    // Search order: this canvas's own dirs first, then each ancestor's dirs
    // (in child→root order). Mirrors Pd's canvas_path_iterate loop.
    let mut search_dirs = own_dirs.clone();
    search_dirs.extend(ancestor_dirs.iter().cloned());
    search_dirs.extend(extra_dirs.iter().cloned());

    // Libraries declared here plus any inherited from the owner chain. An
    // unresolved class is attributed to these rather than reported MISSING.
    let mut all_libs = declared_libraries(&patch.entries);
    all_libs.extend(ancestor_libs.iter().cloned());
    all_libs.dedup();

    let mut results = Vec::new();

    for e in &patch.entries {
        if e.kind != EntryKind::Obj {
            continue;
        }
        let Some(index) = e.object_index else {
            continue;
        };
        let depth = e.depth.saturating_sub(1);
        let class = e.class().to_string();

        // `pd` used as an object class is a subpatch reference, not an
        // abstraction lookup on disk.
        if class == "pd" {
            continue;
        }

        // Core vanilla classes are not reported (would be very noisy).
        // pd-vanilla extra/ classes ARE reported so users can see that a
        // runtime declare may be required.
        let source = builtin_source(&class);
        if matches!(source, Some(BuiltinSource::Core)) {
            continue;
        }

        let location = if source.is_some() {
            None
        } else {
            locate_abstraction(&class, &search_dirs)
        };
        let found = source.is_some() || location.is_some();
        let found_at = location.as_ref().map(|p| p.display().to_string());
        // Attribute an unresolved class to any declared library (it may live
        // inside a monolithic binary we cannot introspect).
        let declared_libs = if found || all_libs.is_empty() {
            Vec::new()
        } else {
            all_libs.clone()
        };

        results.push(DepEntry {
            file: file.display().to_string(),
            depth,
            index,
            name: class.clone(),
            found,
            found_at: found_at.clone(),
            source,
            declared_libs,
        });

        // Recursive: follow found abstractions (patch files only; compiled and
        // Lua externals are not Pd patches), propagating our full search chain
        // so the child can resolve refs via our declares too.
        if recursive
            && let Some(ref abs_path) = location
            && is_patch_file(abs_path)
        {
            let sub_results = analyse_file_with_ancestors(
                abs_path,
                recursive,
                visited,
                &search_dirs,
                extra_dirs,
                &all_libs,
            );
            results.extend(sub_results);
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_builtin_core_classes() {
        assert!(is_builtin("expr"));
        assert!(is_builtin("expr~"));
        assert!(is_builtin("fexpr~"));
        assert!(is_builtin("v"));
        assert!(is_builtin("clone"));
        assert!(is_builtin("hdl"));
        assert!(is_builtin("vdl"));
        assert!(is_builtin("tabread~"));
        assert!(is_builtin("tabwrite~"));
        assert!(is_builtin("listbox"));
        assert!(is_builtin("makenote"));
        assert!(is_builtin("stripnote"));
        assert!(is_builtin("namecanvas"));
    }

    #[test]
    fn is_builtin_extra_classes() {
        assert!(is_builtin("bob~"));
        assert!(is_builtin("slop~"));
        assert!(is_builtin("pdcontrol"));
        assert!(is_builtin("sigmund~"));
    }

    #[test]
    fn builtin_source_distinguishes_core_and_extra() {
        assert_eq!(builtin_source("osc~"), Some(BuiltinSource::Core));
        assert_eq!(builtin_source("expr"), Some(BuiltinSource::Core));
        assert_eq!(builtin_source("listbox"), Some(BuiltinSource::Core));
        assert_eq!(builtin_source("bob~"), Some(BuiltinSource::CoreExtra));
        assert_eq!(builtin_source("slop~"), Some(BuiltinSource::CoreExtra));
        assert_eq!(builtin_source("definitely_not_a_class"), None);
    }

    #[test]
    fn declared_libraries_parses_lib_stdlib_and_import() {
        let input = "#N canvas 0 22 450 300 12;\n\
                     #X declare -lib cyclone -path ./abs -stdlib zexy;\n\
                     #X obj 20 20 import else;\n\
                     #X obj 20 60 coll;\n";
        let patch = parse(input).unwrap();
        let libs = declared_libraries(&patch.entries);
        assert_eq!(libs, vec!["cyclone", "zexy", "else"]);
    }

    #[test]
    fn declared_libraries_dedups_and_ignores_paths() {
        let input = "#N canvas 0 22 450 300 12;\n\
                     #X declare -lib cyclone;\n\
                     #X declare -lib cyclone -path foo;\n";
        let patch = parse(input).unwrap();
        assert_eq!(declared_libraries(&patch.entries), vec!["cyclone"]);
    }

    #[test]
    fn declared_libraries_empty_when_none() {
        let input = "#N canvas 0 22 450 300 12;\n#X obj 20 20 coll;\n";
        let patch = parse(input).unwrap();
        assert!(declared_libraries(&patch.entries).is_empty());
    }

    #[test]
    fn abstraction_io_counts_top_level_only() {
        use std::io::Write;
        let dir = std::env::temp_dir().join(format!("pdtk_io_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("voice.pd");
        // Two inlets (one control, one signal), one outlet at top level; the
        // inlet inside the subpatch must NOT be counted.
        let mut f = std::fs::File::create(&path).unwrap();
        write!(
            f,
            "#N canvas 0 22 450 300 12;\n\
             #X obj 20 20 inlet;\n\
             #X obj 60 20 inlet~;\n\
             #X obj 20 200 outlet;\n\
             #N canvas 0 22 200 200 sub 0;\n\
             #X obj 10 10 inlet;\n\
             #X restore 100 100 pd sub;\n"
        )
        .unwrap();
        assert_eq!(abstraction_io_counts(&path), Some((2, 1)));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn abstraction_io_counts_missing_file_is_none() {
        assert_eq!(
            abstraction_io_counts(Path::new("/nonexistent/does_not_exist.pd")),
            None
        );
    }

    #[test]
    fn resolve_abstraction_finds_sibling_patch() {
        use std::io::Write;
        let dir = std::env::temp_dir().join(format!("pdtk_res_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let caller = dir.join("main.pd");
        let abs = dir.join("myabs.pd");
        std::fs::File::create(&abs)
            .unwrap()
            .write_all(b"#N canvas 0 22 450 300 12;\n#X obj 20 20 inlet;\n")
            .unwrap();
        let content = "#N canvas 0 22 450 300 12;\n#X obj 10 10 myabs;\n";
        std::fs::File::create(&caller)
            .unwrap()
            .write_all(content.as_bytes())
            .unwrap();
        let resolved = resolve_abstraction("myabs", &caller, content);
        assert_eq!(resolved.as_deref(), Some(abs.as_path()));
        // An unknown class resolves to nothing.
        assert_eq!(resolve_abstraction("nope", &caller, content), None);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn text_define_classifies_as_text() {
        // class() returns split_whitespace().nth(4), the first token only.
        // So `text define foo` becomes class `text`, which is in CORE.
        // This pins the behavior that multi-word builtins don't need stripping.
        assert!(is_builtin("text"));
        assert!(is_builtin("array"));
    }
}

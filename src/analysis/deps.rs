use crate::model::EntryKind;
use crate::parser::parse;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Comprehensive list of known vanilla PD built-in object names.
pub fn is_builtin(name: &str) -> bool {
    BUILTINS.contains(name)
}

const BUILTIN_NAMES: &[&str] = &[
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
    "vu",
    "cnv",
    "floatatom",
    "symbolatom",
    // Subpatch / abstraction
    "pd",
    "inlet",
    "outlet",
    "inlet~",
    "outlet~",
    "declare",
    "import",
];

static BUILTINS: std::sync::LazyLock<std::collections::HashSet<&'static str>> =
    std::sync::LazyLock::new(|| BUILTIN_NAMES.iter().copied().collect());

#[derive(Debug, Clone, serde::Serialize)]
pub struct DepEntry {
    pub file: String,
    pub depth: usize,
    pub index: usize,
    pub name: String,
    pub found: bool,
    pub found_at: Option<String>,
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

/// Search for `name.pd` in a list of directories.
fn locate_abstraction(name: &str, search_dirs: &[PathBuf]) -> Option<PathBuf> {
    let filename = format!("{}.pd", name);
    for dir in search_dirs {
        let candidate = dir.join(&filename);
        if candidate.exists() {
            return Some(candidate);
        }
    }
    None
}

/// Analyse one file and collect dependency entries.
/// `visited` prevents re-processing the same file in recursive mode.
pub fn analyse_file(file: &Path, recursive: bool, visited: &mut HashSet<PathBuf>) -> Vec<DepEntry> {
    analyse_file_with_ancestors(file, recursive, visited, &[], &[])
}

/// Like `analyse_file`, but also searches `extra_dirs` as a fallback after
/// the patch's own directory and `#X declare -path` entries.
pub fn analyse_file_with_extra(
    file: &Path,
    recursive: bool,
    visited: &mut HashSet<PathBuf>,
    extra_dirs: &[PathBuf],
) -> Vec<DepEntry> {
    analyse_file_with_ancestors(file, recursive, visited, &[], extra_dirs)
}

/// Internal: analyse a file, honoring ancestor canvases' search directories.
///
/// Matches Pd's `canvas_path_iterate` semantics: when resolving abstraction
/// references, Pd walks the owner chain of canvases, trying each ancestor's
/// own directory and its `#X declare -path` entries. Thus a child abstraction
/// with no declares of its own still resolves references via its caller's
/// declares.
fn analyse_file_with_ancestors(
    file: &Path,
    recursive: bool,
    visited: &mut HashSet<PathBuf>,
    ancestor_dirs: &[PathBuf],
    extra_dirs: &[PathBuf],
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

        if is_builtin(&class) {
            continue;
        }
        // `pd` used as an object class is a subpatch reference, not an
        // abstraction lookup on disk.
        if class == "pd" {
            continue;
        }

        let location = locate_abstraction(&class, &search_dirs);
        let found = location.is_some();
        let found_at = location.as_ref().map(|p| p.display().to_string());

        results.push(DepEntry {
            file: file.display().to_string(),
            depth,
            index,
            name: class.clone(),
            found,
            found_at: found_at.clone(),
        });

        // Recursive: follow found abstractions, propagating our full search
        // chain so the child can resolve refs via our declares too.
        if recursive && let Some(ref abs_path) = location {
            let sub_results =
                analyse_file_with_ancestors(abs_path, recursive, visited, &search_dirs, extra_dirs);
            results.extend(sub_results);
        }
    }

    results
}

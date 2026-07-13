mod integration;
use integration::{fixture_path, pdtk_output};

fn main_pd() -> std::path::PathBuf {
    fixture_path("deps_externals").join("main.pd")
}

#[test]
fn deps_resolves_pd_lua_external() {
    // A `.pd_lua` external must resolve (Pd loads it via the pdlua loader),
    // not report MISSING.
    let out = pdtk_output(&["deps", main_pd().to_str().unwrap()]);
    let line = out
        .lines()
        .find(|l| l.contains("lua_obj") && !l.contains("lua_lib_obj"))
        .unwrap_or_else(|| panic!("no lua_obj line in:\n{out}"));
    assert!(
        !line.contains("MISSING"),
        "lua_obj must be found; got: {line}"
    );
    assert!(
        line.contains(".pd_lua"),
        "should report the .pd_lua path; got: {line}"
    );
}

#[test]
fn deps_resolves_pd_luax_library() {
    let out = pdtk_output(&["deps", main_pd().to_str().unwrap()]);
    let line = out
        .lines()
        .find(|l| l.contains("lua_lib_obj"))
        .unwrap_or_else(|| panic!("no lua_lib_obj line in:\n{out}"));
    assert!(!line.contains("MISSING"), "got: {line}");
}

#[test]
fn deps_resolves_pat_abstraction() {
    let out = pdtk_output(&["deps", main_pd().to_str().unwrap()]);
    let line = out
        .lines()
        .find(|l| l.contains("pat_obj"))
        .unwrap_or_else(|| panic!("no pat_obj line in:\n{out}"));
    assert!(!line.contains("MISSING"), "got: {line}");
}

#[test]
fn deps_resolves_class_in_folder_abstraction() {
    // Pd's `name/name.pd` convention.
    let out = pdtk_output(&["deps", main_pd().to_str().unwrap()]);
    let line = out
        .lines()
        .find(|l| l.contains("folder_obj"))
        .unwrap_or_else(|| panic!("no folder_obj line in:\n{out}"));
    assert!(!line.contains("MISSING"), "got: {line}");
}

#[test]
#[cfg(target_os = "linux")]
fn deps_resolves_compiled_external() {
    let out = pdtk_output(&["deps", main_pd().to_str().unwrap()]);
    let line = out
        .lines()
        .find(|l| l.contains("compiled_obj"))
        .unwrap_or_else(|| panic!("no compiled_obj line in:\n{out}"));
    assert!(!line.contains("MISSING"), "got: {line}");
}

#[test]
fn deps_still_reports_truly_missing() {
    let out = pdtk_output(&["deps", main_pd().to_str().unwrap()]);
    let line = out
        .lines()
        .find(|l| l.contains("truly_missing"))
        .unwrap_or_else(|| panic!("no truly_missing line in:\n{out}"));
    assert!(line.contains("MISSING"), "got: {line}");
}

use crate::errors::PdtkError;
use crate::io;
use std::path::Path;

/// Default canvas X position (`GLIST_DEFCANVASXLOC` from g_canvas.h).
pub const CANVAS_X: u32 = 0;

/// Default canvas Y position — platform-specific in PD source (g_canvas.h):
/// `GLIST_DEFCANVASYLOC` = 22 on macOS, 50 on all other platforms.
#[cfg(target_os = "macos")]
pub const CANVAS_Y: u32 = 22;
#[cfg(not(target_os = "macos"))]
pub const CANVAS_Y: u32 = 50;

/// Default canvas width (`GLIST_DEFCANVASWIDTH` from g_canvas.c).
pub const CANVAS_WIDTH: u32 = 450;

/// Default canvas height (`GLIST_DEFCANVASHEIGHT` from g_canvas.c).
pub const CANVAS_HEIGHT: u32 = 300;

/// Default font size (`DEFAULTFONT` from s_main.c).
pub const CANVAS_FONT: u32 = 12;

/// Generate and optionally write a blank `.pd` patch.
///
/// The canvas header written is `#N canvas <x> <y> <width> <height> <font>;`,
/// matching what PD produces for a new blank file via File > New.
///
/// When `output` is `None` the content is returned as a `String` for the
/// caller to print.  When `output` is `Some(path)` the file is written to
/// disk; `force` must be `true` to overwrite an existing file.
pub fn run(
    output: Option<&str>,
    width: u32,
    height: u32,
    x: u32,
    y: u32,
    font: u32,
    force: bool,
) -> Result<String, PdtkError> {
    let content = format!("#N canvas {x} {y} {width} {height} {font};\n");

    match output {
        None => Ok(content),
        Some(path) => {
            if !force && Path::new(path).exists() {
                return Err(PdtkError::Usage(format!(
                    "file already exists: {path} (use --force to overwrite)"
                )));
            }
            io::write_patch_file(path, &content)?;
            Ok(String::new())
        }
    }
}

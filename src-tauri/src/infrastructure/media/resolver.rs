use std::path::PathBuf;

/// Resolves where to find a media tool binary (`ffmpeg`/`ffprobe`),
/// preferring a bundled copy over whatever is on the system `PATH`
/// (section 20). Resolution order:
///
/// 1. An explicit override via `XPFLOW_FFMPEG_PATH` / `XPFLOW_FFPROBE_PATH`
///    (useful for development and for tests).
/// 2. A sidecar binary next to the running executable — either
///    `<exe_dir>/<name>` or `<exe_dir>/bin/<name>` — which is where a
///    packaged build places `ffmpeg-<target-triple>` /
///    `ffprobe-<target-triple>` sidecars for Windows x64/ARM64 and macOS
///    arm64/x64 (Tauri's sidecar convention).
/// 3. Whatever `<name>` resolves to on the system `PATH`.
///
/// No development-machine path is ever hardcoded — every step here is a
/// generic, portable lookup.
pub fn resolve_binary(name: &str) -> Option<PathBuf> {
    let env_key = format!("XPFLOW_{}_PATH", name.to_uppercase());
    if let Some(path) = std::env::var_os(&env_key) {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Some(path);
        }
    }

    let exe_name = exe_name(name);

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            let sidecar = exe_dir.join(&exe_name);
            if sidecar.is_file() {
                return Some(sidecar);
            }
            let bundled = exe_dir.join("bin").join(&exe_name);
            if bundled.is_file() {
                return Some(bundled);
            }
        }
    }

    which(name)
}

fn exe_name(name: &str) -> String {
    if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_string()
    }
}

/// Minimal, dependency-free `which`: checks each `PATH` entry for an
/// executable with this name.
pub fn which(binary: &str) -> Option<PathBuf> {
    let path_var = std::env::var_os("PATH")?;
    let exe_name = exe_name(binary);

    std::env::split_paths(&path_var)
        .map(|dir| dir.join(&exe_name))
        .find(|candidate| candidate.is_file())
}

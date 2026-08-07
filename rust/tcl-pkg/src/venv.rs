// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Virtual environment creation and management.
//!
//! A tclpkg virtual environment is a
//! directory (conventionally `.venv/`) containing a pinned `tclsh` wrapper,
//! bash/fish activation scripts, and a `lib/` tree for materialised packages.
//! The model follows the usual `venv` convention: activation prepends `<venv>/bin` to
//! `$PATH` and sets `$TCLLIBPATH`; the wrapper always sets `TCLLIBPATH` so
//! non-interactive use works without activation.

use std::path::{Path, PathBuf};

use chrono::Utc;

use crate::errors::TclPkgError;

fn venv_error(message: impl Into<String>, hint: Option<&str>) -> TclPkgError {
    TclPkgError::venv(message, hint.map(ToString::to_string))
}

/// Find a suitable `tclsh` on PATH (preferring newer versions), or `None`.
#[must_use]
pub fn find_tclsh() -> Option<PathBuf> {
    for name in ["tclsh9.0", "tclsh8.6", "tclsh8.5", "tclsh8.4", "tclsh"] {
        if let Some(path) = which(name) {
            return Some(path);
        }
    }
    None
}

/// Locate `name` on `PATH`, returning the first executable match.
#[must_use]
pub fn which(name: &str) -> Option<PathBuf> {
    let paths = std::env::var_os("PATH")?;
    for dir in std::env::split_paths(&paths) {
        let candidate = dir.join(name);
        if is_executable(&candidate) {
            return Some(candidate);
        }
    }
    None
}

#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
}

fn resolve_tclsh(requested: Option<&str>) -> Result<PathBuf, TclPkgError> {
    let Some(req) = requested else {
        return find_tclsh().ok_or_else(|| {
            venv_error(
                "no tclsh found on PATH",
                Some("install Tcl or specify --tcl <version>"),
            )
        });
    };
    if let Some(path) = which(&format!("tclsh{req}")) {
        return Ok(path);
    }
    if let Some(path) = which("tclsh") {
        let actual = tclsh_version_string(&path);
        if !actual.is_empty() && !actual.starts_with(req) {
            return Err(venv_error(
                format!("tclsh{req} not found; plain tclsh is {actual}"),
                Some(&format!("install Tcl {req} or omit --tcl")),
            ));
        }
        return Ok(path);
    }
    Err(venv_error(
        format!("tclsh{req} not found on PATH"),
        Some("install the requested Tcl version or omit --tcl"),
    ))
}

fn tclsh_version_string(tclsh: &Path) -> String {
    use tcl_sandbox::{Profile, SandboxPolicy};

    let cwd = tclsh
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_path_buf);
    // Trusted internal probe: no network, but tclsh needs its library to start,
    // so PATH/HOME and the Tcl library env pass through. The script is fed on
    // stdin via `tclsh -`.
    let mut profile = Profile::new("tclsh-probe", tclsh, cwd)
        .arg("-")
        .network(false)
        .stdin_bytes(b"puts [info patchlevel]".to_vec());
    for name in ["PATH", "HOME", "TCL_LIBRARY", "TCLLIBPATH", "TMPDIR"] {
        profile = profile.pass_env(name);
    }
    match crate::exec::execute(&profile, &SandboxPolicy::default()) {
        Ok(out) if out.success => String::from_utf8_lossy(&out.stdout).trim().to_string(),
        _ => String::new(),
    }
}

/// Options for [`create_venv`].
#[derive(Debug, Default)]
pub struct CreateOptions<'a> {
    pub tcl_version: Option<&'a str>,
    pub system_site_packages: bool,
    pub prompt: Option<&'a str>,
    pub project_root: Option<&'a Path>,
}

/// Create a new tclpkg virtual environment at `venv_path`. Returns the absolute
/// path.
pub fn create_venv(venv_path: &Path, opts: &CreateOptions) -> Result<PathBuf, TclPkgError> {
    let venv_path = absolutise(venv_path);
    if venv_path.exists() {
        return Err(venv_error(
            format!("directory already exists: {}", venv_path.display()),
            Some("use --force to overwrite or choose a different path"),
        ));
    }

    let tclsh = resolve_tclsh(opts.tcl_version)?;
    let version_str = {
        let v = tclsh_version_string(&tclsh);
        if v.is_empty() {
            opts.tcl_version.unwrap_or("unknown").to_string()
        } else {
            v
        }
    };
    let prompt_label = opts.prompt.map_or_else(
        || {
            venv_path
                .file_name()
                .map_or_else(String::new, |n| n.to_string_lossy().into_owned())
        },
        ToString::to_string,
    );

    let bin_dir = venv_path.join("bin");
    let lib_dir = venv_path.join("lib");
    std::fs::create_dir_all(&bin_dir)
        .map_err(|e| venv_error(format!("cannot create venv: {e}"), None))?;
    std::fs::create_dir_all(&lib_dir)
        .map_err(|e| venv_error(format!("cannot create venv: {e}"), None))?;

    let tclsh_display = tclsh.to_string_lossy();
    let mut cfg = vec![
        format!("tcl_version = {version_str}"),
        format!("tcl_executable = {tclsh_display}"),
        "venv_tool = tcl-lsp".to_string(),
        format!("created = {}", Utc::now().format("%Y-%m-%dT%H:%M:%SZ")),
        format!("prompt = {prompt_label}"),
        format!(
            "include-system-site-packages = {}",
            if opts.system_site_packages {
                "true"
            } else {
                "false"
            }
        ),
    ];
    if let Some(root) = opts.project_root {
        cfg.push(format!("project_root = {}", root.display()));
    }
    write_file(&venv_path.join("tclvenv.cfg"), &(cfg.join("\n") + "\n"))?;

    write_tclsh_wrapper(&bin_dir.join("tclsh"), &tclsh_display, &venv_path)?;
    write_file(
        &bin_dir.join("activate"),
        &activate_bash(&venv_path, &prompt_label),
    )?;
    write_file(
        &bin_dir.join("activate.fish"),
        &activate_fish(&venv_path, &prompt_label),
    )?;
    write_file(
        &lib_dir.join("pkgIndex.tcl"),
        "# Auto-generated by tclpkg — do not edit.\n# This file is regenerated by `tcl pkg install`.\n",
    )?;

    Ok(venv_path)
}

/// Parse `tclvenv.cfg` into ordered key/value pairs.
pub fn read_venv_config(venv_path: &Path) -> Result<Vec<(String, String)>, TclPkgError> {
    let cfg_file = venv_path.join("tclvenv.cfg");
    if !cfg_file.is_file() {
        return Err(venv_error(
            format!(
                "not a tclpkg venv: {} (missing tclvenv.cfg)",
                venv_path.display()
            ),
            None,
        ));
    }
    let text = std::fs::read_to_string(&cfg_file)
        .map_err(|e| venv_error(format!("cannot read venv config: {e}"), None))?;
    let mut result = Vec::new();
    for line in text.lines() {
        if let Some((key, value)) = line.split_once('=') {
            result.push((key.trim().to_string(), value.trim().to_string()));
        }
    }
    Ok(result)
}

/// Remove a virtual environment directory.
///
/// `force` overrides the active-venv guard (the documented "Force deletion even
/// if active"). The `tclvenv.cfg` marker check is *not* overridable even under
/// `force` — refusing to `remove_dir_all` a non-venv directory is a data-loss
/// safeguard, not an activeness check.
pub fn delete_venv(venv_path: &Path, force: bool) -> Result<(), TclPkgError> {
    if !venv_path.is_dir() {
        return Err(venv_error(
            format!("venv not found: {}", venv_path.display()),
            None,
        ));
    }
    if !venv_path.join("tclvenv.cfg").is_file() {
        return Err(venv_error(
            format!("directory is not a tclpkg venv: {}", venv_path.display()),
            Some("missing tclvenv.cfg — refusing to delete to avoid data loss"),
        ));
    }
    if !force && let Some(active) = std::env::var_os("TCL_VENV") {
        let active = PathBuf::from(active);
        if active.canonicalize().ok() == venv_path.canonicalize().ok() {
            return Err(venv_error(
                "cannot delete the currently active venv",
                Some("run 'deactivate' first or use --force"),
            ));
        }
    }
    std::fs::remove_dir_all(venv_path)
        .map_err(|e| venv_error(format!("cannot delete venv: {e}"), None))
}

/// Re-link an existing venv to `new_tcl`, rewriting `tclvenv.cfg` and the
/// `bin/tclsh` wrapper. Returns the resolved tclsh path.
pub fn update_venv(venv_path: &Path, new_tcl: &str) -> Result<PathBuf, TclPkgError> {
    let venv_path = absolutise(venv_path);
    read_venv_config(&venv_path)?; // validate the venv exists

    let tclsh = which(&format!("tclsh{new_tcl}"))
        .or_else(|| which("tclsh"))
        .ok_or_else(|| venv_error(format!("tclsh{new_tcl} not found on PATH"), None))?;
    let tclsh_display = tclsh.to_string_lossy();

    let cfg_file = venv_path.join("tclvenv.cfg");
    let text = std::fs::read_to_string(&cfg_file)
        .map_err(|e| venv_error(format!("cannot read venv config: {e}"), None))?;
    let new_lines: Vec<String> = text
        .lines()
        .map(|line| {
            if line.starts_with("tcl_version") {
                format!("tcl_version = {new_tcl}")
            } else if line.starts_with("tcl_executable") {
                format!("tcl_executable = {tclsh_display}")
            } else {
                line.to_string()
            }
        })
        .collect();
    write_file(&cfg_file, &(new_lines.join("\n") + "\n"))?;

    write_tclsh_wrapper(
        &venv_path.join("bin").join("tclsh"),
        &tclsh_display,
        &venv_path,
    )?;
    Ok(tclsh)
}

fn absolutise(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir().map_or_else(|_| path.to_path_buf(), |cwd| cwd.join(path))
    }
}

fn write_file(path: &Path, content: &str) -> Result<(), TclPkgError> {
    std::fs::write(path, content)
        .map_err(|e| venv_error(format!("cannot write {}: {e}", path.display()), None))
}

fn write_tclsh_wrapper(path: &Path, tclsh: &str, _venv: &Path) -> Result<(), TclPkgError> {
    let script = format!(
        "#!/bin/sh\n# Auto-generated by tclpkg — do not edit.\nVENV=\"$(cd \"$(dirname \"$0\")/..\" && pwd)\"\n: \"${{TCLLIBPATH:=}}\"\nexport TCLLIBPATH=\"$VENV/lib${{TCLLIBPATH:+ $TCLLIBPATH}}\"\nexport TCL_VENV=\"$VENV\"\nexec \"{tclsh}\" \"$@\"\n"
    );
    write_file(path, &script)?;
    make_executable(path);
    Ok(())
}

#[cfg(unix)]
fn make_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(meta) = std::fs::metadata(path) {
        let mut perms = meta.permissions();
        perms.set_mode(perms.mode() | 0o111);
        let _ = std::fs::set_permissions(path, perms);
    }
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) {}

fn activate_bash(venv: &Path, prompt: &str) -> String {
    let venv = venv.display();
    let activate_path = format!("{venv}/bin/activate");
    format!(
        "# Auto-generated by tclpkg — source this file, do not execute.\n# Usage: source {activate_path}\n\ndeactivate () {{\n    if [ -n \"${{_OLD_TCL_VENV_PATH:-}}\" ]; then\n        PATH=\"${{_OLD_TCL_VENV_PATH}}\"\n        export PATH\n        unset _OLD_TCL_VENV_PATH\n    fi\n    if [ -n \"${{_OLD_TCL_VENV_TCLLIBPATH:-}}\" ]; then\n        TCLLIBPATH=\"${{_OLD_TCL_VENV_TCLLIBPATH}}\"\n        export TCLLIBPATH\n    else\n        unset TCLLIBPATH\n    fi\n    unset _OLD_TCL_VENV_TCLLIBPATH\n    if [ -n \"${{_OLD_TCL_VENV_PS1:-}}\" ]; then\n        PS1=\"${{_OLD_TCL_VENV_PS1}}\"\n        export PS1\n        unset _OLD_TCL_VENV_PS1\n    fi\n    unset TCL_VENV\n}}\n\n_OLD_TCL_VENV_PATH=\"$PATH\"\n_OLD_TCL_VENV_TCLLIBPATH=\"${{TCLLIBPATH:-}}\"\n_OLD_TCL_VENV_PS1=\"${{PS1:-}}\"\n\nexport TCL_VENV=\"{venv}\"\nexport TCLLIBPATH=\"{venv}/lib${{TCLLIBPATH:+ $TCLLIBPATH}}\"\nexport PATH=\"{venv}/bin:$PATH\"\nPS1=\"({prompt}) ${{PS1:-}}\"\nexport PS1\n"
    )
}

fn activate_fish(venv: &Path, prompt: &str) -> String {
    let venv = venv.display();
    let activate_path = format!("{venv}/bin/activate.fish");
    let _ = prompt;
    format!(
        "# Auto-generated by tclpkg — source this file in fish.\n# Usage: source {activate_path}\n\nfunction deactivate\n    if set -q _OLD_TCL_VENV_PATH\n        set -gx PATH $_OLD_TCL_VENV_PATH\n        set -e _OLD_TCL_VENV_PATH\n    end\n    if set -q _OLD_TCL_VENV_TCLLIBPATH\n        set -gx TCLLIBPATH $_OLD_TCL_VENV_TCLLIBPATH\n        set -e _OLD_TCL_VENV_TCLLIBPATH\n    else\n        set -e TCLLIBPATH\n    end\n    set -e TCL_VENV\n    functions -e deactivate\nend\n\nset -gx _OLD_TCL_VENV_PATH $PATH\nif set -q TCLLIBPATH\n    set -gx _OLD_TCL_VENV_TCLLIBPATH $TCLLIBPATH\nend\n\nset -gx TCL_VENV \"{venv}\"\nset -gx TCLLIBPATH \"{venv}/lib $TCLLIBPATH\"\nset -gx PATH \"{venv}/bin\" $PATH\n"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "tclpkg-venv-{tag}-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn create_reads_back_and_deletes() {
        // Only run when a tclsh is actually discoverable.
        if find_tclsh().is_none() {
            return;
        }
        let base = tmp("create");
        std::fs::create_dir_all(&base).unwrap();
        let venv = base.join(".venv");
        let created = create_venv(&venv, &CreateOptions::default()).unwrap();
        assert!(created.join("tclvenv.cfg").is_file());
        assert!(created.join("bin/tclsh").is_file());
        assert!(created.join("bin/activate").is_file());
        assert!(created.join("lib/pkgIndex.tcl").is_file());

        let cfg = read_venv_config(&created).unwrap();
        assert!(cfg.iter().any(|(k, _)| k == "tcl_version"));

        delete_venv(&created, false).unwrap();
        assert!(!created.exists());
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn create_refuses_existing() {
        if find_tclsh().is_none() {
            return;
        }
        let base = tmp("exists");
        let venv = base.join(".venv");
        std::fs::create_dir_all(&venv).unwrap();
        let err = create_venv(&venv, &CreateOptions::default()).unwrap_err();
        assert!(err.to_string().contains("already exists"));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn delete_refuses_non_venv() {
        let base = tmp("nonvenv");
        std::fs::create_dir_all(&base).unwrap();
        let err = delete_venv(&base, false).unwrap_err();
        assert!(err.to_string().contains("not a tclpkg venv"));
        // even under --force, a non-venv directory is
        // refused — force overrides the *active-venv* guard, never the
        // data-loss (missing-marker) guard.
        let err = delete_venv(&base, true).unwrap_err();
        assert!(err.to_string().contains("not a tclpkg venv"));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn delete_force_deletes_a_marker_venv() {
        // A real venv (with the marker) deletes under --force
        // when it is not the active one — the `force` flag is now plumbed
        // through `delete_venv` rather than ignored.
        let venv = tmp("force-del");
        std::fs::create_dir_all(&venv).unwrap();
        std::fs::write(venv.join("tclvenv.cfg"), "venv_tool = tcl-lsp\n").unwrap();
        delete_venv(&venv, true).expect("force delete of an inactive venv");
        assert!(!venv.exists());
    }
}

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

use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use chrono::Utc;

use crate::errors::TclPkgError;

mod config;
mod posix_source;

fn venv_error(message: impl Into<String>, hint: Option<&str>) -> TclPkgError {
    TclPkgError::venv(message, hint.map(ToString::to_string))
}

/// Prepend one actual Unicode path to a Tcl environment-list value, preserving
/// the inherited source verbatim. Each supported list grammar must retain its
/// own original suffix interpretation. This constructs environment text only;
/// it supplies no external system-encoding or filesystem lookup guarantee.
pub fn prepare_tcllibpath(
    directory: &Path,
    inherited: Option<&OsStr>,
) -> Result<OsString, TclPkgError> {
    let directory = directory.to_str().ok_or_else(|| {
        venv_error(
            "TCLLIBPATH directory has no Unicode list-source representation",
            None,
        )
    })?;
    let inherited = match inherited {
        Some(value) => value.to_str().ok_or_else(|| {
            venv_error(
                "inherited TCLLIBPATH has no Unicode list-source representation",
                None,
            )
        })?,
        None => "",
    };
    if directory.as_bytes().contains(&0) || inherited.as_bytes().contains(&0) {
        return Err(venv_error("TCLLIBPATH cannot contain NUL", None));
    }
    let mut output = tcl_syntax::list::list_element(directory);
    if !inherited.is_empty() {
        output.push(' ');
        output.push_str(inherited);
    }
    for &escapes in tcl_dialect::EscapeSyntax::ALL {
        for syntax in [
            tcl_dialect::ListParse::Strict,
            tcl_dialect::ListParse::Lenient,
        ] {
            let original =
                tcl_syntax::list::split_list_bytes_in(inherited.as_bytes(), syntax, escapes)
                    .map_err(|_| {
                        venv_error(
                            "inherited TCLLIBPATH is not a complete supported Tcl list",
                            None,
                        )
                    })?;
            let combined = tcl_syntax::list::split_list_bytes_in(
                output.as_bytes(),
                syntax,
                escapes,
            )
            .map_err(|_| {
                venv_error(
                    "TCLLIBPATH directory cannot be represented in the supported Tcl list grammars",
                    None,
                )
            })?;
            if combined.len() != original.len() + 1
                || combined.first().map(|value| value.as_ref()) != Some(directory.as_bytes())
                || combined.iter().skip(1).ne(original.iter())
            {
                return Err(venv_error(
                    "TCLLIBPATH construction changed a supported Tcl list interpretation",
                    None,
                ));
            }
        }
    }
    Ok(OsString::from(output))
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
        return find_tclsh()
            .ok_or_else(|| {
                venv_error(
                    "no tclsh found on PATH",
                    Some("install Tcl or specify --tcl <version>"),
                )
            })
            .and_then(|path| posix_source::selected_executable(&path));
    };
    if let Some(path) = which(&format!("tclsh{req}")) {
        return posix_source::selected_executable(&path);
    }
    if let Some(path) = which("tclsh") {
        let path = posix_source::selected_executable(&path)?;
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
    let prompt_label = match opts.prompt {
        Some(prompt) => prompt.to_owned(),
        None => venv_path.file_name().map_or(Ok(String::new()), |name| {
            name.to_str()
                .map(ToString::to_string)
                .ok_or_else(|| venv_error("venv name has no Unicode prompt representation", None))
        })?,
    };
    let wrapper = posix_source::wrapper(&tclsh)?;
    let activation = posix_source::activation(&venv_path, &prompt_label)?;
    let fish = activate_fish(&venv_path, &prompt_label);

    let mut cfg = vec![
        ("tcl_version".to_owned(), version_str),
        ("tcl_executable".to_owned(), config::path_value(&tclsh)?),
        ("venv_tool".to_owned(), "tcl-lsp".to_owned()),
        (
            "created".to_owned(),
            Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        ),
        ("prompt".to_owned(), prompt_label),
        (
            "include-system-site-packages".to_owned(),
            opts.system_site_packages.to_string(),
        ),
    ];
    if let Some(root) = opts.project_root {
        cfg.push(("project_root".to_owned(), config::path_value(root)?));
    }
    let cfg_source = config::encode(&cfg)?;
    let bin_dir = venv_path.join("bin");
    let lib_dir = venv_path.join("lib");
    std::fs::create_dir_all(&bin_dir)
        .map_err(|e| venv_error(format!("cannot create venv: {e}"), None))?;
    std::fs::create_dir_all(&lib_dir)
        .map_err(|e| venv_error(format!("cannot create venv: {e}"), None))?;
    write_file(&venv_path.join("tclvenv.cfg"), &cfg_source)?;

    write_executable(&bin_dir.join("tclsh"), &wrapper)?;
    write_file(&bin_dir.join("activate"), &activation)?;
    write_file(&bin_dir.join("activate.fish"), &fish)?;
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
    config::decode(&text)
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
        let active = active.canonicalize().map_err(|error| {
            venv_error(
                format!("cannot establish active venv path identity: {error}"),
                Some("resolve TCL_VENV or use --force to override the active-venv guard"),
            )
        })?;
        let selected = venv_path.canonicalize().map_err(|error| {
            venv_error(
                format!("cannot establish selected venv path identity: {error}"),
                None,
            )
        })?;
        if active == selected {
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
    let mut cfg = read_venv_config(&venv_path)?;

    let tclsh = which(&format!("tclsh{new_tcl}"))
        .or_else(|| which("tclsh"))
        .ok_or_else(|| venv_error(format!("tclsh{new_tcl} not found on PATH"), None))?;
    let tclsh = posix_source::selected_executable(&tclsh)?;
    let wrapper = posix_source::wrapper(&tclsh)?;
    config::replace_selected(&mut cfg, new_tcl, &tclsh)?;
    let source = config::encode(&cfg)?;
    write_file(&venv_path.join("tclvenv.cfg"), &source)?;

    write_executable(&venv_path.join("bin").join("tclsh"), &wrapper)?;
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

fn write_executable(path: &Path, source: &str) -> Result<(), TclPkgError> {
    write_file(path, source)?;
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
    fn tcllibpath_source_preserves_each_original_suffix_interpretation() {
        for directory in [
            "/plain/lib",
            "/space and {braces}/lib",
            "/quote\"[literal];$value\\tail/lib",
            "/café/lib",
            "/trailing\\",
            "/line\nbreak",
        ] {
            for inherited in [
                None,
                Some(""),
                Some("  {old path} \t\n"),
                Some(r"old\x4142 other\400"),
                Some(r#"{old\n} \"literal\" "#),
            ] {
                let output =
                    prepare_tcllibpath(Path::new(directory), inherited.map(OsStr::new)).unwrap();
                let text = output.to_str().unwrap();
                if let Some(original) = inherited {
                    assert!(text.ends_with(original));
                }
                for &escapes in tcl_dialect::EscapeSyntax::ALL {
                    let actual = tcl_syntax::list::split_list_bytes_in(
                        text.as_bytes(),
                        tcl_dialect::ListParse::Strict,
                        escapes,
                    )
                    .unwrap();
                    let original = tcl_syntax::list::split_list_bytes_in(
                        inherited.unwrap_or("").as_bytes(),
                        tcl_dialect::ListParse::Strict,
                        escapes,
                    )
                    .unwrap();
                    assert_eq!(actual[0].as_ref(), directory.as_bytes());
                    assert_eq!(&actual[1..], original.as_slice());
                }
            }
        }
        assert_eq!(
            prepare_tcllibpath(Path::new("/plain/lib"), None).unwrap(),
            OsString::from("/plain/lib")
        );
        for original in ["{unclosed", "\"unclosed", "{closed}junk", "valid\0invalid"] {
            assert!(
                prepare_tcllibpath(Path::new("/plain/lib"), Some(OsStr::new(original))).is_err()
            );
        }
        assert!(prepare_tcllibpath(Path::new("bad\0path"), None).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn tcllibpath_source_refuses_opaque_os_values_without_treating_them_as_absent() {
        use std::os::unix::ffi::OsStringExt;
        let opaque = OsString::from_vec(b"original-\xff".to_vec());
        assert!(prepare_tcllibpath(Path::new("/plain/lib"), Some(&opaque)).is_err());
        assert!(prepare_tcllibpath(Path::new(&opaque), None).is_err());
        assert!(prepare_tcllibpath(Path::new("/plain/lib"), None).is_ok());
        assert!(prepare_tcllibpath(Path::new("/plain/lib"), Some(OsStr::new(""))).is_ok());
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
        // when it is not the active one.
        let venv = tmp("force-del");
        std::fs::create_dir_all(&venv).unwrap();
        std::fs::write(venv.join("tclvenv.cfg"), "venv_tool = tcl-lsp\n").unwrap();
        delete_venv(&venv, true).expect("force delete of an inactive venv");
        assert!(!venv.exists());
    }
}

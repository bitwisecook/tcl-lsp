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

//! Executable lookup for native tools.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

/// Resolve an explicit path or the first executable on `PATH`.
///
/// Keep the selected filename (including symlinks) and make the result
/// absolute so the caller can change the child's working directory.
#[must_use]
pub fn which(program: impl AsRef<Path>) -> Option<PathBuf> {
    let program = program.as_ref();
    if program.file_name()? != program.as_os_str() {
        return executable_at(program).and_then(|path| std::path::absolute(path).ok());
    }
    which_on_path(program, &std::env::var_os("PATH")?)
}

fn which_on_path(program: &Path, path: &OsStr) -> Option<PathBuf> {
    std::env::split_paths(path).find_map(|directory| {
        executable_at(&directory.join(program)).and_then(|path| std::path::absolute(path).ok())
    })
}

#[cfg(unix)]
fn executable_at(path: &Path) -> Option<PathBuf> {
    use std::os::unix::fs::PermissionsExt as _;

    std::fs::metadata(path)
        .is_ok_and(|metadata| metadata.is_file() && metadata.permissions().mode() & 0o111 != 0)
        .then(|| path.to_owned())
}

#[cfg(windows)]
fn executable_at(path: &Path) -> Option<PathBuf> {
    let extensions = std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".to_owned());
    windows_executable(path, &extensions)
}

#[cfg(any(windows, test))]
fn windows_executable(path: &Path, extensions: &str) -> Option<PathBuf> {
    let suffixes = extensions
        .split(';')
        .map(str::trim)
        .filter_map(|suffix| suffix.strip_prefix('.'))
        .filter(|suffix| {
            !suffix.is_empty() && suffix.bytes().all(|byte| byte.is_ascii_alphanumeric())
        });
    if let Some(extension) = path.extension()
        && suffixes
            .clone()
            .any(|suffix| extension.eq_ignore_ascii_case(OsStr::new(suffix)))
    {
        return path.is_file().then(|| path.to_owned());
    }

    // A numeric suffix in names such as tclsh9.1 is part of the basename.
    suffixes
        .map(|suffix| {
            let mut candidate = path.as_os_str().to_os_string();
            candidate.push(".");
            candidate.push(suffix);
            PathBuf::from(candidate)
        })
        .find(|candidate| candidate.is_file())
}

#[cfg(not(any(unix, windows)))]
fn executable_at(path: &Path) -> Option<PathBuf> {
    path.is_file().then(|| path.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Scratch(PathBuf);

    impl Scratch {
        fn new(label: &str) -> Self {
            let directory =
                std::env::temp_dir().join(format!("tcl-executable-{label}-{}", std::process::id()));
            std::fs::create_dir(&directory).expect("create executable fixture directory");
            Self(directory)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[cfg(unix)]
    #[test]
    fn search_skips_non_executable_path_entries() {
        use std::os::unix::fs::PermissionsExt as _;

        let root = Scratch::new("permissions");
        let later = root.0.join("later");
        std::fs::create_dir(&later).unwrap();
        let name = Path::new("tclsh9.1");
        let blocked = root.0.join(name);
        let runnable = later.join(name);
        for (file, mode) in [(&blocked, 0o644), (&runnable, 0o755)] {
            std::fs::write(file, "").unwrap();
            std::fs::set_permissions(file, std::fs::Permissions::from_mode(mode)).unwrap();
        }
        let path = std::env::join_paths([&root.0, &later]).unwrap();

        assert_eq!(which_on_path(name, &path), Some(runnable));
        assert_eq!(which(blocked), None);
    }

    #[test]
    fn windows_suffixes_are_appended_in_order_after_the_tcl_release() {
        let root = Scratch::new("suffixes");
        let name = root.0.join("tclsh9.1");
        let exe = root.0.join("tclsh9.1.EXE");
        let bat = root.0.join("tclsh9.1.BAT");
        for file in [&exe, &bat] {
            std::fs::write(file, "").unwrap();
        }

        assert_eq!(windows_executable(&name, ".BAT;.EXE"), Some(bat));
        assert_eq!(windows_executable(&name, ".EXE;.BAT"), Some(exe.clone()));
        assert_eq!(windows_executable(&exe, ".exe"), Some(exe));
    }
}

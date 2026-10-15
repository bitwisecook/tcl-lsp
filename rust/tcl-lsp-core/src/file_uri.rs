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

//! File URI source construction from the host's original filesystem paths.

use std::fmt::Write as _;
use std::path::Path;

/// Literal path characters admitted by a URI-producing protocol surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileUriEscaping {
    /// RFC 3986 path characters, including literal sub-delimiters, `:` and `@`.
    PathCharacters,
    /// The `ls_types` file-path spelling: unreserved bytes and `/` only.
    Unreserved,
}

/// Render a rooted host path, without canonicalising or reading the filesystem.
/// Invalid Unicode, NUL and relative paths decline. Windows path separators and
/// extended-length prefixes are interpreted only on a Windows host; a POSIX
/// backslash remains filename data. This API does not parse incoming URIs.
#[must_use]
pub fn from_path(path: &Path, escaping: FileUriEscaping) -> Option<String> {
    path.has_root().then_some(())?;
    from_rooted_spelling(path.to_str()?, cfg!(windows), escaping)
}

fn from_rooted_spelling(path: &str, windows: bool, escaping: FileUriEscaping) -> Option<String> {
    if path.as_bytes().contains(&0) {
        return None;
    }
    let normalised;
    let path = if windows {
        normalised = path.replace('\\', "/");
        normalised.as_str()
    } else {
        path
    };
    let extended_unc;
    let path = if windows && let Some(extended) = path.strip_prefix("//?/") {
        if let Some((kind, rest)) = extended.split_once('/')
            && kind.eq_ignore_ascii_case("UNC")
        {
            extended_unc = format!("//{rest}");
            extended_unc.as_str()
        } else {
            extended
        }
    } else {
        path
    };
    let rooted = path.starts_with('/')
        || windows
            && matches!(path.as_bytes(), [drive, b':', b'/', ..] if drive.is_ascii_alphabetic());
    rooted.then_some(())?;
    let mut result = String::from("file://");
    if windows && let Some(remote) = path.strip_prefix("//") {
        let (authority, tail) = remote.split_once('/').unwrap_or((remote, ""));
        result.push_str(&escaped_path(authority, FileUriEscaping::Unreserved));
        result.push('/');
        result.push_str(&escaped_path(tail, escaping));
    } else {
        if !path.starts_with('/') {
            result.push('/');
        }
        result.push_str(&escaped_path(path, escaping));
    }
    Some(result)
}

pub(crate) fn escaped_path(path: &str, escaping: FileUriEscaping) -> String {
    let mut out = String::with_capacity(path.len());
    for &byte in path.as_bytes() {
        let literal = byte.is_ascii_alphanumeric()
            || matches!(byte, b'-' | b'.' | b'_' | b'~' | b'/')
            || escaping == FileUriEscaping::PathCharacters
                && matches!(
                    byte,
                    b'!' | b'$'
                        | b'&'
                        | b'\''
                        | b'('
                        | b')'
                        | b'*'
                        | b'+'
                        | b','
                        | b';'
                        | b'='
                        | b':'
                        | b'@'
                );
        if literal {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

#[cfg(test)]
pub(crate) fn standalone_windows_path(path: &str, escaping: FileUriEscaping) -> Option<String> {
    from_rooted_spelling(path, true, escaping)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admitted_uri_spellings_preserve_path_data_under_each_escaping_policy() {
        for (escaping, expected) in [
            (
                FileUriEscaping::PathCharacters,
                "file:///work/a$b&c:d@e%20%23%3F%25%5B%5D%C3%A9.tcl",
            ),
            (
                FileUriEscaping::Unreserved,
                "file:///work/a%24b%26c%3Ad%40e%20%23%3F%25%5B%5D%C3%A9.tcl",
            ),
        ] {
            assert_eq!(
                from_rooted_spelling("/work/a$b&c:d@e #?%[]é.tcl", false, escaping).as_deref(),
                Some(expected)
            );
        }
    }

    #[test]
    fn standalone_windows_geometry_preserves_drive_unc_and_extended_namespace() {
        for (path, expected) in [
            (r"C:\work\a#b.tcl", "file:///C:/work/a%23b.tcl"),
            (r"\\server\share\a#b.tcl", "file://server/share/a%23b.tcl"),
            (r"\\ho@st\share\a.tcl", "file://ho%40st/share/a.tcl"),
            (r"\\?\C:\work\a.tcl", "file:///C:/work/a.tcl"),
            (r"\\?\UNC\server\share\a.tcl", "file://server/share/a.tcl"),
        ] {
            assert_eq!(
                standalone_windows_path(path, FileUriEscaping::PathCharacters).as_deref(),
                Some(expected)
            );
        }
        assert!(standalone_windows_path(r"C:relative.tcl", FileUriEscaping::Unreserved).is_none());
    }

    #[cfg(not(windows))]
    #[test]
    fn host_path_admission_retains_posix_backslashes_and_declines_relative_or_nul_paths() {
        assert_eq!(
            from_path(Path::new(r"/work/a\b.tcl"), FileUriEscaping::PathCharacters).as_deref(),
            Some("file:///work/a%5Cb.tcl")
        );
        assert_eq!(
            from_path(Path::new("//?/file.tcl"), FileUriEscaping::PathCharacters).as_deref(),
            Some("file:////%3F/file.tcl")
        );
        assert!(from_path(Path::new("relative.tcl"), FileUriEscaping::Unreserved).is_none());
        assert!(from_path(Path::new("/work/a\0b"), FileUriEscaping::Unreserved).is_none());
    }

    #[cfg(unix)]
    #[test]
    fn non_unicode_paths_do_not_publish_a_replacement_character_uri() {
        use std::os::unix::ffi::OsStringExt;
        let path =
            std::path::PathBuf::from(std::ffi::OsString::from_vec(b"/work/\xff.tcl".to_vec()));
        assert!(from_path(&path, FileUriEscaping::PathCharacters).is_none());
        assert!(from_path(&path, FileUriEscaping::Unreserved).is_none());
    }
}

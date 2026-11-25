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

//! Lossless, forward file-URI spelling for counted native paths.

use std::fmt::Write as _;
use std::path::Path;

/// Literal path characters admitted by a URI-producing protocol surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileUriEscaping {
    /// RFC 3986 path characters, including literal sub-delimiters, `:` and `@`.
    PathCharacters,
    /// The unreserved file-path spelling, including `/` separators.
    Unreserved,
}

/// Render a rooted host path, without canonicalising or reading the filesystem.
/// Invalid Unicode, NUL and relative paths decline. Windows path separators and
/// extended-length drive/UNC prefixes are interpreted only on a Windows host;
/// other verbatim namespace subtypes decline rather than publishing a URI that
/// loses their native root. A POSIX backslash remains filename data. This API
/// does not parse incoming URIs.
#[must_use]
pub fn from_path(path: &Path, escaping: FileUriEscaping) -> Option<String> {
    path.has_root().then_some(())?;
    from_rooted_spelling(path.to_str()?, cfg!(windows), escaping)
}

/// Render a rooted, lossless path spelling under the supplied platform geometry.
/// This pure projection never performs any host file operation.
#[must_use]
pub fn from_rooted_spelling(
    path: &str,
    windows: bool,
    escaping: FileUriEscaping,
) -> Option<String> {
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
        } else if matches!(extended.as_bytes(), [drive, b':', b'/', ..] if drive.is_ascii_alphabetic())
        {
            extended
        } else {
            // A verbatim Volume GUID or other non-drive/non-UNC namespace
            // cannot lose `//?/`: that would turn its actual native root into
            // an ordinary, potentially relative path on URI conversion.
            return None;
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

/// Exact byte projection for generated host path URI consumers.
/// Entries are literal ASCII or uppercase percent escapes, under the same
/// forward owner as `from_path`; they supply no inverse or path admission.
#[must_use]
pub fn file_uri_path_byte_projection(escaping: FileUriEscaping) -> Vec<String> {
    (0..=u8::MAX)
        .map(|byte| {
            if path_byte_is_literal(byte, escaping) {
                char::from(byte).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect()
}

fn path_byte_is_literal(byte: u8, escaping: FileUriEscaping) -> bool {
    is_unreserved(byte)
        || byte == b'/'
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
            )
}

fn is_unreserved(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~')
}

/// Encode a URI path using the requested escaped spelling.
#[must_use]
pub fn escaped_path(path: &str, escaping: FileUriEscaping) -> String {
    escape(path, |byte| path_byte_is_literal(byte, escaping))
}

/// Encode a single URI component, leaving only RFC 3986 unreserved bytes.
#[must_use]
pub fn escaped_component(component: &str) -> String {
    escape(component, is_unreserved)
}

fn escape(text: &str, is_literal: impl Fn(u8) -> bool) -> String {
    let mut out = String::with_capacity(text.len());
    for &byte in text.as_bytes() {
        if is_literal(byte) {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

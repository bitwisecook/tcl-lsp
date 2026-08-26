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

//! One canonical string form for document URIs, shared by both sides.
//!
//! The document store keys on the URI the client sent, which is self-consistent
//! within a session. But the server *constructs* URIs too — the workspace scan,
//! the autoload / cross-file `source` resolver, the entry-point resolver — and
//! those went straight through `Uri::from_file_path` with no shared
//! canonicalisation. When the two spellings differ, every cross-file feature
//! (find-references, workspace symbols, rename) sees one file as two: it is
//! keyed by URI string, so two spellings are two documents (issue #1214).
//!
//! # What is normalised, and what deliberately is not
//!
//! Two narrowly scoped syntactic spellings:
//!
//! 1. **Percent-encoding case** — `%3a` and `%3A` are the same octet, so the
//!    hex digits are pinned upper-case. Both `percent_encoding` (what
//!    `Uri::from_file_path` uses) and JavaScript's `encodeURIComponent` (what
//!    `vscode-uri` uses) already emit upper-case, so this only ever repairs an
//!    odd client.
//! 2. **Windows drive-letter case** — `vscode-uri`'s `URI.file()` lower-cases
//!    the drive letter (`file:///c%3A/…`), while `ls_types`' `from_file_path`
//!    upper-cases it (`file:///C%3A/…`). That single character is what made a
//!    server-scanned file and the same file opened in the editor look like two
//!    documents on Windows. Pinning it lower-case makes the server's spelling
//!    the editor's spelling.
//!
//! What is **not** done here is case-folding the rest of the path. NTFS and
//! APFS use different case-insensitivity algorithms, both of them locale- and
//! version-dependent, and emulating either from a URI string is the wrong
//! layer: it would answer "same file" for paths that are genuinely different on
//! a case-sensitive volume mounted under a case-insensitive one. A client that
//! spells a path in two cases is inconsistent with *itself*, which is the
//! client's bug to fix (VS Code is not; `uriConverters` exists for those that
//! are).
//!
//! Server-constructed file paths have one additional boundary repair:
//! [`repair_file_uri_from_path`] makes `ls_types::Uri::from_file_path` agree
//! with VS Code's `URI.file`. A leading `//` becomes a URI authority on every
//! host; on Windows only, the extended-length `//?/` marker is removed first.
//! The repair is deliberately called only by the path-to-URI constructor, not
//! by this module's generic incoming-URI normaliser: a first path segment named
//! `?` and a path beginning `//` must retain their meaning when already
//! expressed as URIs, including on POSIX and macOS.
//!
//! # Tolerant parse
//!
//! [`repair_uri_string`] is the other half: some clients (`JetBrains`- and
//! Neovim-style) send a folder URI with the spaces left raw. That is not a
//! valid URI, so it fails to *deserialise*, and the whole `initialize` request
//! fails — the server never starts. Repairing percent-encodes the characters a
//! URI may not carry raw and re-parses, so such a client is accepted and then
//! normalised, rather than rejected.

use std::borrow::Cow;
use std::fmt::Write as _;

use tower_lsp_server::ls_types::Uri;

/// Characters that may appear literally anywhere in a URI reference:
/// `unreserved` + `sub-delims` + `gen-delims`, plus `%` for existing escapes
/// (RFC 3986 §2). Anything else has to be percent-encoded.
fn is_uri_legal(b: u8) -> bool {
    b.is_ascii_alphanumeric()
        || matches!(
            b,
            b'-' | b'.'
                | b'_'
                | b'~'
                | b'!'
                | b'$'
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
                | b'/'
                | b'?'
                | b'#'
                | b'['
                | b']'
                | b'@'
                | b'%'
        )
}

/// The canonical string form of `raw`: percent-escape hex pinned upper-case and
/// a Windows drive letter pinned lower-case.
///
/// Idempotent, and a byte-for-byte no-op on a URI that is already canonical —
/// which every URI VS Code sends is. Purely syntactic: it never touches the
/// file system and never decodes an escape, so it cannot turn one path into
/// another.
#[must_use]
pub fn canonical_uri_string(raw: &str) -> Cow<'_, str> {
    let bytes = raw.as_bytes();
    let needs_hex_fix = bytes.iter().enumerate().any(|(i, &b)| {
        b == b'%'
            && matches!(bytes.get(i + 1..i + 3), Some(h) if h.iter().any(u8::is_ascii_lowercase))
    });
    let drive = drive_letter_offset(raw);
    let needs_drive_fix = drive.is_some_and(|i| bytes[i].is_ascii_uppercase());
    if !needs_hex_fix && !needs_drive_fix {
        return Cow::Borrowed(raw);
    }
    let mut out: Vec<u8> = bytes.to_vec();
    if needs_hex_fix {
        let mut i = 0;
        while i + 2 < out.len() {
            if out[i] == b'%' && out[i + 1].is_ascii_hexdigit() && out[i + 2].is_ascii_hexdigit() {
                out[i + 1] = out[i + 1].to_ascii_uppercase();
                out[i + 2] = out[i + 2].to_ascii_uppercase();
                i += 3;
            } else {
                i += 1;
            }
        }
    }
    if let Some(i) = drive {
        out[i] = out[i].to_ascii_lowercase();
    }
    Cow::Owned(String::from_utf8(out).unwrap_or_else(|_| raw.to_owned()))
}

/// Repair the file-URI shape `ls_types::Uri::from_file_path` produces for a
/// filesystem path beginning with `//`.
///
/// VS Code's `URI.file` represents a leading `//host/` as the authority on
/// every host, preserving POSIX/macOS double-slash paths as `file://host/…`
/// while also producing the standard UNC form on Windows. Windows has one
/// additional case: the encoded extended-length marker (`?`) is namespace
/// metadata, not part of the URI, and is removed for both drive and `UNC`
/// forms. `windows` is explicit rather than hidden behind `#[cfg]` so every
/// branch is type-checked and unit-tested on every CI host.
///
/// This function must only receive the output of `Uri::from_file_path`; it is
/// not a generic URI canonicalisation. Returns `None` for an ordinary path.
pub(crate) fn repair_file_uri_from_path(raw: &str, windows: bool) -> Option<String> {
    // `from_file_path` prepends `file://` on Unix and `file:///` on Windows.
    // Removing exactly that platform prefix recovers its encoded path without
    // having to guess what four or five slashes meant. In particular,
    // `file:///%3F/foo` is the valid POSIX path `/?/foo`, not a Windows
    // extended-length marker.
    let encoded_path = if windows {
        raw.strip_prefix("file:///")?
    } else {
        raw.strip_prefix("file://")?
    };
    let authority_and_path = encoded_path.strip_prefix("//")?;

    if windows {
        let extended = authority_and_path
            .strip_prefix("%3F/")
            .or_else(|| authority_and_path.strip_prefix("%3f/"))
            .or_else(|| authority_and_path.strip_prefix("?/"));
        if let Some(extended) = extended {
            if let Some((kind, unc)) = extended.split_once('/')
                && kind.eq_ignore_ascii_case("UNC")
            {
                return Some(format!("file://{unc}"));
            }
            return Some(format!("file:///{extended}"));
        }
    }

    // This is the host/path split used by vscode-uri's `URI.file`: everything
    // before the first slash is the authority; the rest is the absolute path.
    // A bare `//host` receives `/`, while `///path` has an empty authority and
    // becomes the ordinary local URI `file:///path`.
    match authority_and_path.split_once('/') {
        Some((authority, path)) => Some(format!("file://{authority}/{path}")),
        None if authority_and_path.is_empty() => Some("file:///".to_owned()),
        None => Some(format!("file://{authority_and_path}/")),
    }
}

/// Byte offset of the Windows drive letter in `raw`, when its path starts with
/// one — `file:///C:/…` or `file:///C%3A/…`.
///
/// Only for a `file:` URI with an empty authority, which is the only shape
/// either `URI.file()` or `Uri::from_file_path` produces for a Windows path.
fn drive_letter_offset(raw: &str) -> Option<usize> {
    const PREFIX: &str = "file:///";
    let rest = raw.strip_prefix(PREFIX)?;
    let letter = rest.as_bytes().first().copied()?;
    if !letter.is_ascii_alphabetic() {
        return None;
    }
    let tail = rest.get(1..)?;
    // `C:/…`, `C:` at the very end, `C%3A/…`, or `C%3a` — a colon written
    // either way, and with or without a trailing path.
    let colon = tail == ":"
        || tail.starts_with(":/")
        || tail.eq_ignore_ascii_case("%3a")
        || tail.len() > 3 && tail[..3].eq_ignore_ascii_case("%3a") && tail.as_bytes()[3] == b'/';
    colon.then_some(PREFIX.len())
}

/// Percent-encode the characters a client left raw, so a URI string that does
/// not parse has a chance of parsing.
///
/// Returns `None` when `raw` already parses (nothing to repair) or when the
/// repair still does not parse (the string is not a URI at all, and the caller
/// should let the ordinary error path report it).
///
/// Existing escapes are preserved: a `%` followed by two hex digits is left
/// alone, and a stray `%` is escaped to `%25` so a literal per-cent sign in a
/// filename survives.
#[must_use]
pub fn repair_uri_string(raw: &str) -> Option<String> {
    if raw.parse::<Uri>().is_ok() {
        return None;
    }
    let bytes = raw.as_bytes();
    let mut out = String::with_capacity(raw.len());
    let mut i = 0;
    while i < bytes.len() {
        let b = bytes[i];
        if b == b'%' {
            let is_escape =
                matches!(bytes.get(i + 1..i + 3), Some(h) if h.iter().all(u8::is_ascii_hexdigit));
            if is_escape {
                out.push_str(&raw[i..i + 3]);
                i += 3;
            } else {
                out.push_str("%25");
                i += 1;
            }
            continue;
        }
        if is_uri_legal(b) {
            out.push(char::from(b));
        } else {
            let _ = write!(out, "%{b:02X}");
        }
        i += 1;
    }
    out.parse::<Uri>().ok().map(|_| out)
}

/// The `file:` URI for an absolute path, for the one target
/// `ls_types::Uri::from_file_path` cannot serve.
///
/// That function gates on `Path::is_absolute`, and `std` answers `false` for
/// **every** path on `wasm32-unknown-unknown`: the target is neither `unix` nor
/// `windows`, and outside those `is_absolute` additionally demands a path
/// *prefix*, which only Windows paths have. `from_file_path` therefore takes
/// its relative-path branch and tries to canonicalise against a filesystem a
/// browser worker does not have, so it returns `None` for `/ws/main.tcl` — and
/// with it every URI the server derives from a path it found itself. The
/// workspace scan indexed nothing at all as a result: each scanned file was
/// dropped before it could be read, on a target where the store had the bytes
/// all along.
///
/// The output is the same spelling `from_file_path`'s non-Windows branch
/// produces — `file://` plus the percent-encoded path — with
/// [`repair_uri_string`]'s encoding rule doing the escaping, so a scanned path
/// meets the canonical form the rest of the server (and the client) uses.
/// Returns `None` for a path with no root, or one that is not valid UTF-8:
/// neither can be spelled as a `file:` URI here.
#[must_use]
pub fn rooted_file_uri(path: &std::path::Path) -> Option<Uri> {
    if !path.has_root() {
        return None;
    }
    let raw = format!("file://{}", path.to_str()?);
    match raw.parse::<Uri>() {
        Ok(uri) => Some(uri),
        Err(_) => repair_uri_string(&raw)?.parse().ok(),
    }
}

/// JSON object keys whose string value is a document / folder URI.
///
/// Every URI-shaped field in the LSP request surface the server handles, so a
/// client that sends an unencoded one anywhere is repaired at the boundary
/// rather than failing to deserialise.
const URI_KEYS: &[&str] = &[
    "uri",
    "scopeUri",
    "rootUri",
    "oldUri",
    "newUri",
    "targetUri",
    "externalUri",
];

/// Walk an incoming message's `params` and put every URI-valued string into the
/// canonical form, repairing one that does not parse at all, in place.
///
/// **This is the client half of the shared canonicalisation.** Applied once, at
/// the transport boundary, before anything is deserialised — so every handler,
/// the document store, and the workspace index all see the same spelling the
/// server's own [`canonical_file_uri`] produces. Doing it per handler instead
/// would be sixty places to keep in step, and the document store would key on
/// one spelling while a later request looked up another.
///
/// Two effects, in order:
///
/// * **Repair** — a folder URI with a raw space (as some `JetBrains`- and
///   Neovim-style clients send) otherwise fails `Uri`'s deserialisation and
///   takes the whole `initialize` down with it, so the session never starts.
/// * **Canonicalise** — [`canonical_uri_string`], so a client that upper-cases
///   a Windows drive letter or lower-cases its escapes still meets the URIs the
///   server constructs for the same files.
///
/// Inert for a conforming client: a VS Code URI comes out byte-for-byte
/// unchanged.
///
/// [`canonical_file_uri`]: crate::canonical_file_uri
pub fn normalise_uris_in_params(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            for (key, child) in map.iter_mut() {
                if URI_KEYS.contains(&key.as_str())
                    && let serde_json::Value::String(s) = child
                {
                    if let Some(repaired) = repair_uri_string(s) {
                        *s = repaired;
                    }
                    if let Cow::Owned(canonical) = canonical_uri_string(s) {
                        *s = canonical;
                    }
                    continue;
                }
                normalise_uris_in_params(child);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                normalise_uris_in_params(item);
            }
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::{
        canonical_uri_string, normalise_uris_in_params, repair_file_uri_from_path,
        repair_uri_string, rooted_file_uri,
    };
    use serde_json::json;

    #[test]
    fn a_vscode_form_uri_is_already_canonical() {
        for uri in [
            "file:///home/me/lib.tcl",
            "file:///home/me/my%20file.tcl",
            "file:///c%3A/Users/me/lib.tcl",
            "file:///c%3A/Users/me/My%20Documents/lib.tcl",
            "untitled:Untitled-1",
        ] {
            assert_eq!(canonical_uri_string(uri), uri, "{uri} must be untouched");
        }
    }

    /// Windows `std::fs::canonicalize` returns an extended-length path.  When
    /// such a path belongs to a `SpecTcl` pack notice, it is converted back to a
    /// URI for `publishDiagnostics`; this is the spelling that made VS Code's
    /// diagnostic queue throw `UriError` instead of displaying the notice.
    #[test]
    fn a_windows_extended_drive_uri_is_repaired() {
        let repaired = repair_file_uri_from_path(
            "file://///%3F/C%3A/Users/me/project/.tcl-lsp/bad.tclspec",
            true,
        )
        .unwrap();
        assert_eq!(
            repaired,
            "file:///C%3A/Users/me/project/.tcl-lsp/bad.tclspec",
        );
        assert_eq!(
            canonical_uri_string(&repaired),
            "file:///c%3A/Users/me/project/.tcl-lsp/bad.tclspec",
        );
    }

    #[test]
    fn windows_unc_uri_shapes_gain_an_authority() {
        for malformed in [
            "file://///server/share/project/spec.tclspec",
            "file://///%3F/UNC/server/share/project/spec.tclspec",
            "file://///%3f/Unc/server/share/project/spec.tclspec",
        ] {
            assert_eq!(
                repair_file_uri_from_path(malformed, true).as_deref(),
                Some("file://server/share/project/spec.tclspec"),
                "{malformed}",
            );
        }
    }

    #[test]
    fn generic_canonicalisation_preserves_posix_question_mark_segment() {
        let uri = "file:///%3F/foo.tcl";
        assert_eq!(canonical_uri_string(uri), uri);
        assert_eq!(repair_file_uri_from_path(uri, true), None);
    }

    #[test]
    fn generic_canonicalisation_preserves_posix_double_slash_path() {
        let uri = "file:////srv/share/a.tcl";
        assert_eq!(canonical_uri_string(uri), uri);
        assert_eq!(
            repair_file_uri_from_path(uri, false).as_deref(),
            Some("file://srv/share/a.tcl"),
        );
    }

    #[test]
    fn posix_double_slash_question_mark_is_not_windows_metadata() {
        assert_eq!(
            repair_file_uri_from_path("file:////%3F/foo.tcl", false).as_deref(),
            Some("file://%3F/foo.tcl"),
        );
    }

    #[test]
    fn path_authority_split_matches_vscode_uri_file() {
        for (raw, expected) in [
            ("file:////server", "file://server/"),
            ("file:////", "file:///"),
            ("file://///local/path", "file:///local/path"),
        ] {
            assert_eq!(
                repair_file_uri_from_path(raw, false).as_deref(),
                Some(expected),
                "{raw}",
            );
        }
    }

    #[test]
    fn browser_and_virtual_workspace_uris_keep_their_structure() {
        for uri in [
            "https://example.test/workspace/%3F/file.tcl?ref=C%3A#source",
            "http://localhost:3000/workspace//file.tcl",
            "vscode-vfs://github/owner/repo/path/file.tcl",
            "vscode-remote://ssh-remote+host/home/me/file.tcl",
            "untitled:Untitled-1",
        ] {
            assert_eq!(canonical_uri_string(uri), uri, "{uri}");
            assert_eq!(repair_file_uri_from_path(uri, cfg!(windows)), None, "{uri}");
        }
    }

    #[test]
    fn incoming_browser_and_virtual_workspace_uris_are_not_reinterpreted() {
        let uris = [
            "https://example.test/workspace/%3F/file.tcl?ref=C%3A#source",
            "http://localhost:3000/workspace//file.tcl",
            "vscode-vfs://github/owner/repo/path/file.tcl",
            "vscode-remote://ssh-remote+host/home/me/file.tcl",
            "file:///%3F/foo.tcl",
            "file:////srv/share/a.tcl",
        ];
        let mut params = json!({
            "workspaceFolders": uris.map(|uri| json!({ "uri": uri })),
        });

        normalise_uris_in_params(&mut params);

        for (index, expected) in uris.iter().enumerate() {
            assert_eq!(params["workspaceFolders"][index]["uri"], *expected);
        }
    }

    /// The mismatch that made one file look like two on Windows:
    /// `ls_types::Uri::from_file_path` upper-cases the drive letter,
    /// `vscode-uri`'s `URI.file()` lower-cases it.
    #[test]
    fn a_windows_drive_letter_is_pinned_lower_case() {
        assert_eq!(
            canonical_uri_string("file:///C%3A/Users/me/lib.tcl"),
            "file:///c%3A/Users/me/lib.tcl",
        );
        assert_eq!(
            canonical_uri_string("file:///C:/Users/me/lib.tcl"),
            "file:///c:/Users/me/lib.tcl",
        );
        // The rest of the path keeps its case — this is drive-letter pinning,
        // not a file-system case-folding emulation.
        assert_eq!(
            canonical_uri_string("file:///C%3A/Users/Me/LIB.TCL"),
            "file:///c%3A/Users/Me/LIB.TCL",
        );
    }

    #[test]
    fn a_rooted_path_gets_the_same_uri_from_file_path_would_give() {
        // The `wasm32-unknown-unknown` fallback must not invent a second
        // spelling: on a host where `from_file_path` works, the two agree.
        let path = std::path::Path::new("/ws/lib/helpers.tcl");
        assert_eq!(
            rooted_file_uri(path).map(|u| u.as_str().to_owned()),
            Some("file:///ws/lib/helpers.tcl".to_owned()),
        );
        #[cfg(unix)]
        assert_eq!(
            rooted_file_uri(path),
            tower_lsp_server::ls_types::Uri::from_file_path(path),
        );
    }

    #[test]
    fn a_rooted_path_with_a_space_is_percent_encoded() {
        assert_eq!(
            rooted_file_uri(std::path::Path::new("/ws/my dir/a.tcl"))
                .map(|u| u.as_str().to_owned()),
            Some("file:///ws/my%20dir/a.tcl".to_owned()),
        );
    }

    #[test]
    fn a_relative_path_has_no_rooted_file_uri() {
        assert!(rooted_file_uri(std::path::Path::new("lib/a.tcl")).is_none());
    }

    #[test]
    fn percent_escape_hex_is_pinned_upper_case() {
        assert_eq!(
            canonical_uri_string("file:///home/me/my%20file%2bmore.tcl"),
            "file:///home/me/my%20file%2Bmore.tcl",
        );
    }

    #[test]
    fn canonicalisation_is_idempotent() {
        for uri in [
            "file:///C%3A/Users/me/a%2bb.tcl",
            "file:///home/me/plain.tcl",
            "file:///C:/x",
        ] {
            let once = canonical_uri_string(uri).into_owned();
            assert_eq!(canonical_uri_string(&once), once, "{uri}");
        }
    }

    /// A path segment that merely *looks* like a drive letter is left alone —
    /// only a `file:` URI whose path starts `X:` is one.
    #[test]
    fn a_non_drive_path_is_not_touched() {
        for uri in [
            "file:///Cat/lib.tcl",
            "file:///home/C%3Along/lib.tcl",
            "https://Example.com/C%3A/x",
        ] {
            assert_eq!(canonical_uri_string(uri), uri, "{uri}");
        }
    }

    #[test]
    fn a_well_formed_uri_needs_no_repair() {
        assert_eq!(repair_uri_string("file:///home/me/my%20file.tcl"), None);
        assert_eq!(repair_uri_string("file:///c%3A/Users/me"), None);
    }

    /// Issue #1214 §B: a folder URI with the space left raw used to fail
    /// deserialisation and take `initialize` down with it.
    #[test]
    fn an_unencoded_space_is_repaired() {
        assert_eq!(
            repair_uri_string("file:///home/me/my project").as_deref(),
            Some("file:///home/me/my%20project"),
        );
    }

    #[test]
    fn existing_escapes_survive_a_repair() {
        assert_eq!(
            repair_uri_string("file:///home/me/a%2Bb/my project").as_deref(),
            Some("file:///home/me/a%2Bb/my%20project"),
        );
        // A stray `%` is a literal per-cent sign, not a broken escape.
        assert_eq!(
            repair_uri_string("file:///home/me/100% done/x.tcl").as_deref(),
            Some("file:///home/me/100%25%20done/x.tcl"),
        );
    }

    #[test]
    fn a_raw_non_ascii_path_is_repaired() {
        assert_eq!(
            repair_uri_string("file:///home/me/café.tcl").as_deref(),
            Some("file:///home/me/caf%C3%A9.tcl"),
        );
    }

    #[test]
    fn a_string_that_is_not_a_uri_at_all_is_left_to_the_error_path() {
        assert_eq!(repair_uri_string("not a uri"), None);
        assert_eq!(repair_uri_string(""), None);
    }

    #[test]
    fn params_are_normalised_wherever_a_uri_key_appears() {
        let mut params = json!({
            "workspaceFolders": [
                { "uri": "file:///home/me/my project", "name": "my project" },
                { "uri": "file:///home/me/ok", "name": "ok" },
            ],
            "textDocument": { "uri": "file:///home/me/a b.tcl" },
            "notAUri": "file:///home/me/left alone",
        });
        normalise_uris_in_params(&mut params);
        assert_eq!(
            params["workspaceFolders"][0]["uri"],
            json!("file:///home/me/my%20project"),
        );
        assert_eq!(
            params["workspaceFolders"][1]["uri"],
            json!("file:///home/me/ok")
        );
        assert_eq!(
            params["textDocument"]["uri"],
            json!("file:///home/me/a%20b.tcl")
        );
        // The folder's display *name* is not a URI and must not be rewritten.
        assert_eq!(params["workspaceFolders"][0]["name"], json!("my project"));
        assert_eq!(params["notAUri"], json!("file:///home/me/left alone"));
    }

    /// A client that spells a Windows path with an upper-case drive letter is
    /// brought to the same spelling the server's own construction uses, so the
    /// two do not look like two documents (issue #1214).
    #[test]
    fn params_are_canonicalised_not_only_repaired() {
        let mut params = json!({
            "textDocument": { "uri": "file:///C%3A/Users/me/lib.tcl" },
        });
        normalise_uris_in_params(&mut params);
        assert_eq!(
            params["textDocument"]["uri"],
            json!("file:///c%3A/Users/me/lib.tcl"),
        );
    }

    /// A repair and a canonicalisation can be needed by the same string.
    #[test]
    fn a_repaired_uri_is_canonicalised_too() {
        let mut params = json!({ "rootUri": "file:///C:/Users/me/my project" });
        normalise_uris_in_params(&mut params);
        assert_eq!(params["rootUri"], json!("file:///c:/Users/me/my%20project"),);
    }
}

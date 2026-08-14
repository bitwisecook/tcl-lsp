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
//! Three purely syntactic things:
//!
//! 1. **Windows file-URI shape** — `ls_types::Uri::from_file_path` turns an
//!    extended-length drive path into `file://///%3F/C%3A/…` and a UNC path
//!    into `file://///server/share/…`. VS Code rejects both because their empty
//!    authority is followed by a path beginning `//`; they are rewritten as
//!    `file:///c%3A/…` and `file://server/share/…`, respectively.
//! 2. **Percent-encoding case** — `%3a` and `%3A` are the same octet, so the
//!    hex digits are pinned upper-case. Both `percent_encoding` (what
//!    `Uri::from_file_path` uses) and JavaScript's `encodeURIComponent` (what
//!    `vscode-uri` uses) already emit upper-case, so this only ever repairs an
//!    odd client.
//! 3. **Windows drive-letter case** — `vscode-uri`'s `URI.file()` lower-cases
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

/// The canonical string form of `raw`: valid Windows file-URI shape,
/// percent-escape hex pinned upper-case, and a Windows drive letter pinned
/// lower-case.
///
/// Idempotent, and a byte-for-byte no-op on a URI that is already canonical —
/// which every URI VS Code sends is. Purely syntactic: it never touches the
/// file system and never decodes an escape, so it cannot turn one path into
/// another.
#[must_use]
pub fn canonical_uri_string(raw: &str) -> Cow<'_, str> {
    // `ls_types::Uri::from_file_path` prefixes every Windows path with
    // `file:///`.  That is right for a drive path, but Windows filesystem APIs
    // may return an extended-length path (`\\?\C:\...` or
    // `\\?\UNC\server\share\...`) and ordinary UNC paths start with `\\`.
    // After slash normalisation those prefixes produce `file://///%3F/...` or
    // `file://///server/...`: fluent-uri accepts both, but VS Code rejects
    // them because they have an empty authority and a path beginning `//`.
    // Repair that dependency-boundary spelling before applying the two
    // ordinary canonicalisations below.  Keeping this string-only also makes
    // the Windows cases testable on every CI host.
    if let Some(windows_shape) = canonical_windows_file_uri_shape(raw) {
        // The repaired shape no longer matches the helper, so this recursion
        // is one step only; it applies escape- and drive-case normalisation to
        // the repaired URI without ever borrowing the temporary string.
        return Cow::Owned(canonical_uri_string(&windows_shape).into_owned());
    }
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

/// Repair the file-URI shapes produced for Windows extended-length and UNC
/// paths by `ls_types::Uri::from_file_path`.
///
/// Returns `None` for an ordinary URI.  A URI with four or more slashes after
/// `file:` has no authority and begins its path with `//`; the first path
/// component is therefore the UNC authority.  The encoded extended-length
/// marker (`?`) is metadata from the Windows path namespace, not part of the
/// file URI, and is removed for both drive and `UNC` forms.
fn canonical_windows_file_uri_shape(raw: &str) -> Option<String> {
    let rest = raw.strip_prefix("file:")?;
    let slash_count = rest.bytes().take_while(|&b| b == b'/').count();
    if slash_count < 3 {
        return None;
    }

    let tail = &rest[slash_count..];
    let extended = tail
        .strip_prefix("%3F/")
        .or_else(|| tail.strip_prefix("%3f/"))
        .or_else(|| tail.strip_prefix("?/"));
    if let Some(extended) = extended {
        if let Some(unc) = extended
            .strip_prefix("UNC/")
            .or_else(|| extended.strip_prefix("unc/"))
        {
            return Some(format!("file://{unc}"));
        }
        return Some(format!("file:///{extended}"));
    }

    (slash_count > 3).then(|| format!("file://{tail}"))
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
    use super::{canonical_uri_string, normalise_uris_in_params, repair_uri_string};
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
        assert_eq!(
            canonical_uri_string("file://///%3F/C%3A/Users/me/project/.tcl-lsp/bad.tclspec"),
            "file:///c%3A/Users/me/project/.tcl-lsp/bad.tclspec",
        );
    }

    #[test]
    fn windows_unc_uri_shapes_gain_an_authority() {
        for malformed in [
            "file://///server/share/project/spec.tclspec",
            "file:////server/share/project/spec.tclspec",
            "file://///%3F/UNC/server/share/project/spec.tclspec",
        ] {
            assert_eq!(
                canonical_uri_string(malformed),
                "file://server/share/project/spec.tclspec",
                "{malformed}",
            );
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

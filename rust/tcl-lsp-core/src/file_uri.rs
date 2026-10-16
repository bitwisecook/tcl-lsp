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

//! File URI construction and checked native document/workspace path conversion.

use std::borrow::Cow;
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
/// extended-length drive/UNC prefixes are interpreted only on a Windows host;
/// other verbatim namespace subtypes decline rather than publishing a URI that
/// loses their native root. A POSIX backslash remains filename data. This API
/// does not parse incoming URIs.
#[must_use]
pub fn from_path(path: &Path, escaping: FileUriEscaping) -> Option<String> {
    path.has_root().then_some(())?;
    from_rooted_spelling(path.to_str()?, cfg!(windows), escaping)
}

/// Readonly link candidate under the supplied host workspace/home policy.
/// Only `~` and `~/` use the supplied absolute home; other relative spellings
/// require the supplied absolute workspace base. This never consults process
/// cwd, a user database, the filesystem or an incoming URI. Host Path joining
/// supplies the spelling; encoding uses the same rooted host-path owner.
#[must_use]
pub(crate) fn document_link_target(
    literal: &str,
    workspace_root: Option<&Path>,
    home: Option<&Path>,
) -> Option<String> {
    if literal.is_empty() {
        return None;
    }
    let expanded = if literal == "~" {
        home.filter(|path| path.is_absolute())?.to_path_buf()
    } else if let Some(rest) = literal.strip_prefix("~/") {
        let home = home.filter(|path| path.is_absolute())?;
        // Repeated tilde separators remain within the supplied home. They
        // cannot turn the suffix into an independently rooted host path.
        let relative = Path::new(rest.trim_start_matches('/'));
        if relative.has_root()
            || relative
                .components()
                .any(|component| matches!(component, std::path::Component::Prefix(_)))
        {
            return None;
        }
        home.join(relative)
    } else {
        literal.into()
    };
    let resolved = if expanded.is_absolute() {
        expanded
    } else {
        workspace_root
            .filter(|path| path.is_absolute())?
            .join(expanded)
    };
    from_path(&resolved, FileUriEscaping::PathCharacters)
}

/// Render the explicit VS Code document-key spelling of an actual rooted Path.
/// Unlike [`from_path`], POSIX leading `//` uses the product's authority alias;
/// this may collapse root spelling and is not an inverse or filesystem identity.
/// Callers needing the original Path must retain it separately. Unicode/NUL and
/// Windows namespace admission are the same as the ordinary forward owner.
#[must_use]
pub fn from_path_for_vscode_document(path: &Path) -> Option<String> {
    path.has_root().then_some(())?;
    vscode_document_spelling(path.to_str()?, cfg!(windows))
}

fn vscode_document_spelling(path: &str, windows: bool) -> Option<String> {
    let ordinary = from_rooted_spelling(path, windows, FileUriEscaping::Unreserved)?;
    if !windows && let Some(remote) = path.strip_prefix("//") {
        let (authority, tail) = remote.split_once('/').unwrap_or((remote, ""));
        return Some(format!(
            "file://{}/{}",
            escaped_path(authority, FileUriEscaping::Unreserved),
            escaped_path(tail, FileUriEscaping::Unreserved),
        ));
    }
    Some(ordinary)
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
    byte.is_ascii_alphanumeric()
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
            )
}

pub(crate) fn escaped_path(path: &str, escaping: FileUriEscaping) -> String {
    let mut out = String::with_capacity(path.len());
    for &byte in path.as_bytes() {
        let literal = path_byte_is_literal(byte, escaping);
        if literal {
            out.push(char::from(byte));
        } else {
            let _ = write!(out, "%{byte:02X}");
        }
    }
    out
}

/// A file document URI cannot be represented under the selected host policy.
/// Refusal is not evidence that a file, workspace or source provider is absent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileUriPathUnavailable {
    /// The current host has no supported native path policy.
    UnsupportedHost,
    /// The original protocol text is not a valid absolute URI.
    InvalidUri,
    /// The URI scheme does not select a filesystem document.
    NotFile,
    /// A query or fragment cannot be represented as a native file path.
    QueryOrFragment,
    /// The authority contains unsupported host, user, port or separator data.
    UnsupportedAuthority,
    /// Strict decoding of the authority or path did not produce valid UTF-8.
    InvalidUnicode,
    /// The decoded authority or path contains a NUL byte.
    Nul,
    /// A decoded path segment contains a native path separator.
    EncodedSeparator,
    /// The path does not have a supported absolute native root.
    UnsupportedNativeGeometry,
}

/// Select the explicit incoming file-URI purpose before native path admission.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileUriPathPurpose {
    /// LSP document/workspace interoperability with VS Code's authority spelling.
    /// This supplies neither a host file-action policy nor a browser virtual path.
    VsCodeDocument,
}

/// Parse an original document/workspace URI and retain its counted native path.
///
/// Under [`FileUriPathPurpose::VsCodeDocument`], POSIX `file://host/share`
/// retains `//host/share`, including a literal `localhost` authority. Windows
/// alone constructs drive or UNC paths. The actual URI parser strictly decodes
/// complete authority/path data; invalid Unicode, NUL, encoded native separators
/// and unsupported native namespace geometries refuse.
///
/// The input is the original protocol URI, not a displayed path or name report.
/// No filesystem canonicalisation, existence, source freshness, file identity,
/// load or edit authority follows. Browser virtual paths and host file actions
/// retain their separate boundaries. Refusal never proves a file/provider absent.
///
/// # Errors
///
/// Returns a typed refusal when the URI or its decoded data cannot be represented
/// under the selected purpose and actual native host policy.
pub fn to_path(
    original_uri: &str,
    purpose: FileUriPathPurpose,
) -> Result<std::path::PathBuf, FileUriPathUnavailable> {
    if !cfg!(any(unix, windows)) {
        return Err(FileUriPathUnavailable::UnsupportedHost);
    }
    let uri =
        fluent_uri::Uri::parse(original_uri).map_err(|_| FileUriPathUnavailable::InvalidUri)?;
    let spelling = match purpose {
        FileUriPathPurpose::VsCodeDocument => document_file_path_spelling(&uri, cfg!(windows))?,
    };
    let path = std::path::PathBuf::from(spelling);
    if !path.is_absolute() {
        return Err(FileUriPathUnavailable::UnsupportedNativeGeometry);
    }
    Ok(path)
}

/// The slash namespace selected by an actual source provider.
/// Neither purpose grants a native host path or file-action capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceFileUriNamespace {
    /// The rooted, counted namespace of a host-filled virtual source store.
    Virtual,
    /// The rooted namespace interpreted by actual WASI filesystem/preopen calls.
    WasiGuest,
}

/// Strict file-URI data for an explicitly selected virtual or guest source
/// namespace. Authority is retained as a counted `//authority/path`; there is
/// no host-OS inference, drive conversion, canonicalisation or existence claim.
/// The actual SourceStore selects the namespace and owns resource lookup.
///
/// # Errors
/// Returns the same strict URI/data refusals as the native document recipe.
pub fn to_source_namespace_path(
    original_uri: &str,
    namespace: SourceFileUriNamespace,
) -> Result<std::path::PathBuf, FileUriPathUnavailable> {
    let uri =
        fluent_uri::Uri::parse(original_uri).map_err(|_| FileUriPathUnavailable::InvalidUri)?;
    let spelling = match namespace {
        SourceFileUriNamespace::Virtual | SourceFileUriNamespace::WasiGuest => {
            document_file_path_spelling(&uri, false)?
        }
    };
    // Rooted slash data is the declared provider namespace. On a browser
    // target Path::is_absolute is unavailable even for this rooted spelling.
    if !spelling.starts_with('/') {
        return Err(FileUriPathUnavailable::UnsupportedNativeGeometry);
    }
    Ok(std::path::PathBuf::from(spelling))
}

// Pure platform spelling recipe. Only to_path selects the actual
// host and admits a native Path; foreign-platform tests grant no host access.
fn document_file_path_spelling(
    uri: &fluent_uri::Uri<&str>,
    windows: bool,
) -> Result<String, FileUriPathUnavailable> {
    use FileUriPathUnavailable as Unavailable;
    if !uri.scheme().as_str().eq_ignore_ascii_case("file") {
        return Err(Unavailable::NotFile);
    }
    if uri.query().is_some() || uri.fragment().is_some() {
        return Err(Unavailable::QueryOrFragment);
    }
    let authority = match uri.authority() {
        None => Cow::Borrowed(""),
        Some(authority) => {
            if authority.userinfo().is_some() || authority.port().is_some() {
                return Err(Unavailable::UnsupportedAuthority);
            }
            match authority.host_parsed() {
                fluent_uri::component::Host::RegName(name) => name
                    .decode()
                    .to_string()
                    .map_err(|_| Unavailable::InvalidUnicode)?,
                fluent_uri::component::Host::Ipv4(..) => Cow::Borrowed(authority.host()),
                _ => return Err(Unavailable::UnsupportedAuthority),
            }
        }
    };
    if authority.contains('\0') {
        return Err(Unavailable::Nul);
    }
    if authority.contains('/')
        || windows
            && (authority.contains(['\\', ':']) || matches!(authority.as_ref(), "." | ".." | "?"))
    {
        return Err(Unavailable::UnsupportedAuthority);
    }
    let segments = uri
        .path()
        .segments_if_absolute()
        .ok_or(Unavailable::UnsupportedNativeGeometry)?;
    for segment in segments {
        let decoded = segment
            .decode()
            .to_string()
            .map_err(|_| Unavailable::InvalidUnicode)?;
        if decoded.contains('\0') {
            return Err(Unavailable::Nul);
        }
        if decoded.contains('/') || windows && decoded.contains('\\') {
            return Err(Unavailable::EncodedSeparator);
        }
    }
    let path = uri
        .path()
        .decode()
        .to_string()
        .map_err(|_| Unavailable::InvalidUnicode)?;
    if !authority.is_empty() {
        let mut spelling = String::from(if windows { "\\\\" } else { "//" });
        spelling.push_str(&authority);
        if windows {
            spelling.push_str(&path.replace('/', "\\"));
        } else {
            spelling.push_str(&path);
        }
        return Ok(spelling);
    }
    if windows {
        let local = path
            .strip_prefix('/')
            .filter(|local| {
                matches!(local.as_bytes(), [drive, b':', b'/', ..] if drive.is_ascii_alphabetic())
            })
            .ok_or(Unavailable::UnsupportedNativeGeometry)?;
        Ok(local.replace('/', "\\"))
    } else {
        Ok(path.into_owned())
    }
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

    #[test]
    fn unsupported_windows_verbatim_subtypes_do_not_lose_their_native_root() {
        for path in [
            r"\\?\Volume{01234567-89ab-cdef-0123-456789abcdef}\project\file.tcl",
            r"\\?\GLOBALROOT\Device\HarddiskVolume1\file.tcl",
            r"\\?\C:relative.tcl",
        ] {
            for escaping in [FileUriEscaping::PathCharacters, FileUriEscaping::Unreserved] {
                assert!(standalone_windows_path(path, escaping).is_none(), "{path}");
            }
        }
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

#[cfg(test)]
mod inverse_tests {
    use super::*;
    use fluent_uri::Uri;

    #[cfg(unix)]
    #[test]
    fn actual_posix_document_paths_keep_the_whole_authority_and_counted_path() {
        for (raw, expected) in [
            ("file://srv/share/a.tcl", "//srv/share/a.tcl"),
            ("file://localhost/share/a.tcl", "//localhost/share/a.tcl"),
            ("file://Srv/share//tail/", "//Srv/share//tail/"),
            (
                "file://m%C3%BCnchen/share/%CE%BB.tcl",
                "//münchen/share/λ.tcl",
            ),
            ("file://ho%40st/share/a.tcl", "//ho@st/share/a.tcl"),
            ("file://127.0.0.1/share/a.tcl", "//127.0.0.1/share/a.tcl"),
            ("file://%3F/foo.tcl", "//?/foo.tcl"),
            ("file:////srv/share/a.tcl", "//srv/share/a.tcl"),
            ("file://///srv/share/a.tcl", "///srv/share/a.tcl"),
            (
                "file:///tmp/literal%252F%5C%EF%BF%BD.tcl",
                "/tmp/literal%2F\\�.tcl",
            ),
            ("file:///C%3A/data.tcl", "/C:/data.tcl"),
            ("file:///", "/"),
        ] {
            let path = to_path(raw, FileUriPathPurpose::VsCodeDocument).unwrap();
            // Path equality may hide duplicate separators; preserve the actual
            // native spelling as data, with no component normalisation.
            assert_eq!(path.to_str(), Some(expected), "{raw}");
            assert!(path.is_absolute(), "{raw}");
        }
    }

    #[test]
    fn unsupported_document_uri_data_is_a_typed_refusal() {
        use FileUriPathUnavailable as Unavailable;
        for (raw, expected) in [
            ("untitled:buffer", Unavailable::NotFile),
            ("https://srv/share/a.tcl", Unavailable::NotFile),
            ("file:///tmp/a.tcl?query", Unavailable::QueryOrFragment),
            ("file:///tmp/a.tcl#fragment", Unavailable::QueryOrFragment),
            (
                "file://user@srv/share/a.tcl",
                Unavailable::UnsupportedAuthority,
            ),
            (
                "file://srv:80/share/a.tcl",
                Unavailable::UnsupportedAuthority,
            ),
            (
                "file://[::1]/share/a.tcl",
                Unavailable::UnsupportedAuthority,
            ),
            (
                "file://srv%2Fother/share/a.tcl",
                Unavailable::UnsupportedAuthority,
            ),
            ("file://%FF/share/a.tcl", Unavailable::InvalidUnicode),
            ("file:///tmp/%FF.tcl", Unavailable::InvalidUnicode),
            ("file://srv%00/share/a.tcl", Unavailable::Nul),
            ("file:///tmp/%00.tcl", Unavailable::Nul),
            ("file:///tmp/a%2Fb.tcl", Unavailable::EncodedSeparator),
            ("file:relative.tcl", Unavailable::UnsupportedNativeGeometry),
            ("file://srv", Unavailable::UnsupportedNativeGeometry),
        ] {
            let uri = Uri::parse(raw).unwrap();
            assert_eq!(
                document_file_path_spelling(&uri, false),
                Err(expected),
                "{raw}"
            );
            #[cfg(any(unix, windows))]
            assert_eq!(
                to_path(raw, FileUriPathPurpose::VsCodeDocument),
                Err(expected),
                "{raw}"
            );
        }
    }

    #[test]
    fn windows_document_recipe_keeps_drive_case_unc_and_whole_path() {
        for (raw, expected) in [
            (
                "file:///C%3A/Users/Me/a%20%CE%BB.tcl",
                r"C:\Users\Me\a λ.tcl",
            ),
            ("file:///c:/Users/Me/a.tcl", r"c:\Users\Me\a.tcl"),
            ("file://Srv/share//tail/", "\\\\Srv\\share\\\\tail\\"),
            ("file://ho%40st/share/a.tcl", r"\\ho@st\share\a.tcl"),
        ] {
            let uri = Uri::parse(raw).unwrap();
            assert_eq!(
                document_file_path_spelling(&uri, true).as_deref(),
                Ok(expected),
                "{raw}"
            );
        }
        use FileUriPathUnavailable as Unavailable;
        for (raw, expected) in [
            ("file:///tmp/a.tcl", Unavailable::UnsupportedNativeGeometry),
            (
                "file://///%3F/Volume%7B1234%7D/a.tcl",
                Unavailable::UnsupportedNativeGeometry,
            ),
            (
                "file://%3F/Volume%7B1234%7D/a.tcl",
                Unavailable::UnsupportedAuthority,
            ),
            ("file://./pipe/a", Unavailable::UnsupportedAuthority),
            (
                "file://srv%5Cother/share/a.tcl",
                Unavailable::UnsupportedAuthority,
            ),
            ("file:///C:/a%5Cb.tcl", Unavailable::EncodedSeparator),
        ] {
            let uri = Uri::parse(raw).unwrap();
            assert_eq!(
                document_file_path_spelling(&uri, true),
                Err(expected),
                "{raw}"
            );
        }
    }

    #[cfg(windows)]
    #[test]
    fn actual_windows_document_paths_have_native_drive_or_unc_roots() {
        for (raw, expected) in [
            ("file:///C%3A/Users/Me/a.tcl", r"C:\Users\Me\a.tcl"),
            ("file://Srv/share/a.tcl", r"\\Srv\share\a.tcl"),
        ] {
            let path = to_path(raw, FileUriPathPurpose::VsCodeDocument).unwrap();
            assert!(path.is_absolute());
            assert_eq!(path.to_str(), Some(expected));
        }
    }
}

#[cfg(all(test, any(unix, windows)))]
mod inverse_parser_tests {
    use super::*;

    #[test]
    fn malformed_original_uris_do_not_reach_native_path_construction() {
        for original in [
            "not a URI",
            "file:///tmp/raw space.tcl",
            "file:///tmp/%GG.tcl",
        ] {
            assert_eq!(
                to_path(original, FileUriPathPurpose::VsCodeDocument),
                Err(FileUriPathUnavailable::InvalidUri),
                "{original}"
            );
        }
    }
}

#[cfg(test)]
mod source_namespace_path_tests {
    use super::*;
    use std::path::PathBuf;
    #[test]
    fn declared_virtual_and_guest_geometry_is_separate_from_native_host_support() {
        for namespace in [
            SourceFileUriNamespace::Virtual,
            SourceFileUriNamespace::WasiGuest,
        ] {
            assert_eq!(
                to_source_namespace_path("file://Srv/share/%C3%A9%252F.tcl", namespace).unwrap(),
                PathBuf::from("//Srv/share/é%2F.tcl")
            );
            assert_eq!(
                to_source_namespace_path("file:///workspace/a.tcl", namespace).unwrap(),
                PathBuf::from("/workspace/a.tcl")
            );
            for raw in [
                "file:a.tcl",
                "file:///a%FF",
                "file:///a%00",
                "file:///a%2Fb",
                "file://host:42/a",
                "file:///a#fragment",
            ] {
                assert!(to_source_namespace_path(raw, namespace).is_err(), "{raw}");
            }
        }
    }
}

#[cfg(test)]
mod document_link_path_tests {
    use super::*;

    #[test]
    fn document_link_paths_keep_supplied_bases_and_existing_host_encoding() {
        let root = std::env::temp_dir().join("link-base");
        let home = std::env::temp_dir().join("link-home");
        for (literal, expected) in [
            ("relative é#?.tcl", root.join("relative é#?.tcl")),
            ("./link/../target.tcl", root.join("./link/../target.tcl")),
            ("~/lib/a.tcl", home.join("lib/a.tcl")),
            ("~//lib/a.tcl", home.join("lib/a.tcl")),
            ("~", home.clone()),
            ("~other/a.tcl", root.join("~other/a.tcl")),
        ] {
            assert_eq!(
                document_link_target(literal, Some(&root), Some(&home)),
                from_path(&expected, FileUriEscaping::PathCharacters),
                "{literal}"
            );
        }
        let absolute = root.join("absolute.tcl");
        assert_eq!(
            document_link_target(absolute.to_str().unwrap(), None, None),
            from_path(&absolute, FileUriEscaping::PathCharacters)
        );
    }

    #[test]
    fn document_link_paths_refuse_missing_or_unrooted_bases_without_cwd() {
        let root = std::env::temp_dir().join("link-base");
        for base in [None, Some(Path::new("")), Some(Path::new("relative"))] {
            assert!(document_link_target("file.tcl", base, None).is_none());
            assert!(document_link_target("~/file.tcl", Some(&root), base).is_none());
            assert!(document_link_target("~", Some(&root), base).is_none());
        }
        assert!(document_link_target("", Some(&root), Some(&root)).is_none());
        assert!(document_link_target("bad\0.tcl", Some(&root), None).is_none());
    }

    #[cfg(not(windows))]
    #[test]
    fn document_link_paths_preserve_posix_foreign_drive_and_double_root_data() {
        for literal in [r"C:\foreign\file.tcl", r"\server\share\file.tcl"] {
            assert_eq!(
                document_link_target(literal, Some(Path::new("/work")), None),
                from_path(
                    &Path::new("/work").join(literal),
                    FileUriEscaping::PathCharacters
                )
            );
        }
        assert_eq!(
            document_link_target("file.tcl", Some(Path::new("//server/share")), None),
            Some("file:////server/share/file.tcl".to_owned())
        );
        assert_eq!(
            document_link_target("./link/../target.tcl", Some(Path::new("/work")), None),
            Some("file:///work/./link/../target.tcl".to_owned())
        );
    }
}

#[cfg(test)]
mod vscode_forward_projection_tests {
    use super::*;

    #[test]
    fn product_alias_is_explicit_and_does_not_replace_lossless_path_spelling() {
        for (path, lossless, product) in [
            (
                "//srv/share/a.tcl",
                "file:////srv/share/a.tcl",
                "file://srv/share/a.tcl",
            ),
            (
                "///local/path",
                "file://///local/path",
                "file:///local/path",
            ),
            ("//server", "file:////server", "file://server/"),
            ("//", "file:////", "file:///"),
            (
                "//?/é%25.tcl",
                "file:////%3F/%C3%A9%2525.tcl",
                "file://%3F/%C3%A9%2525.tcl",
            ),
        ] {
            assert_eq!(
                from_rooted_spelling(path, false, FileUriEscaping::Unreserved).as_deref(),
                Some(lossless)
            );
            assert_eq!(
                vscode_document_spelling(path, false).as_deref(),
                Some(product)
            );
        }
        assert!(vscode_document_spelling("//srv/invalid\0.tcl", false).is_none());
        assert!(vscode_document_spelling("relative/file.tcl", false).is_none());
        assert!(vscode_document_spelling(r"\\?\Volume{test}\file.tcl", true).is_none());
        assert_eq!(
            vscode_document_spelling(r"C:\work\é.tcl", true),
            from_rooted_spelling(r"C:\work\é.tcl", true, FileUriEscaping::Unreserved)
        );
    }
}

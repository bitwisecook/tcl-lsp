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

//! `file normalize` on Unix as the C releases compute it: `.` and `..`
//! dropped by `TclFSNormalizeAbsolutePath`, then the longest existing
//! prefix of the result made canonical by `TclpObjNormalizePath`.
//!
//! One simplification: symbolic links are not followed. Where C asks
//! `realpath` for a prefix that exists, this reads the prefix with its
//! repeated separators collapsed and its `.` and `..` elements resolved
//! lexically (`//tmp` is `/tmp`), and C's `readlink` before a `..` always
//! answers that the name is not a link. `exists` is handed each prefix as C
//! hands it to `access` and `realpath`, separators doubled as they stand. A
//! relative prefix (8.4 and 8.5 translate `~/~a/b` to `~a/b`) is never
//! made canonical, where C would read it against the process's directory.

use super::{PathReading, PathType, ZIPFS_VOLUME, join, path_type, split};

/// `file normalize` of `translated`, the name as `file nativename` gives it:
/// the empty name stays empty, a relative name is joined to `cwd` (the
/// working directory, already normalised) as `Tcl_FSJoinToPath` joins it,
/// and the result goes through [`normalize_absolute`]. To 8.x a translated
/// name `~user` (8.4 and 8.5 give `~a` for `~/~a`) is absolute and is not
/// joined.
///
/// `exists` answers whether a name exists on the host; it stands in for
/// `access` and `realpath`, and a runtime without a filesystem answers
/// `false` for every name.
#[must_use]
pub(super) fn normalize_translated(
    translated: &[u8],
    cwd: &[u8],
    reading: PathReading,
    exists: &dyn Fn(&[u8]) -> bool,
) -> Vec<u8> {
    if translated.is_empty() {
        return Vec::new();
    }
    if path_type(translated, reading) == PathType::Relative {
        let absolute = join(&[cwd, translated], reading);
        return normalize_absolute(&absolute, reading, exists);
    }
    normalize_absolute(translated, reading, exists)
}

/// `file normalize` of an already-translated name on Unix, `path` being
/// absolute (`TclFSNormalizeAbsolutePath`, then `TclFSNormalizeToUniquePath`).
///
/// 8.4 splits the name, drops each `.` and each `..` with the element before
/// it (never the root), and joins what is left. From 8.5 the name is scanned
/// in place, which leaves a doubled separator where a `.` or `..` follows
/// the root (`/./a` is `//a`, `/x/.././a` is `//a`). From 9.0 a name under
/// the zipfs volume `//zipfs:/` never backs up past the volume and keeps its
/// trailing separator, and a name `//host:…` belongs to a virtual
/// filesystem, so the host is not asked about it.
///
/// Then, unless the name is virtual, the directory part before the last
/// separator is made canonical where it exists; where it does not, the
/// longest prefix ending before a separator that exists is
/// (`TclpObjNormalizePath`). So `/./tmp/x` is `/tmp/x`, as `//tmp` exists,
/// but `/./tmp` stays `//tmp`, its directory part being only `/`.
#[must_use]
pub(super) fn normalize_absolute(
    path: &[u8],
    reading: PathReading,
    exists: &dyn Fn(&[u8]) -> bool,
) -> Vec<u8> {
    let collapsed = if reading == PathReading::Tcl84 {
        drop_dots_by_elements(path)
    } else {
        drop_dots_in_place(path, reading)
    };
    if reading == PathReading::Tcl9 && is_virtual(&collapsed) {
        return collapsed;
    }
    canonical_prefix(collapsed, exists)
}

/// 8.4's `TclFSNormalizeAbsolutePath`: split, drop `.` and `..` (a `..`
/// with the element before it, unless that is the first), join.
fn drop_dots_by_elements(path: &[u8]) -> Vec<u8> {
    let mut kept = Vec::new();
    for element in split(path, PathReading::Tcl84) {
        match element.as_ref() {
            b"." => {}
            b".." => {
                if kept.len() > 1 {
                    kept.pop();
                }
            }
            _ => kept.push(element),
        }
    }
    join(&kept, PathReading::Tcl84)
}

/// The length of the zipfs volume that starts `path` (`TclIsZipfsPath`),
/// where a `\` may stand for each `/`.
fn zipfs_volume_len(path: &[u8]) -> Option<usize> {
    let matches = path.len() >= ZIPFS_VOLUME.len()
        && ZIPFS_VOLUME
            .iter()
            .zip(path)
            .all(|(&want, &got)| got == want || (want == b'/' && got == b'\\'));
    matches.then_some(ZIPFS_VOLUME.len())
}

/// `TclFSNormalizeAbsolutePath` from 8.5 on Unix, where no name is a link.
///
/// `sep` walks from one separator to the next; `out` is the result once it
/// differs from `path` (C's `retVal`, `None` while it is still `path`).
fn drop_dots_in_place(path: &[u8], reading: PathReading) -> Vec<u8> {
    let at = |index: usize| path.get(index).copied();
    let ends_element = |index: usize| matches!(at(index), None | Some(b'/'));
    let volume = if reading == PathReading::Tcl9 {
        zipfs_volume_len(path)
    } else {
        None
    };
    let floor = volume.map_or(0, |len| len - 1);
    let mut sep = floor;
    let mut first = true;
    let mut out: Option<Vec<u8>> = None;
    'scan: while sep < path.len() {
        let mut start = sep;
        if !first {
            sep += 1;
        }
        sep += path[sep..]
            .iter()
            .position(|&byte| byte == b'/')
            .unwrap_or(path.len() - sep);
        if sep + 1 >= path.len() {
            if let Some(out) = out.as_mut() {
                out.extend_from_slice(&path[start..sep]);
            }
            break;
        }
        if at(sep + 1) == Some(b'.') {
            if let Some(out) = out.as_mut() {
                out.extend_from_slice(&path[start..sep]);
                start = sep;
            }
            loop {
                if ends_element(sep + 2) {
                    let out = out.get_or_insert_with(|| path[..sep].to_vec());
                    if out.is_empty() {
                        out.push(path[sep]);
                    }
                    sep += 2;
                    start = sep;
                } else if at(sep + 2) == Some(b'.') && ends_element(sep + 3) {
                    let out = out.get_or_insert_with(|| path[..sep].to_vec());
                    if out.is_empty() {
                        out.push(path[sep]);
                    }
                    let cut = out[floor..]
                        .iter()
                        .rposition(|&byte| byte == b'/')
                        .map(|cut| cut + floor);
                    if let Some(cut) = cut {
                        out.truncate(cut.max(1));
                    }
                    sep += 3;
                    start = sep;
                    if cut == Some(0) && sep < path.len() {
                        out.clear();
                    }
                } else {
                    break;
                }
                if !(sep < path.len() && at(sep + 1) == Some(b'.')) {
                    continue 'scan;
                }
            }
        }
        first = false;
        if let Some(out) = out.as_mut() {
            out.extend_from_slice(&path[start..sep]);
        }
    }
    let mut out = out.unwrap_or_else(|| path.to_vec());
    if volume.is_some_and(|len| out.len() == len - 1) {
        out.push(b'/');
    }
    out
}

/// Whether 9.x leaves `path` to a virtual filesystem
/// (`TclFSNormalizeToUniquePath`): it starts with two separators and the
/// name up to the next ends in `:`.
fn is_virtual(path: &[u8]) -> bool {
    let [lead @ (b'/' | b'\\'), second, rest @ ..] = path else {
        return false;
    };
    if second != lead || rest.is_empty() {
        return false;
    }
    let end = rest
        .iter()
        .position(|byte| byte == lead)
        .unwrap_or(rest.len());
    end > 0 && rest[end - 1] == b':'
}

/// `realpath` of `prefix` without links: where it is absolute and exists,
/// its elements with repeated separators, `.` and `..` resolved.
fn real_path(prefix: &[u8], exists: &dyn Fn(&[u8]) -> bool) -> Option<Vec<u8>> {
    if prefix.first() != Some(&b'/') || !exists(prefix) {
        return None;
    }
    let mut elements: Vec<&[u8]> = Vec::new();
    for element in prefix.split(|&byte| byte == b'/') {
        match element {
            b"" | b"." => {}
            b".." => {
                elements.pop();
            }
            element => elements.push(element),
        }
    }
    let mut out = Vec::with_capacity(prefix.len());
    for element in elements {
        out.push(b'/');
        out.extend_from_slice(element);
    }
    if out.is_empty() {
        out.push(b'/');
    }
    Some(out)
}

/// `TclpObjNormalizePath` from offset 0: the part before the last separator
/// made canonical where it exists, else the longest prefix ending before a
/// separator that does.
fn canonical_prefix(mut path: Vec<u8>, exists: &dyn Fn(&[u8]) -> bool) -> Vec<u8> {
    let from = usize::from(path.first() == Some(&b'/'));
    let last_dir = path[from..]
        .iter()
        .rposition(|&byte| byte == b'/')
        .map(|index| index + from);
    let mut resolved = last_dir.and_then(|end| Some((end, real_path(&path[..end], exists)?)));
    if resolved.is_none() {
        let mut checkpoint = 0;
        for index in from..path.len() {
            if path[index] == b'/' {
                if !exists(&path[..index]) {
                    break;
                }
                checkpoint = index;
            }
        }
        if checkpoint == 0 {
            return path;
        }
        resolved = real_path(&path[..checkpoint], exists).map(|real| (checkpoint, real));
    }
    if let Some((end, real)) = resolved
        && real != path[..end]
    {
        path.splice(..end, real);
    }
    path
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The names the host of the tables below has, none of them a link.
    const HOST: &[&str] = &[
        "/",
        "/tmp",
        "/usr",
        "/usr/bin",
        "/usr/bin/env",
        "/usr/lib",
        "/usr/lib/os-release",
        "/usr/share",
        "/etc",
        "/etc/passwd",
        "/root",
    ];

    /// Whether `name`, read without links, is one of [`HOST`]'s names.
    fn exists(name: &[u8]) -> bool {
        real_path(name, &|_| true)
            .is_some_and(|real| HOST.iter().any(|known| known.as_bytes() == real))
    }

    const READINGS: [PathReading; 4] = [
        PathReading::Tcl84,
        PathReading::Tcl85,
        PathReading::Tcl86,
        PathReading::Tcl9,
    ];

    /// `file normalize` as tclsh 8.4.20, 8.5.19, 8.6.18 and 9.0.4 give it
    /// (9.1.0 as 9.0.4), with the working directory `/tmp`, every release
    /// alike.
    const NORMALISED: &[(&str, &str)] = &[
        (".", "/tmp"),
        ("..", "/"),
        ("./", "/tmp"),
        ("a", "/tmp/a"),
        ("a/", "/tmp/a"),
        ("a/../b", "/tmp/b"),
        ("./~a", "/tmp/~a"),
        ("a/./b", "/tmp/a/b"),
        ("../..", "/"),
        ("../../../..", "/"),
        ("./a/../../b", "/b"),
        ("tmp/x", "/tmp/tmp/x"),
        ("../usr/bin/x", "/usr/bin/x"),
        ("./../etc", "/etc"),
        (".//a", "/tmp/a"),
        ("a//b/", "/tmp/a/b"),
        ("a/~b", "/tmp/a/~b"),
        ("/", "/"),
        ("//", "/"),
        ("///", "/"),
        ("/.", "/"),
        ("/..", "/"),
        ("/./", "/"),
        ("/../..", "/"),
        ("/a", "/a"),
        ("/a/", "/a"),
        ("/a/.", "/a"),
        ("/a/..", "/"),
        ("/a/../..", "/"),
        ("/a/./", "/a"),
        ("/a/../", "/"),
        ("/./tmp/x", "/tmp/x"),
        ("/tmp", "/tmp"),
        ("/tmp/", "/tmp"),
        ("/tmp/.", "/tmp"),
        ("/tmp/..", "/"),
        ("/tmp/./x", "/tmp/x"),
        ("/tmp/../tmp/x", "/tmp/x"),
        ("/tmp//x", "/tmp/x"),
        ("/tmp///x//", "/tmp/x"),
        ("//tmp/x", "/tmp/x"),
        ("//tmp/x/y", "/tmp/x/y"),
        ("///tmp/x", "/tmp/x"),
        ("/usr/bin", "/usr/bin"),
        ("/usr//bin/", "/usr/bin"),
        ("/usr/./bin/../lib", "/usr/lib"),
        ("/usr/bin/../../etc", "/etc"),
        ("/usr/bin/env", "/usr/bin/env"),
        ("/usr/bin/env/x", "/usr/bin/env/x"),
        ("/etc/./passwd", "/etc/passwd"),
        ("/etc/passwd/", "/etc/passwd"),
        ("/etc/passwd/x", "/etc/passwd/x"),
        ("/etc/passwd/x/y", "/etc/passwd/x/y"),
        ("/./etc/passwd", "/etc/passwd"),
        ("/./usr/bin/x", "/usr/bin/x"),
        ("/./usr/bin/env", "/usr/bin/env"),
        ("/nonex", "/nonex"),
        ("/nonex/", "/nonex"),
        ("/nonex/.", "/nonex"),
        ("/nonex/..", "/"),
        ("/nonex/../tmp", "/tmp"),
        ("/nonex/a/../b", "/nonex/b"),
        ("/nonex/./a", "/nonex/a"),
        ("/nonex/x/y/z", "/nonex/x/y/z"),
        ("//a/..", "/"),
        ("//a/../..", "/"),
        ("//a/../tmp/x", "/tmp/x"),
        ("//./tmp/x", "/tmp/x"),
        ("/..a", "/..a"),
        ("/.a/b", "/.a/b"),
        ("/a./.", "/a."),
        ("/.../x", "/.../x"),
        ("/a/.../..", "/a"),
        ("/./.", "/"),
        ("/./..", "/"),
        ("/../tmp", "/tmp"),
        ("/../tmp/x", "/tmp/x"),
        ("/tmp/../..", "/"),
        ("/tmp/./../usr/bin/x", "/usr/bin/x"),
        ("/tmp/x/./", "/tmp/x"),
        ("/tmp/x/../", "/tmp"),
        ("/tmp/./.", "/tmp"),
        ("/usr/./lib/./os-release", "/usr/lib/os-release"),
        ("/usr/share/../lib/./x", "/usr/lib/x"),
        ("/root/.", "/root"),
        ("/root/./x", "/root/x"),
        ("/a/~b", "/a/~b"),
        ("/a/./~b", "/a/~b"),
        ("/a/.//./b", "/a/b"),
        ("/a/./../b", "/b"),
        ("/./../a", "/a"),
        ("//.", "/"),
        ("/./x/..", "/"),
        ("/./.././tmp/x", "/tmp/x"),
        ("/a/b/c/../../..", "/"),
        ("/tmp/.../..", "/tmp"),
        ("/a/.../b/./..", "/a/..."),
        ("/..//..", "/"),
        ("/./tmp/./x/../y", "/tmp/y"),
        ("./.", "/tmp"),
        ("a/./../..", "/"),
        (".../x", "/tmp/.../x"),
        ("././~a/../b", "/tmp/b"),
    ];

    /// The same, where the releases differ: 8.4.20, 8.5.19, 8.6.18 and
    /// 9.0.4 (9.1.0 as 9.0.4).
    const NORMALISED_BY_READING: &[(&str, [&str; 4])] = &[
        ("/./a", ["/a", "//a", "//a", "//a"]),
        ("/././a", ["/a", "//a", "//a", "//a"]),
        ("/./a/./b", ["/a/b", "//a/b", "//a/b", "//a/b"]),
        ("/x/.././a", ["/a", "//a", "//a", "//a"]),
        ("/.././a", ["/a", "//a", "//a", "//a"]),
        ("//./a", ["/a", "//a", "//a", "//a"]),
        ("/./tmp", ["/tmp", "//tmp", "//tmp", "//tmp"]),
        ("/./tmp/", ["/tmp", "//tmp", "//tmp", "//tmp"]),
        ("//./tmp", ["/tmp", "//tmp", "//tmp", "//tmp"]),
        (
            "//./nonex/x",
            ["/nonex/x", "//nonex/x", "//nonex/x", "//nonex/x"],
        ),
        ("//tmp", ["/tmp", "//tmp", "/tmp", "//tmp"]),
        ("//nonex", ["/nonex", "//nonex", "/nonex", "//nonex"]),
        (
            "//nonex/a",
            ["/nonex/a", "//nonex/a", "/nonex/a", "//nonex/a"],
        ),
        ("//a", ["/a", "//a", "/a", "//a"]),
        ("//a/", ["/a", "/a", "/a", "//a/"]),
        ("//a//", ["/a", "/a", "/a", "//a/"]),
        ("//a/.", ["/a", "//a", "/a", "//a"]),
        ("//a/./", ["/a", "/a", "/a", "//a"]),
        ("//a/b/..", ["/a", "//a", "/a", "//a"]),
        ("//a/./b/", ["/a/b", "/a/b", "/a/b", "//a/b"]),
        ("//zipfs:/", ["/zipfs:", "/zipfs:", "/zipfs:", "//zipfs:/"]),
        (
            "//zipfs:/a",
            ["/zipfs:/a", "//zipfs:/a", "/zipfs:/a", "//zipfs:/a"],
        ),
        (
            "//zipfs:/a/..",
            ["/zipfs:", "//zipfs:", "/zipfs:", "//zipfs:/"],
        ),
        ("//zipfs:/..", ["/", "/", "/", "//zipfs:/"]),
        (
            "//zipfs:/a/../b",
            ["/zipfs:/b", "//zipfs:/b", "/zipfs:/b", "//zipfs:/b"],
        ),
        (
            "//zipfs:/./a",
            ["/zipfs:/a", "//zipfs:/a", "/zipfs:/a", "//zipfs:/a"],
        ),
        (
            "//zipfs:/a/./b/",
            ["/zipfs:/a/b", "/zipfs:/a/b", "/zipfs:/a/b", "//zipfs:/a/b"],
        ),
        ("//zipfs:/a/../..", ["/", "/", "/", "//zipfs:/"]),
        (
            "//zipfs:/a/b/../../c",
            ["/zipfs:/c", "//zipfs:/c", "/zipfs:/c", "//zipfs:/c"],
        ),
        ("//zipfs:/../tmp", ["/tmp", "//tmp", "/tmp", "//zipfs:/tmp"]),
        ("//zipfs:", ["/zipfs:", "//zipfs:", "/zipfs:", "//zipfs:"]),
        ("//zipfs:a/../b", ["/b", "//b", "/b", "//b"]),
        (
            "//zipfsab",
            ["/zipfsab", "//zipfsab", "/zipfsab", "//zipfs:b"],
        ),
        (
            "//zipfs:/tmp/x",
            [
                "/zipfs:/tmp/x",
                "//zipfs:/tmp/x",
                "/zipfs:/tmp/x",
                "//zipfs:/tmp/x",
            ],
        ),
        (
            "//foo:/a/./b",
            ["/foo:/a/b", "//foo:/a/b", "/foo:/a/b", "//foo:/a/b"],
        ),
        (
            "//foo:/tmp/../x",
            ["/foo:/x", "//foo:/x", "/foo:/x", "//foo:/x"],
        ),
        ("//foo:", ["/foo:", "//foo:", "/foo:", "//foo:"]),
        ("//tmp:/x", ["/tmp:/x", "//tmp:/x", "/tmp:/x", "//tmp:/x"]),
        ("/.//a", ["/a", "//a", "//a", "//a"]),
        ("///./a", ["/a", "//a", "//a", "//a"]),
        ("//a/./../b", ["/b", "//b", "/b", "//b"]),
        ("//a/b/../c/./d/", ["/a/c/d", "/a/c/d", "/a/c/d", "//a/c/d"]),
        ("//zipfs:/./../a", ["/a", "//a", "/a", "//zipfs:/a"]),
        (
            "//zipfs:/a//..",
            ["/zipfs:", "/zipfs:", "/zipfs:", "//zipfs:/"],
        ),
        ("//zipfs:\\a/../b", ["/b", "//b", "/b", "//zipfs:\\a/b"]),
    ];

    /// Names the releases translate apart (`file nativename`), with
    /// `HOME=/root`: the translated name, then `file normalize`.
    const NORMALISED_TRANSLATED: &[(PathReading, &str, &str)] = &[
        (PathReading::Tcl84, "/root", "/root"),
        (PathReading::Tcl86, "/root/x", "/root/x"),
        (PathReading::Tcl84, "~a", "~a"),
        (PathReading::Tcl85, "~a", "~a"),
        (PathReading::Tcl86, "/root/~a", "/root/~a"),
        (PathReading::Tcl9, "~", "/tmp/~"),
        (PathReading::Tcl9, "~/x", "/tmp/~/x"),
        (PathReading::Tcl9, "~/~a", "/tmp/~/~a"),
        (PathReading::Tcl84, "", ""),
        (PathReading::Tcl9, "", ""),
    ];

    fn normalised(name: &str, reading: PathReading) -> String {
        let translated = join(&[name], reading);
        let got = normalize_translated(&translated, b"/tmp", reading, &exists);
        String::from_utf8(got).expect("ASCII")
    }

    #[test]
    fn normalize_reads_dots_and_existing_prefixes_as_each_release_does() {
        for (column, reading) in READINGS.into_iter().enumerate() {
            for &(name, expected) in NORMALISED {
                assert_eq!(
                    normalised(name, reading),
                    expected,
                    "{reading:?} file normalize {{{name}}}"
                );
            }
            for &(name, expected) in NORMALISED_BY_READING {
                assert_eq!(
                    normalised(name, reading),
                    expected[column],
                    "{reading:?} file normalize {{{name}}}"
                );
            }
        }
        for &(reading, translated, expected) in NORMALISED_TRANSLATED {
            let got = normalize_translated(translated.as_bytes(), b"/tmp", reading, &exists);
            assert_eq!(
                got,
                expected.as_bytes(),
                "{reading:?} file normalize of {{{translated}}}"
            );
        }
    }

    #[test]
    fn a_host_without_a_filesystem_leaves_every_prefix_as_it_is() {
        let none = |_: &[u8]| false;
        for (name, expected) in [
            ("/./tmp/x", "//tmp/x"),
            ("//tmp", "//tmp"),
            ("/a/../b", "/b"),
        ] {
            let got = normalize_absolute(name.as_bytes(), PathReading::Tcl86, &none);
            assert_eq!(got, expected.as_bytes(), "{name}");
        }
    }
}

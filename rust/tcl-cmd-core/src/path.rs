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

//! The path-name cores of `file`: `split`, `join`, `dirname`, `tail`,
//! `extension` and `rootname`, as `tclFileName.c` reads a name on Unix.
//!
//! Byte→byte operations on `/`-separated names: no filesystem and no host, so
//! each runtime hands in the name's bytes and builds its own result. A name
//! reads as its elements: the root `/` where it starts with a separator, then
//! every non-empty run between separators, so repeated and trailing
//! separators are not elements (`a//b/` is `a b`). `dirname` and `tail` are
//! the elements but the last and the last, as `TclPathPart` takes them;
//! `extension` is `TclGetExtension`'s: from the name's last dot to its end,
//! where no separator follows the dot (`.bashrc` is all extension, `a.b/c`
//! has none). The releases agree on every name these read (tclsh 8.4.20 to
//! 9.1.0) except two this reads one way: a leading `//`, which 9.0 keeps as a
//! root of its own where 8.x reads the root `/`, read here as 8.x reads it;
//! and an element starting with `~`, which 8.x splits and joins as a home
//! directory and 9.0 as a plain name, read here as 9.0 reads it.

use std::borrow::Cow;

/// `file split name`: the root `/` where `name` starts with a separator, then
/// each non-empty element between separators.
#[must_use]
pub fn split(name: &[u8]) -> Vec<&[u8]> {
    let mut elements = Vec::new();
    let mut rest = name;
    if let Some(after) = name.strip_prefix(b"/") {
        elements.push(&name[..1]);
        rest = after;
    }
    elements.extend(
        rest.split(|&byte| byte == b'/')
            .filter(|part| !part.is_empty()),
    );
    elements
}

/// Whether `name` is absolute in the Unix reading: it starts with `/`.
fn is_absolute(name: &[u8]) -> bool {
    name.first() == Some(&b'/')
}

/// The name `elements` spell: the root, then the rest joined by `/`.
fn render(elements: &[&[u8]]) -> Vec<u8> {
    let mut out = Vec::new();
    for (index, element) in elements.iter().enumerate() {
        if index > 0 && out.last() != Some(&b'/') {
            out.push(b'/');
        }
        out.extend_from_slice(element);
    }
    out
}

/// `file join name ?name …?`: each name's elements after the last absolute
/// name's, so an absolute name drops what came before it and an empty one
/// adds nothing (`file join a /b c` is `/b/c`).
#[must_use]
pub fn join<N: AsRef<[u8]>>(names: &[N]) -> Vec<u8> {
    let mut elements: Vec<&[u8]> = Vec::new();
    for name in names {
        let name = name.as_ref();
        if is_absolute(name) {
            elements.clear();
        }
        elements.extend(split(name));
    }
    render(&elements)
}

/// `file tail name`: the last element, or the empty string where the only
/// element is the root (`/` and the empty name have no tail).
#[must_use]
pub fn tail(name: &[u8]) -> &[u8] {
    match split(name).as_slice() {
        [_] if is_absolute(name) => b"",
        [.., last] => last,
        [] => b"",
    }
}

/// `file dirname name`: the elements but the last, `.` where a relative name
/// has one element or none, and the root where it is the only element.
#[must_use]
pub fn dirname(name: &[u8]) -> Cow<'_, [u8]> {
    let elements = split(name);
    match elements.as_slice() {
        [] | [_] if !is_absolute(name) => Cow::Borrowed(b"."),
        [] | [_] => Cow::Borrowed(b"/"),
        [head @ .., _] => Cow::Owned(render(head)),
    }
}

/// `file extension name`: from the last dot to the end of `name`, or the
/// empty string where `name` has no dot or a separator follows its last.
#[must_use]
pub fn extension(name: &[u8]) -> &[u8] {
    let Some(dot) = name.iter().rposition(|&byte| byte == b'.') else {
        return b"";
    };
    if name[dot..].contains(&b'/') {
        return b"";
    }
    &name[dot..]
}

/// `file rootname name`: `name` without its [`extension`].
#[must_use]
pub fn rootname(name: &[u8]) -> &[u8] {
    &name[..name.len() - extension(name).len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Each answer as tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0 give it,
    /// every release alike.
    const PARTS: &[(&str, &str, &str)] = &[
        ("dirname", "a", "."),
        ("tail", "a", "a"),
        ("extension", "a", ""),
        ("rootname", "a", "a"),
        ("dirname", "a/b", "a"),
        ("tail", "a/b", "b"),
        ("extension", "a/b", ""),
        ("rootname", "a/b", "a/b"),
        ("dirname", "a/b/c.txt", "a/b"),
        ("tail", "a/b/c.txt", "c.txt"),
        ("extension", "a/b/c.txt", ".txt"),
        ("rootname", "a/b/c.txt", "a/b/c"),
        ("dirname", "a.b/c", "a.b"),
        ("tail", "a.b/c", "c"),
        ("extension", "a.b/c", ""),
        ("rootname", "a.b/c", "a.b/c"),
        ("dirname", ".bashrc", "."),
        ("tail", ".bashrc", ".bashrc"),
        ("extension", ".bashrc", ".bashrc"),
        ("rootname", ".bashrc", ""),
        ("dirname", "a/.bashrc", "a"),
        ("tail", "a/.bashrc", ".bashrc"),
        ("extension", "a/.bashrc", ".bashrc"),
        ("rootname", "a/.bashrc", "a/"),
        ("dirname", "a/b/", "a"),
        ("tail", "a/b/", "b"),
        ("extension", "a/b/", ""),
        ("rootname", "a/b/", "a/b/"),
        ("dirname", "a//b", "a"),
        ("tail", "a//b", "b"),
        ("extension", "a//b", ""),
        ("rootname", "a//b", "a//b"),
        ("dirname", "./a", "."),
        ("tail", "./a", "a"),
        ("extension", "./a", ""),
        ("rootname", "./a", "./a"),
        ("dirname", "../a", ".."),
        ("tail", "../a", "a"),
        ("extension", "../a", ""),
        ("rootname", "../a", "../a"),
        ("dirname", "a/./b", "a/."),
        ("tail", "a/./b", "b"),
        ("extension", "a/./b", ""),
        ("rootname", "a/./b", "a/./b"),
        ("dirname", "a/../b", "a/.."),
        ("tail", "a/../b", "b"),
        ("extension", "a/../b", ""),
        ("rootname", "a/../b", "a/../b"),
        ("dirname", "", "."),
        ("tail", "", ""),
        ("extension", "", ""),
        ("rootname", "", ""),
        ("dirname", ".", "."),
        ("tail", ".", "."),
        ("extension", ".", "."),
        ("rootname", ".", ""),
        ("dirname", "..", "."),
        ("tail", "..", ".."),
        ("extension", "..", "."),
        ("rootname", "..", "."),
        ("dirname", "a.", "."),
        ("tail", "a.", "a."),
        ("extension", "a.", "."),
        ("rootname", "a.", "a"),
        ("dirname", "a.b.c", "."),
        ("tail", "a.b.c", "a.b.c"),
        ("extension", "a.b.c", ".c"),
        ("rootname", "a.b.c", "a.b"),
        ("dirname", "a/b.", "a"),
        ("tail", "a/b.", "b."),
        ("extension", "a/b.", "."),
        ("rootname", "a/b.", "a/b"),
        ("dirname", "x.tar.gz", "."),
        ("tail", "x.tar.gz", "x.tar.gz"),
        ("extension", "x.tar.gz", ".gz"),
        ("rootname", "x.tar.gz", "x.tar"),
        ("dirname", "dir.d/file", "dir.d"),
        ("tail", "dir.d/file", "file"),
        ("extension", "dir.d/file", ""),
        ("rootname", "dir.d/file", "dir.d/file"),
        ("dirname", "a b/c d.e", "a b"),
        ("tail", "a b/c d.e", "c d.e"),
        ("extension", "a b/c d.e", ".e"),
        ("rootname", "a b/c d.e", "a b/c d"),
        ("dirname", "...", "."),
        ("tail", "...", "..."),
        ("extension", "...", "."),
        ("rootname", "...", ".."),
        ("dirname", "a/...", "a"),
        ("tail", "a/...", "..."),
        ("extension", "a/...", "."),
        ("rootname", "a/...", "a/.."),
        ("dirname", ".a.b", "."),
        ("tail", ".a.b", ".a.b"),
        ("extension", ".a.b", ".b"),
        ("rootname", ".a.b", ".a"),
        ("dirname", "a/.a.b", "a"),
        ("tail", "a/.a.b", ".a.b"),
        ("extension", "a/.a.b", ".b"),
        ("rootname", "a/.a.b", "a/.a"),
        ("dirname", "a..b", "."),
        ("tail", "a..b", "a..b"),
        ("extension", "a..b", ".b"),
        ("rootname", "a..b", "a."),
        ("dirname", "/", "/"),
        ("tail", "/", ""),
        ("extension", "/", ""),
        ("rootname", "/", "/"),
        ("dirname", "/a", "/"),
        ("tail", "/a", "a"),
        ("extension", "/a", ""),
        ("rootname", "/a", "/a"),
        ("dirname", "/a/b", "/a"),
        ("tail", "/a/b", "b"),
        ("extension", "/a/b", ""),
        ("rootname", "/a/b", "/a/b"),
        ("dirname", "/a/b/", "/a"),
        ("tail", "/a/b/", "b"),
        ("extension", "/a/b/", ""),
        ("rootname", "/a/b/", "/a/b/"),
        ("dirname", "/a.b/c.d", "/a.b"),
        ("tail", "/a.b/c.d", "c.d"),
        ("extension", "/a.b/c.d", ".d"),
        ("rootname", "/a.b/c.d", "/a.b/c"),
        ("dirname", "/./a", "/."),
        ("tail", "/./a", "a"),
        ("extension", "/./a", ""),
        ("rootname", "/./a", "/./a"),
        ("dirname", "/..", "/"),
        ("tail", "/..", ".."),
        ("extension", "/..", "."),
        ("rootname", "/..", "/."),
        ("dirname", "/a/..", "/a"),
        ("tail", "/a/..", ".."),
        ("extension", "/a/..", "."),
        ("rootname", "/a/..", "/a/."),
        ("dirname", "//", "/"),
        ("tail", "//", ""),
        ("extension", "//", ""),
        ("rootname", "//", "//"),
        ("dirname", "/a//", "/"),
        ("tail", "/a//", "a"),
        ("extension", "/a//", ""),
        ("rootname", "/a//", "/a//"),
        ("dirname", "a/b//", "a"),
        ("tail", "a/b//", "b"),
        ("extension", "a/b//", ""),
        ("rootname", "a/b//", "a/b//"),
        ("dirname", "./", "."),
        ("tail", "./", "."),
        ("extension", "./", ""),
        ("rootname", "./", "./"),
        ("dirname", "../", "."),
        ("tail", "../", ".."),
        ("extension", "../", ""),
        ("rootname", "../", "../"),
        ("dirname", "/.", "/"),
        ("tail", "/.", "."),
        ("extension", "/.", "."),
        ("rootname", "/.", "/"),
        ("dirname", "a/.", "a"),
        ("tail", "a/.", "."),
        ("extension", "a/.", "."),
        ("rootname", "a/.", "a/"),
        ("dirname", "a b", "."),
        ("tail", "a b", "a b"),
        ("extension", "a b", ""),
        ("rootname", "a b", "a b"),
        ("dirname", " ", "."),
        ("tail", " ", " "),
        ("extension", " ", ""),
        ("rootname", " ", " "),
        ("dirname", "a/ b", "a"),
        ("tail", "a/ b", " b"),
        ("extension", "a/ b", ""),
        ("rootname", "a/ b", "a/ b"),
        ("dirname", "a/b.c/", "a"),
        ("tail", "a/b.c/", "b.c"),
        ("extension", "a/b.c/", ""),
        ("rootname", "a/b.c/", "a/b.c/"),
        ("dirname", "/.x", "/"),
        ("tail", "/.x", ".x"),
        ("extension", "/.x", ".x"),
        ("rootname", "/.x", "/"),
        ("dirname", "/x.", "/"),
        ("tail", "/x.", "x."),
        ("extension", "/x.", "."),
        ("rootname", "/x.", "/x"),
        ("dirname", "./.", "."),
        ("tail", "./.", "."),
        ("extension", "./.", "."),
        ("rootname", "./.", "./"),
        ("dirname", "a/b/.c", "a/b"),
        ("tail", "a/b/.c", ".c"),
        ("extension", "a/b/.c", ".c"),
        ("rootname", "a/b/.c", "a/b/"),
        ("dirname", "a/b//c", "a/b"),
        ("tail", "a/b//c", "c"),
        ("dirname", "a///b", "a"),
        ("tail", "a///b", "b"),
        ("dirname", "/a//b", "/a"),
        ("tail", "/a//b", "b"),
        ("dirname", "a//b/c/", "a/b"),
        ("tail", "a//b/c/", "c"),
        ("dirname", "a/b//c/d", "a/b/c"),
        ("tail", "a/b//c/d", "d"),
    ];
    const SPLITS: &[(&str, &[&str])] = &[
        ("a", &["a"]),
        ("a/b", &["a", "b"]),
        ("a/b/c.txt", &["a", "b", "c.txt"]),
        ("a.b/c", &["a.b", "c"]),
        (".bashrc", &[".bashrc"]),
        ("a/.bashrc", &["a", ".bashrc"]),
        ("a/b/", &["a", "b"]),
        ("a//b", &["a", "b"]),
        ("./a", &[".", "a"]),
        ("../a", &["..", "a"]),
        ("a/./b", &["a", ".", "b"]),
        ("a/../b", &["a", "..", "b"]),
        ("", &[]),
        (".", &["."]),
        ("..", &[".."]),
        ("a.", &["a."]),
        ("a.b.c", &["a.b.c"]),
        ("a/b.", &["a", "b."]),
        ("x.tar.gz", &["x.tar.gz"]),
        ("dir.d/file", &["dir.d", "file"]),
        ("a b/c d.e", &["a b", "c d.e"]),
        ("...", &["..."]),
        ("a/...", &["a", "..."]),
        (".a.b", &[".a.b"]),
        ("a/.a.b", &["a", ".a.b"]),
        ("a..b", &["a..b"]),
        ("/", &["/"]),
        ("/a", &["/", "a"]),
        ("/a/b", &["/", "a", "b"]),
        ("/a/b/", &["/", "a", "b"]),
        ("/a.b/c.d", &["/", "a.b", "c.d"]),
        ("/./a", &["/", ".", "a"]),
        ("/..", &["/", ".."]),
        ("/a/..", &["/", "a", ".."]),
        ("//", &["/"]),
        ("/a//", &["/", "a"]),
        ("a/b//", &["a", "b"]),
        ("./", &["."]),
        ("../", &[".."]),
        ("/.", &["/", "."]),
        ("a/.", &["a", "."]),
        ("a b", &["a b"]),
        (" ", &[" "]),
        ("a/ b", &["a", " b"]),
        ("a/b.c/", &["a", "b.c"]),
        ("/.x", &["/", ".x"]),
        ("/x.", &["/", "x."]),
        ("./.", &[".", "."]),
        ("a/b/.c", &["a", "b", ".c"]),
        ("a/b//c", &["a", "b", "c"]),
        ("a///b", &["a", "b"]),
        ("/a//b", &["/", "a", "b"]),
        ("a//b/c/", &["a", "b", "c"]),
        ("a/b//c/d", &["a", "b", "c", "d"]),
    ];
    const JOINS: &[(&[&str], &str)] = &[
        (&["a", "b"], "a/b"),
        (&["a", "/b"], "/b"),
        (&["/a", "b"], "/a/b"),
        (&["a", "b/"], "a/b"),
        (&["a/", "b"], "a/b"),
        (&["a", "", "b"], "a/b"),
        (&["", "a"], "a"),
        (&["a", "."], "a/."),
        (&["a", ".."], "a/.."),
        (&["/", "a"], "/a"),
        (&["a", "b", "c"], "a/b/c"),
        (&["a", "./b"], "a/./b"),
        (&["a/b", "/c/d"], "/c/d"),
        (&["/a", "/b"], "/b"),
        (&["a", "b/", "c"], "a/b/c"),
        (&["a"], "a"),
        (&[""], ""),
        (&["/"], "/"),
        (&["a/./b", "c"], "a/./b/c"),
        (&["a", "../b"], "a/../b"),
        (&["a", "b//"], "a/b"),
        (&["a//", "b"], "a/b"),
        (&["/", "/"], "/"),
        (&["", ""], ""),
        (&["", "/"], "/"),
        (&["a/", "/"], "/"),
        (&[".", "a"], "./a"),
        (&["./", "a"], "./a"),
        (&["a", ".b"], "a/.b"),
        (&["a", "b.c"], "a/b.c"),
        (&["/a/", "b/"], "/a/b"),
        (&["a//b", "c"], "a/b/c"),
        (&[" ", "a"], " /a"),
        (&["a", " "], "a/ "),
        (&[".", "."], "./."),
        (&["..", ".."], "../.."),
        (&["/", ".."], "/.."),
        (&["a/b/", "./c"], "a/b/./c"),
        (&["a/"], "a"),
        (&["a//b"], "a/b"),
        (&["/a//b/"], "/a/b"),
        (&["a", "/"], "/"),
        (&["/", "a/"], "/a"),
        (&["a/b/c/"], "a/b/c"),
        (&["a", "", ""], "a"),
        (&["", "", "a"], "a"),
        (&["/a/b/", "c//d/"], "/a/b/c/d"),
        (&["a", "b/../c"], "a/b/../c"),
        (&["x/", "y/", "z/"], "x/y/z"),
        (&["/x", "y", "/z", "w"], "/z/w"),
        (&["a///", "b"], "a/b"),
        (&["a", "///b"], "/b"),
    ];

    fn text(bytes: &[u8]) -> &str {
        std::str::from_utf8(bytes).expect("ASCII")
    }

    #[test]
    fn each_part_of_a_name_reads_as_tclsh_reads_it() {
        for &(operation, name, expected) in PARTS {
            let name = name.as_bytes();
            let got = match operation {
                "dirname" => text(&dirname(name)).to_owned(),
                "tail" => text(tail(name)).to_owned(),
                "extension" => text(extension(name)).to_owned(),
                "rootname" => text(rootname(name)).to_owned(),
                other => unreachable!("no {other} row"),
            };
            assert_eq!(got, expected, "file {operation} {{{}}}", text(name));
        }
        for &(name, expected) in SPLITS {
            let got: Vec<&str> = split(name.as_bytes()).into_iter().map(text).collect();
            assert_eq!(got, expected, "file split {{{name}}}");
        }
        for &(names, expected) in JOINS {
            assert_eq!(text(&join(names)), expected, "file join {names:?}");
        }
    }
}

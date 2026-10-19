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
//! `pathtype`, `extension` and `rootname`, as `tclFileName.c` and
//! `tclPathObj.c` read a name on Unix.
//!
//! Byte→byte operations on `/`-separated names: no filesystem, so each
//! runtime hands in the name's bytes and builds its own result. A name reads
//! as its elements: its root where it has one, then every non-empty run
//! between separators, so repeated and trailing separators are not elements
//! (`a//b/` is `a b`). `dirname` and `tail` are the elements but the last and
//! the last, as `TclPathPart` takes them; `extension` is `TclGetExtension`'s:
//! from the name's last dot to its end, where no separator follows the dot
//! (`.bashrc` is all extension, `a.b/c` has none).
//!
//! The releases read two things apart, and a [`PathReading`] says which way:
//! a leading `//` followed by a name, which 9.0 keeps as a root of its own
//! (`//host`) where 8.x reads the root `/`; and a `~`, which 8.x reads as a
//! home directory where 9.0 reads a plain character. To 8.x a name starting
//! with `~` is absolute, its first element `~user` is its root in `join`, an
//! element starting with `~` anywhere else is protected as `./~x` by `split`,
//! and `join` drops that `./` again; `dirname` and `tail` of a name that is
//! only `~user` read the home directory itself ([`home_user`]).

use std::borrow::Cow;

use tcl_dialect::TclVersion;
use tcl_platform::{Env, Filesystem};

use crate::error::CmdError;

mod canonical;

/// How a release reads a leading `//` and a `~`.
///
/// From 8.5, `join` hands back a last name that is absolute (or the only
/// name) unchanged where it is already in its final form, so the `./` it
/// would drop before a `~` stays (`file join /./~a` is `/./~a`, where 8.4
/// gives `/~a`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathReading {
    /// Tcl 8.4: `~user` is a home directory and a leading `//` is the root
    /// `/`.
    Tcl84,
    /// Tcl 8.5: as [`Self::Tcl86`], except that the check for a final form
    /// misses a leading `//` (`file join a //b` is `//b`, where 8.4 and 8.6
    /// give `/b`).
    Tcl85,
    /// Tcl 8.6: as [`Self::Tcl84`], with the final-form shortcut.
    Tcl86,
    /// Tcl 9.0 and 9.1: `~` is a plain character, and a leading `//` followed
    /// by a name is a root of its own (`//host`).
    Tcl9,
}

impl PathReading {
    /// The reading of `release`.
    #[must_use]
    pub const fn of_release(release: TclVersion) -> Self {
        match release {
            TclVersion::V8_4 => Self::Tcl84,
            TclVersion::V8_5 => Self::Tcl85,
            TclVersion::V8_6 => Self::Tcl86,
            TclVersion::V9_0 | TclVersion::V9_1 => Self::Tcl9,
        }
    }

    /// Whether a `~` names a home directory.
    const fn tilde_is_home(self) -> bool {
        !matches!(self, Self::Tcl9)
    }
}

/// What `file pathtype` reads a name as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathType {
    /// From a root: `/`, `//host`, 8.x's `~user`, 9.0's `//zipfs:/`.
    Absolute,
    /// From the working directory.
    Relative,
    /// From a volume's working directory: to 9.0, a name of nine bytes or
    /// more that starts `//zipfs` but not `//zipfs:/`.
    VolumeRelative,
}

impl PathType {
    /// The word `file pathtype` answers.
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            Self::Absolute => "absolute",
            Self::Relative => "relative",
            Self::VolumeRelative => "volumerelative",
        }
    }
}

/// The volume Tcl 9's zipfs filesystem lists (`ZIPFS_VOLUME`).
const ZIPFS_VOLUME: &[u8] = b"//zipfs:/";

/// A name's root (`TclGetPathType`'s drive name).
struct Root<'a> {
    /// The bytes of the name the root takes up.
    len: usize,
    /// The text the root stands for, which a volume-relative zipfs name
    /// writes apart (`//zipfsab` is the volume `//zipfs:` and `b`).
    text: &'a [u8],
    kind: PathType,
    /// Whether zipfs, not the native filesystem, reads it.
    zipfs: bool,
}

/// The length of the `//host` root that starts `name`, where `reading` keeps
/// one: two separators, then a name up to the next separator.
fn network_root(name: &[u8], reading: PathReading) -> Option<usize> {
    match name {
        [b'/', b'/', next, rest @ ..] if reading == PathReading::Tcl9 && *next != b'/' => Some(
            3 + rest
                .iter()
                .position(|&byte| byte == b'/')
                .unwrap_or(rest.len()),
        ),
        _ => None,
    }
}

/// `name`'s root where it has one. To 9.0 zipfs reads a name first
/// (`TclFSNonnativePathType`): one starting with its volume is absolute
/// under it, and one as long that starts with the volume but its `:/` is
/// relative to the volume `//zipfs:`. Otherwise the native root
/// (`TclpGetNativePathType`): `/`, a `//host`, or to 8.x the `~user` before
/// the first separator.
fn root(name: &[u8], reading: PathReading) -> Option<Root<'_>> {
    if reading == PathReading::Tcl9 && name.len() >= ZIPFS_VOLUME.len() {
        if name.starts_with(ZIPFS_VOLUME) {
            return Some(Root {
                len: ZIPFS_VOLUME.len(),
                text: ZIPFS_VOLUME,
                kind: PathType::Absolute,
                zipfs: true,
            });
        }
        let volume = &ZIPFS_VOLUME[..ZIPFS_VOLUME.len() - 1];
        if name.starts_with(&volume[..volume.len() - 1]) {
            return Some(Root {
                len: volume.len(),
                text: volume,
                kind: PathType::VolumeRelative,
                zipfs: true,
            });
        }
    }
    let len = match name.first()? {
        b'~' if reading.tilde_is_home() => name
            .iter()
            .position(|&byte| byte == b'/')
            .unwrap_or(name.len()),
        b'/' => network_root(name, reading).unwrap_or(1),
        _ => return None,
    };
    Some(Root {
        len,
        text: &name[..len],
        kind: PathType::Absolute,
        zipfs: false,
    })
}

/// `file pathtype name` on Unix.
#[must_use]
pub fn path_type(name: &[u8], reading: PathReading) -> PathType {
    root(name, reading).map_or(PathType::Relative, |root| root.kind)
}

/// `file split name`: the root where `name` starts with a separator or the
/// zipfs volume, then each non-empty element between separators, an element
/// after the first that starts with `~` written `./~x` where `reading` reads
/// `~` as a home.
#[must_use]
pub fn split(name: &[u8], reading: PathReading) -> Vec<Cow<'_, [u8]>> {
    let mut elements = Vec::new();
    let mut rest = name;
    let zipfs = reading == PathReading::Tcl9 && name.starts_with(ZIPFS_VOLUME);
    if zipfs || name.first() == Some(&b'/') {
        let root = if zipfs {
            ZIPFS_VOLUME.len()
        } else {
            network_root(name, reading).unwrap_or(1)
        };
        elements.push(Cow::Borrowed(&name[..root]));
        rest = &name[root..];
    }
    let parts = rest
        .split(|&byte| byte == b'/')
        .filter(|part| !part.is_empty());
    for (index, part) in parts.enumerate() {
        let leads = index == 0 && elements.is_empty();
        elements.push(if reading.tilde_is_home() && part[0] == b'~' && !leads {
            Cow::Owned([b"./", part].concat())
        } else {
            Cow::Borrowed(part)
        });
    }
    elements
}

/// Append one name's relative part to `out` as `TclpNativeJoinPath` does: a
/// separator where `out` does not end in one, then the part with its
/// repeated and trailing separators dropped. To 8.x a part `./~x` joined
/// after something drops its `./`. Once zipfs has read a name, a part
/// follows its volume `//zipfs:` with no separator (Tcl bug 1215dca7).
fn append(out: &mut Vec<u8>, part: &[u8], reading: PathReading, after_volume: bool) {
    let part = match part.strip_prefix(b"./") {
        Some(rest) if reading.tilde_is_home() && !out.is_empty() && rest.first() == Some(&b'~') => {
            rest
        }
        _ => part,
    };
    if part.is_empty() {
        return;
    }
    if !out.is_empty() && out.last() != Some(&b'/') && !after_volume {
        out.push(b'/');
    }
    let mut needs_separator = false;
    let mut index = 0;
    while index < part.len() {
        if part[index] == b'/' {
            while part.get(index + 1) == Some(&b'/') {
                index += 1;
            }
            if index + 1 < part.len() && needs_separator {
                out.push(b'/');
            }
        } else {
            out.push(part[index]);
            needs_separator = true;
        }
        index += 1;
    }
}

/// Whether `join` hands back `name`, its last name with root length `root`,
/// as given (`TclJoinPath`'s quick return): no separator after its root is
/// doubled or trailing, and from 8.6 none follows a root ending in one. 8.5
/// reads from just after the root `/`, so a leading `//` passes it.
fn kept_as_given(name: &[u8], root: usize, reading: PathReading) -> bool {
    let rest = &name[root..];
    let doubled_after_root = root > 0
        && name[root - 1] == b'/'
        && rest.first() == Some(&b'/')
        && reading != PathReading::Tcl85;
    reading != PathReading::Tcl84
        && !doubled_after_root
        && !rest
            .iter()
            .enumerate()
            .any(|(index, &byte)| byte == b'/' && matches!(rest.get(index + 1), None | Some(b'/')))
}

/// `file join name ?name …?`: each name's relative part after the last
/// name with a root, so such a name drops what came before it and an empty
/// one adds nothing (`file join a /b c` is `/b/c`).
#[must_use]
pub fn join<N: AsRef<[u8]>>(names: &[N], reading: PathReading) -> Vec<u8> {
    let mut out = Vec::new();
    let mut zipfs = false;
    let last = names.len().saturating_sub(1);
    for (index, name) in names.iter().enumerate() {
        let name = name.as_ref();
        let root = root(name, reading);
        if index == last
            && root.as_ref().is_none_or(|root| !root.zipfs)
            && (root.is_some() || index == 0)
            && kept_as_given(name, root.as_ref().map_or(0, |root| root.len), reading)
        {
            return name.to_vec();
        }
        let mut part = name;
        if let Some(root) = root {
            out.clear();
            out.extend_from_slice(root.text);
            part = &name[root.len..];
            zipfs = root.zipfs;
        }
        let after_volume = zipfs && out == ZIPFS_VOLUME[..ZIPFS_VOLUME.len() - 1];
        append(&mut out, part, reading, after_volume);
    }
    out
}

/// The user whose home directory `dirname` and `tail` read in place of
/// `name` (`TclPathPart`): where `reading` reads `~` as a home and `name` is
/// only `~user` and separators, the `user` (empty for `~`).
#[must_use]
pub fn home_user(name: &[u8], reading: PathReading) -> Option<&[u8]> {
    let root = root(name, reading)?;
    (name[0] == b'~' && name[root.len..].iter().all(|&byte| byte == b'/'))
        .then(|| &name[1..root.len])
}

/// The home directory of `user` (`TclGetHomeDir`, 9.0's
/// `MakeTildeRelativePath`): `$HOME` for the empty user, the host's user
/// database for any other.
///
/// # Errors
///
/// `couldn't find HOME environment variable to expand path` where `HOME` is
/// unset, and `user "…" doesn't exist` for a user the host cannot name; each
/// with the error code 8.6 added (8.4 and 8.5 set none).
pub fn home_directory(
    env: &dyn Env,
    user: &[u8],
    release: TclVersion,
) -> Result<Vec<u8>, CmdError> {
    let error = |message: String, code: &str| {
        if release >= TclVersion::V8_6 {
            CmdError::with_error_code(message, code)
        } else {
            CmdError::new(message)
        }
    };
    let reading = PathReading::of_release(release);
    if user.is_empty() {
        return match env.get("HOME") {
            Some(home) => Ok(join(&[home], reading)),
            None => Err(error(
                "couldn't find HOME environment variable to expand path".to_owned(),
                "TCL VALUE PATH HOMELESS",
            )),
        };
    }
    let user = String::from_utf8_lossy(user);
    match env.user_home(&user) {
        Some(home) => Ok(join(&[home], reading)),
        None => Err(error(
            format!("user \"{user}\" doesn't exist"),
            "TCL VALUE PATH NOUSER",
        )),
    }
}

/// `file tildeexpand name` (9.0's `TclResolveTildePath`): a name starting
/// `~user` as that user's home joined with what follows its separator, any
/// other name as it is.
///
/// # Errors
///
/// Where the home directory cannot be found ([`home_directory`]).
pub fn tilde_expand<'a>(
    name: &'a [u8],
    release: TclVersion,
    env: &dyn Env,
) -> Result<Cow<'a, [u8]>, CmdError> {
    if name.first() != Some(&b'~') {
        return Ok(Cow::Borrowed(name));
    }
    let split = name
        .iter()
        .position(|&byte| byte == b'/')
        .unwrap_or(name.len());
    let home = home_directory(env, &name[1..split], release)?;
    Ok(Cow::Owned(match name.get(split + 1..) {
        Some(rest) => join(&[home.as_slice(), rest], PathReading::of_release(release)),
        None => home,
    }))
}

/// The name `file nativename` gives on Unix (`Tcl_FSGetTranslatedPath`):
/// `name` joined, where the release reads its first element `~user` as a
/// home directory, that user's home in its place (`~/x` is `$HOME/x`). What
/// follows `~user/` joins as one part, so a `./` before a `~` drops; 8.4
/// and 8.5 join it as a name of its own, so a `~` there starts it again
/// (`~/~a` is `~a`, where 8.6 gives `$HOME/~a`). After `~user//` each
/// element joins in turn.
///
/// # Errors
///
/// Where the home directory cannot be found ([`home_directory`]).
pub fn translate<'a>(
    name: &'a [u8],
    release: TclVersion,
    env: &dyn Env,
) -> Result<Cow<'a, [u8]>, CmdError> {
    let reading = PathReading::of_release(release);
    if !reading.tilde_is_home() || name.first() != Some(&b'~') {
        return Ok(Cow::Owned(join(&[name], reading)));
    }
    let root = root(name, reading).map_or(name.len(), |root| root.len);
    let mut out = home_directory(env, &name[1..root], release)?;
    if root == name.len() {
        return Ok(Cow::Owned(out));
    }
    let rest = &name[root + 1..];
    if rest.first() != Some(&b'/') {
        if matches!(reading, PathReading::Tcl84 | PathReading::Tcl85) {
            return Ok(Cow::Owned(join(&[out.as_slice(), rest], reading)));
        }
        append(&mut out, rest, reading, false);
        return Ok(Cow::Owned(out));
    }
    for element in split(name, reading).iter().skip(1) {
        append(&mut out, element, reading, false);
    }
    Ok(Cow::Owned(out))
}

/// `file normalize name`: the [`translate`]d name made absolute against the
/// working directory, its `.` and `..` dropped as each release drops them and
/// its longest existing prefix on `fs` made canonical ([`canonical`]), links
/// not followed; with no filesystem, no prefix exists.
///
/// # Errors
///
/// Where the home directory cannot be found ([`home_directory`]).
pub fn normalize(
    name: &[u8],
    release: TclVersion,
    env: &dyn Env,
    fs: Option<&dyn Filesystem>,
) -> Result<Vec<u8>, CmdError> {
    let reading = PathReading::of_release(release);
    let translated = translate(name, release, env)?;
    let cwd = env.cwd().unwrap_or_else(|_| String::from("/"));
    let exists = |prefix: &[u8]| fs.is_some_and(|fs| fs.exists(&String::from_utf8_lossy(prefix)));
    Ok(canonical::normalize_translated(
        &translated,
        cwd.as_bytes(),
        reading,
        &exists,
    ))
}

/// The name `dirname` and `tail` read for `name` (`TclPathPart`): where
/// 8.x reads `name` as only `~user`, that user's home, normalised; `name`
/// itself otherwise.
///
/// # Errors
///
/// Where the home directory cannot be found ([`home_directory`]).
pub fn part_name<'a>(
    name: &'a [u8],
    release: TclVersion,
    env: &dyn Env,
    fs: Option<&dyn Filesystem>,
) -> Result<Cow<'a, [u8]>, CmdError> {
    if home_user(name, PathReading::of_release(release)).is_none() {
        return Ok(Cow::Borrowed(name));
    }
    normalize(name, release, env, fs).map(Cow::Owned)
}

/// `file tail name`: the last element, or the empty string where the only
/// element is a root (`/` and the empty name have no tail).
#[must_use]
pub fn tail(name: &[u8], reading: PathReading) -> Cow<'_, [u8]> {
    let mut elements = split(name, reading);
    match elements.len() {
        1 if path_type(name, reading) != PathType::Relative => Cow::Borrowed(b""),
        0 => Cow::Borrowed(b""),
        _ => elements.pop().expect("a last element"),
    }
}

/// `file dirname name`: the elements but the last joined, `.` where a
/// relative name has one element or none, and the only element where it is
/// a root.
#[must_use]
pub fn dirname(name: &[u8], reading: PathReading) -> Cow<'_, [u8]> {
    let mut elements = split(name, reading);
    match elements.len() {
        0 => Cow::Borrowed(b"."),
        1 if path_type(name, reading) == PathType::Relative => Cow::Borrowed(b"."),
        1 => elements.pop().expect("the root"),
        count => Cow::Owned(join(&elements[..count - 1], reading)),
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

    /// Each answer as tclsh 8.4.20 and 8.6.18, 8.5.19, and 9.0.4 and 9.1.0
    /// give it, where the releases read a leading
    /// `//` or a `~` apart.
    const SPLITS_BY_READING: &[(&str, [&[&str]; 3])] = &[
        ("//a", [&["/", "a"], &["/", "a"], &["//a"]]),
        ("//a/b", [&["/", "a", "b"], &["/", "a", "b"], &["//a", "b"]]),
        (
            "//a//b",
            [&["/", "a", "b"], &["/", "a", "b"], &["//a", "b"]],
        ),
        ("//a/", [&["/", "a"], &["/", "a"], &["//a"]]),
        ("///a", [&["/", "a"], &["/", "a"], &["/", "a"]]),
        ("a/~b", [&["a", "./~b"], &["a", "./~b"], &["a", "~b"]]),
        ("~b/c", [&["~b", "c"], &["~b", "c"], &["~b", "c"]]),
        ("~", [&["~"], &["~"], &["~"]]),
        ("~/", [&["~"], &["~"], &["~"]]),
        ("a/~", [&["a", "./~"], &["a", "./~"], &["a", "~"]]),
        (
            "a/./~b",
            [&["a", ".", "./~b"], &["a", ".", "./~b"], &["a", ".", "~b"]],
        ),
        ("./~b", [&[".", "./~b"], &[".", "./~b"], &[".", "~b"]]),
        (
            "x/~/y",
            [&["x", "./~", "y"], &["x", "./~", "y"], &["x", "~", "y"]],
        ),
        ("/~a", [&["/", "./~a"], &["/", "./~a"], &["/", "~a"]]),
        ("//~a", [&["/", "./~a"], &["/", "./~a"], &["//~a"]]),
        ("~a/~b", [&["~a", "./~b"], &["~a", "./~b"], &["~a", "~b"]]),
        ("a~/b", [&["a~", "b"], &["a~", "b"], &["a~", "b"]]),
    ];
    const JOINS_BY_READING: &[(&[&str], [&str; 3])] = &[
        (&["//a"], ["/a", "//a", "//a"]),
        (&["//a", "b"], ["/a/b", "/a/b", "//a/b"]),
        (&["a", "//b"], ["/b", "//b", "//b"]),
        (&["//a/b"], ["/a/b", "//a/b", "//a/b"]),
        (&["//a//b"], ["/a/b", "/a/b", "//a/b"]),
        (&["a", "//b/"], ["/b", "/b", "//b/"]),
        (&["a", "//b", "c"], ["/b/c", "/b/c", "//b/c"]),
        (&["a", "///b"], ["/b", "/b", "/b"]),
        (&["//", "a"], ["/a", "/a", "/a"]),
        (&["a", "//~b"], ["/~b", "//~b", "//~b"]),
        (&["a", "~b"], ["~b", "~b", "a/~b"]),
        (&["a", "~b", "c"], ["~b/c", "~b/c", "a/~b/c"]),
        (&["a", "./~b"], ["a/~b", "a/~b", "a/./~b"]),
        (&["a", "./~b/c"], ["a/~b/c", "a/~b/c", "a/./~b/c"]),
        (&["./~b"], ["./~b", "./~b", "./~b"]),
        (&["/", "~b"], ["~b", "~b", "/~b"]),
        (&["a", "~"], ["~", "~", "a/~"]),
        (&["a", "~/x"], ["~/x", "~/x", "a/~/x"]),
        (&["a", "~b/"], ["~b/", "~b/", "a/~b"]),
        (&["a", "", "~b"], ["~b", "~b", "a/~b"]),
        (&["~a", "b"], ["~a/b", "~a/b", "~a/b"]),
        (&["a", "b/~c"], ["a/b/~c", "a/b/~c", "a/b/~c"]),
        (&["a", "b/./~c"], ["a/b/./~c", "a/b/./~c", "a/b/./~c"]),
        (&["a", "~b/./~c"], ["~b/./~c", "~b/./~c", "a/~b/./~c"]),
        (&["a", "/~b"], ["/~b", "/~b", "/~b"]),
    ];
    /// A name, its `dirname`, `tail` and whether `pathtype` reads it as
    /// non-relative: 8.x first, then 9.x.
    type PartRow = (
        &'static str,
        [&'static str; 2],
        [&'static str; 2],
        [bool; 2],
    );
    /// `dirname` and `tail` where the releases differ (8.5 reads them as 8.4
    /// and 8.6 do), and `pathtype` of the same names.
    const PARTS_BY_READING: &[PartRow] = &[
        ("//a", ["/", "//a"], ["a", ""], [true, true]),
        ("//a/b", ["/a", "//a"], ["b", "b"], [true, true]),
        ("//a//b", ["/a", "//a"], ["b", "b"], [true, true]),
        ("a/~b", ["a", "a"], ["./~b", "~b"], [false, false]),
        ("~b/c", ["~b", "~b"], ["c", "c"], [true, false]),
        ("~/x", ["~", "~"], ["x", "x"], [true, false]),
        ("a/~", ["a", "a"], ["./~", "~"], [false, false]),
        ("x/~/y", ["x/~", "x/~"], ["y", "y"], [false, false]),
        ("a/./~b", ["a/.", "a/."], ["./~b", "~b"], [false, false]),
        ("/~a", ["/", "/"], ["./~a", "~a"], [true, true]),
        ("//~a", ["/", "//~a"], ["./~a", ""], [true, true]),
        ("~a/~b/c", ["~a/~b", "~a/~b"], ["c", "c"], [true, false]),
        ("a/~b/c", ["a/~b", "a/~b"], ["c", "c"], [false, false]),
    ];

    fn text(bytes: &[u8]) -> &str {
        std::str::from_utf8(bytes).expect("ASCII")
    }

    const READINGS: [PathReading; 4] = [
        PathReading::Tcl84,
        PathReading::Tcl85,
        PathReading::Tcl86,
        PathReading::Tcl9,
    ];

    /// The column of a `*_BY_READING` row `reading` answers from: 8.4 and
    /// 8.6 share the first.
    fn column(reading: PathReading) -> usize {
        match reading {
            PathReading::Tcl84 | PathReading::Tcl86 => 0,
            PathReading::Tcl85 => 1,
            PathReading::Tcl9 => 2,
        }
    }

    /// `file join` where 8.5 on hands back a last absolute name as given and
    /// 8.4 drops its `./` before a `~`: 8.4.20, 8.5.19, 8.6.18 and 9.0.4.
    const JOINS_AS_GIVEN: &[(&[&str], [&str; 4])] = &[
        (&["/./~b"], ["/~b", "/./~b", "/./~b", "/./~b"]),
        (&["a", "/./~b"], ["/~b", "/./~b", "/./~b", "/./~b"]),
        (&["/./~b", "c"], ["/~b/c", "/~b/c", "/~b/c", "/./~b/c"]),
        (&["/x", "./~b"], ["/x/~b", "/x/~b", "/x/~b", "/x/./~b"]),
        (&["a", "/./b"], ["/./b", "/./b", "/./b", "/./b"]),
    ];

    #[test]
    fn each_part_of_a_name_reads_as_tclsh_reads_it() {
        for reading in READINGS {
            for &(operation, name, expected) in PARTS {
                let name = name.as_bytes();
                let got = match operation {
                    "dirname" => text(&dirname(name, reading)).to_owned(),
                    "tail" => text(&tail(name, reading)).to_owned(),
                    "extension" => text(extension(name)).to_owned(),
                    "rootname" => text(rootname(name)).to_owned(),
                    other => unreachable!("no {other} row"),
                };
                assert_eq!(
                    got,
                    expected,
                    "{reading:?} file {operation} {{{}}}",
                    text(name)
                );
            }
            for &(name, expected) in SPLITS {
                let elements = split(name.as_bytes(), reading);
                let got: Vec<&str> = elements.iter().map(|e| text(e)).collect();
                assert_eq!(got, expected, "{reading:?} file split {{{name}}}");
            }
            for &(names, expected) in JOINS {
                assert_eq!(
                    text(&join(names, reading)),
                    expected,
                    "{reading:?} file join {names:?}"
                );
            }
        }
    }

    #[test]
    fn a_leading_double_slash_and_a_tilde_read_as_each_release_reads_them() {
        for (release, reading) in READINGS.into_iter().enumerate() {
            for &(names, expected) in JOINS_AS_GIVEN {
                assert_eq!(
                    text(&join(names, reading)),
                    expected[release],
                    "{reading:?} file join {names:?}"
                );
            }
            let column = column(reading);
            for &(name, expected) in SPLITS_BY_READING {
                let elements = split(name.as_bytes(), reading);
                let got: Vec<&str> = elements.iter().map(|e| text(e)).collect();
                assert_eq!(got, expected[column], "{reading:?} file split {{{name}}}");
            }
            for &(names, expected) in JOINS_BY_READING {
                assert_eq!(
                    text(&join(names, reading)),
                    expected[column],
                    "{reading:?} file join {names:?}"
                );
            }
            let column = usize::from(reading == PathReading::Tcl9);
            for &(name, dirnames, tails, absolute) in PARTS_BY_READING {
                let bytes = name.as_bytes();
                assert_eq!(
                    text(&dirname(bytes, reading)),
                    dirnames[column],
                    "{reading:?} file dirname {{{name}}}"
                );
                assert_eq!(
                    text(&tail(bytes, reading)),
                    tails[column],
                    "{reading:?} file tail {{{name}}}"
                );
                assert_eq!(
                    path_type(bytes, reading) != PathType::Relative,
                    absolute[column],
                    "{reading:?} file pathtype {{{name}}}"
                );
            }
        }
    }

    #[test]
    fn only_a_bare_home_name_reads_its_home_in_dirname_and_tail() {
        for (name, user) in [
            ("~", Some("")),
            ("~/", Some("")),
            ("~b", Some("b")),
            ("~b//", Some("b")),
            ("~b/c", None),
            ("a/~b", None),
            ("/~", None),
        ] {
            for reading in [PathReading::Tcl84, PathReading::Tcl85, PathReading::Tcl86] {
                assert_eq!(
                    home_user(name.as_bytes(), reading).map(text),
                    user,
                    "{reading:?} {name}"
                );
            }
            assert_eq!(
                home_user(name.as_bytes(), PathReading::Tcl9),
                None,
                "Tcl9 {name}"
            );
        }
    }

    #[test]
    fn every_release_maps_to_its_reading() {
        let readings: Vec<PathReading> = TclVersion::ALL
            .into_iter()
            .map(PathReading::of_release)
            .collect();
        assert_eq!(
            readings,
            [
                PathReading::Tcl84,
                PathReading::Tcl85,
                PathReading::Tcl86,
                PathReading::Tcl9,
                PathReading::Tcl9
            ]
        );
    }

    struct Home(Option<&'static str>);

    impl Env for Home {
        fn get(&self, key: &str) -> Option<String> {
            (key == "HOME")
                .then_some(self.0)
                .flatten()
                .map(str::to_owned)
        }
        fn user_home(&self, user: &str) -> Option<String> {
            (user == "root").then(|| "/root/".to_owned())
        }
        fn set(&self, _: &str, _: &str) {}
        fn unset(&self, _: &str) {}
        fn vars(&self) -> Vec<(String, String)> {
            Vec::new()
        }
        fn cwd(&self) -> Result<String, tcl_platform::HostError> {
            Ok("/tmp".to_owned())
        }
        fn chdir(&self, _: &str) -> Result<(), tcl_platform::HostError> {
            Ok(())
        }
    }

    /// `file nativename` and `file normalize` as tclsh 8.4.20, 8.5.19 and
    /// 8.6.18 (one column, 8.5 apart where it differs) and 9.0.4 give them,
    /// with `HOME=/root` and the working directory `/tmp`.
    const TRANSLATIONS: &[(&str, TclVersion, &str, &str)] = &[
        ("~", TclVersion::V8_6, "/root", "/root"),
        ("~/", TclVersion::V8_6, "/root", "/root"),
        ("~//", TclVersion::V8_6, "/root", "/root"),
        ("~/x", TclVersion::V8_6, "/root/x", "/root/x"),
        ("~/x/../y", TclVersion::V8_4, "/root/x/../y", "/root/y"),
        ("~/~a", TclVersion::V8_6, "/root/~a", "/root/~a"),
        ("~/./~a", TclVersion::V8_6, "/root/~a", "/root/~a"),
        ("~//./~a", TclVersion::V8_6, "/root/./~a", "/root/~a"),
        ("~/~a", TclVersion::V8_4, "~a", "~a"),
        ("~/~a", TclVersion::V8_5, "~a", "~a"),
        ("~//~a", TclVersion::V8_5, "/root/~a", "/root/~a"),
        ("~/a/./~b", TclVersion::V8_4, "/root/a/./~b", "/root/a/~b"),
        ("~", TclVersion::V9_0, "~", "/tmp/~"),
        ("~/x/../y", TclVersion::V9_0, "~/x/../y", "/tmp/~/y"),
        ("~nosuch/x", TclVersion::V9_0, "~nosuch/x", "/tmp/~nosuch/x"),
        ("//a", TclVersion::V8_6, "/a", "/a"),
        ("//a", TclVersion::V8_5, "//a", "//a"),
        ("//a", TclVersion::V9_0, "//a", "//a"),
        ("//a/b/..", TclVersion::V8_4, "/a/b/..", "/a"),
        ("//a/b/..", TclVersion::V8_5, "//a/b/..", "//a"),
        ("//a/b/..", TclVersion::V9_0, "//a/b/..", "//a"),
        ("//a/..", TclVersion::V9_0, "//a/..", "/"),
        ("//a/../..", TclVersion::V8_5, "//a/../..", "/"),
        ("//a/./b/", TclVersion::V8_5, "/a/./b", "/a/b"),
        ("//a/./b/", TclVersion::V9_0, "//a/./b", "//a/b"),
        ("///a/b", TclVersion::V9_0, "/a/b", "/a/b"),
        ("a/../b", TclVersion::V9_0, "a/../b", "/tmp/b"),
        ("a/~b", TclVersion::V8_6, "a/~b", "/tmp/a/~b"),
        ("/a/./~b", TclVersion::V8_4, "/a/./~b", "/a/~b"),
        ("/a/./~b", TclVersion::V8_6, "/a/./~b", "/a/~b"),
        ("//a/", TclVersion::V9_0, "//a/", "//a/"),
        ("//a///", TclVersion::V9_0, "//a/", "//a/"),
        ("//a/./", TclVersion::V9_0, "//a/.", "//a"),
        ("//a/", TclVersion::V8_5, "/a", "/a"),
        ("//a/.", TclVersion::V8_5, "//a/.", "//a"),
        ("", TclVersion::V8_6, "", ""),
        ("", TclVersion::V9_0, "", ""),
    ];

    #[test]
    fn nativename_and_normalize_read_a_home_and_a_host_root_as_each_release_does() {
        let env = Home(Some("/root"));
        for &(name, release, native, normal) in TRANSLATIONS {
            let got = translate(name.as_bytes(), release, &env).expect("translates");
            assert_eq!(text(&got), native, "{release:?} file nativename {{{name}}}");
            let got = normalize(name.as_bytes(), release, &env, None).expect("normalises");
            assert_eq!(text(&got), normal, "{release:?} file normalize {{{name}}}");
        }
    }

    #[test]
    fn dirname_and_tail_of_a_bare_home_read_the_home() {
        let env = Home(Some("/root"));
        for name in ["~", "~/", "~//"] {
            let part = part_name(name.as_bytes(), TclVersion::V8_6, &env, None).expect("a home");
            assert_eq!(text(&dirname(&part, PathReading::Tcl86)), "/");
            assert_eq!(text(&tail(&part, PathReading::Tcl86)), "root");
            let part = part_name(name.as_bytes(), TclVersion::V9_0, &env, None).expect("a name");
            assert_eq!(text(&part), name);
        }
        let part = part_name(b"~/x", TclVersion::V8_6, &env, None).expect("a name");
        assert_eq!(text(&part), "~/x");
        let error = part_name(b"~nosuch", TclVersion::V8_6, &env, None).expect_err("no user");
        assert_eq!(error.message().unwrap(), "user \"nosuch\" doesn't exist");
        assert_eq!(error.error_code().unwrap(), Some("TCL VALUE PATH NOUSER"));
        let error = part_name(b"~nosuch", TclVersion::V8_5, &env, None).expect_err("no user");
        assert_eq!(error.error_code().unwrap(), None);
        let error = part_name(b"~", TclVersion::V8_6, &Home(None), None).expect_err("homeless");
        assert_eq!(
            error.message().unwrap(),
            "couldn't find HOME environment variable to expand path"
        );
        assert_eq!(error.error_code().unwrap(), Some("TCL VALUE PATH HOMELESS"));
        for (home, dir, last) in [
            ("/", "/", ""),
            ("/homewontexist/test", "/homewontexist", "test"),
        ] {
            let part = part_name(b"~", TclVersion::V8_4, &Home(Some(home)), None).expect("a home");
            assert_eq!(
                text(&dirname(&part, PathReading::Tcl86)),
                dir,
                "HOME={home}"
            );
            assert_eq!(text(&tail(&part, PathReading::Tcl86)), last, "HOME={home}");
        }
    }

    /// A name, its `split`, `dirname`, `tail`, `pathtype` and `nativename`.
    type ZipfsRow = (
        &'static str,
        &'static [&'static str],
        &'static str,
        &'static str,
        PathType,
        &'static str,
    );
    /// Tcl 9.0.4 and 9.1.0 on names zipfs reads.
    const ZIPFS_NAMES: &[ZipfsRow] = &[
        (
            "//zipfs:/",
            &["//zipfs:/"],
            "//zipfs:/",
            "",
            PathType::Absolute,
            "//zipfs:/",
        ),
        (
            "//zipfs:/x/y",
            &["//zipfs:/", "x", "y"],
            "//zipfs:/x",
            "y",
            PathType::Absolute,
            "//zipfs:/x/y",
        ),
        (
            "//zipfs://x",
            &["//zipfs:/", "x"],
            "//zipfs:/",
            "x",
            PathType::Absolute,
            "//zipfs:/x",
        ),
        (
            "//zipfs:/./x",
            &["//zipfs:/", ".", "x"],
            "//zipfs:/.",
            "x",
            PathType::Absolute,
            "//zipfs:/./x",
        ),
        (
            "//zipfs:",
            &["//zipfs:"],
            "//zipfs:",
            "",
            PathType::Absolute,
            "//zipfs:",
        ),
        (
            "//zipfs:x",
            &["//zipfs:x"],
            "//zipfs:x",
            "",
            PathType::VolumeRelative,
            "//zipfs:x",
        ),
        (
            "//zipfs:xy/z",
            &["//zipfs:xy", "z"],
            "//zipfs:xy",
            "z",
            PathType::VolumeRelative,
            "//zipfs:xy/z",
        ),
        (
            "//zipfsx:/a",
            &["//zipfsx:", "a"],
            "//zipfs::",
            "a",
            PathType::VolumeRelative,
            "//zipfs::/a",
        ),
        (
            "//zipfsxy",
            &["//zipfsxy"],
            "//zipfsxy",
            "",
            PathType::VolumeRelative,
            "//zipfs:y",
        ),
        (
            "//zipfsabc/d",
            &["//zipfsabc", "d"],
            "//zipfs:bc",
            "d",
            PathType::VolumeRelative,
            "//zipfs:bc/d",
        ),
        (
            "//zipfsa/b",
            &["//zipfsa", "b"],
            "//zipfsa",
            "b",
            PathType::VolumeRelative,
            "//zipfs:b",
        ),
        (
            "//ZIPFS:/x",
            &["//ZIPFS:", "x"],
            "//ZIPFS:",
            "x",
            PathType::Absolute,
            "//ZIPFS:/x",
        ),
    ];
    const ZIPFS_JOINS: &[(&[&str], &str)] = &[
        (&["//zipfs:/", "a"], "//zipfs:/a"),
        (&["a", "//zipfs:/b"], "//zipfs:/b"),
        (&["//zipfs:", "a"], "//zipfs:/a"),
        (&["//zipfs:/a/", "b"], "//zipfs:/a/b"),
        (&["//zipfs:/a", "//b"], "//b"),
        (&["//zipfs:x", "a"], "//zipfs:x/a"),
        (&["a", "//zipfs:x"], "//zipfs:x"),
        (&["//zipfsxy", "b"], "//zipfs:y/b"),
        (&["//zipfs:/", "//zipfsab/c"], "//zipfs:b/c"),
        (&["//zipfsx/", "a"], "//zipfs:a"),
        (&["//zipfs//", "a", "b"], "//zipfs:a/b"),
        (&["//zipfsx/", "", "a"], "//zipfs:a"),
    ];

    #[test]
    fn tcl9_reads_the_zipfs_volume_as_a_root_of_its_own() {
        let env = Home(Some("/root"));
        let reading = PathReading::Tcl9;
        for &(name, elements, dir, last, kind, native) in ZIPFS_NAMES {
            let bytes = name.as_bytes();
            let got = split(bytes, reading);
            let got: Vec<&str> = got.iter().map(|e| text(e)).collect();
            assert_eq!(got, elements, "file split {name}");
            assert_eq!(text(&dirname(bytes, reading)), dir, "file dirname {name}");
            assert_eq!(text(&tail(bytes, reading)), last, "file tail {name}");
            assert_eq!(path_type(bytes, reading), kind, "file pathtype {name}");
            let got = translate(bytes, TclVersion::V9_1, &env).expect("translates");
            assert_eq!(text(&got), native, "file nativename {name}");
        }
        for &(names, expected) in ZIPFS_JOINS {
            assert_eq!(text(&join(names, reading)), expected, "file join {names:?}");
        }
        assert_eq!(
            path_type(b"//zipfs:/x", PathReading::Tcl86),
            PathType::Absolute
        );
        assert_eq!(
            text(&join(&["//zipfs:/", "a"], PathReading::Tcl86)),
            "/zipfs:/a"
        );
    }

    #[test]
    fn a_named_user_reads_the_hosts_user_database() {
        let env = Home(Some("/root"));
        let part = part_name(b"~root", TclVersion::V8_6, &env, None).expect("a home");
        assert_eq!(text(&tail(&part, PathReading::Tcl86)), "root");
        let native = translate(b"~root/~a", TclVersion::V8_6, &env).expect("a home");
        assert_eq!(text(&native), "/root/~a");
        let native = translate(b"~root/~a", TclVersion::V8_4, &env).expect("a home");
        assert_eq!(text(&native), "~a");
        assert_eq!(
            text(&home_directory(&env, b"root", TclVersion::V9_0).expect("a home")),
            "/root"
        );
        let error = home_directory(&env, b"nosuch", TclVersion::V9_0).expect_err("no user");
        assert_eq!(error.message().unwrap(), "user \"nosuch\" doesn't exist");
        assert_eq!(error.error_code().unwrap(), Some("TCL VALUE PATH NOUSER"));
    }

    /// `file tildeexpand` as tclsh 9.0.4 and 9.1.0 give it with `HOME=/root`.
    #[test]
    fn tildeexpand_joins_the_home_with_what_follows_its_separator() {
        let env = Home(Some("/root"));
        for (name, expected) in [
            ("~", "/root"),
            ("~/x", "/root/x"),
            ("~//x/", "/x"),
            ("~/~a", "/root/~a"),
            ("~/./~a", "/root/./~a"),
            ("~root", "/root"),
            ("~root//a//b", "/a/b"),
            ("a/~b", "a/~b"),
            ("", ""),
        ] {
            let got = tilde_expand(name.as_bytes(), TclVersion::V9_0, &env).expect("expands");
            assert_eq!(text(&got), expected, "file tildeexpand {name}");
        }
        let error = tilde_expand(b"~/x", TclVersion::V9_0, &Home(None)).expect_err("homeless");
        assert_eq!(error.error_code().unwrap(), Some("TCL VALUE PATH HOMELESS"));
    }
}

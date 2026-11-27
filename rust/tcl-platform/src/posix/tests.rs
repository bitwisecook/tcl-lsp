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

use super::*;

fn io(code: i32) -> Posix {
    Posix::from_io(&std::io::Error::from_raw_os_error(code)).expect("reported")
}

#[cfg(any(unix, target_os = "wasi"))]
#[test]
fn a_raw_errno_reports_tcl_name_and_text() {
    for (code, id, message) in [
        (libc::ENOENT, "ENOENT", "no such file or directory"),
        (libc::EACCES, "EACCES", "permission denied"),
        (libc::EISDIR, "EISDIR", "is a directory"),
        (libc::EPERM, "EPERM", "operation not permitted"),
    ] {
        let posix = io(code);
        assert_eq!(
            (posix.id(Table::Tcl90), &*posix.message(Table::Tcl90)),
            (id, message)
        );
    }
}

#[cfg(any(unix, target_os = "wasi"))]
#[test]
fn each_release_words_an_errno_from_its_own_table() {
    let words = |code, table| {
        let posix = io(code);
        (posix.id(table), posix.message(table).into_owned())
    };
    assert_eq!(
        words(libc::EISDIR, Table::Tcl84),
        ("EISDIR", "illegal operation on a directory".into())
    );
    assert_eq!(
        words(libc::EISDIR, Table::Tcl86),
        ("EISDIR", "illegal operation on a directory".into())
    );
    assert_eq!(
        words(libc::EEXIST, Table::Tcl86),
        ("EEXIST", "file already exists".into())
    );
    assert_eq!(
        words(libc::EEXIST, Table::Tcl90),
        ("EEXIST", "file exists".into())
    );
    // 8.4's table has no ECANCELED: C reports `unknown error` and strerror.
    let (id, message) = words(libc::ECANCELED, Table::Tcl84);
    assert_eq!(id, "unknown error");
    assert!(!message.contains("os error"), "{message}");
}

#[cfg(target_os = "linux")]
#[test]
fn a_shared_number_takes_the_name_c_tcl_keeps() {
    let name = |code| Posix::from_errno(code).and_then(|p| p.name());
    assert_eq!(name(libc::EAGAIN), Some("EAGAIN"));
    assert_eq!(name(libc::EDEADLK), Some("EDEADLK"));
    assert_eq!(name(libc::ENOTSUP), Some("ENOTSUP"));
}

#[cfg(any(unix, target_os = "wasi"))]
#[test]
fn an_errno_tcl_does_not_name_is_an_unknown_error() {
    let posix = io(9999);
    assert_eq!(posix.id(Table::LATEST), "unknown error");
    assert!(!posix.message(Table::LATEST).contains("os error"));
}

#[test]
fn an_error_without_an_errno_falls_back_to_its_kind() {
    let error = std::io::Error::new(ErrorKind::IsADirectory, "synthetic");
    assert_eq!(
        Posix::from_io(&error).and_then(|p| p.name()),
        Some("EISDIR")
    );
    assert_eq!(Posix::from_io(&std::io::Error::other("x")), None);
}

#[test]
fn every_numbered_name_is_in_a_table() {
    for (_, name) in tables::NUMBERS {
        assert!(Posix::named(name).is_some(), "{name}");
    }
}

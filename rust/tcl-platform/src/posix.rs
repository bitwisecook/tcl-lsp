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

//! Tcl's POSIX error identity: `Tcl_ErrnoId` and `Tcl_ErrnoMsg`
//! (`generic/tclPosixStr.c`), which `Tcl_PosixError` puts in
//! `-errorcode {POSIX <ID> <message>}` and after a failing command's
//! `couldn't open "…": `. The text is Tcl's own table, not the host's
//! `strerror`, and the table changed in 8.6 and again in 9.0.

use std::borrow::Cow;
use std::io::ErrorKind;

#[rustfmt::skip]
mod tables;

/// The `tclPosixStr.c` a release ships.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Table {
    /// Tcl 8.4 and 8.5.
    Tcl84,
    /// Tcl 8.6.
    Tcl86,
    /// Tcl 9.0 and 9.1.
    Tcl90,
}

impl Table {
    /// The newest table, for text that names no release.
    pub const LATEST: Self = Self::Tcl90;

    fn ids(self) -> &'static [&'static str] {
        match self {
            Self::Tcl84 => tables::IDS_84,
            Self::Tcl86 => tables::IDS_86,
            Self::Tcl90 => tables::IDS_90,
        }
    }

    fn messages(self) -> &'static [(&'static str, &'static str)] {
        match self {
            Self::Tcl84 => tables::MESSAGES_84,
            Self::Tcl86 => tables::MESSAGES_86,
            Self::Tcl90 => tables::MESSAGES_90,
        }
    }

    fn message(self, name: &str) -> Option<&'static str> {
        self.messages()
            .iter()
            .find(|(n, _)| *n == name)
            .map(|&(_, m)| m)
    }
}

const ALL: [Table; 3] = [Table::Tcl90, Table::Tcl86, Table::Tcl84];

/// One failed OS call's errno: its name when Tcl knows one, and the host's
/// text for an errno no table describes (`strerror`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Posix {
    name: Option<&'static str>,
    host_text: Option<String>,
}

impl Posix {
    /// The errno Tcl calls `name` (`ENOENT`, `EACCES`, …).
    #[must_use]
    pub fn named(name: &str) -> Option<Self> {
        let name = ALL
            .iter()
            .find_map(|t| t.ids().iter().copied().find(|n| *n == name))?;
        Some(Self {
            name: Some(name),
            host_text: None,
        })
    }

    /// The host errno `code`, or `None` when no table names it.
    #[must_use]
    pub fn from_errno(code: i32) -> Option<Self> {
        let &(_, name) = tables::NUMBERS.iter().find(|(n, _)| *n == code)?;
        Self::named(name)
    }

    /// The errno behind an I/O failure: its raw code when it has one, else the
    /// errno its kind stands for.
    #[must_use]
    pub fn from_io(error: &std::io::Error) -> Option<Self> {
        let Some(code) = error.raw_os_error() else {
            return kind_errno(error.kind()).and_then(Self::named);
        };
        let text = error.to_string();
        let text = text
            .strip_suffix(&format!(" (os error {code})"))
            .unwrap_or(&text);
        let name = Self::from_errno(code).and_then(|p| p.name);
        Some(Self {
            name,
            host_text: Some(text.to_owned()),
        })
    }

    /// The errno's name in any release's table.
    #[must_use]
    pub fn name(&self) -> Option<&'static str> {
        self.name
    }

    /// `Tcl_ErrnoId` under `table`: the name, or `unknown error`.
    #[must_use]
    pub fn id(&self, table: Table) -> &'static str {
        self.name
            .filter(|name| table.ids().contains(name))
            .unwrap_or("unknown error")
    }

    /// `Tcl_ErrnoMsg` under `table`, falling back to the host's text as C
    /// falls back to `strerror`.
    #[must_use]
    pub fn message(&self, table: Table) -> Cow<'static, str> {
        if let Some(message) = self.name.and_then(|name| table.message(name)) {
            return Cow::Borrowed(message);
        }
        match (&self.host_text, self.name) {
            (Some(text), _) => Cow::Owned(text.clone()),
            (None, Some(name)) => {
                Cow::Borrowed(ALL.iter().find_map(|t| t.message(name)).unwrap_or(name))
            }
            (None, None) => Cow::Borrowed("unknown POSIX error"),
        }
    }
}

/// The errno an [`ErrorKind`] stands for when the error carries no raw code.
fn kind_errno(kind: ErrorKind) -> Option<&'static str> {
    Some(match kind {
        ErrorKind::NotFound => "ENOENT",
        ErrorKind::PermissionDenied => "EACCES",
        ErrorKind::AlreadyExists => "EEXIST",
        ErrorKind::WouldBlock => "EAGAIN",
        ErrorKind::IsADirectory => "EISDIR",
        ErrorKind::NotADirectory => "ENOTDIR",
        ErrorKind::DirectoryNotEmpty => "ENOTEMPTY",
        ErrorKind::ReadOnlyFilesystem => "EROFS",
        ErrorKind::ConnectionRefused => "ECONNREFUSED",
        ErrorKind::ConnectionReset => "ECONNRESET",
        ErrorKind::ConnectionAborted => "ECONNABORTED",
        ErrorKind::NotConnected => "ENOTCONN",
        ErrorKind::AddrInUse => "EADDRINUSE",
        ErrorKind::AddrNotAvailable => "EADDRNOTAVAIL",
        ErrorKind::HostUnreachable => "EHOSTUNREACH",
        ErrorKind::NetworkUnreachable => "ENETUNREACH",
        ErrorKind::NetworkDown => "ENETDOWN",
        ErrorKind::BrokenPipe => "EPIPE",
        ErrorKind::InvalidInput => "EINVAL",
        ErrorKind::TimedOut => "ETIMEDOUT",
        ErrorKind::Interrupted => "EINTR",
        ErrorKind::Unsupported => "ENOTSUP",
        ErrorKind::OutOfMemory => "ENOMEM",
        ErrorKind::StorageFull => "ENOSPC",
        ErrorKind::QuotaExceeded => "EDQUOT",
        ErrorKind::FileTooLarge => "EFBIG",
        ErrorKind::NotSeekable => "ESPIPE",
        ErrorKind::ResourceBusy => "EBUSY",
        ErrorKind::ExecutableFileBusy => "ETXTBSY",
        ErrorKind::Deadlock => "EDEADLK",
        ErrorKind::CrossesDevices => "EXDEV",
        ErrorKind::TooManyLinks => "EMLINK",
        ErrorKind::InvalidFilename => "ENAMETOOLONG",
        ErrorKind::ArgumentListTooLong => "E2BIG",
        ErrorKind::StaleNetworkFileHandle => "ESTALE",
        _ => return None,
    })
}

#[cfg(test)]
mod tests;

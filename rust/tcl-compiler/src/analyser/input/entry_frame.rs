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

//! A procedure activation the caller evaluates a document inside.

/// The procedure frame a driver evaluates a whole document inside, instead of
/// the fresh interpreter's global frame. C Tcl evaluates a `tclIndex` inside
/// `auto_load_index` and a `pkgIndex.tcl` inside `tclPkgUnknown`: a bare
/// `set` there is a local, a global needs `global` or `::`, and the loader's
/// own locals (`dir`) and `global` links are in place before the first line.
/// Only the driver that runs the script that way supplies it; the analyser
/// never infers it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct EntryProcedureFrame {
    locals: Vec<(String, Option<String>)>,
    globals: Vec<String>,
}

impl EntryProcedureFrame {
    /// A procedure frame with no locals and no `global` links yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// A local the frame holds on entry, with its value when the driver
    /// knows it; `None` means it exists with unknown contents.
    #[must_use]
    pub fn with_local(mut self, name: impl Into<String>, value: Option<String>) -> Self {
        self.locals.push((name.into(), value));
        self
    }

    /// A `global NAME` link the frame holds on entry.
    #[must_use]
    pub fn with_global(mut self, name: impl Into<String>) -> Self {
        self.globals.push(name.into());
        self
    }

    /// The entry locals in declaration order.
    pub fn locals(&self) -> impl Iterator<Item = (&str, Option<&str>)> {
        self.locals
            .iter()
            .map(|(name, value)| (name.as_str(), value.as_deref()))
    }

    /// The entry `global` links in declaration order.
    pub fn globals(&self) -> impl Iterator<Item = &str> {
        self.globals.iter().map(String::as_str)
    }
}

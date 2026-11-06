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

//! A completed actual Jim cat construction, borrowed until its result transfers.

use super::*;

pub(crate) struct OriginalJimStringCatConstruction<'a> {
    inputs: &'a [obj::NativeObjectLifetime],
    output: obj::NativeObjectLifetime,
    source: Option<crate::interp::OriginalSourceConstruction>,
}

impl<'a> OriginalJimStringCatConstruction<'a> {
    pub(super) fn completed(
        inputs: &'a [obj::NativeObjectLifetime],
        output: *mut TclObj,
        source: Option<crate::interp::OriginalSourceConstruction>,
    ) -> Self {
        Self {
            inputs,
            output: obj::NativeObjectLifetime::retain(output),
            source,
        }
    }

    pub(crate) fn checked_parts(
        &self,
        actual: &Interp,
    ) -> Result<
        (
            Vec<*mut TclObj>,
            *mut TclObj,
            Option<crate::interp::OriginalSourceConstruction>,
        ),
        ValueError,
    > {
        if let Some(source) = &self.source {
            source.check(actual)?;
        }
        let inputs = self
            .inputs
            .iter()
            .map(obj::NativeObjectLifetime::checked_ptr)
            .collect::<Result<Vec<_>, _>>()?;
        Ok((inputs, self.output.checked_ptr()?, self.source.clone()))
    }
}

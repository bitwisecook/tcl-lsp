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

//! BIG-IP object specs for `gtm pool`, one file per following tmsh word.

use crate::bigip::BigipObjectSpec;

mod a;
mod aaaa;
mod cname;
mod https;
mod mx;
mod naptr;
mod srv;
mod svcb;

/// The spec tables of `gtm pool`, in the order they are filed.
#[must_use]
pub(super) fn tables() -> Vec<&'static [BigipObjectSpec]> {
    vec![
        a::SPECS,
        aaaa::SPECS,
        cname::SPECS,
        https::SPECS,
        mx::SPECS,
        naptr::SPECS,
        srv::SPECS,
        svcb::SPECS,
    ]
}

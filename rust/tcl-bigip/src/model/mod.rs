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

//! BIG-IP object model.
//!
//! The long-tail kinds share [`minimal::BigipMinimalObject`] /
//! [`minimal::BigipGenericObject`]; rich typed kinds (pool, virtual,
//! node, monitor, profile, rule, …) live in their per-module submodules.

pub mod enums;
pub mod r#gen;
pub mod minimal;
pub mod port_names;

pub use enums::{DataGroupType, ProfileType};
pub use r#gen::*;
pub use minimal::{BigipGenericObject, BigipMinimalObject};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_structs_carry_defaults() {
        // Non-empty string defaults survive into the Rust Default.
        let pool = BigipPool::default();
        assert_eq!(pool.module, "ltm");
        assert!(pool.full_path.is_empty());
        // Enum field defaults resolve.
        assert_eq!(BigipProfile::default().profile_type, ProfileType::Other);
        assert_eq!(BigipDataGroup::default().kind, DataGroupType::Internal);
    }
}

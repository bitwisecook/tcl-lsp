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

//! Settings: toggles, per-folder overrides, the global apply, reloads and resolved values.

use super::*;

#[path = "configuration/feature_toggles.rs"]
mod feature_toggles;
#[path = "configuration/folder_settings.rs"]
mod folder_settings;
#[path = "configuration/formatter_settings.rs"]
mod formatter_settings;
#[path = "configuration/global_settings.rs"]
mod global_settings;
#[path = "configuration/reload.rs"]
mod reload;
#[path = "configuration/resolved_settings.rs"]
mod resolved_settings;
#[path = "configuration/setting_parsers.rs"]
mod setting_parsers;

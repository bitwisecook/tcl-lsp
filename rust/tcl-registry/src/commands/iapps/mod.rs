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

//! `iapps` command specifications.

#![allow(non_snake_case)]

mod iapp__apm_config;
mod iapp__conf;
mod iapp__debug;
mod iapp__destination;
mod iapp__downgrade;
mod iapp__downgrade_template;
mod iapp__get_items;
mod iapp__is;
mod iapp__make_safe_password;
mod iapp__pool_members;
mod iapp__substa;
mod iapp__template;
mod iapp__tmos_version;
mod iapp__upgrade;
mod iapp__upgrade_template;
mod script__help;
mod script__init;
mod script__run;
mod script__tabc;
mod tmsh__add_help;
mod tmsh__add_tabc;
mod tmsh__begin_transaction;
mod tmsh__builtin_help;
mod tmsh__builtin_tabc;
mod tmsh__cancel_transaction;
mod tmsh__cd;
mod tmsh__clear_screen;
mod tmsh__commit_transaction;
mod tmsh__create;
mod tmsh__delete;
mod tmsh__display;
mod tmsh__display_threshold;
mod tmsh__get_config;
mod tmsh__get_field_names;
mod tmsh__get_field_value;
mod tmsh__get_name;
mod tmsh__get_status;
mod tmsh__get_type;
mod tmsh__include;
mod tmsh__list;
mod tmsh__log;
mod tmsh__log_dest;
mod tmsh__log_level;
mod tmsh__modify;
mod tmsh__pwd;
mod tmsh__reset_stats;
mod tmsh__show;
mod tmsh__stateless;
mod tmsh__version;

use crate::spec::CommandSpec;

/// Return all `iapps` command specifications.
#[must_use]
pub fn iapps_command_specs() -> Vec<CommandSpec> {
    vec![
        iapp__apm_config::spec(),
        iapp__conf::spec(),
        iapp__debug::spec(),
        iapp__destination::spec(),
        iapp__downgrade::spec(),
        iapp__downgrade_template::spec(),
        iapp__get_items::spec(),
        iapp__is::spec(),
        iapp__make_safe_password::spec(),
        iapp__pool_members::spec(),
        iapp__substa::spec(),
        iapp__template::spec(),
        iapp__tmos_version::spec(),
        iapp__upgrade::spec(),
        iapp__upgrade_template::spec(),
        script__help::spec(),
        script__init::spec(),
        script__run::spec(),
        script__tabc::spec(),
        tmsh__add_help::spec(),
        tmsh__add_tabc::spec(),
        tmsh__begin_transaction::spec(),
        tmsh__builtin_help::spec(),
        tmsh__builtin_tabc::spec(),
        tmsh__cancel_transaction::spec(),
        tmsh__cd::spec(),
        tmsh__clear_screen::spec(),
        tmsh__commit_transaction::spec(),
        tmsh__create::spec(),
        tmsh__delete::spec(),
        tmsh__display::spec(),
        tmsh__display_threshold::spec(),
        tmsh__get_config::spec(),
        tmsh__get_field_names::spec(),
        tmsh__get_field_value::spec(),
        tmsh__get_name::spec(),
        tmsh__get_status::spec(),
        tmsh__get_type::spec(),
        tmsh__include::spec(),
        tmsh__list::spec(),
        tmsh__log::spec(),
        tmsh__log_dest::spec(),
        tmsh__log_level::spec(),
        tmsh__modify::spec(),
        tmsh__pwd::spec(),
        tmsh__reset_stats::spec(),
        tmsh__show::spec(),
        tmsh__stateless::spec(),
        tmsh__version::spec(),
    ]
}

/// The `tmsh::` surface alone — the same interned specs
/// `iapps_command_specs` registers (tagged `IAPPS|TMSH`: real in both an
/// iApp's host script and a tmsh script), collected separately so
/// `load_surface(f5-tmsh)` can register the tmsh shell's command pack
/// without the iApp-only surface (D8).
#[must_use]
pub fn tmsh_command_specs() -> Vec<CommandSpec> {
    vec![
        tmsh__add_help::spec(),
        tmsh__add_tabc::spec(),
        tmsh__begin_transaction::spec(),
        tmsh__builtin_help::spec(),
        tmsh__builtin_tabc::spec(),
        tmsh__cancel_transaction::spec(),
        tmsh__cd::spec(),
        tmsh__clear_screen::spec(),
        tmsh__commit_transaction::spec(),
        tmsh__create::spec(),
        tmsh__delete::spec(),
        tmsh__display::spec(),
        tmsh__display_threshold::spec(),
        tmsh__get_config::spec(),
        tmsh__get_field_names::spec(),
        tmsh__get_field_value::spec(),
        tmsh__get_name::spec(),
        tmsh__get_status::spec(),
        tmsh__get_type::spec(),
        tmsh__include::spec(),
        tmsh__list::spec(),
        tmsh__log::spec(),
        tmsh__log_dest::spec(),
        tmsh__log_level::spec(),
        tmsh__modify::spec(),
        tmsh__pwd::spec(),
        tmsh__reset_stats::spec(),
        tmsh__show::spec(),
        tmsh__stateless::spec(),
        tmsh__version::spec(),
    ]
}

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

//! `expect` command specifications.

mod close;
mod debug;
mod disconnect;
mod exit;
mod exp_continue;
mod exp_internal;
mod exp_pid;
mod exp_version;
mod expect_after;
mod expect_background;
mod expect_before;
mod expect_cmd;
mod expect_tty;
mod expect_user;
mod fork;
mod interact;
mod log_file;
mod log_user;
mod match_max;
mod overlay;
mod parity;
mod remove_nulls;
mod send;
mod send_error;
mod send_log;
mod send_tty;
mod send_user;
mod sleep;
mod spawn;
mod strace;
mod stty;
mod system;
mod timestamp;
mod trap;
mod wait;

use crate::spec::CommandSpec;

/// Return all `expect` command specifications.
#[must_use]
pub fn expect_command_specs() -> Vec<CommandSpec> {
    vec![
        close::spec(),
        debug::spec(),
        disconnect::spec(),
        exit::spec(),
        exp_continue::spec(),
        exp_internal::spec(),
        exp_pid::spec(),
        exp_version::spec(),
        expect_cmd::spec(),
        expect_after::spec(),
        expect_background::spec(),
        expect_before::spec(),
        expect_tty::spec(),
        expect_user::spec(),
        fork::spec(),
        interact::spec(),
        log_file::spec(),
        log_user::spec(),
        match_max::spec(),
        overlay::spec(),
        parity::spec(),
        remove_nulls::spec(),
        send::spec(),
        send_error::spec(),
        send_log::spec(),
        send_tty::spec(),
        send_user::spec(),
        sleep::spec(),
        spawn::spec(),
        strace::spec(),
        stty::spec(),
        system::spec(),
        timestamp::spec(),
        trap::spec(),
        wait::spec(),
    ]
}

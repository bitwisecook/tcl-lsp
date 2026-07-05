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

//! `f5 pull` — fetch one object from a live device as an SCF stanza.
//!
//! Handles credential resolution and arg shaping; the live GET is implemented
//! but exercised only against a live device. With `--json` the raw iControl
//! JSON is emitted as 2-space-indented JSON; otherwise the object is rendered
//! to an SCF stanza and passed through `render_config`.

use super::emit::render_config;
use super::remote::auth::{ResolveOptions, resolve_credentials};
use super::remote::json_compat::dumps_indent2;
use super::remote::object_io::{object_to_scf_stanza, pull_object};

/// Parameters for [`run_pull`].
#[allow(clippy::struct_excessive_bools)]
pub struct PullArgs<'a> {
    pub kind: &'a str,
    pub full_path: &'a str,
    pub host: Option<&'a str>,
    pub user: Option<&'a str>,
    pub password: Option<&'a str>,
    pub port: Option<u16>,
    pub no_prompt: bool,
    pub insecure: bool,
    pub json: bool,
    pub timeout: f64,
    pub format: &'a str,
    pub transaction: bool,
}

/// Run `f5 pull`, returning the process exit code (0 / 2).
#[must_use]
pub fn run_pull(args: &PullArgs) -> u8 {
    let creds = match resolve_credentials(&ResolveOptions {
        host: args.host,
        user: args.user,
        password: args.password,
        port: args.port,
        ssh_port: None,
        interactive: !args.no_prompt,
    }) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: {e}");
            return 2;
        }
    };

    let obj = match pull_object(
        &creds,
        args.kind,
        args.full_path,
        args.insecure,
        args.timeout,
    ) {
        Ok(o) => o,
        Err(e) => {
            eprintln!("error: pull failed: {e}");
            return 2;
        }
    };

    if args.json {
        println!("{}", dumps_indent2(&obj));
    } else {
        let scf = object_to_scf_stanza(args.kind, &obj);
        print!(
            "{}",
            render_config(&scf, args.format, "create", args.transaction, "")
        );
    }
    0
}

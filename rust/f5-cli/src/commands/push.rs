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

//! `f5 push` — send one object to a live device via iControl REST.
//!
//! The `--dry-run` surface (request summary on stderr + the 2-space-indented
//! JSON body on stdout) is emitted *before* any credential resolution; the
//! live PUT/POST is implemented but exercised only against a live device.

use serde_json::Value;

use super::remote::auth::{ResolveOptions, resolve_credentials};
use super::remote::json_compat::dumps_indent2;
use super::remote::object_io::{self, dry_run_plan, parse_payload};
use super::remote::os_error_string;

/// Parameters for [`run_push`].
#[allow(clippy::struct_excessive_bools)]
pub struct PushArgs<'a> {
    pub kind: &'a str,
    pub payload: &'a str,
    pub host: Option<&'a str>,
    pub user: Option<&'a str>,
    pub password: Option<&'a str>,
    pub port: Option<u16>,
    pub no_prompt: bool,
    pub insecure: bool,
    pub create: bool,
    pub dry_run: bool,
    pub timeout: f64,
}

/// Run `f5 push`, returning the process exit code (0 success / 2 error). All
/// errors are printed to stderr as `error: …`.
#[must_use]
pub fn run_push(args: &PushArgs) -> u8 {
    let raw = if args.payload == "-" {
        let mut buf = String::new();
        if let Err(e) = std::io::Read::read_to_string(&mut std::io::stdin(), &mut buf) {
            eprintln!("error: {e}");
            return 2;
        }
        buf
    } else {
        match std::fs::read(args.payload) {
            Ok(bytes) => match String::from_utf8(bytes) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("error: {e}");
                    return 2;
                }
            },
            Err(e) => {
                eprintln!("error: {}", os_error_string(&e, args.payload));
                return 2;
            }
        }
    };

    let payload: Value = match parse_payload(&raw) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: {e}");
            return 2;
        }
    };

    if args.dry_run {
        let plan = dry_run_plan(&payload, args.create);
        eprintln!("would {} {} {}", plan.verb_label, args.kind, plan.target);
        println!("{}", dumps_indent2(&payload));
        return 0;
    }

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

    match object_io::push_object(
        &creds,
        args.kind,
        &payload,
        args.create,
        args.insecure,
        args.timeout,
    ) {
        Ok(result) => {
            println!("{}", dumps_indent2(&result));
            0
        }
        Err(e) => {
            eprintln!("error: push failed: {e}");
            2
        }
    }
}

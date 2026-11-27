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

//! The iRules surface as the registry models it, for comparison with the
//! BIG-IP rule checker's own tables (`scripts/dev/bigip-probes/open-questions/`).
//!
//! Reads command names on standard input, one per line, and prints one JSON
//! object per name: whether the f5-irules context resolves a spec, its arity,
//! `excluded_events`, `event_requires`, argument-prefix event forms, form
//! synopses, and the events where a bare call is legal according to
//! [`CommandRegistry::is_irules_call_legal_in_event`].
//!
//! Usage: `cargo run -q -p tcl-registry --example dump_irules_surface < names.txt`

use std::fmt::Write as _;
use std::io::BufRead as _;

use tcl_dialect::model::SurfaceQuery;
use tcl_dialect::model::family::Family;
use tcl_registry::events::EventRegistry;
use tcl_registry::model::ingress::static_context_for;
use tcl_registry::profiles::ProfileRegistry;

fn esc(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out
}

fn list(items: &[&str]) -> String {
    let parts: Vec<String> = items.iter().map(|s| format!("\"{}\"", esc(s))).collect();
    format!("[{}]", parts.join(","))
}

fn requires_json(r: Option<&tcl_registry::events::EventRequires>) -> String {
    r.map_or_else(
        || "null".to_string(),
        |r| {
            format!(
                "{{\"client_side\":{},\"server_side\":{},\"transport\":{},\"profiles\":{},\"also_in\":{},\"flow\":{}}}",
                r.client_side,
                r.server_side,
                r.transport.map_or_else(|| "null".to_string(), |t| format!("\"{t}\"")),
                list(r.profiles),
                list(r.also_in),
                r.flow
            )
        },
    )
}

fn main() {
    tcl_registry::install_shipped().expect("the shipped surface set installs");
    let registry = static_context_for("f5-irules").commands();
    let events = EventRegistry::build();
    let profiles = ProfileRegistry::build();
    let mut all_events = events.all_event_names();
    all_events.sort_unstable();

    for line in std::io::stdin().lock().lines() {
        let name = line.expect("stdin");
        let name = name.trim();
        if name.is_empty() {
            continue;
        }
        let spec =
            registry.get_for_surface(name, Some(SurfaceQuery::any_release(Family::F5Irules)));
        let Some(spec) = spec else {
            println!("{{\"name\":\"{}\",\"in_registry\":false}}", esc(name));
            continue;
        };
        let legal: Vec<&str> = all_events
            .iter()
            .copied()
            .filter(|ev| registry.is_irules_call_legal_in_event(name, &[], ev, &events, &profiles))
            .collect();
        let forms: Vec<String> = spec
            .event_requirement_forms
            .iter()
            .map(|f| {
                format!(
                    "{{\"prefix\":{},\"only_in\":{},\"requires\":{}}}",
                    list(f.argument_prefix),
                    list(f.only_in),
                    requires_json(f.requires.as_ref())
                )
            })
            .collect();
        let synopses: Vec<&str> = spec.forms.iter().map(|f| f.synopsis).collect();
        println!(
            "{{\"name\":\"{}\",\"in_registry\":true,\"arity\":[{},{}],\"excluded_events\":{},\"event_requires\":{},\"event_forms\":[{}],\"synopses\":{},\"legal_events\":{}}}",
            esc(name),
            spec.arity.min,
            if spec.arity.max == u16::MAX {
                -1
            } else {
                i32::from(spec.arity.max)
            },
            list(spec.excluded_events),
            requires_json(spec.event_requires.as_ref()),
            forms.join(","),
            list(&synopses),
            list(&legal),
        );
    }
}

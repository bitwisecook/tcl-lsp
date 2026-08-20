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

//! The `switch` builtin. Option parsing and pattern selection (exact/glob/regexp
//! incl. `default` and the TIP #75 `-matchvar`/`-indexvar` side-channel) are the
//! shared [`tcl_cmd_core::switch`] core, over `ValueOps` + the `regex`-crate
//! engine. The VM owns the per-target parts: extracting the pattern/body pairs
//! (inline or brace-list), resolving a `-` fall-through, the variable writes, and
//! evaluating the chosen body as a transparent script (`return`/`break`/
//! `continue` propagate).
//!
//! `-regexp` switches now match through the engine (previously they fell back to
//! exact); exact switches still use the `JUMP_TABLE` opcode, so this runtime form
//! is invoked for `-glob`/`-regexp`/`-nocase`/dynamic cases.

use tcl_cmd_core::switch::{self as core_switch, Selection};
use tcl_runtime_api::Completion;

use crate::cmd_regexp::CrateEngine;
use crate::interp::{Vm, err, ok};
use crate::value::Value;

pub(crate) fn register(vm: &mut Vm) {
    vm.register("switch", cmd_switch);
}

const USAGE_LIST: &str = "switch ?-option ...? string {?pattern body ...? ?default body?}";

fn cmd_switch(vm: &mut Vm, args: &[Value]) -> Completion<Value> {
    // Options + the `string` index are shared (the VM's argv is name-stripped).
    let opts = match core_switch::parse_options(vm, args) {
        Ok(o) => o,
        Err(e) => return err(e.into_message()),
    };
    let value = args[opts.value_index].clone();
    let rest = &args[opts.value_index + 1..];

    // Pattern/body pairs: a single trailing argument is the brace-list form.
    let pairs: Vec<(Value, Value)> = if rest.len() == 1 {
        let items = match rest[0].as_list() {
            Ok(i) => i,
            Err(e) => return err(e.message),
        };
        if items.is_empty() {
            return err(format!("wrong # args: should be \"{USAGE_LIST}\""));
        }
        if !items.len().is_multiple_of(2) {
            // The "misplaced comment" heuristic: a pattern beginning with `#`.
            let hint = items.iter().step_by(2).any(|p| p.to_str().starts_with('#'));
            return err(core_switch::extra_pattern_error(hint).into_message());
        }
        items
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| (c[0].clone(), c[1].clone()))
            .collect()
    } else {
        if !rest.len().is_multiple_of(2) {
            return err(core_switch::extra_pattern_error(false).into_message());
        }
        rest.as_chunks::<2>()
            .0
            .iter()
            .map(|c| (c[0].clone(), c[1].clone()))
            .collect()
    };

    // A trailing `-` fall-through body has nothing to fall through to.
    if let Some((pat, body)) = pairs.last()
        && &*body.to_str() == "-"
    {
        return err(core_switch::no_body_error(&pat.to_str()).into_message());
    }

    let patterns: Vec<Value> = pairs.iter().map(|(p, _)| p.clone()).collect();
    let sel = match core_switch::select::<Vm, CrateEngine, Value>(vm, &opts, &value, &patterns) {
        Ok(s) => s,
        Err(e) => return err(e.into_message()),
    };
    let Selection::Matched { index, writes } = sel else {
        return ok(Value::empty());
    };
    // TIP #75 `-matchvar`/`-indexvar` writes happen before the body runs.
    for (name, val) in writes {
        if let Err(e) = vm.set_var(&name.to_str(), val) {
            return e;
        }
    }
    // Resolve a `-` fall-through to the next real body (the trailing-`-` check
    // above guarantees one exists).
    let mut b = index;
    while &*pairs[b].1.to_str() == "-" {
        b += 1;
    }
    let body = pairs[b].1.to_str().to_string();
    match vm.eval_source(&body) {
        Ok(c) => c,
        Err(e) => err(e.message),
    }
}

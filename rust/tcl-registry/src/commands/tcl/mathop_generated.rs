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

//! `tcl::mathop` operator ensemble.
//!
//! Each operator with a real `::tcl::mathop` command form is registered in
//! three spellings (bare `+`, `tcl::mathop::+`, `::tcl::mathop::+`).
//! **Derived** from `tcl_syntax::expr::operators` (`BinOp::spec()`/
//! `UnaryOp::spec()`) — the single source of truth for which operators
//! exist, their dialect gating, and whether they have a mathop command form
//! at all — rather than hand-typed per this file's own former precedent
//! (~84 literal `CommandSpec` entries, prone to exactly the drift issue #984
//! found: missing `lt`/`le`/`gt`/`ge`, bogus `&&`/`||`/`@` entries that were
//! never real `::tcl::mathop` members in any released Tcl).
//!
//! `max`/`min` are correctly **absent** here for the same underlying reason
//! `&&`/`||` are: neither is a `BinOp`/`UnaryOp` grammar operator at all —
//! `max`/`min` are `expr` *math functions* (`mathfunc_generated.rs`), an
//! unrelated Tcl feature (verified against tclsh 8.6/9.0 — `info commands
//! ::tcl::mathop::*` never lists them). A bare `max`/`min` registry entry
//! that looked like a real builtin previously made a user `proc max {...}`
//! — ordinary, working Tcl, since bare `max` never resolves without an
//! explicit `namespace import ::tcl::mathop::*` the ensemble doesn't even
//! define — read as "renaming a builtin" by the optimiser's
//! command-rebinding scan, silently suppressing an otherwise-valid O103
//! fold. `&&`/`||` are absent for a different, structural reason: `BinOp::And`/
//! `BinOp::Or`'s `spec().mathop_shape` is `None` (TIP 174 excludes them —
//! a command can't short-circuit its own already-evaluated arguments).
use crate::prelude::*;
use tcl_syntax::expr::operators::{ALL_BIN_OPS, ALL_UNARY_OPS, CommandArity, OperatorSpec};

/// All `tcl::mathop` operator-command specs (every spelling).
pub fn specs() -> Vec<CommandSpec> {
    let mut out = Vec::new();
    for &op in ALL_BIN_OPS {
        push_spellings(&mut out, op.spec());
    }
    for &op in ALL_UNARY_OPS {
        push_spellings(&mut out, op.spec());
    }
    out
}

/// The three spellings a mathop operator command is invocable under.
const PREFIXES: [&str; 3] = ["", "tcl::mathop::", "::tcl::mathop::"];

/// Push a `CommandSpec` for each spelling of `spec`, or nothing at all when
/// `spec` has no `::tcl::mathop` command form (`mathop_shape: None` — `&&`,
/// `||`, every iRules word operator).
fn push_spellings(out: &mut Vec<CommandSpec>, spec: OperatorSpec) {
    let Some(shape) = spec.mathop_shape else {
        return;
    };
    let arity = to_registry_arity(shape.command_arity());
    for prefix in PREFIXES {
        let name = leak(format!("{prefix}{}", spec.spelling));
        let synopsis = leak(format!("{name} ?arg ...?"));
        let snippet = leak(format!(
            "The {} operator command. Available via namespace import ::tcl::mathop::*.",
            spec.spelling
        ));
        out.push(CommandSpec {
            name,
            dialects: spec.dialects,
            traits: Traits::OPERATOR_COMMAND,
            arity,
            hover: Some(HoverSnippet {
                summary: spec.summary,
                synopsis: leak_slice(vec![synopsis]),
                snippet,
                source: "Tcl man page mathop.n",
                examples: "",
                return_value: "",
            }),
            forms: leak_slice(vec![FormSpec {
                kind: FormKind::Default,
                synopsis,
            }]),
            ..CommandSpec::DEFAULT
        });
    }
}

/// `CommandArity` (`tcl-syntax`, `{min: u8, max: Option<u8>}`) to `Arity`
/// (`tcl-registry`, `{min: u16, max: u16}` with `u16::MAX` as "unlimited") —
/// the one-directional conversion the dependency graph requires
/// (`tcl-registry` depends on `tcl-syntax`, never the reverse, so `Arity`
/// can't be the shared type).
fn to_registry_arity(a: CommandArity) -> Arity {
    let min = u16::from(a.min);
    let max = a.max.map_or(Arity::UNLIMITED, u16::from);
    Arity::new(min, max)
}

/// Leak an owned `String` to a `'static` `&str` — safe and cheap here: every
/// mathop `CommandSpec` is built exactly once, at registry-construction
/// time, from a small fixed set (3 spellings x ~30 gated operators), not
/// per-lookup.
fn leak(s: String) -> &'static str {
    &*s.leak()
}

/// Leak an owned `Vec<T>` to a `'static` `&[T]` — see [`leak`].
fn leak_slice<T>(v: Vec<T>) -> &'static [T] {
    &*v.leak()
}

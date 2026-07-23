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

//! `tcl::mathfunc` — the `expr` math functions as command-table entries
//! (TIP 232: `::tcl::mathfunc::<name>` is a real, overridable command, which
//! is *how* a user `proc ::tcl::mathfunc::foo {x} {...}` is able to shadow
//! the built-in `foo` inside `expr {foo($x)}` — see the numeric-tower
//! contract, `docs/design/contracts/numeric-tower-and-expr-semantics.md`).
//!
//! **Derived** from `tcl_syntax::expr::mathfunc::all()` (`MathFuncSpec`) —
//! previously there was no registry data for math functions at all (zero
//! hover, zero completion for `abs`/`sin`/`max`/… anywhere in the LSP; a
//! confirmed gap, not a design choice).
//!
//! Only the two namespace-qualified spellings (`tcl::mathfunc::name`,
//! `::tcl::mathfunc::name`) are registered — unlike `mathop_generated.rs`'s
//! operators, a bare `abs`/`sin`/… is **not** an ordinary command: `expr`'s
//! grammar resolves `name(args)` as a function-call production distinct
//! from a bare command lookup, and nobody `namespace import`s
//! `::tcl::mathfunc::*` in practice (unlike `::tcl::mathop::*`, whose
//! operators are commonly imported for exactly that purpose).
//!
//! Every function gates to at least `DialectSet::TCL85_PLUS`: TIP 232 (Tcl
//! 8.5) is what created the `::tcl::mathfunc` command-table mechanism
//! itself, so even an 8.4-vintage function (`abs`, `sin`, …, gated `Tcl84`
//! by [`tcl_syntax::expr::mathfunc::MathFuncSince`] because *that* axis
//! tracks the `expr` grammar, not the command table) only became callable
//! as `::tcl::mathfunc::abs` from 8.5 on. A function introduced later
//! (`isnan` in 9.0, `gamma` in 9.1, …) keeps its own, tighter gate.
use crate::prelude::*;
use tcl_syntax::expr::mathfunc::{self, MathFuncSince};

/// All `tcl::mathfunc` math-function command specs (both qualified
/// spellings).
#[must_use]
pub fn specs() -> Vec<CommandSpec> {
    let mut out = Vec::new();
    for spec in mathfunc::all() {
        push_spellings(&mut out, &spec);
    }
    out
}

/// The two spellings a math function is invocable under as a command (see
/// the module docs for why there's no bare third spelling here, unlike
/// `mathop_generated.rs`).
const PREFIXES: [&str; 2] = ["tcl::mathfunc::", "::tcl::mathfunc::"];

fn push_spellings(out: &mut Vec<CommandSpec>, spec: &mathfunc::MathFuncSpec) {
    let dialects = Some(since_to_dialects(spec.since));
    let arity = to_registry_arity(spec.arity);
    for prefix in PREFIXES {
        let name = leak(format!("{prefix}{}", spec.name));
        let synopsis = leak(format!("{name}({})", arg_list(spec.arity)));
        let snippet = leak(format!(
            "The {} math function ({}). Available via ::tcl::mathfunc::{} or inside expr as {}(...).",
            spec.name, spec.summary, spec.name, spec.name
        ));
        out.push(CommandSpec {
            name,
            dialects,
            traits: Traits::PURE,
            arity,
            return_type: Some(TclType::Numeric),
            hover: Some(HoverSnippet {
                summary: spec.summary,
                synopsis: leak_slice(vec![synopsis]),
                snippet,
                source: "Tcl man page mathfunc.n",
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

/// The command-table gate for a function first recognised in `since`'s
/// `expr` grammar release — see the module docs for why this is never
/// looser than `TCL85_PLUS` regardless of `since`.
fn since_to_dialects(since: MathFuncSince) -> DialectSet {
    match since {
        MathFuncSince::Tcl84 | MathFuncSince::Tcl85 => DialectSet::TCL85_PLUS,
        MathFuncSince::Tcl90 => DialectSet::TCL90_PLUS,
        MathFuncSince::Tcl91 => DialectSet::TCL91,
    }
}

/// A generic `a, b, c, ...` placeholder argument list sized to `arity`'s
/// minimum (variadic functions like `max`/`min` show one trailing `...`).
fn arg_list(arity: tcl_syntax::expr::operators::CommandArity) -> &'static str {
    let names = ["x", "y", "z"];
    let mut parts: Vec<&str> = names.iter().take(arity.min as usize).copied().collect();
    if arity.max.is_none() || usize::from(arity.max.unwrap_or(0)) > parts.len() {
        parts.push("...");
    }
    leak(parts.join(", "))
}

/// `CommandArity` (`tcl-syntax`) to `Arity` (`tcl-registry`) — see
/// `mathop_generated.rs`'s identical helper for the rationale.
fn to_registry_arity(a: tcl_syntax::expr::operators::CommandArity) -> Arity {
    let min = u16::from(a.min);
    let max = a.max.map_or(Arity::UNLIMITED, u16::from);
    Arity::new(min, max)
}

/// Leak an owned `String` to a `'static` `&str` — safe and cheap here: every
/// mathfunc `CommandSpec` is built exactly once, at registry-construction
/// time, from a small fixed set (2 spellings x ~56 functions).
fn leak(s: String) -> &'static str {
    &*s.leak()
}

/// Leak an owned `Vec<T>` to a `'static` `&[T]` — see [`leak`].
fn leak_slice<T>(v: Vec<T>) -> &'static [T] {
    &*v.leak()
}

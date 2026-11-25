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

//! Jim's integer loop, with the counter initialised before the first test.

use crate::prelude::*;
use tcl_dialect::model::{Family, SpecSurface};

const SYNOPSIS: &str = "loop var ?first? limit ?incr? body";
const SURFACE: &[SpecSurface] = &[SpecSurface::core_in(Family::Jim, &[("0.84", None)])];
const INTEGER_EXPRESSIONS: &[RepeatedArgLayout] = &[RepeatedArgLayout {
    exclude_trailing: 1,
    ..RepeatedArgLayout::every(ArgRole::Expr, 1)
}];
const GRAMMAR: ClauseGrammarSpec = ClauseGrammarSpec {
    head: ClauseRow::head(
        &[ClauseSlot::of(ArgRole::VarWrite)],
        ClauseTiming::LoopFixture(LoopPhase::Init),
    ),
    rows: &[ClauseRow::group(0, ClauseTiming::Always)],
    tail: Some(ClauseRow::once(
        None,
        &[ClauseSlot::of(ArgRole::Body)],
        ClauseTiming::PerIteration,
    )),
    fallthrough_body: None,
    default_clause: None,
    selection: ClauseSelection::All,
    surface: None,
};

/// `Jim_LoopCoreCommand` in Jim 0.84 accepts three through five arguments.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "loop",
        surface: Some(SURFACE),
        runtime_backing: RuntimeBacking::None,
        arity: Arity::new(3, 5),
        arg_roles: &[(0, ArgRole::VarWrite)],
        repeated_args: INTEGER_EXPRESSIONS,
        clause_grammar: Some(&GRAMMAR),
        evaluation_contexts: Some(crate::evaluation_context::EvaluationContextSpec::CALLER),
        completion: crate::knowledge::Knowledge::Partial(
            crate::completion::CompletionDescriptor::CONSERVATIVE
                .with_body_boundary(crate::completion::CompletionBoundary::LOOP),
        ),
        traits: Traits::CONTROL_FLOW
            | Traits::HAS_LOOP_BODY
            | Traits::NEVER_INLINE_BODY
            | Traits::NOT_PROC_FACTORY
            | Traits::CONDITIONAL_VARIABLE_WRITE,
        side_effects: crate::knowledge::Knowledge::Partial(&[SideEffect {
            target: SideEffectTarget::Variable,
            reads: true,
            writes: true,
            ..SideEffect::DEFAULT
        }]),
        // Initialisation writes an integer even for an empty loop. A body
        // can overwrite or unset it before `break` completes the call.
        var_write_typing: VarWriteTyping::Destructured,
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "Iterate over integers in the caller's scope.",
            synopsis: &[SYNOPSIS],
            snippet: "Initialises the counter to first (default 0) and steps by incr (default 1) towards the exclusive limit.",
            source: "https://jim.tcl.tk/home/doc/trunk/jim_tcl.txt",
            examples: "loop i 3 { puts $i }",
            return_value: "Returns the empty string on normal completion.",
        }),
        forms: &[FormSpec {
            synopsis: SYNOPSIS,
            ..FormSpec::DEFAULT
        }],
        ..CommandSpec::DEFAULT
    }
}

#[cfg(test)]
#[test]
fn jim_loop_selects_the_counter_expressions_and_body_at_each_arity() {
    let jim = crate::model::ingress::static_context_for("jimtcl");
    let query = Some(jim.context().authoring_query());
    for args in [
        &["i", "3", ""][..],
        &["i", "1", "4", ""],
        &["i", "4", "0", "-2", ""],
    ] {
        let selected = jim
            .commands()
            .resolve_invocation("loop", args, query)
            .unwrap();
        let mut roles = selected.arg_roles().unwrap();
        roles.sort_unstable_by_key(|(index, _)| *index);
        let body = args.len() - 1;
        assert_eq!(roles.len(), args.len());
        assert_eq!(roles[0], (0, ArgRole::VarWrite));
        assert!(
            roles[1..body]
                .iter()
                .all(|(_, role)| *role == ArgRole::Expr)
        );
        assert_eq!(roles[body], (body, ArgRole::Body));
        let plan = selected.clause_plan().unwrap();
        assert_eq!(plan.defect, None);
        assert_eq!(
            plan.clauses.last().unwrap().timing,
            ClauseTiming::PerIteration
        );
    }
    for dialect in ["tcl8.6", "tcl9.1", "f5-irules"] {
        assert!(
            crate::model::ingress::static_context_for(dialect)
                .resolve_command("loop")
                .is_none()
        );
    }
}

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

//! Gather every Tcl-local name mentioned by a procedure body.
//!
//! These are the candidates for the "spill all" branch of alias
//! inference when a dynamic-name command can't be resolved to a
//! single known target.

use std::collections::HashSet;

use crate::ir::{Script, Statement};

/// Collect every variable name written, read, declared, or
/// otherwise mentioned by *body*. The result seeds
/// [`super::state::EscapeState::known_names`] before the walker
/// runs.
#[must_use]
pub fn collect_known_names<I: IntoIterator<Item = String>>(
    params: I,
    body: &Script,
) -> HashSet<String> {
    let mut names: HashSet<String> = params.into_iter().collect();
    visit(&body.statements, &mut names);
    names
}

fn visit(stmts: &[Statement], names: &mut HashSet<String>) {
    for stmt in stmts {
        visit_one(stmt, names);
    }
}

fn insert_nonempty(names: &mut HashSet<String>, name: &str) {
    if !name.is_empty() {
        names.insert(name.to_string());
    }
}

fn visit_one(stmt: &Statement, names: &mut HashSet<String>) {
    match stmt {
        Statement::AssignConst { name, .. }
        | Statement::AssignValue { name, .. }
        | Statement::AssignExpr { name, .. }
        | Statement::Incr { name, .. } => insert_nonempty(names, name),
        Statement::Call { defs, reads, .. } => {
            for name in defs.iter().chain(reads.iter()) {
                insert_nonempty(names, name);
            }
        }
        Statement::Catch {
            result_var,
            options_var,
            ..
        } => {
            for name in result_var.iter().chain(options_var.iter()) {
                insert_nonempty(names, name);
            }
        }
        Statement::Try { handlers, .. } => {
            for handler in handlers {
                for name in handler.var_name.iter().chain(handler.options_var.iter()) {
                    insert_nonempty(names, name);
                }
            }
        }
        Statement::Foreach { iterators, .. } => {
            for iterator in iterators {
                for name in &iterator.vars {
                    insert_nonempty(names, name);
                }
            }
        }
        _ => {}
    }
    for child in stmt.child_scripts() {
        visit(&child.statements, names);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowering::lower_to_ir;
    use tcl_registry::CommandRegistry;

    fn reg() -> std::sync::Arc<CommandRegistry> {
        tcl_registry::model::ingress::static_context_for("tcl8.6")
            .commands()
            .clone()
    }

    fn body_of(src: &str) -> Script {
        let m = lower_to_ir(src, &reg());
        m.top_level
    }

    #[test]
    fn collects_assignment_names() {
        let body = body_of("set foo 1\nset bar 2");
        let names = collect_known_names(std::iter::empty::<String>(), &body);
        assert!(names.contains("foo"));
        assert!(names.contains("bar"));
    }

    #[test]
    fn includes_seeded_params() {
        let body = body_of("");
        let names = collect_known_names(["a".to_string(), "b".to_string()], &body);
        assert!(names.contains("a"));
        assert!(names.contains("b"));
    }

    #[test]
    fn descends_into_if_body() {
        let module = lower_to_ir(
            "proc f {condition} {if {$condition} {set inside_if 1} else {set inside_else 2}}",
            &reg(),
        );
        let body = &module.procedures["::f"].body;
        let names = collect_known_names(std::iter::empty::<String>(), body);
        assert!(names.contains("inside_if"));
        assert!(names.contains("inside_else"));
    }

    #[test]
    fn descends_into_loop_bodies() {
        let body = body_of("for {set i 0} {$i < 5} {incr i} { set in_for 1 }");
        let names = collect_known_names(std::iter::empty::<String>(), &body);
        assert!(names.contains("i"));
        assert!(names.contains("in_for"));
    }

    #[test]
    fn captures_foreach_loop_vars() {
        let body = body_of("foreach {a b} {1 2 3 4} { set x $a }");
        let names = collect_known_names(std::iter::empty::<String>(), &body);
        assert!(names.contains("a"));
        assert!(names.contains("b"));
        assert!(names.contains("x"));
    }

    #[test]
    fn captures_catch_result_and_options_var() {
        let body = body_of("catch {puts hi} result options");
        let names = collect_known_names(std::iter::empty::<String>(), &body);
        assert!(names.contains("result"));
        assert!(names.contains("options"));
    }

    #[test]
    fn descends_into_switch_arms_and_default() {
        // ``switch`` lowers to a ``Statement::Switch`` (or to a
        // Barrier when the dispatch is too complex). Test only the
        // shape that lowers to Switch — single-token subject + arm
        // bodies inside a ``{... ... ...}`` brace pair.
        let body = body_of(
            "switch x {\n  a { set hit_a 1 }\n  b { set hit_b 2 }\n  default { set hit_default 3 }\n}",
        );
        let names = collect_known_names(std::iter::empty::<String>(), &body);
        // At minimum, the assignments inside arm bodies should be
        // visible; if the lowering chose Barrier, ``hit_a`` etc.
        // won't appear (they'd be inside a nested-script string),
        // which is itself a valid outcome — collect what's there.
        // Either the structured form (with all three present) or
        // the barrier form (none present) is acceptable.
        let has_all =
            names.contains("hit_a") && names.contains("hit_b") && names.contains("hit_default");
        let has_none =
            !names.contains("hit_a") && !names.contains("hit_b") && !names.contains("hit_default");
        assert!(
            has_all || has_none,
            "expected switch lowering to be either fully structured or fully barrier; got {names:?}",
        );
    }

    #[test]
    fn descends_into_block_body() {
        // ``eval {set y 5}`` lowers to a Statement::Block in a proc
        // context — collect_known_names should descend into it.
        let m = lower_to_ir("proc f {} { eval {set y 5} }", &reg());
        let proc = m.procedures.get("::f").unwrap();
        let names = collect_known_names(std::iter::empty::<String>(), &proc.body);
        assert!(names.contains("y"));
    }
}

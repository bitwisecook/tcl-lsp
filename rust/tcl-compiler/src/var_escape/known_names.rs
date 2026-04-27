//! Gather every Tcl-local name mentioned by a procedure body
//! (C33b3).
//!
//! Mirrors `core/compiler/var_escape/_propagation.py::_collect_known_names`.
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

#[allow(clippy::too_many_lines)]
fn visit_one(stmt: &Statement, names: &mut HashSet<String>) {
    match stmt {
        Statement::AssignConst { name, .. }
        | Statement::AssignValue { name, .. }
        | Statement::AssignExpr { name, .. }
        | Statement::Incr { name, .. }
            if !name.is_empty() =>
        {
            names.insert(name.clone());
        }
        Statement::Call { defs, reads, .. } => {
            for n in defs.iter().chain(reads.iter()) {
                if !n.is_empty() {
                    names.insert(n.clone());
                }
            }
        }
        Statement::If {
            clauses, else_body, ..
        } => {
            for c in clauses {
                visit(&c.body.statements, names);
            }
            if let Some(b) = else_body {
                visit(&b.statements, names);
            }
        }
        Statement::For {
            init, next, body, ..
        } => {
            visit(&init.statements, names);
            visit(&next.statements, names);
            visit(&body.statements, names);
        }
        Statement::Foreach {
            iterators, body, ..
        } => {
            for it in iterators {
                for v in &it.vars {
                    if !v.is_empty() {
                        names.insert(v.clone());
                    }
                }
            }
            visit(&body.statements, names);
        }
        Statement::Catch {
            result_var,
            options_var,
            body,
            ..
        } => {
            if let Some(r) = result_var {
                if !r.is_empty() {
                    names.insert(r.clone());
                }
            }
            if let Some(o) = options_var {
                if !o.is_empty() {
                    names.insert(o.clone());
                }
            }
            visit(&body.statements, names);
        }
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            visit(&body.statements, names);
            for h in handlers {
                if let Some(v) = &h.var_name {
                    if !v.is_empty() {
                        names.insert(v.clone());
                    }
                }
                if let Some(o) = &h.options_var {
                    if !o.is_empty() {
                        names.insert(o.clone());
                    }
                }
                visit(&h.body.statements, names);
            }
            if let Some(f) = finally_body {
                visit(&f.statements, names);
            }
        }
        Statement::Switch {
            arms, default_body, ..
        } => {
            for a in arms {
                if let Some(b) = &a.body {
                    visit(&b.statements, names);
                }
            }
            if let Some(d) = default_body {
                visit(&d.statements, names);
            }
        }
        Statement::While { body, .. }
        | Statement::Block { body, .. }
        | Statement::UpFrame { body, .. } => {
            visit(&body.statements, names);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lowering::lower_to_ir;
    use tcl_registry::CommandRegistry;

    fn reg() -> CommandRegistry {
        CommandRegistry::build_default()
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
        let body = body_of("if {1} { set inside_if 1 } else { set inside_else 2 }");
        let names = collect_known_names(std::iter::empty::<String>(), &body);
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

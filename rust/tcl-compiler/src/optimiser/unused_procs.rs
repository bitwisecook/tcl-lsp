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

//! Unused-proc commenting pass (O124) for iRules.
//!
//! When an iRule defines procs that are never called from any
//! event handler (transitively), this pass suggests commenting
//! them out. Only applies to iRules with at least one non-RULE_INIT
//! event — iRules that look like libraries (only procs, maybe a
//! `RULE_INIT` setting static variables) are excluded, since their
//! procs are called externally.
//!
//! The pass reports `O124` with a replacement that prefixes every
//! non-empty body line with `# ` and prepends an explanatory
//! banner. The code is gated on `ctx.dialect == Some(tcl_dialect::DialectProfile::irules())`.
//! If any reachable proc has a `has_barrier` flag (dynamic
//! dispatch — `eval`, `uplevel`, etc.), the pass bails out to
//! avoid false positives.

use std::collections::HashSet;
use tcl_core_types::DiagCode;

use crate::compilation_unit::CompilationUnit;
use crate::ir::when_event_name;
use crate::taint::is_irules_dialect;

use super::helpers::spans::full_rewrite_span;
use super::{Optimisation, PassContext};

/// Run the unused-procs pass.
///
/// No-op unless [`PassContext::dialect`] resolves to the iRules profile.  Both
/// spellings reach it: the canonical `f5-irules` (which
/// [`tcl_dialect::DialectProfile::irules`] returns directly) and the `irules`
/// alias, which `DialectProfile::by_name` canonicalises to the same profile —
/// the two names `active_dialect()` accepts interchangeably for iRules.
pub fn run(ctx: &mut PassContext<'_>, cu: &CompilationUnit) {
    if !is_irules_dialect(ctx.dialect) {
        return;
    }

    let ir_module = &cu.ir_module;
    if ir_module.procedures.is_empty() {
        return;
    }

    // Partition procs: event handlers (`::when::*`) vs user procs.
    let mut event_procs: HashSet<String> = HashSet::new();
    let mut event_names: HashSet<String> = HashSet::new();
    let mut user_procs: Vec<String> = Vec::new();

    for qname in ir_module.procedures.keys() {
        if qname.starts_with("::when::") {
            event_procs.insert(qname.clone());
            event_names.insert(when_event_name(qname).to_owned());
        } else {
            user_procs.push(qname.clone());
        }
    }

    if user_procs.is_empty() {
        return;
    }

    // Skip library iRules — only procs + optional RULE_INIT.
    if is_library_irule(&event_names) {
        return;
    }

    // Reachability from any event handler.
    let reachable = reachable_procs(&event_procs, &ctx.interproc);

    // Conservative escape hatch: any reachable proc with a
    // dynamic barrier could dynamically dispatch to an otherwise
    // "unused" proc. Suppress O124 entirely. Deliberately we do
    // *not* check `has_unknown_calls` — it fires for impure
    // built-in commands (`pool`, `puts`, …) that cannot invoke
    // user procs.
    for qname in &reachable {
        if let Some(summary) = ctx.interproc.procedures.get(qname)
            && summary.has_barrier
        {
            return;
        }
    }

    // Report user procs not reachable from any event.
    let mut unused: Vec<&String> = user_procs
        .iter()
        .filter(|q| !reachable.contains(*q))
        .collect();
    unused.sort();

    for qname in unused {
        let Some(ir_proc) = ir_module.procedures.get(qname) else {
            continue;
        };
        let span = full_rewrite_span(ctx.source, ir_proc.span);
        let range = span.as_range();
        if range.end > ctx.source.len() || range.start >= ctx.source.len() {
            continue;
        }
        let proc_text = &ctx.source[range];
        let replacement = comment_out(proc_text, &ir_proc.name);
        let message = format!(
            "Proc '{}' is not called from any event and can be removed",
            ir_proc.name,
        );

        ctx.report(Optimisation::new(
            DiagCode::O124,
            message,
            span,
            replacement,
        ));
    }
}

// Library-iRule detection

/// Return `true` when the set of event names looks like a library
/// iRule — nothing except optionally `RULE_INIT`.
fn is_library_irule(event_names: &HashSet<String>) -> bool {
    event_names.iter().all(|n| n == "RULE_INIT")
}

// Reachability walk

fn reachable_procs(
    roots: &HashSet<String>,
    interproc: &crate::interprocedural::InterproceduralAnalysis,
) -> HashSet<String> {
    let mut visited: HashSet<String> = HashSet::new();
    let mut stack: Vec<String> = roots.iter().cloned().collect();
    while let Some(current) = stack.pop() {
        if !visited.insert(current.clone()) {
            continue;
        }
        if let Some(summary) = interproc.procedures.get(&current) {
            for callee in &summary.calls {
                if !visited.contains(callee) {
                    stack.push(callee.clone());
                }
            }
        }
    }
    visited
}

// Comment-out renderer

/// Comment every non-empty line of `text` (preserving empties as
/// `#`) and prepend an explanatory banner.
fn comment_out(text: &str, proc_name: &str) -> String {
    let mut out = format!("# [O124] Unused proc — '{proc_name}' is not called from any event\n");
    let line_count = text.split('\n').count();
    for (i, line) in text.split('\n').enumerate() {
        if line.trim().is_empty() {
            out.push('#');
        } else {
            out.push_str("# ");
            out.push_str(line);
        }
        if i + 1 < line_count {
            out.push('\n');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::interprocedural::{InterproceduralAnalysis, ProcSummary};

    fn summary_with_calls(name: &str, calls: &[&str], barrier: bool) -> ProcSummary {
        let mut s = ProcSummary::unknown(name);
        s.calls = calls.iter().map(|c| (*c).to_owned()).collect();
        s.has_barrier = barrier;
        s
    }

    fn ip_with(procs: &[(&str, &[&str], bool)]) -> InterproceduralAnalysis {
        let mut ip = InterproceduralAnalysis::default();
        for (name, calls, barrier) in procs {
            ip.procedures
                .insert((*name).into(), summary_with_calls(name, calls, *barrier));
        }
        ip
    }

    // helper tests

    #[test]
    fn library_irule_detected_when_only_rule_init_events() {
        let mut s = HashSet::new();
        assert!(is_library_irule(&s));
        s.insert("RULE_INIT".into());
        assert!(is_library_irule(&s));
        s.insert("HTTP_REQUEST".into());
        assert!(!is_library_irule(&s));
    }

    #[test]
    fn reachable_procs_follows_call_chain() {
        let ip = ip_with(&[
            ("::when::HTTP_REQUEST", &["::a"], false),
            ("::a", &["::b"], false),
            ("::b", &[], false),
            ("::unused", &[], false),
        ]);
        let roots: HashSet<String> = ["::when::HTTP_REQUEST".to_string()].into_iter().collect();
        let reached = reachable_procs(&roots, &ip);
        assert!(reached.contains("::when::HTTP_REQUEST"));
        assert!(reached.contains("::a"));
        assert!(reached.contains("::b"));
        assert!(!reached.contains("::unused"));
    }

    #[test]
    fn reachable_procs_handles_cycles() {
        let ip = ip_with(&[
            ("::when::HTTP_REQUEST", &["::a"], false),
            ("::a", &["::b"], false),
            ("::b", &["::a"], false),
        ]);
        let roots: HashSet<String> = ["::when::HTTP_REQUEST".to_string()].into_iter().collect();
        let reached = reachable_procs(&roots, &ip);
        assert_eq!(reached.len(), 3);
    }

    #[test]
    fn comment_out_prefixes_every_line() {
        let out = comment_out("proc foo {} {\n  puts hi\n}\n", "foo");
        assert!(out.starts_with("# [O124] Unused proc — 'foo'"));
        assert!(out.contains("# proc foo {} {"));
        assert!(out.contains("#   puts hi"));
        // Empty trailing line becomes `#`.
        assert!(out.ends_with('#'));
    }

    #[test]
    fn dialect_gate_rejects_non_irules() {
        // The canonical spelling, straight from the profile accessor.
        assert!(is_irules_dialect(Some(
            tcl_dialect::DialectProfile::irules()
        )));
        // And the `irules` *alias*, which only reaches the same profile by
        // going through `by_name`. Asserting `irules()` twice here would pin
        // nothing: the alias leg is the half that can actually regress.
        assert!(is_irules_dialect(Some(
            tcl_registry::model::ingress::resolve_environment("irules").analyser_profile()
        )));
        assert_eq!(
            tcl_registry::model::ingress::resolve_environment("irules")
                .analyser_profile()
                .name,
            tcl_dialect::DialectProfile::irules().name,
            "the `irules` alias must still canonicalise onto the iRules profile"
        );
        assert!(!is_irules_dialect(Some(
            tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile()
        )));
        assert!(!is_irules_dialect(None));
    }

    // end-to-end tests

    fn run_pass(
        source: &str,
        dialect: Option<&'static tcl_dialect::DialectProfile>,
        ip: InterproceduralAnalysis,
    ) -> Vec<Optimisation> {
        // `when` (and any other dialect-gated structured command)
        // is registry-resolved, so the test registry must carry the
        // dialect's command set before lowering iRule code. The shared
        // per-profile cache matches how production resolves it — and the
        // profile catalog canonicalises the `"irules"` alias, so both
        // spellings load the iRules pack exactly as the optimiser passes
        // recognise both via `is_irules_dialect`.
        let registry = tcl_registry::model::ingress::static_context_for(
            dialect.map_or("tcl", |profile| profile.name),
        )
        .commands();
        let cu = CompilationUnit::build_for(source, registry, false);
        let mut ctx = PassContext::with_dialect(&cu.source, ip, dialect);
        run(&mut ctx, &cu);
        ctx.optimisations
    }

    #[test]
    fn non_irules_dialect_produces_nothing() {
        // Even with an unused user proc, a non-irules dialect
        // yields no O124 — gated.
        let source = "proc ::foo {} { set x 1 }\n";
        let ip = ip_with(&[("::foo", &[], false)]);
        assert!(
            run_pass(
                source,
                Some(tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile()),
                ip.clone()
            )
            .is_empty()
        );
        assert!(run_pass(source, None, ip).is_empty());
    }

    #[test]
    fn library_irule_produces_nothing() {
        // Only procs + RULE_INIT → library → skip.
        let source = "proc ::helper {} { set x 1 }\nwhen RULE_INIT { set static::y 0 }\n";
        let ip = ip_with(&[("::helper", &[], false), ("::when::RULE_INIT", &[], false)]);
        let opts = run_pass(source, Some(tcl_dialect::DialectProfile::irules()), ip);
        assert!(
            opts.is_empty(),
            "library iRule should not emit O124: {opts:?}"
        );
    }

    #[test]
    fn reachable_proc_not_flagged() {
        let source = "proc ::helper {} { set x 1 }\nwhen HTTP_REQUEST { ::helper }\n";
        let ip = ip_with(&[
            ("::helper", &[], false),
            ("::when::HTTP_REQUEST", &["::helper"], false),
        ]);
        let opts = run_pass(source, Some(tcl_dialect::DialectProfile::irules()), ip);
        assert!(
            opts.is_empty(),
            "reachable proc should not be flagged: {opts:?}"
        );
    }

    #[test]
    fn barrier_in_reachable_proc_suppresses_pass() {
        let source = "proc ::a {} { eval {set x 1} }\n\
                      proc ::b {} { set y 2 }\n\
                      when HTTP_REQUEST { ::a }\n";
        let ip = ip_with(&[
            ("::a", &[], true), // has_barrier
            ("::b", &[], false),
            ("::when::HTTP_REQUEST", &["::a"], false),
        ]);
        let opts = run_pass(source, Some(tcl_dialect::DialectProfile::irules()), ip);
        assert!(
            opts.is_empty(),
            "barrier in reachable proc should suppress all O124: {opts:?}",
        );
    }

    #[test]
    fn unused_proc_reported_with_o124() {
        let source = "proc ::used {} { return 1 }\n\
                      proc ::dead {} { return 2 }\n\
                      when HTTP_REQUEST { ::used }\n";
        let ip = ip_with(&[
            ("::used", &[], false),
            ("::dead", &[], false),
            ("::when::HTTP_REQUEST", &["::used"], false),
        ]);
        let opts = run_pass(source, Some(tcl_dialect::DialectProfile::irules()), ip);
        assert_eq!(opts.len(), 1);
        let opt = &opts[0];
        assert_eq!(opt.code, DiagCode::O124);
        assert!(opt.message.contains("dead"));
        assert!(opt.replacement.contains("[O124] Unused proc"));
        assert!(opt.replacement.contains("# proc ::dead"));
    }

    #[test]
    fn irules_alias_dialect_accepted() {
        let source = "proc ::used {} { return 1 }\n\
                      proc ::dead {} { return 2 }\n\
                      when HTTP_REQUEST { ::used }\n";
        let ip = ip_with(&[
            ("::used", &[], false),
            ("::dead", &[], false),
            ("::when::HTTP_REQUEST", &["::used"], false),
        ]);
        let opts = run_pass(source, Some(tcl_dialect::DialectProfile::irules()), ip);
        assert_eq!(opts.len(), 1);
    }

    #[test]
    fn unused_procs_reported_in_qualified_name_order() {
        let source = "proc ::beta {} { return 2 }\n\
                      proc ::alpha {} { return 1 }\n\
                      proc ::gamma {} { return 3 }\n\
                      when HTTP_REQUEST { set x 0 }\n";
        let ip = ip_with(&[
            ("::alpha", &[], false),
            ("::beta", &[], false),
            ("::gamma", &[], false),
            ("::when::HTTP_REQUEST", &[], false),
        ]);
        let opts = run_pass(source, Some(tcl_dialect::DialectProfile::irules()), ip);
        assert_eq!(opts.len(), 3);
        // Results are sorted by qualified name.
        let names: Vec<&str> = opts.iter().map(|o| o.message.as_str()).collect();
        assert!(names[0].contains("alpha"));
        assert!(names[1].contains("beta"));
        assert!(names[2].contains("gamma"));
    }

    // Sanity: per-proc summaries can carry non-empty parameter
    // traits without affecting this pass (it only consults `calls`
    // + `has_barrier`).
    #[test]
    fn param_traits_ignored_by_pass() {
        let mut ip = InterproceduralAnalysis::default();
        let mut s = summary_with_calls("::foo", &[], false);
        s.param_traits.insert(
            "x".into(),
            [crate::interprocedural::ProcArgTrait::UsedInCondition]
                .into_iter()
                .collect(),
        );
        ip.procedures.insert("::foo".into(), s);
        // Force-use the HashMap import the outer module expects.
        let _scratch = HashMap::<&str, i32>::new();

        let source = "proc ::foo {x} { return $x }\nwhen HTTP_REQUEST { set x 0 }\n";
        ip.procedures.insert(
            "::when::HTTP_REQUEST".into(),
            summary_with_calls("::when::HTTP_REQUEST", &[], false),
        );
        let opts = run_pass(source, Some(tcl_dialect::DialectProfile::irules()), ip);
        assert_eq!(opts.len(), 1);
    }
}

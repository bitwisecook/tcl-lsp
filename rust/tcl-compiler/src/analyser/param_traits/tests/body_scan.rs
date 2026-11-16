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

//! The deep pass: descent through bodies and lambdas, its depth bound and the merge with the shallow pass.

use super::*;

#[test]
fn selected_lambda_source_refuses_a_foreign_document_image() {
    let source = "proc f {body} {apply {x {eval $x}} $body}";
    let analysis = crate::analyser::Analyser::new().analyse(source, "tcl9.0");
    let offset = u32::try_from(source.find("apply").expect("apply head")).unwrap();
    let changed = source.replace("$body", "$other");
    assert!(
        selected_lambda_source(
            &changed,
            &analysis,
            Some(offset),
            0,
            analysis.body_lexer_config.expect("analysed lexer config"),
        )
        .is_none()
    );
}

/// Deep-pass helper that mirrors [`infer`] for ergonomics.
fn infer_deep(params: &[&str], body: &str) -> HashMap<String, HashSet<ProcArgTrait>> {
    infer_owned(params, body, true)
}

#[test]
fn deep_pass_surfaces_eval_trait_inside_braced_body() {
    // `$body` is used inside a nested `foreach` body — the
    // shallow pass walks only top-level commands, so it
    // misses the trait.  The deep pass descends into the
    // braced `foreach` body and surfaces `Eval`.
    let body = "foreach item $items {\n  uplevel 1 $body\n}";
    let shallow = infer(&["items", "body"], body);
    let deep = infer_deep(&["items", "body"], body);
    // Shallow catches `items` (LoopList) but misses `body`.
    assert_trait(&shallow, "items", ProcArgTrait::LoopList);
    assert!(
        !shallow
            .get("body")
            .is_some_and(|s| s.contains(&ProcArgTrait::Eval)),
        "shallow pass should not surface nested Eval, got {shallow:?}",
    );
    // Deep catches both.
    assert_trait(&deep, "items", ProcArgTrait::LoopList);
    assert_trait(&deep, "body", ProcArgTrait::Eval);
}

#[test]
fn deep_pass_descends_through_multiple_levels() {
    // `$inner` is buried two levels deep: `if` → `while` →
    // `eval $inner`.  Shallow misses it; deep finds it.
    let body = "if {1} {\n  while {1} {\n    eval $inner\n  }\n}";
    let deep = infer_deep(&["inner"], body);
    assert_trait(&deep, "inner", ProcArgTrait::Eval);
}

#[test]
fn deep_pass_respects_max_depth() {
    // Build a body nested past `MAX_DEPTH` (8 levels of
    // `if {1} { ... }`) with `eval $deep_var` at the
    // innermost level.  The recursion should stop before
    // reaching the innermost level and the trait should not
    // be surfaced.  Using `MAX_DEPTH + 2` (10) levels of
    // nesting puts the eval below the recursion bound.
    let depth_to_nest = usize::from(MAX_DEPTH) + 2;
    let mut body = String::from("eval $deep_var");
    for _ in 0..depth_to_nest {
        body = format!("if {{1}} {{ {body} }}");
    }
    let deep = infer_deep(&["deep_var"], &body);
    assert!(
        !deep
            .get("deep_var")
            .is_some_and(|s| s.contains(&ProcArgTrait::Eval)),
        "MAX_DEPTH bound should keep deeply-nested eval from being surfaced, got {deep:?}",
    );
}

/// Depth coverage for the `apply` seam: `scan_deep`'s `apply`
/// (`ArgRole::LambdaLiteral`) handling must thread its own `depth + 1`
/// rather than re-enter `infer_param_traits_deep_with_config` — the
/// *public*, depth-0 entry point. Re-entering resets the *logical*
/// [`MAX_DEPTH`] counter to 0 on every `apply` boundary while the
/// *native* Rust call stack (`scan_deep` ↔
/// `infer_param_traits_deep_at_depth`) keeps growing one frame group per
/// level regardless — unboundedly, for however deep the input
/// alternates, so `MAX_DEPTH` (8) never bites.
///
/// 2000 alternating pairs (4000 real nesting levels) is far beyond
/// anything such a bypass would survive on a small-stack
/// thread; this must terminate cleanly, not hang or overflow the
/// stack — the same rationale as the big-stack helpers in
/// `analyser::commands::tests` / `lowering::tests` (`cargo test`'s
/// per-test thread has the same undersized default stack production hits).
#[test]
fn deep_pass_bounds_alternating_if_apply_nesting() {
    const PAIRS: usize = 2000;
    let mut body = String::from("puts leaf");
    for _ in 0..PAIRS {
        body = format!("if {{1}} {{ apply {{x {{ {body} }}}} 1 }}");
    }
    // With `depth` correctly threaded, an `if`/`apply` pair costs 2
    // logical levels, so real recursion stops at MAX_DEPTH (8) — a
    // handful of native frames regardless of PAIRS. This deliberately
    // runs on the *default* test-thread stack (no big-stack wrapper,
    // unlike the sibling tests in this file and in
    // `commands::tests`/`lowering::tests`): if the reset bug ever comes
    // back, real recursion runs all the way down through every one of
    // PAIRS × 2 levels again — verified locally, reverting just the
    // `depth + 1` fix below to `0` reliably overflows the stack and
    // aborts the test process outright on this same (small,
    // default-sized) thread, before the wall-clock assertion even gets
    // a chance to run. The elapsed-time check below is a second,
    // softer signal for the rare platform/build where the crash
    // threshold happens to sit a little higher.
    let registry = CommandRegistry::build_default();
    let start = std::time::Instant::now();
    let _ = infer_param_traits_deep(&["p"], &body, env_for(&registry, LexerConfig::default()));
    let elapsed = start.elapsed();
    assert!(
        elapsed < std::time::Duration::from_secs(2),
        "took {elapsed:?} — alternating if/apply nesting is no longer bounded by \
             MAX_DEPTH (reset-to-0 bypass regression?)"
    );
}

#[test]
fn deep_pass_skips_dynamic_body_args() {
    // `if {1} $body` — the body is a `$var` reference,
    // not a braced literal.  The deep pass shouldn't try
    // to descend into it (we have no body text to scan);
    // the shallow pass already surfaces the `Body` trait
    // via the registry's role scan.  This pins that
    // contract: dynamic body args don't double-count.
    let body = "if {1} $body";
    let deep = infer_deep(&["body"], body);
    assert_trait(&deep, "body", ProcArgTrait::Body);
}

#[test]
fn merge_traits_unions_shallow_and_deep() {
    let mut shallow: HashMap<String, HashSet<ProcArgTrait>> = HashMap::new();
    shallow
        .entry("p1".into())
        .or_default()
        .insert(ProcArgTrait::VarRead);
    let mut deep: HashMap<String, HashSet<ProcArgTrait>> = HashMap::new();
    deep.entry("p1".into())
        .or_default()
        .insert(ProcArgTrait::Eval);
    deep.entry("p2".into())
        .or_default()
        .insert(ProcArgTrait::Body);

    let merged = merge_traits(shallow, deep);
    // p1 gains Eval from deep without losing VarRead from shallow.
    assert!(merged.get("p1").unwrap().contains(&ProcArgTrait::VarRead));
    assert!(merged.get("p1").unwrap().contains(&ProcArgTrait::Eval));
    // p2 (deep-only) lands in the merged map.
    assert!(merged.get("p2").unwrap().contains(&ProcArgTrait::Body));
}

#[test]
fn merge_traits_with_empty_deep_returns_shallow_unchanged() {
    let mut shallow: HashMap<String, HashSet<ProcArgTrait>> = HashMap::new();
    shallow
        .entry("p1".into())
        .or_default()
        .insert(ProcArgTrait::VarWrite);
    let merged = merge_traits(shallow.clone(), HashMap::new());
    assert_eq!(merged.get("p1"), shallow.get("p1"));
}

#[test]
fn deep_pass_matches_shallow_for_top_level_only_bodies() {
    // When there are no nested bodies, the deep pass
    // should return exactly what the shallow pass does.
    let body = "set $x 1\nupvar 1 $var local";
    let shallow = infer(&["x", "var"], body);
    let deep = infer_deep(&["x", "var"], body);
    assert_eq!(shallow, deep);
}

#[test]
fn deep_pass_empty_params_returns_empty_map() {
    let deep = infer_deep(&[], "foreach item $items { uplevel 1 $body }");
    assert!(deep.is_empty());
}

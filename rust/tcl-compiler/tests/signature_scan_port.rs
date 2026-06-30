//! Port of `tests/test_signature_scan.py` — the lightweight
//! signature-only scan suite. Verifies the Rust `extract_signatures`
//! pulls proc/class/package-require/source/alias signatures out of a
//! Tcl source the same way the Python `analyser.signature_scan` did.
//!
//! These are signature-extraction STRUCTURE facts (param names/counts/
//! defaults, qualified names, what gets scanned vs skipped), not Tcl
//! runtime semantics, so assertions are made directly without a tclsh
//! proof. The one place Tcl's own arg-list parsing is leaned on
//! (`{a {b 2} args}` → 3 params, last is the varargs sentinel) is
//! cited inline.
//!
//! Field-name map Python → Rust:
//!   `result.all_procs`              -> result.procs
//!   `result.all_classes`            -> result.classes
//!   pd.params[i].`default_value=="2`" -> `Some("2".to_string())`
//!   `command_aliases`["`::x`"]=(t,e)  -> `SignatureCommandAlias`{ target, extras }
//!
//! GAPs (Python assertions with no Rust equivalent) are flagged with
//! `// GAP:` comments where they would otherwise appear.

use tcl_compiler::signature_scan::params::parse_param_list;
use tcl_compiler::signature_scan::{SignatureScanResult, extract_signatures};
use tcl_registry::CommandRegistry;

fn run(src: &str) -> SignatureScanResult {
    let registry = CommandRegistry::build_default();
    extract_signatures(src, &registry)
}

// ---------------------------------------------------------------------------
// TestProcs
// ---------------------------------------------------------------------------

#[test]
fn top_level_proc() {
    // py: test_top_level_proc
    let r = run("proc greet {name} { puts $name }");
    assert!(r.procs.contains_key("::greet"));
    let pd = &r.procs["::greet"];
    assert_eq!(pd.name, "greet");
    let param_names: Vec<&str> = pd.params.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(param_names, ["name"]);
}

#[test]
fn proc_in_namespace_eval() {
    // py: test_proc_in_namespace_eval
    let r = run("namespace eval math { proc add {a b} { return [+ $a $b] } }");
    assert!(r.procs.contains_key("::math::add"));
    assert_eq!(r.procs["::math::add"].name, "add");
}

#[test]
fn nested_namespace_eval() {
    // py: test_nested_namespace_eval
    let r = run("namespace eval a { namespace eval b { proc c {} {} } }");
    assert!(r.procs.contains_key("::a::b::c"));
}

#[test]
fn absolute_proc_name_inside_namespace() {
    // py: test_absolute_proc_name_inside_namespace
    // Absolute name (starts with ::) overrides the namespace prefix.
    let r = run("namespace eval a { proc ::foo::bar {} {} }");
    assert!(r.procs.contains_key("::foo::bar"));
    assert!(!r.procs.contains_key("::a::foo::bar"));
}

#[test]
fn proc_with_defaults_and_args() {
    // py: test_proc_with_defaults_and_args
    // tclsh: `proc f {a {b 2} args} {}; llength [info args f]` -> 3, and
    // the trailing `args` is Tcl's varargs sentinel (8.6/9.0).
    let r = run("proc f {a {b 2} args} {}");
    let pd = &r.procs["::f"];
    let param_names: Vec<&str> = pd.params.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(param_names, ["a", "b", "args"]);
    assert!(pd.params[1].has_default);
    assert_eq!(pd.params[1].default_value.as_deref(), Some("2"));
}

// ---------------------------------------------------------------------------
// TestPackageRequire
// ---------------------------------------------------------------------------

#[test]
fn package_simple_require() {
    // py: test_simple_require
    let r = run("package require Tcl 8.6");
    assert_eq!(r.package_requires.len(), 1);
    let pr = &r.package_requires[0];
    assert_eq!(pr.name, "Tcl");
    assert_eq!(pr.version.as_deref(), Some("8.6"));
}

#[test]
fn package_exact_flag() {
    // py: test_exact_flag
    let r = run("package require -exact struct::list 1.8.5");
    assert_eq!(r.package_requires.len(), 1);
    let pr = &r.package_requires[0];
    assert_eq!(pr.name, "struct::list");
    assert_eq!(pr.version.as_deref(), Some("1.8.5"));
}

#[test]
fn package_require_without_version() {
    // py: test_require_without_version
    let r = run("package require struct::list");
    assert_eq!(r.package_requires.len(), 1);
    assert_eq!(r.package_requires[0].version, None);
}

#[test]
fn package_other_subcommands_ignored() {
    // py: test_other_package_subcommands_ignored
    // package provide / package ifneeded must not produce require entries.
    let r = run("package provide foo 1.0\npackage ifneeded foo 1.0 {source foo.tcl}");
    assert!(r.package_requires.is_empty());
}

// ---------------------------------------------------------------------------
// TestSource
// ---------------------------------------------------------------------------

#[test]
fn literal_source() {
    // py: test_literal_source
    let r = run(r#"source "lib/helpers.tcl""#);
    assert_eq!(r.source_targets.len(), 1);
    assert_eq!(r.source_targets[0].raw_path, "lib/helpers.tcl");
    assert!(r.source_targets[0].is_literal);
}

#[test]
fn substituted_source_is_not_literal() {
    // py: test_substituted_source_is_not_literal
    let r = run("source $dir/helpers.tcl");
    assert_eq!(r.source_targets.len(), 1);
    assert!(!r.source_targets[0].is_literal);
}

#[test]
fn source_with_encoding_flag() {
    // py: test_source_with_encoding_flag
    let r = run("source -encoding utf-8 foo.tcl");
    assert_eq!(r.source_targets.len(), 1);
    assert_eq!(r.source_targets[0].raw_path, "foo.tcl");
}

// ---------------------------------------------------------------------------
// TestInterpAlias
// ---------------------------------------------------------------------------

#[test]
fn simple_alias() {
    // py: test_simple_alias
    let r = run("interp alias {} myset {} set");
    assert!(r.command_aliases.contains_key("::myset"));
    let alias = &r.command_aliases["::myset"];
    assert_eq!(alias.target, "set");
    assert!(alias.extras.is_empty());
}

#[test]
fn alias_with_prepended_args() {
    // py: test_alias_with_prepended_args
    let r = run("interp alias {} = {} expr double");
    let alias = &r.command_aliases["::="];
    assert_eq!(alias.target, "expr");
    assert_eq!(alias.extras, ["double"]);
}

// ---------------------------------------------------------------------------
// TestOOClass
// ---------------------------------------------------------------------------

#[test]
fn oo_class_create() {
    // py: test_oo_class_create
    let r = run("oo::class create Shape { method area {} {} }");
    assert!(r.classes.contains_key("::Shape"));
    assert_eq!(r.classes["::Shape"].name, "Shape");
}

#[test]
fn oo_class_in_namespace() {
    // py: test_oo_class_in_namespace
    let r = run("namespace eval geom { oo::class create Point { method x {} {} } }");
    assert!(r.classes.contains_key("::geom::Point"));
}

#[test]
fn fully_qualified_oo_class_head() {
    // py: test_fully_qualified_oo_class_head
    // `::oo::class` is the fully-qualified spelling of the same global
    // command, so a class declared with it must be recognised too.
    let r = run("::oo::class create ::Shape { method area {} {} }");
    assert!(r.classes.contains_key("::Shape"));
    assert_eq!(r.classes["::Shape"].name, "Shape");
}

// ---------------------------------------------------------------------------
// TestItclClass
// ---------------------------------------------------------------------------

#[test]
fn itcl_class() {
    // py: test_itcl_class
    let r = run("itcl::class Widget { method paint {} {} }");
    assert!(r.classes.contains_key("::Widget"));
}

// ---------------------------------------------------------------------------
// TestConditionalGuards — procs under if / catch / try are still indexed.
// ---------------------------------------------------------------------------

#[test]
fn proc_in_if_then_branch() {
    // py: test_proc_in_if_then_branch
    let r = run("if {$::tcl_version >= 9} { proc only9 {} {} }");
    assert!(r.procs.contains_key("::only9"));
}

#[test]
fn proc_in_if_else_branch() {
    // py: test_proc_in_if_else_branch
    let r = run("if {0} {} else { proc fallback {} {} }");
    assert!(r.procs.contains_key("::fallback"));
}

#[test]
fn proc_in_both_branches() {
    // py: test_proc_in_both_branches
    // The second definition wins (map overwrite), but it's still indexed.
    let src = r#"
        if {$::tcl_platform(platform) eq "windows"} {
            proc path_sep {} { return \; }
        } else {
            proc path_sep {} { return : }
        }
        "#;
    let r = run(src);
    assert!(r.procs.contains_key("::path_sep"));
}

#[test]
fn proc_in_elseif_branch() {
    // py: test_proc_in_elseif_branch
    let src = "if {$a} { proc a_proc {} {} } elseif {$b} { proc b_proc {} {} } else { proc c_proc {} {} }";
    let r = run(src);
    for name in ["::a_proc", "::b_proc", "::c_proc"] {
        assert!(r.procs.contains_key(name), "missing {name}");
    }
}

#[test]
fn proc_in_explicit_then() {
    // py: test_proc_in_explicit_then
    let r = run("if {1} then { proc thenproc {} {} }");
    assert!(r.procs.contains_key("::thenproc"));
}

#[test]
fn proc_in_catch_body() {
    // py: test_proc_in_catch_body
    let r = run("catch { proc might_fail {} {} }");
    assert!(r.procs.contains_key("::might_fail"));
}

#[test]
fn proc_in_try_body() {
    // py: test_proc_in_try_body
    let r = run("try { proc tried {} {} }");
    assert!(r.procs.contains_key("::tried"));
}

#[test]
fn proc_in_try_handlers_and_finally() {
    // py: test_proc_in_try_handlers_and_finally
    let src = r"
        try {
            proc main {} {}
        } on error {msg opts} {
            proc on_err {} {}
        } trap {POSIX EACCES} {msg opts} {
            proc on_eacces {} {}
        } finally {
            proc cleanup {} {}
        }
        ";
    let r = run(src);
    for name in ["::main", "::on_err", "::on_eacces", "::cleanup"] {
        assert!(r.procs.contains_key(name), "missing {name}");
    }
}

#[test]
fn conditional_proc_respects_namespace() {
    // py: test_conditional_proc_respects_namespace
    let src = r"
        namespace eval util {
            if {$::tcl_version >= 9} { proc helper {} {} }
        }
        ";
    let r = run(src);
    assert!(r.procs.contains_key("::util::helper"));
}

#[test]
fn dynamic_body_is_skipped_not_crashed() {
    // py: test_dynamic_body_is_skipped_not_crashed
    // `if` with a substituted body must not make the scanner crash, and
    // the proc that lived in the variable can't be statically recursed into.
    let r = run("set body { proc x {} {} }\nif {1} $body");
    assert!(!r.procs.contains_key("::x"));
}

// ---------------------------------------------------------------------------
// TestAbsoluteNamesInsideNamespace
// ---------------------------------------------------------------------------

#[test]
fn absolute_namespace_eval_rebases_prefix() {
    // py: test_absolute_namespace_eval_rebases_prefix
    // `namespace eval ::foo` indexes under `::foo`, not nested under the
    // enclosing namespace's prefix.
    let r = run("namespace eval a { namespace eval ::foo { proc bar {} {} } }");
    assert!(r.procs.contains_key("::foo::bar"));
    assert!(!r.procs.contains_key("::a::foo::bar"));
}

#[test]
fn absolute_oo_class_name_rebases() {
    // py: test_absolute_oo_class_name_rebases
    let r = run("namespace eval a { oo::class create ::Shape { method area {} {} } }");
    assert!(r.classes.contains_key("::Shape"));
    assert!(!r.classes.contains_key("::a::Shape"));
}

#[test]
fn absolute_itcl_class_name_rebases() {
    // py: test_absolute_itcl_class_name_rebases
    let r = run("namespace eval a { itcl::class ::Widget { method paint {} {} } }");
    assert!(r.classes.contains_key("::Widget"));
    assert!(!r.classes.contains_key("::a::Widget"));
}

// ---------------------------------------------------------------------------
// TestInterpAliasFiltering
// ---------------------------------------------------------------------------

#[test]
fn non_local_slave_alias_ignored() {
    // py: test_non_local_slave_alias_ignored
    // `interp alias slave foo {} puts` installs `foo` in a child
    // interpreter and must not appear in the workspace alias table.
    let r = run("interp alias slave foo {} puts");
    assert!(r.command_aliases.is_empty());
}

#[test]
fn non_local_target_interp_ignored() {
    // py: test_non_local_target_interp_ignored
    // Target path other than `{}` means the alias points at a command in
    // a different interpreter; skip it for the same reason.
    let r = run("interp alias {} foo slave puts");
    assert!(r.command_aliases.is_empty());
}

#[test]
fn local_alias_still_recorded() {
    // py: test_local_alias_still_recorded
    let r = run("interp alias {} myset {} set");
    assert!(r.command_aliases.contains_key("::myset"));
}

// ---------------------------------------------------------------------------
// TestConditionalPackageRequire
// ---------------------------------------------------------------------------

#[test]
fn if_body_require_marked_conditional() {
    // py: test_if_body_require_marked_conditional
    let r = run("if {$::tcl_version >= 9} { package require Tcl 9 }");
    assert_eq!(r.package_requires.len(), 1);
    assert!(r.package_requires[0].conditional);
}

#[test]
fn catch_body_require_marked_conditional() {
    // py: test_catch_body_require_marked_conditional
    let r = run("catch { package require Optional 1.0 }");
    assert_eq!(r.package_requires.len(), 1);
    assert!(r.package_requires[0].conditional);
}

#[test]
fn try_body_and_handlers_marked_conditional() {
    // py: test_try_body_and_handlers_marked_conditional
    let r = run("try { package require A 1 } on error {msg opts} { package require B 1 }");
    let mut got: Vec<(&str, bool)> = r
        .package_requires
        .iter()
        .map(|pr| (pr.name.as_str(), pr.conditional))
        .collect();
    got.sort_unstable();
    assert_eq!(got, [("A", true), ("B", true)]);
}

#[test]
fn top_level_require_not_conditional() {
    // py: test_top_level_require_not_conditional
    let r = run("package require Tcl 8.6");
    assert!(!r.package_requires[0].conditional);
}

// ---------------------------------------------------------------------------
// TestCommandInvocations
// ---------------------------------------------------------------------------

#[test]
fn invocations_populated_top_level() {
    // py: test_invocations_populated_top_level
    let r = run("puts hello\nputs world\nset x 1\n");
    let names: Vec<&str> = r
        .command_invocations
        .iter()
        .map(|inv| inv.name.as_str())
        .collect();
    assert_eq!(names.iter().filter(|n| **n == "puts").count(), 2);
    assert_eq!(names.iter().filter(|n| **n == "set").count(), 1);
}

#[test]
fn invocations_populated_inside_namespace() {
    // py: test_invocations_populated_inside_namespace
    let r = run("namespace eval util { puts hi; puts bye }");
    let names: Vec<&str> = r
        .command_invocations
        .iter()
        .map(|inv| inv.name.as_str())
        .collect();
    assert_eq!(names.iter().filter(|n| **n == "puts").count(), 2);
    // The `namespace` command itself is also an invocation.
    assert!(names.contains(&"namespace"));
}

#[test]
fn resolved_qualified_name_left_none() {
    // py: test_resolved_qualified_name_left_none
    // Signature scan intentionally skips scope resolution; downstream
    // readers must tolerate resolved_qualified_name == None.
    let r = run("puts hi");
    assert!(
        r.command_invocations
            .iter()
            .all(|inv| inv.resolved_qualified_name.is_none())
    );
}

// ---------------------------------------------------------------------------
// TestSegmenterRecovery
// ---------------------------------------------------------------------------

#[test]
fn unclosed_brace_does_not_swallow_later_procs() {
    // py: test_unclosed_brace_does_not_swallow_later_procs
    // A stray open brace in a proc body should not hide `late`: the
    // segmenter's recovery heuristic finds the next top-level command
    // and resumes there.
    let src = "proc early {} {\n    # intentionally missing close brace\n\nproc late {} {}\n";
    let r = run(src);
    assert!(r.procs.contains_key("::late"));
}

// ---------------------------------------------------------------------------
// TestAbsenceOfHeavyFields
// ---------------------------------------------------------------------------

#[test]
fn no_heavy_fields() {
    // py: test_no_diagnostics
    // The Python result carried (empty) `diagnostics`, `regex_patterns`,
    // `stub_commands`, `stub_expr_defs`, `all_variables`, and
    // `suppressed_lines` fields to prove the lightweight scan never
    // populates them. The Rust `SignatureScanResult` takes this one step
    // further: those heavy fields are not part of the struct at all (see
    // types.rs — only procs/classes/package_requires/source_targets/
    // command_aliases/namespace_imports/auto_path_entries/
    // command_invocations exist), so "they stay empty" is enforced at the
    // type level rather than by an assertion.
    //
    // GAP: SignatureScanResult has no diagnostics / regex_patterns /
    // stub_commands / stub_expr_defs / all_variables / suppressed_lines
    // fields, so the per-field `== []` / `== {}` assertions have no Rust
    // equivalent — there is nothing to assert empty because the fields
    // do not exist.
    //
    // The salvageable runtime fact is that this pathological source (an
    // empty proc name, a bare `regexp`, a bare `set`) is scanned without
    // panicking. We also pin the observed shape: the empty-name `proc`
    // still produces a record (keyed by the bare `::` prefix — it is NOT
    // dropped), and no spurious class is invented. (The Python scan kept
    // a similar record; its test only checked the heavy fields stayed
    // empty, which is the part that has no Rust counterpart.)
    let src = "proc {} {} {}\nregexp {} $x\nset";
    let r = run(src);
    assert!(r.procs.contains_key("::"));
    assert!(r.classes.is_empty());
}

// ---------------------------------------------------------------------------
// Direct parse_param_list coverage — mirrors the param-shape facts the
// pytest exercises through `proc` declarations, asserted against the
// public `parse_param_list` entry point directly.
// ---------------------------------------------------------------------------

#[test]
fn param_list_defaults_and_args() {
    // Same param shape as py test_proc_with_defaults_and_args, but
    // against the standalone parser entry point.
    let params = parse_param_list("a {b 2} args");
    let names: Vec<&str> = params.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["a", "b", "args"]);
    assert!(!params[0].has_default);
    assert!(params[1].has_default);
    assert_eq!(params[1].default_value.as_deref(), Some("2"));
    assert!(!params[2].has_default);
}

#[test]
fn param_list_empty() {
    assert!(parse_param_list("").is_empty());
    assert!(parse_param_list("   \t\n ").is_empty());
}

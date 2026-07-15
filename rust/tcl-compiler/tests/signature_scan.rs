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

//! Lightweight signature-only scan suite. Verifies `extract_signatures`
//! pulls proc/class/package-require/source/alias signatures out of a
//! Tcl source.
//!
//! These are signature-extraction STRUCTURE facts (param names/counts/
//! defaults, qualified names, what gets scanned vs skipped), not Tcl
//! runtime semantics, so assertions are made directly without a tclsh
//! proof. The one place Tcl's own arg-list parsing is leaned on
//! (`{a {b 2} args}` → 3 params, last is the varargs sentinel) is
//! cited inline.
//!
//! GAPs (assertions with no equivalent here) are flagged with
//! `// GAP:` comments where they would otherwise appear.

use tcl_compiler::signature_scan::params::parse_param_list;
use tcl_compiler::signature_scan::{SignatureScanResult, extract_signatures};
use tcl_registry::CommandRegistry;

fn run(src: &str) -> SignatureScanResult {
    let registry = CommandRegistry::build_default();
    extract_signatures(src, &registry)
}

// ---------------------------------------------------------------------------
// Procs
// ---------------------------------------------------------------------------

#[test]
fn top_level_proc() {
    let r = run("proc greet {name} { puts $name }");
    assert!(r.procs.contains_key("::greet"));
    let pd = &r.procs["::greet"];
    assert_eq!(pd.name, "greet");
    let param_names: Vec<&str> = pd.params.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(param_names, ["name"]);
}

#[test]
fn proc_in_namespace_eval() {
    let r = run("namespace eval math { proc add {a b} { return [+ $a $b] } }");
    assert!(r.procs.contains_key("::math::add"));
    assert_eq!(r.procs["::math::add"].name, "add");
}

#[test]
fn nested_namespace_eval() {
    let r = run("namespace eval a { namespace eval b { proc c {} {} } }");
    assert!(r.procs.contains_key("::a::b::c"));
}

#[test]
fn absolute_proc_name_inside_namespace() {
    // Absolute name (starts with ::) overrides the namespace prefix.
    let r = run("namespace eval a { proc ::foo::bar {} {} }");
    assert!(r.procs.contains_key("::foo::bar"));
    assert!(!r.procs.contains_key("::a::foo::bar"));
}

#[test]
fn proc_with_defaults_and_args() {
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
// package require
// ---------------------------------------------------------------------------

#[test]
fn package_simple_require() {
    let r = run("package require Tcl 8.6");
    assert_eq!(r.package_requires.len(), 1);
    let pr = &r.package_requires[0];
    assert_eq!(pr.name, "Tcl");
    assert_eq!(pr.version.as_deref(), Some("8.6"));
}

#[test]
fn package_exact_flag() {
    let r = run("package require -exact struct::list 1.8.5");
    assert_eq!(r.package_requires.len(), 1);
    let pr = &r.package_requires[0];
    assert_eq!(pr.name, "struct::list");
    assert_eq!(pr.version.as_deref(), Some("1.8.5"));
}

#[test]
fn package_require_without_version() {
    let r = run("package require struct::list");
    assert_eq!(r.package_requires.len(), 1);
    assert_eq!(r.package_requires[0].version, None);
}

#[test]
fn package_other_subcommands_ignored() {
    // package provide / package ifneeded must not produce require entries.
    let r = run("package provide foo 1.0\npackage ifneeded foo 1.0 {source foo.tcl}");
    assert!(r.package_requires.is_empty());
}

// ---------------------------------------------------------------------------
// source
// ---------------------------------------------------------------------------

#[test]
fn literal_source() {
    let r = run(r#"source "lib/helpers.tcl""#);
    assert_eq!(r.source_targets.len(), 1);
    assert_eq!(r.source_targets[0].raw_path, "lib/helpers.tcl");
    assert!(r.source_targets[0].is_literal);
}

#[test]
fn substituted_source_is_not_literal() {
    let r = run("source $dir/helpers.tcl");
    assert_eq!(r.source_targets.len(), 1);
    assert!(!r.source_targets[0].is_literal);
}

#[test]
fn source_with_encoding_flag() {
    let r = run("source -encoding utf-8 foo.tcl");
    assert_eq!(r.source_targets.len(), 1);
    assert_eq!(r.source_targets[0].raw_path, "foo.tcl");
}

// ---------------------------------------------------------------------------
// interp alias
// ---------------------------------------------------------------------------

#[test]
fn simple_alias() {
    let r = run("interp alias {} myset {} set");
    assert!(r.command_aliases.contains_key("::myset"));
    let alias = &r.command_aliases["::myset"];
    assert_eq!(alias.target, "set");
    assert!(alias.extras.is_empty());
}

#[test]
fn alias_with_prepended_args() {
    let r = run("interp alias {} = {} expr double");
    let alias = &r.command_aliases["::="];
    assert_eq!(alias.target, "expr");
    assert_eq!(alias.extras, ["double"]);
}

// ---------------------------------------------------------------------------
// rename
// ---------------------------------------------------------------------------

#[test]
fn simple_rename() {
    let r = run("rename puts my_puts");
    assert!(r.renames.contains_key("::my_puts"));
    let rn = &r.renames["::my_puts"];
    assert_eq!(rn.target, "puts");
    assert_eq!(rn.qualified_name, "::my_puts");
}

#[test]
fn rename_in_namespace_eval_is_namespace_relative() {
    // Unlike `interp alias` (always global), a bare NEW name is qualified
    // against the enclosing namespace — the same rule `proc` follows.
    let r = run("namespace eval math { rename + plus }");
    assert!(r.renames.contains_key("::math::plus"));
}

#[test]
fn rename_to_empty_string_deletes_and_is_not_recorded() {
    // `rename OLD {}` deletes OLD; no new name is introduced.
    let r = run("rename puts {}");
    assert!(r.renames.is_empty());
}

// ---------------------------------------------------------------------------
// oo::class
// ---------------------------------------------------------------------------

#[test]
fn oo_class_create() {
    let r = run("oo::class create Shape { method area {} {} }");
    assert!(r.classes.contains_key("::Shape"));
    assert_eq!(r.classes["::Shape"].name, "Shape");
}

#[test]
fn oo_class_in_namespace() {
    let r = run("namespace eval geom { oo::class create Point { method x {} {} } }");
    assert!(r.classes.contains_key("::geom::Point"));
}

#[test]
fn fully_qualified_oo_class_head() {
    // `::oo::class` is the fully-qualified spelling of the same global
    // command, so a class declared with it must be recognised too.
    let r = run("::oo::class create ::Shape { method area {} {} }");
    assert!(r.classes.contains_key("::Shape"));
    assert_eq!(r.classes["::Shape"].name, "Shape");
}

// ---------------------------------------------------------------------------
// itcl::class
// ---------------------------------------------------------------------------

#[test]
fn itcl_class() {
    let r = run("itcl::class Widget { method paint {} {} }");
    assert!(r.classes.contains_key("::Widget"));
}

// ---------------------------------------------------------------------------
// Conditional guards — procs under if / catch / try are still indexed.
// ---------------------------------------------------------------------------

#[test]
fn proc_in_if_then_branch() {
    let r = run("if {$::tcl_version >= 9} { proc only9 {} {} }");
    assert!(r.procs.contains_key("::only9"));
}

#[test]
fn proc_in_if_else_branch() {
    let r = run("if {0} {} else { proc fallback {} {} }");
    assert!(r.procs.contains_key("::fallback"));
}

#[test]
fn proc_in_both_branches() {
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
    let src = "if {$a} { proc a_proc {} {} } elseif {$b} { proc b_proc {} {} } else { proc c_proc {} {} }";
    let r = run(src);
    for name in ["::a_proc", "::b_proc", "::c_proc"] {
        assert!(r.procs.contains_key(name), "missing {name}");
    }
}

#[test]
fn proc_in_explicit_then() {
    let r = run("if {1} then { proc thenproc {} {} }");
    assert!(r.procs.contains_key("::thenproc"));
}

#[test]
fn proc_in_catch_body() {
    let r = run("catch { proc might_fail {} {} }");
    assert!(r.procs.contains_key("::might_fail"));
}

#[test]
fn proc_in_try_body() {
    let r = run("try { proc tried {} {} }");
    assert!(r.procs.contains_key("::tried"));
}

#[test]
fn proc_in_try_handlers_and_finally() {
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
    // `if` with a substituted body must not make the scanner crash, and
    // the proc that lived in the variable can't be statically recursed into.
    let r = run("set body { proc x {} {} }\nif {1} $body");
    assert!(!r.procs.contains_key("::x"));
}

// ---------------------------------------------------------------------------
// Absolute names inside namespace
// ---------------------------------------------------------------------------

#[test]
fn absolute_namespace_eval_rebases_prefix() {
    // `namespace eval ::foo` indexes under `::foo`, not nested under the
    // enclosing namespace's prefix.
    let r = run("namespace eval a { namespace eval ::foo { proc bar {} {} } }");
    assert!(r.procs.contains_key("::foo::bar"));
    assert!(!r.procs.contains_key("::a::foo::bar"));
}

#[test]
fn absolute_oo_class_name_rebases() {
    let r = run("namespace eval a { oo::class create ::Shape { method area {} {} } }");
    assert!(r.classes.contains_key("::Shape"));
    assert!(!r.classes.contains_key("::a::Shape"));
}

#[test]
fn absolute_itcl_class_name_rebases() {
    let r = run("namespace eval a { itcl::class ::Widget { method paint {} {} } }");
    assert!(r.classes.contains_key("::Widget"));
    assert!(!r.classes.contains_key("::a::Widget"));
}

// ---------------------------------------------------------------------------
// interp alias filtering
// ---------------------------------------------------------------------------

#[test]
fn non_local_slave_alias_ignored() {
    // `interp alias slave foo {} puts` installs `foo` in a child
    // interpreter and must not appear in the workspace alias table.
    let r = run("interp alias slave foo {} puts");
    assert!(r.command_aliases.is_empty());
}

#[test]
fn non_local_target_interp_ignored() {
    // Target path other than `{}` means the alias points at a command in
    // a different interpreter; skip it for the same reason.
    let r = run("interp alias {} foo slave puts");
    assert!(r.command_aliases.is_empty());
}

#[test]
fn local_alias_still_recorded() {
    let r = run("interp alias {} myset {} set");
    assert!(r.command_aliases.contains_key("::myset"));
}

// ---------------------------------------------------------------------------
// Conditional package require
// ---------------------------------------------------------------------------

#[test]
fn if_body_require_marked_conditional() {
    let r = run("if {$::tcl_version >= 9} { package require Tcl 9 }");
    assert_eq!(r.package_requires.len(), 1);
    assert!(r.package_requires[0].conditional);
}

#[test]
fn catch_body_require_marked_conditional() {
    let r = run("catch { package require Optional 1.0 }");
    assert_eq!(r.package_requires.len(), 1);
    assert!(r.package_requires[0].conditional);
}

#[test]
fn try_body_and_handlers_marked_conditional() {
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
    let r = run("package require Tcl 8.6");
    assert!(!r.package_requires[0].conditional);
}

// ---------------------------------------------------------------------------
// Command invocations
// ---------------------------------------------------------------------------

#[test]
fn invocations_populated_top_level() {
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
// Segmenter recovery
// ---------------------------------------------------------------------------

#[test]
fn unclosed_brace_does_not_swallow_later_procs() {
    // A stray open brace in a proc body should not hide `late`: the
    // segmenter's recovery heuristic finds the next top-level command
    // and resumes there.
    let src = "proc early {} {\n    # intentionally missing close brace\n\nproc late {} {}\n";
    let r = run(src);
    assert!(r.procs.contains_key("::late"));
}

// ---------------------------------------------------------------------------
// Absence of heavy fields
// ---------------------------------------------------------------------------

#[test]
fn no_heavy_fields() {
    // `SignatureScanResult` has no heavy fields — no diagnostics,
    // regex_patterns, stub_commands, stub_expr_defs, all_variables, or
    // suppressed_lines (see types.rs — only procs/classes/package_requires/
    // source_targets/command_aliases/namespace_imports/auto_path_entries/
    // command_invocations exist), so "the lightweight scan never populates
    // them" is enforced at the type level rather than by an assertion.
    //
    // The salvageable runtime fact is that this pathological source (an
    // empty proc name, a bare `regexp`, a bare `set`) is scanned without
    // panicking. We also pin the observed shape: the empty-name `proc`
    // still produces a record (keyed by the bare `::` prefix — it is NOT
    // dropped), and no spurious class is invented.
    let src = "proc {} {} {}\nregexp {} $x\nset";
    let r = run(src);
    assert!(r.procs.contains_key("::"));
    assert!(r.classes.is_empty());
}

// ---------------------------------------------------------------------------
// Direct parse_param_list coverage — the param-shape facts, asserted
// against the public `parse_param_list` entry point directly.
// ---------------------------------------------------------------------------

#[test]
fn param_list_defaults_and_args() {
    // Same param shape as the proc-with-defaults-and-args case, but
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

// ---------------------------------------------------------------------------
// Registry-driven class-definer recognition (walker `definer_family`)
// ---------------------------------------------------------------------------
//
// The walker classifies class-definer heads from registry data — a
// `definition_body` grammar plus, for TclOO, the `IS_OO_METACLASS` trait —
// never from a hardcoded name list.

/// Drift guard: the registry's recognised class-definer set must equal the
/// scanner's former hardcoded list.  A new `IS_OO_METACLASS` +
/// `definition_body` bearer (or a new snit/itcl-family definer) is
/// *supposed* to be discovered by the scanner from the day it is
/// registered — when one appears, extend the expectation here (and the
/// discovery tests below) rather than the walker.
#[test]
fn registry_definer_set_equals_former_hardcoded_list() {
    use tcl_registry::Traits;
    use tcl_registry::definer::DefinerFamily;
    let registry = CommandRegistry::build_default();
    let mut recognised: Vec<&str> = registry
        .command_names()
        .filter(|name| {
            registry.get(name).is_some_and(|spec| {
                spec.definition_body.is_some_and(|g| match g.family {
                    DefinerFamily::TclOo => spec.traits.contains(Traits::IS_OO_METACLASS),
                    DefinerFamily::Snit | DefinerFamily::Itcl => true,
                })
            })
        })
        .collect();
    recognised.sort_unstable();
    assert_eq!(
        recognised,
        [
            "itcl::class",
            "oo::abstract",
            "oo::class",
            "oo::configurable",
            "oo::singleton",
            "snit::type",
            "snit::widget",
            "snit::widgetadaptor",
        ],
        "registry class-definer set drifted from the scanner's expectations"
    );
}

#[test]
fn every_registry_metaclass_is_discovered() {
    // Each TclOO metaclass creates classes through the same
    // `METACLASS create NAME ?BODY?` interface; the scanner discovers all
    // of them via the registry, in both bare and fully-qualified
    // spellings (`get` resolves the leading `::`).
    for head in [
        "oo::class",
        "::oo::class",
        "oo::configurable",
        "::oo::configurable",
        "oo::abstract",
        "::oo::abstract",
        "oo::singleton",
        "::oo::singleton",
    ] {
        let r = run(&format!("{head} create Pin {{ method go {{}} {{}} }}"));
        assert!(
            r.classes.contains_key("::Pin"),
            "{head} create Pin must be discovered; got {:?}",
            r.classes.keys().collect::<Vec<_>>()
        );
    }
}

#[test]
fn snit_definers_are_discovered_via_registry() {
    for head in [
        "snit::type",
        "::snit::type",
        "snit::widget",
        "::snit::widget",
        "snit::widgetadaptor",
        "::snit::widgetadaptor",
    ] {
        let r = run(&format!("{head} Gadget {{ method poke {{}} {{}} }}"));
        assert!(
            r.classes.contains_key("::Gadget"),
            "{head} Gadget must be discovered; got {:?}",
            r.classes.keys().collect::<Vec<_>>()
        );
    }
}

#[test]
fn oo_object_create_makes_instances_not_classes() {
    // `oo::object` carries IS_OO_METACLASS but no `definition_body` —
    // `oo::object create x` manufactures a plain instance, so the scanner
    // must not record a class (the trait alone is not the recognition
    // rule).
    let r = run("oo::object create widgetInstance");
    assert!(
        r.classes.is_empty(),
        "oo::object creations are instances, not classes; got {:?}",
        r.classes.keys().collect::<Vec<_>>()
    );
}

#[test]
fn oo_define_and_objdefine_are_not_class_creations() {
    // The inverse guard: `oo::define` / `oo::objdefine` carry the TclOO
    // `definition_body` grammar but no IS_OO_METACLASS — they extend an
    // existing class named at arg 1, so no class record is created even
    // for the `oo::define Cls create …`-shaped word sequence.
    let r = run("oo::define create method go {} {}");
    assert!(
        r.classes.is_empty(),
        "oo::define must not be mistaken for a class creation; got {:?}",
        r.classes.keys().collect::<Vec<_>>()
    );
}

#[test]
fn oo_class_method_words_do_not_abbreviate() {
    // TclOO method dispatch is exact-match in C (`oo::class cr Foo` →
    // "unknown method cr"), unlike ensemble subcommands — the scanner
    // must not prefix-resolve the `create` word.
    let r = run("oo::class cr Foo { method go {} {} }");
    assert!(r.classes.is_empty());
}

// ---------------------------------------------------------------------------
// Ensemble subcommand words resolve through the registry (canonical names)
// ---------------------------------------------------------------------------

#[test]
fn namespace_eval_abbreviation_discovers_body_procs() {
    // C Tcl's ensemble dispatch accepts a unique prefix (`namespace ev`
    // works in tclsh 8.6); the scanner resolves the word through the same
    // registry rule, so the body is still walked.
    let r = run("namespace ev ns { proc inner {} {} }");
    assert!(
        r.procs.contains_key("::ns::inner"),
        "namespace ev body must be walked; got {:?}",
        r.procs.keys().collect::<Vec<_>>()
    );
}

#[test]
fn package_require_abbreviation_recorded() {
    // `package req Tcl` is accepted by tclsh (unique prefix of require).
    let r = run("package req Tcl 8.6");
    assert_eq!(r.package_requires.len(), 1);
    assert_eq!(r.package_requires[0].name, "Tcl");
    // An ambiguous prefix (`package pr` — prefer/present/provide) is a
    // tclsh error; nothing is recorded.
    let r = run("package pr Tcl");
    assert!(r.package_requires.is_empty());
}

#[test]
fn interp_alias_ambiguous_abbreviation_skipped() {
    // `interp al` / `interp alia` are ambiguous with `aliases` — tclsh
    // errors — so no alias is recorded; only the exact spelling resolves.
    let r = run("interp al {} myalias {} puts");
    assert!(r.command_aliases.is_empty());
    let r = run("interp alias {} myalias {} puts");
    assert!(r.command_aliases.contains_key("::myalias"));
}

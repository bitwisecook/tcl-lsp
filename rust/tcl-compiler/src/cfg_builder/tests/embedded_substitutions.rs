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

//! Effects of `[…]` substitutions in values, arguments, `return`, `switch` words and conditions.

use super::*;

#[test]
fn cfg_condition_possible_writes_keep_original_aliases_and_replacements() {
    // naming.diagnostic.original-materialized-write-footprint
    // docs/design/analysis/name-resolution-proofs/diagnostic-original-materialized-write-footprint.md
    // These are analysis-only <cond> footprints, not native stores/entry.
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
    let registry = context.commands();
    let input =
        crate::analyser::ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
    for (source, expected) in [
        (
            "if {[catch {set {café(open} VALUE} result]} {puts ${café(open}}",
            Some("café(open"),
        ),
        (
            "interp alias {} assign {} set {$literal}; if {[assign VALUE]} {puts ${$literal}}",
            Some("$literal"),
        ),
        (
            "proc catch args {}; if {[catch {set hidden VALUE} result]} {puts $hidden}",
            None,
        ),
    ] {
        let unit = crate::compilation_unit::CompilationUnit::build_with_analysis_input(
            source,
            crate::compilation_unit::UnitBuildOptions {
                registry,
                defer_top_level: false,
                config,
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            &input,
        );
        let defs = find_call_defs(&unit.top_level.cfg, "<cond>").unwrap_or_default();
        if let Some(expected) = expected {
            assert!(
                defs.iter().any(|name| name == expected),
                "{source}: {defs:?}"
            );
        } else {
            assert!(
                !defs.iter().any(|name| name == "hidden" || name == "result"),
                "known replacement: {defs:?}"
            );
        }
    }
}

#[test]
fn condition_var_writes_use_the_exact_custom_registry_roles() {
    let mut registry = native_registry();
    let mut custom = registry.get("list").expect("list spec").clone();
    custom.arg_roles = &[(0, tcl_registry::ArgRole::VarWrite)];
    custom.arg_role_resolver = None;
    registry.insert(custom);

    let module = crate::lowering::lower_to_ir("if {[list slot]} {puts yes}", &registry);
    let cfg = build_cfg_with_registry(&module, false, &registry);
    let condition_defs = cfg
        .top_level
        .blocks
        .values()
        .flat_map(|block| &block.statements)
        .find_map(|statement| match statement {
            Statement::Call { command, defs, .. } if command == "<cond>" => Some(defs),
            _ => None,
        })
        .expect("condition command substitutions produce a synthetic CFG call");
    assert!(
        condition_defs.iter().any(|name| name == "slot"),
        "custom VarWrite role was replaced by a default Tcl registry: {condition_defs:?}"
    );
}

#[test]
fn embedded_subst_in_assign_value_emits_its_word_effects() {
    // `set foo [setter]` where setter upvars caller_x.  The
    // resulting CFG should have a word-effects `<word-effects>`
    // Call with `caller_x` in its defs, emitted BEFORE the
    // `set foo ...` AssignValue.
    let module = lower_module(
        "proc setter {} { upvar 1 caller_x x; return $x }\n\
             set foo [setter]",
    );
    let cfg = build_cfg(&module, false);
    let cmd = find_call_with_def(&cfg.top_level, "caller_x")
        .expect("expected a Call carrying caller_x in defs");
    assert_eq!(cmd, "<word-effects>");
}

/// The `(defs, reads)` of the first call named `command` in `func`.
fn defs_and_reads<'a>(func: &'a Function, command: &str) -> Option<(&'a [String], &'a [String])> {
    func.blocks.values().find_map(|block| {
        block.statements.iter().find_map(|stmt| match stmt {
            Statement::Call {
                command: name,
                defs,
                reads,
                ..
            } if name == command => Some((defs.as_slice(), reads.as_slice())),
            _ => None,
        })
    })
}

/// Where a statement's `[…]` substitutions write a place its own words read,
/// the call that carries the write reads it too — the version before the
/// write feeds the word that runs ahead of it. A read inside a command the
/// statement runs, or inside the expression word of one, is a read of the
/// statement.
#[test]
fn the_call_carrying_a_write_reads_the_place_its_statement_reads() {
    for (body, carrier) in [
        ("set r [expr {$x + [set x 10] + $x}]", "<word-effects>"),
        ("expr {$x + [set x 10]}", "<word-effects>"),
        ("set r \"$x [set x 10]\"", "<word-effects>"),
        ("incr r [expr {$x + [set x 10]}]", "<word-effects>"),
        ("return [expr {$x + [set x 10]}]", "<word-effects>"),
        (
            "set r [expr {[string length $x] + [set x 10]}]",
            "<word-effects>",
        ),
        (
            "set r [expr {[expr {$x + 1}] + [set x 10]}]",
            "<word-effects>",
        ),
        ("if {$x + [set x 10] > 3} {puts a}", "<cond>"),
        ("while {$x + [set x 10] < 3} {break}", "<cond>"),
        ("puts [expr {$x + [set x 10] + $x}]", "puts"),
        ("foo $x [incr x] $x", "foo"),
    ] {
        let module = lower_module(&format!("proc p {{}} {{ set x 1; set r 5; {body} }}"));
        let cfg = build_cfg(&module, false);
        let p = fixture_id_if_present(&cfg.procedures, "::p")
            .and_then(|id| cfg.procedures.get(&id))
            .expect("::p CFG");
        let (defs, reads) = defs_and_reads(p, carrier).expect(body);
        assert!(defs.iter().any(|name| name == "x"), "{body}: {defs:?}");
        assert!(reads.iter().any(|name| name == "x"), "{body}: {reads:?}");
    }
}

/// A place no word of the statement reads is no read of the call that
/// carries its write: the store the write overwrites stays dead, as it was
/// before a statement's reads were told from its writes, and a `gets` that
/// fills a loop's variable reads nothing of it.
#[test]
fn the_call_carrying_a_write_leaves_a_place_its_statement_never_reads_alone() {
    for (body, carrier, place) in [
        ("set r [expr {[set x 10] + 1}]", "<word-effects>", "x"),
        ("puts [set x 2]", "puts", "x"),
        (
            "while {[gets $fd line] >= 0} {puts $line}",
            "<cond>",
            "line",
        ),
        ("if {[set x 10] > $r} {puts a}", "<cond>", "x"),
    ] {
        let module = lower_module(&format!(
            "proc p {{fd}} {{ set x 1; set r 5; set line {{}}; {body} }}"
        ));
        let cfg = build_cfg(&module, false);
        let p = fixture_id_if_present(&cfg.procedures, "::p")
            .and_then(|id| cfg.procedures.get(&id))
            .expect("::p CFG");
        let (defs, reads) = defs_and_reads(p, carrier).expect(body);
        assert!(defs.iter().any(|name| name == place), "{body}: {defs:?}");
        assert!(!reads.iter().any(|name| name == place), "{body}: {reads:?}");
    }
}

#[test]
fn embedded_subst_in_return_emits_its_word_effects_before_terminator() {
    let module = lower_module(
        "proc setter {} { upvar 1 caller_x x; set x 1; return 0 }\n\
             proc outer {} { return [setter] }",
    );
    let cfg = build_cfg(&module, false);
    let outer = fixture_id_if_present(&cfg.procedures, "::outer")
        .and_then(|id| cfg.procedures.get(&id))
        .expect("::outer CFG");
    assert_eq!(
        find_call_with_def(outer, "caller_x"),
        Some("<word-effects>")
    );
    assert!(
        outer
            .blocks
            .values()
            .any(|block| { matches!(block.terminator, Some(Terminator::Return { .. })) })
    );
}

#[test]
fn switch_word_substitutions_invalidate_before_dispatch() {
    let module = lower_module(
        "proc setter {name} { upvar 1 $name value; set value 1; return 0 }\n\
             proc outer {} {\n\
                 set subject 1\n\
                 set pattern 1\n\
                 switch [setter subject] [setter pattern] { return ok } default { return no }\n\
             }",
    );
    let cfg = build_cfg(&module, false);
    let outer = fixture_id_if_present(&cfg.procedures, "::outer")
        .and_then(|id| cfg.procedures.get(&id))
        .expect("::outer CFG");
    let boundary = outer
        .blocks
        .values()
        .find_map(|block| {
            block.statements.iter().find(|statement| {
                matches!(statement, Statement::Call { defs, .. }
                    if defs.iter().any(|name| name == "subject")
                        && defs.iter().any(|name| name == "pattern"))
            })
        })
        .expect("both argument writes must precede body dispatch");
    let Statement::Call {
        command, tokens, ..
    } = boundary
    else {
        unreachable!()
    };
    if command == "switch" {
        // An unresolved compiler protocol retains one runtime invocation.
        // Its argv effects occur before any entered script phase.
        let tokens = tokens.as_ref().expect("original argv carrier");
        assert!(tokens.evaluates_words());
        assert!(tokens.words()[1].sole_command_substitution().is_some());
        assert!(tokens.words()[2].sole_command_substitution().is_some());
        assert_eq!(outer.blocks.values().flat_map(|block| &block.statements)
                .filter(|statement| matches!(statement, Statement::Call {command, tokens, ..}
                    if command == "switch" && tokens.as_ref().is_none_or(crate::ir::CommandTokens::evaluates_words)))
                .count(), 1, "original invocation must execute once");
    } else {
        assert!(boundary.synthetic_marker().is_some());
        assert!(outer.blocks.values().any(|block| {
            block
                .statements
                .iter()
                .any(|statement| std::ptr::eq(statement, boundary))
                && matches!(block.terminator, Some(Terminator::Branch { .. }))
        }));
    }
}

#[test]
fn braced_switch_words_do_not_invalidate_before_dispatch() {
    let module = lower_module(
        "proc setter {name} { upvar 1 $name value; set value 1; return 0 }\n\
             proc outer {} {\n\
                 switch {[setter subject]} {[setter pattern]} { return ok }\n\
             }",
    );
    let cfg = build_cfg(&module, false);
    let outer = fixture_id_if_present(&cfg.procedures, "::outer")
        .and_then(|id| cfg.procedures.get(&id))
        .expect("::outer CFG");
    assert_eq!(find_call_with_def(outer, "subject"), None);
    assert_eq!(find_call_with_def(outer, "pattern"), None);
}

#[test]
fn embedded_subst_in_assign_expr_and_expr_eval_emits_invalidation() {
    let module = lower_module(
        "proc setter {} { upvar 1 caller_x x; set x 1; return 0 }\n\
             proc assign_expr {} { set result [expr {[setter] + 1}] }\n\
             proc expr_eval {} { expr {[setter] + 1} }",
    );
    let cfg = build_cfg(&module, false);
    for proc_name in ["::assign_expr", "::expr_eval"] {
        let function = cfg
            .procedures
            .get(&fixture_id(&module.procedures, proc_name))
            .expect("procedure CFG");
        assert_eq!(
            find_call_with_def(function, "caller_x"),
            Some("<word-effects>"),
            "missing embedded invalidation in {proc_name}"
        );
    }
}

#[test]
fn embedded_subst_in_call_arg_merges_into_call_defs() {
    // `puts [setter]` — Call host with embedded substitution.
    // The defs should merge into the existing Call's defs (no
    // word effects needed since the host is a Call).
    let module = lower_module(
        "proc setter {} { upvar 1 caller_x x; return $x }\n\
             puts [setter]",
    );
    let cfg = build_cfg(&module, false);
    let defs =
        find_call_defs(&cfg.top_level, "puts").expect("puts call should be in top-level CFG");
    assert!(
        defs.contains(&"caller_x".to_string()),
        "expected caller_x merged into puts's defs, got {defs:?}",
    );
    // No word effects should appear (the Call branch
    // merged in place).
    let synthetic = find_call_with_def(&cfg.top_level, "caller_x");
    assert_eq!(
        synthetic,
        Some("puts"),
        "embedded extras should merge into the Call host, not a synthetic",
    );
}

#[test]
fn embedded_subst_param_form_resolves_call_site_arg() {
    // setter takes a parameter `name`; its upvar source is `$name`.
    // `set foo [setter myvar]` — the embedded call passes "myvar",
    // which becomes the caller-side def.
    let module = lower_module(
        "proc setter {name} { upvar 1 $name x; set x 1 }\n\
             set foo [setter myvar]",
    );
    let cfg = build_cfg(&module, false);
    let cmd =
        find_call_with_def(&cfg.top_level, "myvar").expect("expected word effects carrying myvar");
    assert_eq!(cmd, "<word-effects>");
}

/// On tclsh 9.0.4:
///
/// ```tcl
/// proc far {name} {upvar 1 $name v; set v 2; return ok}
/// interp alias {} via {} far x
/// proc p {} {set x 1; set y [via]; return $x}
/// puts [p] ;# -> 2
/// ```
#[test]
fn embedded_alias_prefix_terminal_proc_invalidates_its_caller_target() {
    let module = lower_module(
        "proc far {name} {upvar 1 $name v; set v 2; return ok}\n\
             interp alias {} via {} far x\n\
             proc p {} {set x 1; set y [via]; return $x}",
    );
    let cfg = build_cfg(&module, false);
    let p = fixture_id_if_present(&cfg.procedures, "::p")
        .and_then(|id| cfg.procedures.get(&id))
        .expect("::p CFG");
    assert_eq!(find_call_with_def(p, "x"), Some("<word-effects>"));
}

#[test]
fn embedded_terminal_proc_honours_namespace_and_unknown_resolution() {
    let namespace_module = lower_module(
        "namespace eval ::a {\n\
                 proc mutate {} {upvar 1 x v; set v 2; return ok}\n\
                 proc p {} {set x 1; set y [mutate]; return $x}\n\
             }\n\
             namespace eval ::b {\n\
                 proc mutate {} {upvar 1 y v; set v 3; return ok}\n\
             }",
    );
    let namespace_cfg = build_cfg(&namespace_module, false);
    let p = fixture_id_if_present(&namespace_cfg.procedures, "::a::p")
        .and_then(|id| namespace_cfg.procedures.get(&id))
        .expect("::a::p CFG");
    assert_eq!(find_call_with_def(p, "x"), Some("<word-effects>"));
    assert_eq!(find_call_with_def(p, "y"), None);

    let unknown_module = lower_module(
        "proc unknown {name args} {upvar 1 $name v; set v 2; return ok}\n\
             proc p {} {set missing 1; set y [missing]; return $missing}",
    );
    let unknown_cfg = build_cfg(&unknown_module, false);
    let p = fixture_id_if_present(&unknown_cfg.procedures, "::p")
        .and_then(|id| unknown_cfg.procedures.get(&id))
        .expect("::p CFG");
    assert_eq!(find_call_with_def(p, "missing"), Some("<word-effects>"));
}

#[test]
fn embedded_subst_unknown_command_ignored() {
    // `[not_upvar]` — unknown command, should produce no
    // word effects.
    let module = lower_module("proc setter {} { set x 1 }\nset foo [setter]");
    let cfg = build_cfg(&module, false);
    // setter has no upvar, so neither direct nor embedded form
    // contributes — the only Call in the CFG should be
    // for the literal `setter` lookup (none here) or nothing.
    for block in cfg.top_level.blocks.values() {
        for stmt in &block.statements {
            if let Statement::Call { command, .. } = stmt {
                assert_ne!(
                    command, "<word-effects>",
                    "no word effects should appear for non-upvar embedded calls",
                );
            }
        }
    }
}

#[test]
fn embedded_subst_no_bracket_in_text_short_circuits() {
    // `set foo "plain string"` — value has no `[`, so the
    // embedded-substitution scan should short-circuit on the
    // `text.contains('[')` guard and produce no extras.
    let module = lower_module(
        "proc setter {} { upvar 1 caller_x x }\n\
             set foo plain",
    );
    let cfg = build_cfg(&module, false);
    let synthetic = find_call_with_def(&cfg.top_level, "caller_x");
    assert!(
        synthetic.is_none(),
        "no word effects expected when text has no `[`, got {synthetic:?}",
    );
}

#[test]
fn embedded_subst_synthetic_appears_before_host_assign() {
    // The word effects must land BEFORE the host
    // AssignValue in program order, so SSA / dataflow correctly
    // see the invalidation before any later use of the variable.
    let module = lower_module(
        "proc setter {} { upvar 1 caller_x x }\n\
             set foo [setter]",
    );
    let cfg = build_cfg(&module, false);
    let entry = &cfg.top_level.blocks[&cfg.top_level.entry];
    // Find the word effects' index and the AssignValue's index; assert
    // ordering.
    let mut synthetic_idx = None;
    let mut assign_idx = None;
    for (i, stmt) in entry.statements.iter().enumerate() {
        match stmt {
            _ if stmt.word_effects_host() == Some(crate::ir::WordEffectsHost::Statement) => {
                synthetic_idx = Some(i);
            }
            Statement::AssignValue { name, .. } if name == "foo" => {
                assign_idx = Some(i);
            }
            _ => {}
        }
    }
    let s = synthetic_idx.expect("<word-effects> should be in entry block");
    let a = assign_idx.expect("set foo AssignValue should be in entry block");
    assert!(s < a, "word effects at {s} should precede assign at {a}");
}

// A known upvar proc's call-by-name write reached
// only through a nested `[...]` (a wrapping command around it, or a
// loop/branch condition) must still be recovered — real tcllib
// `cmdline::getoptions` repro: `while {[set err [getopt argv $opts
// opt arg]]} { ... }`.

#[test]
fn embedded_subst_recovers_a_wrapped_upvar_proc_call() {
    // TP — `set x [set err [setter]]`: `setter` is nested one bracket
    // deeper than the embedded-substitution scan's own top-level word
    // check reaches (`set` itself is never a known upvar proc), so
    // recovering it requires recursing into the inner text rather than
    // pattern-matching the (bracket-stripped) word list.
    let module = lower_module(
        "proc setter {} { upvar 1 caller_x x; set x 1 }\n\
             set out [set err [setter]]",
    );
    let cfg = build_cfg(&module, false);
    let cmd = find_call_with_def(&cfg.top_level, "caller_x")
        .expect("expected word effects recovering caller_x through the wrapper");
    assert_eq!(cmd, "<word-effects>");
}

#[test]
fn condition_out_vars_recovers_catch_scan_gets_regexp_and_upvar_procs() {
    // TP — `condition_out_vars` unions the hardcoded builtin scan with
    // the upvar-proc / global-write-proc resolution; a known upvar
    // proc's write reached through a condition must appear in the
    // union exactly like a `catch` result var already does.
    let module = lower_module(
        "proc setter {} { upvar 1 caller_x x; set x 1 }\n\
             if {[setter]} { puts $caller_x }",
    );
    let cfg = build_cfg(&module, false);
    let defs = find_call_defs(&cfg.top_level, "<cond>").expect("expected a <cond> Call in the CFG");
    assert!(
        defs.contains(&"caller_x".to_string()),
        "expected caller_x in the <cond> Call's defs, got {defs:?}",
    );
}

#[test]
fn read_beside_writes_uses_original_roles_and_preserves_refusals() {
    for (source, expected) in [
        ("puts [set x 2] [info exists x]", vec!["x".to_owned()]),
        ("puts [set x 2]", Vec::new()),
    ] {
        let module = lower_module(source);
        let registry = module.resolved_registry();
        let prepared = prepare_cfg_context_bundle(&module, registry);
        let builder = CfgBuilder::new_with_upvars_and_classes(
            false,
            prepared.context.0.clone(),
            prepared.context.1.clone(),
            prepared.context.2.clone(),
            prepared.context.3.clone(),
            registry,
            prepared.command_classes.clone(),
        )
        .with_procedure_summary_owners(prepared.context.4.clone());
        let statement = module.top_level.statements.last().unwrap();
        let tokens = module
            .top_level
            .retained_source_tokens_for_statement(statement)
            .unwrap();
        let written = vec!["x".to_owned(), "unrelated".to_owned()];
        let reads = builder.read_beside_writes(&written, Some(tokens), |scanner| {
            scanner.scan_lexical_word("", registry)
        });
        assert_eq!(reads, expected, "{source}");
        assert_eq!(
            builder.read_beside_writes(&written, None, |scanner| {
                scanner.scan_lexical_word("", registry)
            }),
            written
        );
        let mut changed = tokens.clone();
        changed.source_binding = None;
        assert_eq!(
            builder.read_beside_writes(&written, Some(&changed), |scanner| {
                scanner.scan_lexical_word("", registry)
            }),
            written
        );
    }
}

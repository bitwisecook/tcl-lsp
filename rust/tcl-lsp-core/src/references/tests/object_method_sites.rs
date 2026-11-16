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

//! External `$obj method` and class-command call sites.

use super::*;

#[test]
fn class_method_references_require_the_original_temporal_entry() {
    let source = "oo::class create C {self method ping {} {return CLASS}}; C ping";
    let sites = retained_method_sites(source, true);
    assert_eq!(sites.len(), 1, "{sites:?}");
    assert_eq!(sites[0].start() as usize, source.rfind("ping").unwrap());
}

#[test]
fn class_method_references_refuse_replacement_during_arguments() {
    for source in [
        "oo::class create C {self method ping {x} {return CLASS}}; C ping [oo::objdefine C method ping {x} {return REPLACED}]",
        "oo::class create C {self method ping {} {return CLASS}}; rename C old; proc C args {return DECOY}; C ping",
    ] {
        assert!(retained_method_sites(source, true).is_empty(), "{source}");
    }
}

#[test]
fn instance_method_references_require_the_retained_head_object() {
    let source =
        "oo::class create C {method ping {} {return INSTANCE}}; set obj [C new]; $obj ping";
    let sites = retained_method_sites(source, false);
    assert_eq!(sites.len(), 1, "{sites:?}");
    assert_eq!(sites[0].start() as usize, source.rfind("ping").unwrap());
}

#[test]
fn instance_method_references_refuse_mutated_dispatch_and_nominal_decoys() {
    for source in [
        "oo::class create C {method ping {x} {return INSTANCE}}; set obj [C new]; $obj ping [oo::define C method ping {x} {return REPLACED}]",
        "oo::class create C {method ping {} {return INSTANCE}}; set obj [C new]; set obj ::puts; $obj ping",
    ] {
        assert!(retained_method_sites(source, false).is_empty(), "{source}");
    }
}

#[test]
fn references_reach_a_method_return_captured_dispatch_site() {
    // `b` is typed only by the object-type lattice (`set b [$a make]`, the
    // method-return edge) — the analyser's `instance_classes` never binds
    // it, so only the lattice reading lets Find References reach the `$b
    // greet` site that semantic tokens and hover resolve.
    let src = "oo::class create A { method make {} { ::return [::B new] } }\n\
                   oo::class create B { method greet {} { ::return \"hi\" } }\n\
                   set a [A new]\n\
                   set b [$a make]\n\
                   $b greet\n";
    let analysis = analyse(src);
    assert!(
        !analysis.instance_classes.contains_key("b"),
        "premise: the analyser walk alone must not bind `b` \
             (instance_classes: {:?})",
        analysis.instance_classes
    );
    // Cursor on the `greet` declaration (line 1, col 29).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        29,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(
        lines.contains(&4),
        "the lattice-typed `$b greet` call site is missing: {refs:?}"
    );
    // …and from the call site itself, the declaration answers back.
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        4,
        4,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(
        lines.contains(&1),
        "references from the call site must reach the declaration: {refs:?}"
    );
}

#[test]
fn references_do_not_resolve_an_unclosed_braced_instance_head() {
    let src = "oo::class create Dog {\n    method bark {} {}\n}\nset x [Dog new]\n${x bark\n";
    let analysis = analyse(src);
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        &analysis,
        true,
    );
    assert_eq!(
        refs.len(),
        1,
        "only the declaration may resolve; malformed `${{x` is not `x`: {refs:?}"
    );
    assert_eq!(refs[0].start_line, 1);
    assert_eq!(strip_var_decoration("${x}", None), Some("x"));
    assert_eq!(strip_var_decoration("$x", None), Some("x"));
    assert_eq!(strip_var_decoration("$arr(idx)", None), Some("arr(idx)"));
    assert_eq!(strip_var_decoration("$::a:::b", None), Some("::a:::b"));
    assert_eq!(strip_var_decoration("$foo:::", None), Some("foo:::"));
    assert_eq!(strip_var_decoration("${x", None), Some("{x"));
    assert_eq!(strip_var_decoration("x", None), None);
}

// external $obj method sites

#[test]
fn references_from_external_obj_method_site() {
    // Declaration + 2 external call sites (`$d bark`,
    // `[$d bark]`).
    let src = "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\n$d bark\nputs [$d bark]\n";
    let analysis = analyse(src);
    // Cursor on `bark` in `$d bark` (line 4, col 3).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        4,
        3,
        &analysis,
        true,
    );
    // Declaration (line 1) + two external sites (lines 4, 5).
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(lines.contains(&1), "decl missing: {refs:?}");
    assert!(lines.contains(&4), "line-4 call missing: {refs:?}");
    assert!(lines.contains(&5), "line-5 call missing: {refs:?}");
}

#[test]
fn references_from_inside_class_includes_external_sites() {
    // Cursor on the declaration; refs include the external
    // `$d bark` site as well as the declaration.
    let src = "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\n$d bark\n";
    let analysis = analyse(src);
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(lines.contains(&1), "decl missing: {refs:?}");
    assert!(lines.contains(&4), "external call missing: {refs:?}");
}

#[test]
fn find_obj_method_call_sites_covers_top_level_and_subst() {
    let src = "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\n$d bark\nputs [$d bark]\n";
    let analysis = analyse(src);
    let sites = find_obj_method_call_sites(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        &analysis,
        "::Dog",
        "bark",
        false,
    );
    // Two external sites: `$d bark` and `[$d bark]`.
    assert_eq!(sites.len(), 2, "{sites:?}");
}

#[test]
fn find_obj_method_call_sites_finds_calls_in_proc_body() {
    let src = "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\nproc f {} { global d; $d bark }\n";
    let analysis = analyse(src);
    let sites = find_obj_method_call_sites(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        &analysis,
        "::Dog",
        "bark",
        false,
    );
    assert_eq!(sites.len(), 1, "{sites:?}");
}

#[test]
fn find_obj_method_call_sites_matches_bare_created_instance_command() {
    // `Dog create rex` binds `rex` as an object *command*; `rex bark` is a
    // bare-word method dispatch (not `$rex bark`).  The scan resolves it
    // through `created_instance_commands`.
    let src = "oo::class create Dog {\n    method bark {} {}\n}\nDog create rex\nrex bark\n";
    let analysis = analyse(src);
    let sites = find_obj_method_call_sites(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        &analysis,
        "::Dog",
        "bark",
        false,
    );
    assert_eq!(sites.len(), 1, "{sites:?}");
    // The matched span is the `bark` method-name token of `rex bark`.
    let s = sites[0];
    assert_eq!(
        &src[s.start() as usize..s.end() as usize],
        "bark",
        "{sites:?}"
    );
}

// class-command dispatch: `CLASS method` for a
// classmethod / `self method`, a receiver set entirely separate from
// `$obj method` / `NAME method` instance dispatch above.

#[test]
fn find_obj_method_call_sites_matches_class_command_and_inheriting_subclass() {
    // TP — both the plain shape (`ActiveRecord find`) and its
    // inherited-via-superclass sibling (`Table find`, ooutil's
    // `classmethod` propagates to a subclass's own bound command).
    let src = "oo::class create ActiveRecord {\n    classmethod find {args} { return \"found $args\" }\n}\noo::class create Table {\n    superclass ActiveRecord\n}\nTable find foo bar\nActiveRecord find foo bar\n";
    let analysis = Analyser::new().analyse(src, "tcl9.1").clone();
    let sites = find_obj_method_call_sites(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.1").analyser_profile(),
        &analysis,
        "::ActiveRecord",
        "find",
        true,
    );
    assert_eq!(sites.len(), 2, "{sites:?}");
    for s in &sites {
        assert_eq!(
            &src[s.start() as usize..s.end() as usize],
            "find",
            "{sites:?}"
        );
    }
}

/// TP — a bare class-command dispatch written inside an `apply` lambda
/// body or a `namespace eval` body — at the top level or nested inside a
/// method — is a real call.
/// All three shapes were confirmed dispatching under tclsh 9.0.4
/// (`MAKE CALLED` printed three times).
#[test]
fn find_obj_method_call_sites_reaches_lambda_and_namespace_eval_bodies() {
    let src = "oo::class create Factory {\n\
                       classmethod make {} { return 1 }\n\
                       method inst {} { apply {{} { Factory make }} }\n\
                       method nsev {} { namespace eval ::zz { Factory make } }\n\
                   }\n\
                   namespace eval ::top2 { Factory make }\n\
                   Factory create obj\n\
                   obj inst\n\
                   obj nsev\n";
    let analysis = Analyser::new().analyse(src, "tcl9.1").clone();
    let sites = find_obj_method_call_sites(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.1").analyser_profile(),
        &analysis,
        "::Factory",
        "make",
        true,
    );
    assert_eq!(sites.len(), 3, "{sites:?}");
    for s in &sites {
        assert_eq!(
            &src[s.start() as usize..s.end() as usize],
            "make",
            "{sites:?}"
        );
    }
}

/// TN — the instance half must **not** follow the class-command half
/// through a frame shift.  `$f` inside `namespace eval ::zz` names
/// `::zz::f`, and inside an `apply` lambda a fresh local, so neither is
/// a dispatch on the outer `f` (tclsh 9.0.4: both raise `can't read
/// "f": no such variable`).
#[test]
fn find_obj_method_call_sites_excludes_var_receivers_across_a_frame_shift() {
    // A `namespace eval` body reads an unqualified variable from the
    // global namespace when the namespace has none on 8.6 but not on
    // 9.0/9.1; `apply` opens a fresh local frame on every release. The
    // errors are caught so the last line runs: tclsh 8.6 prints `BARK`
    // twice, 9.0 and 9.1 once. Uncaught, 8.6 stops at `apply` and 9.0/9.1
    // at `namespace eval`, and the last line never dispatches.
    let src = "oo::class create Dog {\n\
                       method bark {} {}\n\
                   }\n\
                   set f [Dog new]\n\
                   catch {namespace eval ::zz { $f bark }}\n\
                   catch {apply {{} { $f bark }}}\n\
                   $f bark\n";
    for (dialect, lines) in [
        ("tcl8.6", vec![5, 7]),
        ("tcl9.0", vec![7]),
        ("tcl9.1", vec![7]),
    ] {
        let analysis = Analyser::new().analyse(src, dialect);
        let sites = find_obj_method_call_sites(
            src,
            tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile(),
            &analysis,
            "::Dog",
            "bark",
            false,
        );
        let found = sites
            .iter()
            .map(|site| src[..site.start() as usize].lines().count())
            .collect::<Vec<_>>();
        assert_eq!(found, lines, "{dialect}: {sites:?}");
    }
}

/// TN — a bare (backslash-escaped) `apply` body element is decoded
/// before `apply` evaluates it, so its source slice is not the script
/// that runs; the scan must not re-parse it in place.
#[test]
fn find_obj_method_call_sites_skips_escaped_lambda_body_element() {
    let src = "oo::class create Factory {\n\
                       classmethod make {} { return 1 }\n\
                   }\n\
                   apply {{} Factory\\ make}\n";
    let analysis = Analyser::new().analyse(src, "tcl9.1").clone();
    let sites = find_obj_method_call_sites(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.1").analyser_profile(),
        &analysis,
        "::Factory",
        "make",
        true,
    );
    assert!(sites.is_empty(), "{sites:?}");
}

#[test]
fn find_obj_method_call_sites_excludes_non_inheriting_self_method_subclass() {
    // TN — the references-level precision guard mirroring
    // `self_method_not_inherited_by_a_non_overriding_subclass` in
    // definition.rs: unlike `ooutil`'s `classmethod`, a plain `self
    // method` is not inherited, so `Gadget make` must not be counted
    // as a call site of `Widget`'s `make`.
    let src = "oo::class create Widget {\n    self method make {n} { return \"made $n\" }\n}\noo::class create Gadget {\n    superclass Widget\n}\nWidget make foo\nGadget make foo\n";
    let analysis = analyse(src);
    let sites = find_obj_method_call_sites(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        &analysis,
        "::Widget",
        "make",
        true,
    );
    assert_eq!(sites.len(), 1, "{sites:?}");
    assert_eq!(
        &src[sites[0].start() as usize..sites[0].end() as usize],
        "make",
        "{sites:?}"
    );
}

#[test]
fn references_enumerates_class_command_declaration_and_both_call_sites() {
    // TP — the full end-to-end peek from the finding's own repro:
    // declaration, the inherited-subclass call, and the
    // declaring-class's own call — three references, no duplicates,
    // none missed (requires Part 3 in addition to Part 2: Part 2 alone
    // only fixes single-cursor lookups, not this whole-document scan).
    let src = "oo::class create ActiveRecord {\n    classmethod find {args} { return \"found $args\" }\n}\noo::class create Table {\n    superclass ActiveRecord\n}\nTable find foo bar\nActiveRecord find foo bar\n";
    let analysis = Analyser::new().analyse(src, "tcl9.1").clone();
    // Cursor on the declaration (line 1, `find` at col 16).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.1").analyser_profile(),
        1,
        16,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert_eq!(refs.len(), 3, "{refs:?}");
    assert!(lines.contains(&1), "decl missing: {refs:?}");
    assert!(lines.contains(&6), "Table find call missing: {refs:?}");
    assert!(
        lines.contains(&7),
        "ActiveRecord find call missing: {refs:?}"
    );
}

#[test]
fn references_from_cursor_on_class_command_call_site() {
    // Symmetry with `references_from_cursor_on_bare_obj_command_call_site`:
    // invoking Find All References with the cursor ON the class-command
    // call site (not the declaration) must resolve identically.
    let src = "oo::class create ActiveRecord {\n    classmethod find {args} { return \"found $args\" }\n}\nActiveRecord find foo bar\n";
    let analysis = Analyser::new().analyse(src, "tcl9.1").clone();
    // Cursor on `find` in `ActiveRecord find foo bar` (line 3, col 13).
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl9.1").analyser_profile(),
        3,
        13,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(lines.contains(&1), "decl missing: {refs:?}");
    assert!(lines.contains(&3), "call site missing: {refs:?}");
}

#[test]
fn references_include_bare_created_instance_command_site() {
    // Full peek: cursor on the `method bark` decl surfaces the bare
    // `rex bark` dispatch as a reference.
    let src = "oo::class create Dog {\n    method bark {} {}\n}\nDog create rex\nrex bark\n";
    let analysis = analyse(src);
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        1,
        11,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(lines.contains(&1), "decl missing: {refs:?}");
    assert!(lines.contains(&4), "bare `rex bark` site missing: {refs:?}");
}

#[test]
fn references_from_cursor_on_bare_obj_command_call_site() {
    // Symmetry: invoking Find All References with the cursor
    // ON the `bark` token of a bare `rex bark` dispatch must resolve — not
    // only the declaration-based peek.  `rex` is at col 0, `bark` at col 4.
    let src = "oo::class create Dog {\n    method bark {} {}\n}\nDog create rex\nrex bark\n";
    let analysis = analyse(src);
    let refs = references(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        4,
        4,
        &analysis,
        true,
    );
    let lines: Vec<u32> = refs.iter().map(|r| r.start_line).collect();
    assert!(lines.contains(&1), "decl missing: {refs:?}");
    assert!(lines.contains(&4), "call site missing: {refs:?}");
}

#[test]
fn bare_set_var_receiver_is_not_matched_without_dollar() {
    // FP guard: `set d [Dog new]` binds `d` as a *variable*, not a
    // command.  A bare `d bark` (no `$`) is NOT a valid dispatch in Tcl,
    // so it must not be matched — only `$d bark` counts.
    let src = "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\nd bark\n";
    let analysis = analyse(src);
    let sites = find_obj_method_call_sites(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        &analysis,
        "::Dog",
        "bark",
        false,
    );
    assert!(
        sites.is_empty(),
        "bare var receiver wrongly matched: {sites:?}"
    );
}

#[test]
fn method_source_dispatch_keeps_actual_store_availability_and_lookup_barriers() {
    // naming.core.original-dispatch-region-context
    // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
    // Conditional source reference geometry, not a live receiver or method entry.
    use std::sync::Arc;
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
    let mut registry = tcl_registry::CommandRegistry::build_default();
    registry.insert(tcl_registry::CommandSpec {
        name: "source_dispatch",
        arity: tcl_registry::Arity::at_least(1),
        traits: tcl_registry::Traits::TCLOO_SELF_DISPATCH,
        surface: registry.get("dict").unwrap().surface,
        ..tcl_registry::CommandSpec::DEFAULT
    });
    let store = Arc::new(registry);
    let source = "source_dispatch selected";
    for (release, expected) in [("tcl8.6", 1), ("tcl8.4", 0)] {
        let context = Arc::new(
            tcl_registry::model::ingress::static_context_for(release)
                .with_command_store(Arc::clone(&store)),
        );
        let input =
            tcl_compiler::analyser::ResolvedAnalysisInput::new(profile, profile, context, config);
        let mut analysis = Analyser::new()
            .with_resolved_input(input)
            .analyse(source, profile.name);
        let body = tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap());
        assert_eq!(
            scan_my_method_sites(source, &analysis, &[body], "selected", None).len(),
            expected
        );
        analysis.body_lexer_config.as_mut().unwrap().expand_syntax = !config.expand_syntax;
        assert_eq!(
            scan_my_method_sites(source, &analysis, &[body], "selected", None),
            [] as [tcl_lexer::Span; 0]
        );
        analysis.body_lexer_config = Some(config);
        let foreign =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
        analysis.resolved_input = Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
            profile, profile, foreign, config,
        ));
        assert_eq!(
            scan_my_method_sites(source, &analysis, &[body], "selected", None),
            [] as [tcl_lexer::Span; 0]
        );
        analysis.resolved_input = None;
        assert_eq!(
            scan_my_method_sites(source, &analysis, &[body], "selected", None),
            [] as [tcl_lexer::Span; 0]
        );
    }
    let source = "proc source_dispatch args {}; source_dispatch selected";
    let context = Arc::new(
        tcl_registry::model::ingress::static_context_for("tcl8.6").with_command_store(store),
    );
    let input =
        tcl_compiler::analyser::ResolvedAnalysisInput::new(profile, profile, context, config);
    let analysis = Analyser::new()
        .with_resolved_input(input)
        .analyse(source, profile.name);
    let body = tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap());
    assert_eq!(
        scan_my_method_sites(source, &analysis, &[body], "selected", None),
        [] as [tcl_lexer::Span; 0]
    );
}

#[test]
fn stale_instance_candidate_does_not_add_a_variable_method_reference() {
    let source = "oo::class create C {method run {} {return 1}}\nset receiver 0\n$receiver run\n";
    let mut analysis = analyse(source);
    analysis
        .instance_classes
        .insert("receiver".to_owned(), "::C".to_owned());
    let calls = find_obj_method_call_sites(
        source,
        tcl_registry::model::ingress::resolve_environment("tcl").analyser_profile(),
        &analysis,
        "::C",
        "run",
        false,
    );
    assert!(
        calls.is_empty(),
        "a filewide candidate is not an actual object read: {calls:?}"
    );
}

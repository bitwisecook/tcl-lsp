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

//! Method dispatch: external visibility masks, class-member lookup, `$obj method` and class-command calls.

use super::*;

#[test]
fn original_external_visibility_keeps_variable_and_object_names_separate_from_self_dispatch() {
    // naming.core.original-dispatch-region-context
    // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
    for (construction, receiver) in [
        ("C create my", "my"),
        ("set my [C new]", "$my"),
        ("C create {[self]}", "{[self]}"),
    ] {
        for (method, expected) in [
            ("Hidden", ExternalObjectDispatchMask::Native),
            ("visible", ExternalObjectDispatchMask::Clear),
        ] {
            let source = format!(
                "oo::class create C {{}}; {construction}; oo::objdefine {receiver} {{method {method} {{}} {{}}}}; {receiver} {method}"
            );
            let mut analysis = Analyser::new().analyse(&source, "tcl8.6");
            assert!(
                analysis.original_object_configurations().next().is_some(),
                "{source}"
            );
            analysis.object_member_state.clear();
            analysis.dialect = "presentation label".into();
            assert_eq!(
                external_mask_at(&source, &analysis, method),
                expected,
                "{source}"
            );
            assert_eq!(expected.withholds_workspace_fallback(), method == "Hidden");
        }
    }
}

#[test]
fn original_internal_visibility_uses_the_selected_receiver_local_head() {
    // naming.core.original-dispatch-region-context
    // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
    let source = "proc my args {return DECOY}; oo::class create C {method Hidden {} {}; method run {} {my Hidden}}; C create obj; obj run";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let offset = u32::try_from(source.find("my Hidden").unwrap()).unwrap();
    let selected = analysis
        .retained_command_realm()
        .unwrap()
        .invocation_at_source("my", offset);
    assert!(
        selected
            .receiver_self_method_entry(analysis.resolved_registry().unwrap())
            .is_some()
    );
    assert_eq!(
        external_mask_at(source, &analysis, "Hidden}"),
        ExternalObjectDispatchMask::Clear
    );
    assert!(!external_mask_at(source, &analysis, "Hidden}").withholds_workspace_fallback());
}

#[test]
fn original_external_self_visibility_requires_its_own_entry_after_internal_dispatch() {
    // naming.core.original-dispatch-region-context
    // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
    // Measured EXTERNAL_SELF_UNEXPORTED in native_method_visibility/v2
    // rejects on C8.6/C9. UNEXPORT_FULL_INTERNAL uses actual my dispatch.
    // C9 EXTERNAL_SELF_PRIVATE is a separate declaring-class scope rule,
    // not evidence that an unexported entry is externally callable.
    for dialect in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        for (head, method, expected) in [
            ("my", "Hidden", ExternalObjectDispatchMask::Clear),
            ("[self]", "Hidden", ExternalObjectDispatchMask::Native),
            ("[self]", "visible", ExternalObjectDispatchMask::Clear),
        ] {
            let source = format!(
                "oo::class create C {{method {method} {{}} {{}}; method invoke {{}} {{{head} {method}}}}}; C create c; c invoke"
            );
            let analysis = Analyser::new().analyse(&source, dialect);
            let call = format!("{head} {method}");
            let offset = u32::try_from(source.find(&call).unwrap()).unwrap();
            let binding = analysis
                .retained_command_realm()
                .unwrap()
                .invocation_at_source(head, offset);
            if head == "my" {
                assert!(
                    binding
                        .receiver_self_method_entry(analysis.resolved_registry().unwrap())
                        .is_some()
                );
            } else {
                let selected = tcl_compiler::segmenter::segment_commands_with_offset_and_config(
                    &call,
                    offset,
                    analysis.body_lexer_config.unwrap(),
                )
                .into_iter()
                .next()
                .unwrap();
                let tokens = tcl_compiler::ir::CommandTokens::from_segmented(
                    &tcl_lexer::SourceMap::new(&source),
                    analysis.body_lexer_config.unwrap(),
                    &selected,
                );
                let (_, entry, _) = binding
                    .frozen_object_receiver_method_entry(tokens.word_exprs.first().unwrap())
                    .expect("the reached self head retains an independent external method entry");
                assert_eq!(entry.is_exported(), method == "visible");
            }
            assert_eq!(
                external_mask_at(&source, &analysis, &format!("{method}}}")),
                expected,
                "{dialect}: {source}"
            );
        }
        let deferred = "oo::class create C {method Hidden {} {}; method invoke {} {[self] Hidden}}";
        let analysis = Analyser::new().analyse(deferred, dialect);
        assert_eq!(
            external_mask_at(deferred, &analysis, "Hidden}}"),
            ExternalObjectDispatchMask::Unavailable,
            "source self identity cannot donate a missing actual method entry"
        );
    }
}

#[test]
fn original_source_visibility_keeps_full_lexer_input_and_conditional_mask_purpose() {
    // naming.core.original-dispatch-region-context
    // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
    let source = "\u{feff}oo::class create C {method m {} {}}; set my [C new]; oo::objdefine $my {unexport m}; $my m";
    let profile = tcl_dialect::DialectProfile::plain_tcl();
    let context =
        tcl_registry::model::ingress::resolve_environment("tcl8.6").default_context_registry();
    let config = tcl_lexer::LexerConfig {
        leading_bom: tcl_lexer::LeadingBom::Skip,
        ..tcl_lexer::LexerConfig::for_profile(Some(profile))
    };
    let input =
        tcl_compiler::analyser::ResolvedAnalysisInput::new(profile, profile, context, config);
    let mut analysis = Analyser::new()
        .with_resolved_input(input.clone())
        .analyse(source, "presentation label");
    assert_eq!(analysis.body_lexer_config, Some(config));
    assert!(
        analysis
            .object_member_state
            .get("my")
            .is_some_and(|states| { states.iter().any(|state| state.unexports.contains("m")) })
    );
    assert!(
        analysis.original_object_configurations().next().is_none(),
        "source advice cannot invent a Native allocation"
    );
    assert_eq!(
        external_mask_at(source, &analysis, "m"),
        ExternalObjectDispatchMask::ConditionalSource
    );
    analysis.dialect = "jim".into();
    assert_eq!(
        external_mask_at(source, &analysis, "m"),
        ExternalObjectDispatchMask::ConditionalSource
    );
    let other_namespace = "oo::class create C {method m {} {}}; set my [C new]; oo::objdefine $my {unexport m}; namespace eval other {set my [C new]; $my m}";
    let other = Analyser::new()
        .with_resolved_input(input)
        .analyse(other_namespace, "presentation label");
    assert!(other.object_member_state.contains_key("my"));
    let mask = external_mask_at(other_namespace, &other, "m}");
    assert_eq!(mask, ExternalObjectDispatchMask::Unavailable);
    assert!(mask.withholds_workspace_fallback());
}

#[test]
fn original_external_visibility_withholds_populated_unavailable_stale_and_foreign_input() {
    // naming.core.original-dispatch-region-context
    // docs/design/analysis/name-resolution-proofs/core-original-dispatch-region-context.md
    let source = "oo::class create C {}; C create obj; oo::objdefine obj {method visible {} {return VISIBLE}}; obj visible";
    let baseline = Analyser::new().analyse(source, "tcl8.6");
    assert!(baseline.original_object_configurations().next().is_some());
    assert_eq!(
        external_mask_at(source, &baseline, "visible"),
        ExternalObjectDispatchMask::Clear
    );
    let input = baseline.resolved_input.as_ref().unwrap();
    let older =
        tcl_registry::model::ingress::resolve_environment("tcl8.4").default_context_registry();
    let older_same_store = std::sync::Arc::new(
        older.with_command_store(
            input
                .context_registry()
                .commands()
                .snapshot()
                .shared_registry(),
        ),
    );
    for unavailable in vec![
        {
            let mut next = baseline.clone();
            next.resolved_input = None;
            next
        },
        {
            let mut next = baseline.clone();
            next.analysis_context_unavailable = Some(tcl_registry::model::OverlayMiss {
                environment: "tcl8.6".into(),
                overlay: 999,
            });
            next
        },
        {
            let mut next = baseline.clone();
            next.body_lexer_config.as_mut().unwrap().strict_quoting =
                !input.lexer_config().strict_quoting;
            next
        },
        {
            let mut next = baseline.clone();
            next.resolved_input = Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
                input.analyser_profile(),
                input.unit_profile(),
                older_same_store,
                input.lexer_config(),
            ));
            next
        },
        {
            let mut next = baseline.clone();
            next.resolved_input = Some(tcl_compiler::analyser::ResolvedAnalysisInput::new(
                input.analyser_profile(),
                input.unit_profile(),
                older,
                input.lexer_config(),
            ));
            next
        },
    ]
    .into_boxed_slice()
    {
        assert!(
            unavailable
                .original_object_configurations()
                .next()
                .is_some()
        );
        let mask = external_mask_at(source, &unavailable, "visible");
        assert_eq!(mask, ExternalObjectDispatchMask::Unavailable);
        assert!(mask.withholds_workspace_fallback());
    }
    let changed = source.replace("return VISIBLE", "return CHANGED");
    let mask = external_mask_at(&changed, &baseline, "visible");
    assert_eq!(mask, ExternalObjectDispatchMask::Unavailable);
    assert!(mask.withholds_workspace_fallback());
}

#[test]
fn definition_resolves_a_method_return_captured_dispatch() {
    // `instance_classes` never binds `b` from a method return, so
    // go-to-definition on the consuming `$b greet` has only the lattice's
    // scope-keyed singleton to resolve through — the same accessor every
    // dispatch consumer reads.
    let src = "oo::class create A { method make {} { ::return [::B new] } }\n\
                   oo::class create B { method greet {} { ::return \"hi\" } }\n\
                   set a [A new]\n\
                   set b [$a make]\n\
                   $b greet\n";
    let analysis = analyse(src);
    // Cursor on `greet` at the `$b greet` call site (line 4, col 4).
    let locs = definition(src, 4, 4, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(
        locs[0].start_line, 1,
        "must resolve to ::B::greet's declaration: {locs:?}"
    );
}

// class-member lookup

#[test]
fn definition_bare_sibling_method_call_without_link_abstains() {
    // FP — a bareword sibling method call is NOT
    // actually reachable from another method's body unless `link`
    // exposed it that way; real tclsh: "invalid command name". Must
    // abstain rather than falsely resolve.
    let src =
        "oo::class create C {\n    method greet {} {}\n    method twice {} { greet ; greet }\n}\n";
    let analysis = analyse(src);
    // Cursor on the first `greet` in the `twice` body.
    // Line 2: `    method twice {} { greet ; greet }`
    // Col 22 lands on the `g` of the first `greet`.
    let locs = definition(src, 2, 22, &analysis);
    assert!(locs.is_empty(), "{locs:?}");
}

#[test]
fn definition_linked_sibling_method_call_resolves() {
    // TP — `link greet` (called from the constructor) makes `greet`
    // genuinely bareword-callable from every method body of this class, so
    // the bare call resolves to the method's declaration.
    let src = "oo::class create C {\n    constructor {} { link greet }\n    method greet {} {}\n    method twice {} { greet ; greet }\n}\n";
    let analysis = Analyser::new().analyse(src, "tcl9.0");
    // Line 3: `    method twice {} { greet ; greet }` — same text/col
    // as the un-linked test above, one line further down.
    let locs = definition(src, 3, 22, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    // The method declaration is now on line 2.
    assert_eq!(locs[0].start_line, 2);
}

#[test]
fn definition_my_dispatch_resolves_when_class_extended_via_separate_oo_define() {
    // `Gadget` is
    // created via `oo::class create` with no body, then every method —
    // including the `my Helper` call site itself — is added via a
    // *separate*, later `oo::define Gadget { ... }` block. This is
    // exactly the real corpus shape (`ticklecharts::chart`: `oo::class
    // create` at one line, all methods added via a later `oo::define`).
    // tclsh9.0/8.6 both prove `my Helper` genuinely dispatches to
    // `Helper` here — go-to-definition must resolve it, not abstain.
    let src = "oo::class create Gadget {\n    variable _x\n}\noo::define Gadget {\n    method Helper {} { return hi }\n    method Caller {} { my Helper }\n}\n";
    let analysis = analyse(src);
    // Line 5: `    method Caller {} { my Helper }` — cursor on the
    // `Helper` word inside `my Helper` (col 26).
    let locs = definition(src, 5, 26, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    // The `Helper` method declaration is on line 4.
    assert_eq!(locs[0].start_line, 4);
}

#[test]
fn definition_bare_self_bracket_dispatch_resolves_like_my() {
    assert_self_bracket_dispatch_resolves_like_my("[self]");
}

#[test]
fn definition_self_object_bracket_dispatch_resolves_like_my() {
    assert_self_bracket_dispatch_resolves_like_my("[self object]");
}

#[test]
fn definition_bare_sibling_classmethod_call_without_link_abstains() {
    // FP — same shape as the method case above,
    // for `classmethod`.
    let src =
        "oo::class create C {\n    classmethod factory {} {}\n    method use {} { factory }\n}\n";
    let analysis = analyse(src);
    // Line 2: `    method use {} { factory }`
    // Cursor on `factory` (col 20).
    let locs = definition(src, 2, 20, &analysis);
    assert!(locs.is_empty(), "{locs:?}");
}

#[test]
fn definition_linked_sibling_classmethod_call_resolves() {
    // TP — `link factory` makes the classmethod
    // call resolve.
    let src = "oo::class create C {\n    constructor {} { link factory }\n    classmethod factory {} {}\n    method use {} { factory }\n}\n";
    let analysis = Analyser::new().analyse(src, "tcl9.0");
    // Line 3: `    method use {} { factory }` — same text/col as the
    // un-linked test above, one line further down.
    let locs = definition(src, 3, 20, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    // The classmethod declaration is now on line 2.
    assert_eq!(locs[0].start_line, 2);
}

/// tclsh 8.6, 9.0, 9.1: a method body runs in its object's namespace with
/// `namespace path ::oo::Helpers`. A method declared inside
/// `namespace eval ::app` calls `::helper` (`ns_method.tcl`: `global`),
/// and `::oo::Helpers::helper`, even when defined after the class,
/// shadows `::helper` (`helpers_shadow.tcl`: `helpers`).
#[test]
fn definition_in_a_method_body_follows_the_object_namespace_lookup() {
    let ns = "proc helper {} {return global}\nnamespace eval ::app {\n    proc helper {} {return app}\n    oo::class create C {\n        method use {} { helper }\n    }\n}\n";
    assert_eq!(
        method_body_definition_lines(ns, "helper", 2),
        [[0], [0], [0]]
    );
    let helpers = "proc helper {} {return global}\noo::class create C {\n    method use {} { helper }\n}\nproc ::oo::Helpers::helper {} {return helpers}\n";
    assert_eq!(
        method_body_definition_lines(helpers, "helper", 1),
        [[4], [4], [4]]
    );
    // `self` only reads the call context and `my` dispatches to a method
    // whose own body is inspected, so neither leaves the lookup unproved.
    let introspecting = "proc helper {} {return global}\noo::class create C {\n    method other {} {}\n    method use {} { my other; set me [self]; helper }\n}\n";
    assert_eq!(
        method_body_definition_lines(introspecting, "helper", 1),
        [[0], [0], [0]]
    );
}

/// `my` lives in the object namespace (tclsh 8.6, 9.0, 9.1: `namespace
/// which my` is `::oo::Obj12::my`; `d_my.tcl` prints `method-q` despite a
/// global `proc my`), so it never names the global procedure; nor does
/// `self`, which `::oo::Helpers` holds (`self_shadow.tcl`: `1`). A method
/// that defines `helper` relative to the object namespace shadows the
/// global one for later calls (`relproc_shadow.tcl`: `global local`), so a
/// document with such a body leaves member-body heads unresolved.
#[test]
fn definition_in_a_method_body_demurs_on_object_namespace_commands() {
    let my = "proc my {args} {return global-my}\noo::class create C {\n    method m {} {return [my q]}\n    method q {} {return method-q}\n}\n";
    assert_eq!(
        method_body_definition_lines(my, "my", 1),
        [[], [], []] as [[u32; 0]; 3]
    );
    let own_self = "proc self {args} {return global-self}\noo::class create C {\n    method use {} { self }\n}\n";
    assert_eq!(
        method_body_definition_lines(own_self, "self", 1),
        [[], [], []] as [[u32; 0]; 3]
    );
    // tclsh 9.0, 9.1 (`link_shadow.tcl`: `global method`): `link` makes a
    // method callable as a command in the object namespace, so the call
    // never names the global procedure; the linked method answers.
    let linked = "proc helper {} {return global}\noo::class create C {\n    method helper {} {return method}\n    method mk {} { link helper }\n    method use {} { helper }\n}\n";
    assert_eq!(
        method_body_definition_lines(linked, "helper", 3)[1..],
        [[2], [2]]
    );
    let relproc = "proc helper {} {return global}\noo::class create C {\n    method mk {} { proc helper {} {return local} }\n    method use {} { helper }\n}\n";
    assert_eq!(
        method_body_definition_lines(relproc, "helper", 2),
        [[], [], []] as [[u32; 0]; 3]
    );
}

#[test]
fn definition_jumps_to_constructor_keyword() {
    // Bare `constructor` inside a class body jumps to the
    // constructor's declaration.
    let src = "oo::class create C {\n    constructor {arg} {}\n    method touch_ctor {} { constructor }\n}\n";
    let analysis = analyse(src);
    // Cursor on `constructor` in the `touch_ctor` body
    // (line 2 col 27).
    let locs = definition(src, 2, 27, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    // The analyser anchors the constructor's ``name_span``
    // on the ``constructor`` keyword token (declared on
    // line 1).  The provider keeps a body-span fallback
    // for the empty-span case, but the keyword span is
    // populated here, so the jump lands on line 1.
    assert_eq!(locs[0].start_line, 1);
}

#[test]
fn definition_bare_sibling_property_call_without_link_abstains() {
    // FP — same rationale as the method/classmethod
    // cases above, for a Tcl 9.0+ `oo::configurable` `property`
    // accessor bareword call: `[length]` is not actually reachable
    // without a `link`; real tclsh: "invalid command name".
    let src = "oo::configurable create Widget {\n    property length -get {return 42}\n    method use {} { return [length] }\n}\n";
    let analysis = analyse(src);
    // Line 2: `    method use {} { return [length] }` — col 27 lands
    // on the `l` of `length`.
    let locs = definition(src, 2, 27, &analysis);
    assert!(locs.is_empty(), "{locs:?}");
}

#[test]
fn definition_member_lookup_skipped_outside_class_body() {
    // Same word outside the class body must not surface
    // the method definition.
    let src = "oo::class create C {\n    method greet {} {}\n}\ngreet\n";
    let analysis = analyse(src);
    // Cursor on the bare `greet` on line 3.
    let locs = definition(src, 3, 2, &analysis);
    // No proc / class / member named `greet` is in scope
    // here — the class-member lookup must not leak.
    assert!(locs.is_empty(), "{locs:?}");
}

// $obj method dispatch

#[test]
fn definition_resolves_obj_method_call() {
    // `set d [Dog new]` then `$d bark` — cursor on `bark`
    // jumps to the method declaration on Dog.
    let src = "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\n$d bark\n";
    let analysis = analyse(src);
    // Line 4 `$d bark` — `bark` starts at col 3.
    let locs = definition(src, 4, 3, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    // `method bark` is on line 1.
    assert_eq!(locs[0].start_line, 1);
}

#[test]
fn definition_resolves_a_literal_foreach_installed_method() {
    // `alpha` has no written `method alpha` word anywhere —
    // its only textual appearance is inside the loop's own literal list
    // — but go-to-definition must still land somewhere real rather than
    // finding nothing, exactly as hover/outline/completion do.
    let src = concat!(
        "oo::class create Widget {\n",
        "    foreach m {alpha beta gamma} {\n",
        "        method $m {args} { return $args }\n",
        "    }\n",
        "}\n",
        "set d [Widget new]\n",
        "$d alpha\n",
    );
    let analysis = analyse(src);
    // Line 6 `$d alpha` — `alpha` starts at col 3.
    let locs = definition(src, 6, 3, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    // The list `{alpha beta gamma}` is on line 1.
    assert_eq!(locs[0].start_line, 1, "{locs:?}");
}

#[test]
fn definition_resolves_obj_method_in_bracket() {
    // `[$d bark]` bracketed form.
    let src = "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\nputs [$d bark]\n";
    let analysis = analyse(src);
    // Line 4 `puts [$d bark]` — `bark` starts at col 9.
    let locs = definition(src, 4, 9, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 1);
}

#[test]
fn definition_obj_method_unknown_instance_falls_through() {
    // `$x bark` where `x` has no recorded class — no
    // instance-method resolution, falls through (and finds
    // nothing here).
    let src = "oo::class create Dog {\n    method bark {} {}\n}\n$x bark\n";
    let analysis = analyse(src);
    let locs = definition(src, 3, 3, &analysis);
    assert!(locs.is_empty(), "{locs:?}");
}

#[test]
fn definition_resolves_bare_created_instance_command_method() {
    // `Dog create rex` then `rex bark` — cursor on the bare
    // `bark` jumps to the method declaration, mirroring `$obj bark`.
    let src = "oo::class create Dog {\n    method bark {} {}\n}\nDog create rex\nrex bark\n";
    let analysis = analyse(src);
    // Line 4 `rex bark` — `bark` starts at col 4.
    let locs = definition(src, 4, 4, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 1);
}

#[test]
fn definition_bare_var_receiver_without_dollar_does_not_resolve() {
    // `set d [Dog new]` binds `d` as a variable; a bare `d bark` (no `$`)
    // is not a valid dispatch, so it must not resolve to the method.
    let src = "oo::class create Dog {\n    method bark {} {}\n}\nset d [Dog new]\nd bark\n";
    let analysis = analyse(src);
    let locs = definition(src, 4, 2, &analysis);
    assert!(locs.is_empty(), "{locs:?}");
}

// Class-command dispatch: `CLASS method` for a
// classmethod / `self method`, distinct from `$obj method` / `NAME
// method` instance dispatch above.

#[test]
fn classmethod_call_on_its_own_class_resolves() {
    // Native C9 classmethod on its original declaring class.
    let src = "oo::class create ActiveRecord {\n    classmethod find {args} { return \"found $args\" }\n}\nActiveRecord find foo bar\n";
    let analysis = Analyser::new().analyse(src, "tcl9.1").clone();
    let locs = definition(src, 3, 14, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 1);
    assert_eq!(locs[0].start_character, 16);
}

#[test]
fn classmethod_call_on_an_inheriting_subclass_resolves() {
    // Native C9 delegates inherit the original declaring-class method.
    let src = "oo::class create ActiveRecord {\n    classmethod find {args} { return \"found $args\" }\n}\noo::class create Table {\n    superclass ActiveRecord\n}\nTable find foo bar\n";
    let analysis = Analyser::new().analyse(src, "tcl9.1").clone();
    let locs = definition(src, 6, 7, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 1);
    assert_eq!(locs[0].start_character, 16);
}

#[test]
fn self_method_call_on_its_own_class_resolves() {
    // TP — requires BOTH Part 1 (self recognised at all) and Part 2
    // (the class-command receiver path).
    let src = "oo::class create Widget {\n    self method make {n} { return \"made $n\" }\n}\nWidget make gadget\n";
    let analysis = analyse(src);
    let locs = definition(src, 3, 8, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 1);
    assert_eq!(locs[0].start_character, 16);
}

#[test]
fn self_method_not_inherited_by_a_non_overriding_subclass() {
    // TN — the critical precision test for `is_self_method`: unlike
    // `ooutil`'s `classmethod`, a plain `self method` is NOT inherited
    // by a subclass's own bound command (confirmed against tclsh:
    // `Gadget make` raises `unknown method "make"`). A naive reuse of
    // the ordinary MRO walk (which DOES include Widget in Gadget's
    // MRO) would incorrectly resolve this.
    let src = "oo::class create Widget {\n    self method make {n} { return \"made $n\" }\n}\noo::class create Gadget {\n    superclass Widget\n}\nGadget make foo\n";
    let analysis = analyse(src);
    let locs = definition(src, 6, 8, &analysis);
    assert!(locs.is_empty(), "{locs:?}");
}

#[test]
fn instance_calling_a_classmethod_does_not_resolve() {
    // C8.6 provides no native classmethod worker or package extension.
    // C9's instance-side delegate is a distinct native forwarding protocol.
    let src = "oo::class create ActiveRecord {\n    classmethod find {args} { return \"found $args\" }\n}\nActiveRecord create rec1\nrec1 find foo bar\n";
    let analysis = analyse(src);
    let locs = definition(src, 4, 5, &analysis);
    assert!(locs.is_empty(), "{locs:?}");
}

#[test]
fn native_classmethod_requires_the_selected_engine_or_a_provider() {
    let source = "oo::class create C {classmethod find {} {return OK}}\nC find\n";
    let unavailable = Analyser::new().analyse(source, "tcl8.6").clone();
    assert_eq!(
        definition(source, 1, 2, &unavailable),
        [] as [crate::definition::LspRange; 0]
    );
    let provided = Analyser::new().analyse(source, "tcl9.1").clone();
    let locations = definition(source, 1, 2, &provided);
    assert_eq!(locations.len(), 1, "{locations:?}");
    assert_eq!(locations[0].start_line, 0);
    assert_eq!(locations[0].start_character, 32);
}

#[test]
fn bare_var_receiver_without_a_bound_command_does_not_resolve_class_dispatch() {
    // TN — mirrors
    // `logical_receiver_candidates_keep_instance_and_class_buckets_together`: `d` is
    // a plain variable (never bound as a command by `create`), so
    // neither the instance-command branch nor the
    // class-command branch may fire for it.
    let src = "oo::class create ActiveRecord {\n    classmethod find {args} {}\n}\nset d [ActiveRecord new]\nd find foo bar\n";
    let analysis = Analyser::new().analyse(src, "tcl9.1").clone();
    let locs = definition(src, 4, 2, &analysis);
    assert!(locs.is_empty(), "{locs:?}");
}

#[test]
fn variable_receiver_never_promotes_a_filewide_candidate_to_runtime_proof() {
    let mut analysis = analyse("");
    analysis
        .instance_classes
        .insert("receiver".to_owned(), "::Stale".to_owned());
    assert!(logical_receiver_candidate_at(&analysis, "receiver", true, 0).is_none());
    analysis
        .created_instance_commands
        .insert("receiver".to_owned());
    assert!(logical_receiver_candidate_at(&analysis, "receiver", false, 0).is_none());
}

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

//! Vocabulary versions and classes: the versions this loader knows, per-site notices for newer words and how unknown words classify.

use super::*;

/// `1`, `1.0`, `1.1`, `1.2`, `2.0`, `2.1`, `2.2` and `2.3` are all known; an
/// unknown *minor* loads with a notice rather than being refused.
#[test]
fn the_speclib_version_word_names_a_vocabulary_this_loader_knows() {
    for known in KNOWN_VOCABULARY_VERSIONS {
        let pack = evaluate_pack(&format!(
            "speclib probe {known} {{\n command demo {{ arity 1 }}\n}}"
        ));
        assert!(pack.notices.is_empty(), "{known}: {:?}", pack.notices);
        assert_eq!(&pack.dsl_version, known);
    }
    // An unknown minor within a supported major keeps loading maximally
    // (§6.1): a minor is only ever additive.
    let pack = evaluate_pack("speclib probe 2.9 {\n command demo { arity 1 }\n}");
    assert!(pack.command("demo").is_some(), "the pack still loads");
    assert_eq!(pack.dsl_version, "2.9");
    assert_eq!(pack.load_error, None);
    assert_eq!(
        pack.notices
            .iter()
            .map(|notice| notice.message.as_str())
            .collect::<Vec<_>>(),
        vec![
            "pack declares SpecTcl vocabulary 2.9; this loader knows 2.3 — \
                 newer words may be dropped"
        ]
    );
    // A number *below* every vocabulary this loader knows never named a
    // vocabulary at all — the classic slip is writing the library's own
    // release in the slot — so the notice points at `introduced_version`
    // instead of claiming the loader is behind.
    let pack = evaluate_pack("speclib probe 0.15 {\n command demo { arity 1 }\n}");
    assert!(pack.command("demo").is_some(), "the pack still loads");
    assert_eq!(
        pack.notices
            .iter()
            .map(|notice| notice.message.as_str())
            .collect::<Vec<_>>(),
        vec![
            "`0.15` is not a SpecTcl vocabulary version (this loader knows \
                 2.3); if it is the library's own version, it belongs in \
                 `introduced_version`, not the `speclib` slot"
        ]
    );
}

#[test]
fn replay_orders_prerelease_and_numbered_vocabulary_versions() {
    for (declared, newer) in [
        ("2.3a1", false),
        ("2.3b1", false),
        ("02.3.0", false),
        ("2.10", true),
    ] {
        let pack = evaluate_pack(&format!(
            "speclib probe {declared} {{\n command demo {{ arity 1 }}\n}}"
        ));
        assert!(
            pack.command("demo").is_some(),
            "{declared}: {:?}",
            pack.notices
        );
        assert_eq!(pack.load_error, None);
        let notice = &pack.notices[0].message;
        assert_eq!(
            notice.contains("newer words may be dropped"),
            newer,
            "{declared}: {notice}"
        );
        assert_eq!(
            notice.contains("is not a SpecTcl vocabulary version"),
            !newer,
            "{declared}: {notice}"
        );
    }
}

/// A 1.1-only word under a 1.0 declaration draws one notice per site:
/// this loader reads the word fine (additions never gate), but a
/// genuinely-1.0 loader drops it silently, so the declaration must say
/// 1.1. Declaring 1.1 clears every notice; the option row's lifecycle
/// flags stay 1.0 vocabulary and never trip it.
#[test]
fn v11_words_under_a_10_declaration_draw_a_per_site_notice() {
    let body = "\n command demo {\n                        arity 1\n                        option -x -introduced 1.2\n                        form Default {demo word} -introduced 1.2\n                        side_effect FileIo -writes -deprecated 2.0\n                        versioned_arg_value 0 utf-8 -introduced 1.2\n                    }\n}";

    let pack = evaluate_pack(&format!("speclib probe 1.0 {{{body}"));
    let v11_notices: Vec<&str> = pack
        .notices
        .iter()
        .filter(|n| n.message.contains("SpecTcl 1.1 vocabulary"))
        .map(|n| n.message.as_str())
        .collect();
    // form -introduced, side_effect -deprecated, and the command-scope
    // versioned_arg_value statement; the option row's -introduced is 1.0
    // vocabulary and stays silent.
    assert_eq!(v11_notices.len(), 3, "{:?}", pack.notices);
    assert!(
        v11_notices
            .iter()
            .all(|m| m.contains("declare `speclib probe 1.1`")),
        "{v11_notices:?}"
    );
    assert!(
        !v11_notices.iter().any(|m| m.contains("`-x`")),
        "option-row lifecycle is 1.0 vocabulary: {v11_notices:?}"
    );

    let pack = evaluate_pack(&format!("speclib probe 1.1 {{{body}"));
    assert!(
        pack.notices.is_empty(),
        "declaring 1.1 clears it: {:?}",
        pack.notices
    );
}

/// The all-versions contract: the same pack body loads to the identical
/// command surface under every vocabulary this loader knows — a pack is
/// never refused, and no known version reads fewer words than another.
#[test]
fn every_known_vocabulary_loads_the_same_command_surface() {
    let body = "{\n command demo {\n                        arity 1..\n                        option -mode -takes mode -values {a b} -closed\n                        form Default {demo ?-mode mode? word}\n                        hover { summary {Demo.} synopsis {demo word} }\n                    }\n command other { arity 0 }\n}";
    let baseline = evaluate_pack(&format!("speclib probe 1.2 {body}"));
    let baseline_names: Vec<&str> = baseline.commands.iter().map(|c| c.spec.name).collect();
    for known in ["1", "1.0", "1.1"] {
        let pack = evaluate_pack(&format!("speclib probe {known} {body}"));
        let names: Vec<&str> = pack.commands.iter().map(|c| c.spec.name).collect();
        assert_eq!(names, baseline_names, "{known}");
        assert!(pack.notices.is_empty(), "{known}: {:?}", pack.notices);
        let demo = pack.command("demo").expect("demo loads");
        assert_eq!(demo.spec.options.len(), 1, "{known}");
    }
}

/// The 1.2 words go through the *same* per-site mechanism the 1.1 words
/// do, which is the point of generalising it: a 1.2 word is a defect
/// under a 1.1 declaration exactly as a 1.1 word is under 1.0, and the
/// notice names the version that actually has the word rather than a
/// hard-coded one.
#[test]
fn v12_words_under_an_older_declaration_draw_a_per_site_notice() {
    // Two 1.2 sites: the arity window's lifecycle flag and `tk_geometry`.
    let body = "\n command demo {\n \
             arity 1\n \
             arity 2 -introduced 3.0\n \
             arg 0 -role Body\n \
             tk_geometry Exclusive\n \
             option -x -introduced 1.2\n \
             }\n}";

    // Under 1.1, only the 1.2 words are noticed — the option row's
    // lifecycle flags have been 1.0 vocabulary all along.
    let pack = evaluate_pack(&format!("speclib probe 1.1 {{{body}"));
    let messages: Vec<&str> = pack.notices.iter().map(|n| n.message.as_str()).collect();
    assert_eq!(
        messages.len(),
        2,
        "one per 1.2 site, and nothing else: {messages:?}"
    );
    assert!(
        messages
            .iter()
            .all(|m| m.contains("is SpecTcl 1.2 vocabulary")
                && m.contains("declare `speclib probe 1.2`")),
        "{messages:?}"
    );

    // Under 1.0 the same two sites are still the only 1.2 ones, and the
    // older mechanism keeps working beside it.
    let older = evaluate_pack(&format!("speclib probe 1.0 {{{body}"));
    assert_eq!(
        older
            .notices
            .iter()
            .filter(|n| n.message.contains("SpecTcl 1.2 vocabulary"))
            .count(),
        2,
        "{:?}",
        older.notices
    );

    // Declaring 1.2 clears them.
    let declared = evaluate_pack(&format!("speclib probe 1.2 {{{body}"));
    assert!(
        declared.notices.is_empty(),
        "declaring 1.2 clears it: {:?}",
        declared.notices
    );
}

#[test]
fn callback_and_timing_vocabulary_reports_every_pre_12_site() {
    let body = r"
 command demo {
   traits DEFERS_BODY
   arity 2
   arg 1 -role Body
   callback_taint_inputs {{1 {%A %K}}}
   script_timing_resolver {words ctx} { timing 1 Deferred }
   option -validatecommand -takes script -role Body -script-timing Deferred \
       -callback-taint-inputs {%P %s %S}
   subcommand child {
     traits DEFERS_BODY
     arity 2
     arg 1 -role Body
     callback_taint_inputs {{1 {%A}}}
     script_timing_resolver {words ctx} { timing 1 Deferred }
   }
 }
}";

    let older = evaluate_pack(&format!("speclib probe 1.1 {{{body}"));
    let vocabulary: Vec<&str> = older
        .notices
        .iter()
        .filter(|notice| notice.message.contains("SpecTcl 1.2 vocabulary"))
        .map(|notice| notice.message.as_str())
        .collect();
    for (word, expected) in [
        ("`callback_taint_inputs`", 2),
        ("`script_timing_resolver`", 2),
        ("`-script-timing`", 1),
        ("`-callback-taint-inputs`", 1),
    ] {
        assert_eq!(
            vocabulary
                .iter()
                .filter(|message| message.contains(word))
                .count(),
            expected,
            "missing per-site notice for {word}: {:?}",
            older.notices
        );
    }
    assert_eq!(
        vocabulary.len(),
        6,
        "only the six 1.2 sites should be reported: {:?}",
        older.notices
    );

    let declared = evaluate_pack(&format!("speclib probe 1.2 {{{body}"));
    assert!(
        declared
            .notices
            .iter()
            .all(|notice| !notice.message.contains("SpecTcl 1.2 vocabulary")),
        "declaring 1.2 clears every vocabulary notice: {:?}",
        declared.notices
    );
}

/// A 2.0 word under a 1.x declaration draws the same per-site notice
/// every newer word does.
#[test]
fn v20_words_under_an_older_declaration_draw_a_per_site_notice() {
    let pack = evaluate_pack(
        "speclib probe 1.2 {\n command demo {\n arity 1\n \
             available {tcl 8.6-}\n }\n}",
    );
    assert!(
        pack.notices.iter().any(|n| {
            n.message.contains("`available` is SpecTcl 2.0 vocabulary")
                && n.message.contains("declare `speclib probe 2.0`")
        }),
        "{:?}",
        pack.notices
    );
}

/// §6.1's three classes, in the forward direction that is the only one
/// they apply in.
///
/// The downgrade fixture pattern: each body's word is absent from this
/// build's vocabulary, and the assertion is that its absence yields
/// *abstention* — a dropped label, a degraded capability, or an excluded
/// command — never a stronger claim.
#[test]
fn unknown_words_classify_by_compatibility_effect() {
    // Presentation: prose. Warn and drop, as ever.
    let pack = evaluate_pack(
        "speclib probe 2.9 {\n command demo {\n arity 1\n \
             marketing_blurb {Buy now.}\n }\n}",
    );
    let demo = pack.command("demo").expect("the command loads");
    assert!(!demo.degraded, "prose costs the command nothing");
    assert_eq!(pack.notices[0].class, VocabularyClass::Presentation);

    // Assistance: a shape word. The command stays known, marked.
    let pack = evaluate_pack(
        "speclib probe 2.9 {\n command demo {\n arity 1\n \
             arg_cardinality 3\n }\n}",
    );
    let demo = pack.command("demo").expect("the command still loads");
    assert!(demo.degraded, "the affected capability must answer Unknown");
    assert!(
        pack.notices
            .iter()
            .any(|n| n.class == VocabularyClass::Assistance),
        "{:?}",
        pack.notices
    );

    // Semantic: a security word. The command is excluded outright.
    let pack = evaluate_pack(
        "speclib probe 2.9 {\n command demo {\n arity 1\n \
             taint_launders yes\n }\n command safe_neighbour { arity 0 }\n}",
    );
    assert!(
        pack.command("demo").is_none(),
        "a command whose security facts cannot be read is excluded"
    );
    assert!(
        pack.notices
            .iter()
            .any(|n| n.class == VocabularyClass::Semantic
                && n.message.contains("excluded from strong analysis")),
        "{:?}",
        pack.notices
    );
}

#[test]
fn native_successful_handler_contracts_are_explicit_semantic_exclusions() {
    for block in [
        "successful_handler -native Leaf",
        "successful_handler -native ArithmeticSequenceArguments",
        "successful_handler -native EnsembleLeaf",
        "successful_handler -native EnsemblePathLeaf",
        "subcommand capture {successful_handler -native CatchOutputs}",
        "refine getter {arity 0; successful_handler -native Leaf}",
        "refine getter {arity 0; successful_handler -native InitialiseEmptyVariable}",
    ] {
        let pack = evaluate_pack(&format!(
            "speclib probe 2.0 {{command demo {{{block}}}; command neighbour {{arity 0}}}}"
        ));
        assert!(
            pack.command("demo").is_none(),
            "native claims cannot be asserted by a pack"
        );
        assert!(pack.command("neighbour").is_some());
        assert!(
            pack.notices.iter().any(|notice| {
                notice.class == VocabularyClass::Semantic
                    && notice.message.contains("successful_handler")
                    && notice.message.contains("excluded from strong analysis")
            }),
            "{:?}",
            pack.notices
        );
    }
}

#[test]
fn native_form_value_overrides_are_explicit_semantic_exclusions() {
    for property in [
        "return_type",
        "arg_types",
        "byte_array_effect",
        "var_elements_effect",
        "safe_on_uninit",
    ] {
        let pack = evaluate_pack(&format!(
            "speclib probe 2.0 {{command demo {{refine getter {{arity 0; {property} none}}}}; command neighbour {{arity 0}}}}"
        ));
        assert!(pack.command("demo").is_none());
        assert!(pack.command("neighbour").is_some());
        assert!(
            pack.notices
                .iter()
                .any(|notice| notice.class == VocabularyClass::Semantic
                    && notice.message.contains("native form value overrides"))
        );
    }
}

/// The classes apply **only** in §6.1's forward direction. An unknown
/// word in a pack whose vocabulary this build knows in full is an
/// author's typo, and keeps today's warn-and-drop treatment exactly —
/// which is what keeps every 1.x pack in the corpus loading unchanged.
#[test]
fn unknown_words_in_a_known_vocabulary_stay_presentation_class() {
    for declared in ["1.2", "2.0"] {
        let pack = evaluate_pack(&format!(
            "speclib probe {declared} {{\n command demo {{\n arity 1\n \
                 taint_launders yes\n }}\n}}"
        ));
        let demo = pack.command("demo").expect("the command loads");
        assert!(!demo.degraded, "{declared}");
        assert!(
            pack.notices
                .iter()
                .all(|n| n.class == VocabularyClass::Presentation),
            "{declared}: {:?}",
            pack.notices
        );
    }
}

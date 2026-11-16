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

//! The taint of a word: literal reference names, escape grammar, retained source words, interpolation and deep nesting.

use super::*;

#[test]
fn configured_word_taint_keeps_literal_reference_names_and_index_reads() {
    for label in ["tcl8.6", "tcl9.1"] {
        let registry = tcl_registry::model::ingress::static_context_for(label).commands();
        let cfg = crate::cfg::Function::new("model", "entry");
        let mut ssa = SsaFunction::trivial("model", cfg.entry, cfg.block_names().to_vec());
        let mut uses = HashMap::new();
        let mut taints = HashMap::new();
        for (name, tainted) in [
            ("literal", false),
            ("$literal", true),
            ("", true),
            ("arr", false),
            ("index", true),
            ("a{b", false),
            ("a{b}c", true),
        ] {
            let symbol = ssa.intern_var(name);
            uses.insert(symbol, 1);
            taints.insert(
                (symbol, 1),
                if tainted {
                    TaintLattice::tainted()
                } else {
                    TaintLattice::clean()
                },
            );
        }
        let ctx = TaintCtx {
            registry,
            ssa: &ssa,
            interproc: None,
            known_procs: None,
            dialect: registry.profile(),
            source: TaintSourceContext::standalone(registry, registry.profile()),
            taint_summaries: None,
            instance_classes: None,
            source_position: None,
            source_tokens: None,
            word_offset: None,
            source_word: None,
        };
        for (word, tainted) in [
            ("$literal", false),
            ("${$literal}", true),
            ("${}", true),
            ("prefix${$literal}", true),
            (r"prefix\$literal", false),
            ("$arr($index)", true),
            ("[missing $literal", true),
        ] {
            assert_eq!(
                word_taint(word, &uses, &taints, ctx).is_tainted(),
                tainted,
                "{label}: {word}"
            );
        }
        assert_eq!(
            word_taint("${a{b}c}", &uses, &taints, ctx).is_tainted(),
            label == "tcl9.1"
        );
        assert!(!word_taint(r"prefix\[gets stdin\]", &uses, &taints, ctx).is_tainted());
    }
}

#[test]
fn taint_escape_fragments_use_the_selected_release_escape_grammar() {
    let before = backslash_subst_in(r"\x4142", tcl_dialect::EscapeSyntax::Tcl84);
    let after = backslash_subst_in(r"\x4142", tcl_dialect::EscapeSyntax::Tcl86);
    assert_ne!(before, after);
    assert_eq!(after, "A42");
}

#[test]
fn mixed_source_word_reuses_the_retained_nested_command_invocation() {
    let environment = tcl_registry::model::ingress::resolve_environment("tcl8.6");
    let context = environment.default_context_registry();
    let registry = context.commands();
    let (_, unit) = retained_warning_unit("set payload \"prefix[string length $input]\"", &context);
    let module = &unit.ir_module;
    let tokens = module
        .top_level
        .statements
        .last()
        .and_then(Statement::tokens)
        .expect("source-backed assignment tokens");
    let word = tokens.words().get(2).expect("mixed source value word");
    let source = TaintSourceContext::for_module_function(registry, module, &unit.top_level);
    let ctx = TaintCtx {
        registry,
        ssa: &unit.top_level.ssa,
        interproc: None,
        known_procs: None,
        dialect: registry.profile(),
        source,
        taint_summaries: None,
        instance_classes: None,
        source_position: Some(tokens.argv[0].start()),
        source_tokens: Some(tokens),
        word_offset: Some(word.source().span.start()),
        source_word: Some(word),
    };
    assert!(
        !word_taint(&word.legacy_text(), &HashMap::new(), &HashMap::new(), ctx).is_tainted(),
        "the actual selected string-length sanitizer remains clean in a mixed word"
    );
    let unavailable = TaintCtx {
        source_tokens: None,
        ..ctx
    };
    assert!(
        word_taint(
            &word.legacy_text(),
            &HashMap::new(),
            &HashMap::new(),
            unavailable
        )
        .is_tainted(),
        "a source span without its retained parent token carrier is unavailable"
    );
}

#[test]
fn taint_source_word_requires_its_actual_retained_owner_and_span() {
    let environment = tcl_registry::model::ingress::resolve_environment("tcl8.6");
    let context = environment.default_context_registry();
    let registry = context.commands();
    let (_, unit) = retained_warning_unit("set input SAFE; set payload $input", &context);
    let module = &unit.ir_module;
    let tokens = unit
        .top_level
        .cfg
        .blocks
        .values()
        .flat_map(|block| &block.statements)
        .filter_map(Statement::tokens)
        .find(|tokens| {
            tokens
                .words()
                .iter()
                .any(|word| word.legacy_text() == "$input")
        })
        .expect("actual source statement tokens");
    let word = tokens.words().get(2).expect("original value word");
    let ctx = TaintCtx {
        registry,
        ssa: &unit.top_level.ssa,
        interproc: None,
        known_procs: None,
        dialect: registry.profile(),
        source: TaintSourceContext::for_module(registry, module),
        taint_summaries: None,
        instance_classes: None,
        source_position: None,
        source_tokens: Some(tokens),
        word_offset: Some(word.source().span.start()),
        source_word: Some(word),
    };
    let (image, span) =
        retained_word_source(word.legacy_text().as_str(), ctx).expect("same retained source owner");
    assert_eq!(
        image.bytes().get(span.as_range()),
        Some(b"$input".as_slice())
    );
    let read_site = word.source();
    let mut read_owner = None;
    for (block, data) in &unit.top_level.ssa.blocks {
        for index in 0..data.statements.len() {
            let point = crate::ssa::SsaSourceView::at_statement(&unit.top_level.ssa, *block, index);
            if let Some(read) = point.read_reference(read_site, "$input")
                && let Some(version) = read.version
            {
                read_owner = Some((read.symbol, version));
            }
        }
    }
    let (symbol, version) = read_owner.expect("the exact original variable read");
    let clean = HashMap::from([((symbol, version), TaintLattice::clean())]);
    let read_ctx = TaintCtx {
        source_position: Some(tokens.argv[0].start()),
        ..ctx
    };
    assert!(
        !original_component_read_taint(read_ctx, read_site.span, "$input", &clean,).is_tainted()
    );
    let tainted = HashMap::from([((symbol, version), TaintLattice::tainted())]);
    assert!(
        word_taint("plain_projection", &HashMap::new(), &tainted, read_ctx).is_tainted(),
        "the source read span, not a compatibility spelling, selects the SSA value"
    );
    let mut detached_tokens = tokens.clone();
    detached_tokens.variable_accesses.clear();
    let detached_ctx = TaintCtx {
        source_tokens: Some(&detached_tokens),
        ..read_ctx
    };
    assert!(
        original_component_read_taint(detached_ctx, read_site.span, "$input", &clean,).is_tainted()
    );
    let detached = TaintCtx {
        source_tokens: None,
        ..ctx
    };
    assert!(retained_word_source(word.legacy_text().as_str(), detached).is_none());
}

#[test]
fn native_byte_escapes_cannot_become_unicode_text_provenance() {
    use tcl_dialect::EscapeSyntax;
    assert_eq!(decoded_text_fragment(r"\xff", EscapeSyntax::Jim), None);
    assert_eq!(
        decoded_text_fragment(r"\x41", EscapeSyntax::Jim).as_deref(),
        Some("A")
    );
    assert_eq!(
        decoded_text_fragment(r"\xff", EscapeSyntax::Tcl90).as_deref(),
        Some("ÿ")
    );
}

/// A deeply-nested `[a [a [a … x]]]` command-substitution word and a
/// deeply-nested `ExprNode` tree, plus a minimal [`TaintCtx`], for the
/// two ctx-taking recursion caps below.
fn deep_bracket_word(depth: usize) -> String {
    format!("{}x{}", "[a ".repeat(depth), "]".repeat(depth))
}

/// `word_taint` recurses once per nested `[cmd …]` command substitution
/// inside a single word's raw text (Tier 1B), and `expr_command_taint`
/// recurses once per `ExprNode` operator-tree level (Tier 1A) — both
/// genuinely unbounded axes, independent of any statement-tree cap.
/// Uncapped, each overflows the native stack (SIGABRT) in the low
/// thousands of levels
/// on a 2 MiB thread (`cargo test`'s default). 3000 is comfortably past
/// that crash range and past both caps (256); the assertion is that each
/// returns at all.
#[test]
fn deeply_nested_taint_walks_survive() {
    use crate::cfg::Function;
    use crate::ssa::SsaFunction;

    let registry = CommandRegistry::build_default();
    let cfg = Function::new("::top", "entry");
    let entry = cfg.entry;
    let ssa = SsaFunction::trivial("::top", entry, cfg.block_names().to_vec());
    let uses: HashMap<Symbol, u32> = HashMap::new();
    let taints: HashMap<ValueKey, TaintLattice> = HashMap::new();
    let ctx = TaintCtx {
        registry: &registry,
        ssa: &ssa,
        interproc: None,
        known_procs: None,
        dialect: None,
        source: TaintSourceContext::standalone(&registry, None),
        taint_summaries: None,
        instance_classes: None,
        source_position: None,
        source_tokens: None,
        word_offset: None,
        source_word: None,
    };

    // Tier 1B: nested `[a [a [a … x]]]` command-substitution text.
    let _ = word_taint(&deep_bracket_word(3000), &uses, &taints, ctx);

    // Tier 1A: a 3000-deep `ExprNode` tree (nested unary `!`).
    let mut node = ExprNode::Literal {
        text: "1".into(),
        start: 0,
        end: 1,
    };
    for _ in 0..3000 {
        node = ExprNode::Unary {
            op: crate::expr_ast::UnaryOp::Not,
            operand: Box::new(node),
        };
    }
    let _ = expr_command_taint(&node, &uses, &taints, ctx, None, 0);
}

/// Taint propagates through an interpolated-string word that embeds a
/// tainted variable: `set out "prefix_${x}_suffix"` where `$x` is tainted.
#[test]
fn interpolated_string_word_propagates_taint() {
    use crate::compilation_unit::CompilationUnit;
    use tcl_registry::CommandRegistry;

    let registry = CommandRegistry::build_default();
    let cu = CompilationUnit::build_for(
        "set x [gets stdin]\nset out \"prefix_${x}_suffix\"",
        &registry,
        false,
    );
    let fu = cu.function("::top").unwrap();
    let out_tainted = fu
        .taints
        .iter()
        .any(|((name, _ver), t)| fu.ssa.var_name(*name) == "out" && t.is_tainted());
    assert!(
        out_tainted,
        "expected 'out' to be tainted via interpolated string embedding tainted $x"
    );
}

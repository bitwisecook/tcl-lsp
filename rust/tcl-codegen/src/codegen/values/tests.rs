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

use super::*;
use crate::codegen::{CodegenCtx, Op};
use tcl_registry::CommandRegistry;

#[test]
fn compiled_word_emission_keeps_jim_expression_deferred_and_finished_data_exact() {
    for (dialect, literal) in [("tcl8.6", true), ("tcl9.0", true), ("jim", false)] {
        let context = tcl_registry::model::ingress::static_context_for(dialect);
        let mut ctx = CodegenCtx::new(false, &[], context.commands());
        ctx.emit_expr(&tcl_compiler::expr_ast::ExprNode::CompiledWord {
            text: "$(1+2)".to_owned(),
            braced: false,
        });
        assert_eq!(
            ctx.instructions.last().unwrap().push_verbatim,
            literal,
            "{dialect}"
        );
        ctx.push_lit_exact("${literal}[data]$(1+2)");
        assert!(ctx.instructions.last().unwrap().push_verbatim);
        assert!(
            ctx.literals
                .entries()
                .iter()
                .any(|entry| entry.bytes() == b"${literal}[data]$(1+2)")
        );
    }
}

#[allow(clippy::unnecessary_wraps)] // test callback follows ConstFoldFn
fn owned_list_fold(args: &[&str]) -> Option<String> {
    Some(format!("owned:{}", args.join(",")))
}

/// The owned registry decides the fold: the route a command declares,
/// whatever callback sits beside it, and with the route declined, the
/// callback.
#[test]
fn original_c84_direct_source_preserves_words_without_compiler_pool_actions() {
    use tcl_runtime_api::{CompileService, ScriptCompileTargetBytes};
    let profile = tcl_dialect::DialectProfile::find("tcl8.4").unwrap();
    let entry: crate::test_support::CapturedNativeEntry =
        crate::test_support::captured_native_entry(profile);
    let service = crate::compile_service::BytecodeCompileService::for_profile(profile);
    let source =
        tcl_runtime_api::SourceImage::native(b"list [info level] [info exists local]".as_slice());
    let module = service
        .compile_plain_script_bytes_with_entry(
            ScriptCompileTargetBytes {
                source: &source,
                namespace: &tcl_runtime_api::ByteNamespacePath::root(),
            },
            profile,
            &entry,
        )
        .unwrap();
    let function = &module.top_level;
    assert!(function.plain_command_dispatch);
    assert!(function.validate_native_compilation_entry().is_ok());
    assert!(
        function
            .literals
            .native_actions()
            .iter()
            .all(|action| { matches!(action, tcl_bytecode::NativeLiteralAction::Register(_)) })
    );
    for head in [b"list".as_slice(), b"info"] {
        assert!(
            function
                .literals
                .entries()
                .iter()
                .any(|literal| literal.bytes() == head)
        );
    }
    assert_eq!(
        function
            .instructions
            .iter()
            .filter(|instruction| { matches!(instruction.op, Op::INVOKE_STK1 | Op::INVOKE_STK4) })
            .count(),
        3
    );
}

#[test]
fn native_decoded_literals_are_exact_bytes_and_never_replayed_as_source() {
    let environment = tcl_registry::model::ingress::resolve_environment("jim");
    let context = environment.default_context_registry();
    let mut ctx = CodegenCtx::new(false, &[], context.commands());
    ctx.escapes = environment.analyser_profile().grammar.escapes;
    ctx.push_decoded_literal(r"\xff");
    ctx.push_decoded_literal(r"\u00ff");
    assert_eq!(ctx.literals.entries()[0].bytes(), &[0xff]);
    assert_eq!(ctx.literals.entries()[1].unicode(), Ok("ÿ"));
    assert!(
        ctx.instructions
            .iter()
            .all(|instruction| instruction.push_verbatim)
    );
}

#[test]
fn value_fold_uses_the_owned_registry_callback_and_binding() {
    let mut registry = CommandRegistry::build_default();
    let mut list = registry.get("list").expect("list spec").clone();
    list.const_fold = Some(owned_list_fold);
    registry.insert(list.clone());

    let mut ctx = CodegenCtx::new(false, &[], &registry);
    assert!(ctx.try_emit_constant_fold("[list a b]"));
    assert!(ctx.literals.entries().iter().any(|lit| lit == "a b"));

    list.semantics = tcl_registry::value_transfer::SemanticsDeclaration::Declined;
    registry.insert(list);
    let mut ctx = CodegenCtx::new(false, &[], &registry);
    assert!(ctx.try_emit_constant_fold("[list a b]"));
    assert!(ctx.literals.entries().iter().any(|lit| lit == "owned:a,b"));
    assert_eq!(
        ctx.command_binding_requirements,
        [tcl_runtime_api::CommandBindingIdentity::new("list", "list")]
            .into_iter()
            .collect()
    );
}

/// A command whose owned registry declines its route and drops its
/// callback has nothing to fold through.
#[test]
fn value_fold_obeys_owned_registry_removal_for_every_old_fast_path() {
    let mut registry = CommandRegistry::build_default();
    let declined = tcl_registry::value_transfer::SemanticsDeclaration::Declined;

    let mut list = registry.get("list").expect("list spec").clone();
    list.const_fold = None;
    list.semantics = declined;
    registry.insert(list);

    let mut format = registry.get("format").expect("format spec").clone();
    format.const_fold_versioned = None;
    format.semantics = declined;
    registry.insert(format);

    let mut dict = registry.get("dict").expect("dict spec").clone();
    let mut subcommands = dict.subcommands.to_vec();
    subcommands
        .iter_mut()
        .find(|sub| sub.name == "create")
        .expect("dict create spec")
        .const_fold = None;
    dict.subcommands = Box::leak(subcommands.into_boxed_slice());
    registry.insert(dict);

    for value in [
        "[list a b]",
        "[format {%s} value]",
        "[dict create key value]",
    ] {
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        assert!(
            !ctx.try_emit_constant_fold(value),
            "owned registry disabled the fold for {value}"
        );
        assert!(ctx.instructions.is_empty(), "partial emission for {value}");
    }
}

#[test]
fn nested_value_fold_retains_every_resolved_command_binding() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(false, &[], &registry);
    assert!(ctx.try_emit_constant_fold("[llength [list a b]]"));
    for name in ["list", "llength"] {
        assert!(
            ctx.command_binding_requirements
                .contains(&tcl_runtime_api::CommandBindingIdentity::new(name, name)),
            "missing {name} dependency: {:?}",
            ctx.command_binding_requirements
        );
    }
}

#[test]
fn consumed_value_fold_retains_an_executable_replay_boundary() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(true, &[], &registry);
    assert!(ctx.try_emit_constant_fold("[list a b]"));

    let start = &ctx.instructions[0];
    assert_eq!(start.op, Op::START_CMD);
    assert_eq!(start.source_cmd_text, "list a b");
    assert_eq!(
        start.source_command_boundary,
        super::super::SourceCommandBoundary::InlineReplay,
        "an inline replay point is not a new outer source-command owner",
    );
    let end = match start.operands.first() {
        Some(Operand::Label(label)) => label,
        other => panic!("fold boundary has no continuation: {other:?}"),
    };
    assert_eq!(ctx.label_positions.get(end), Some(&ctx.instructions.len()));
    assert!(
        ctx.instructions
            .iter()
            .find(|instruction| matches!(instruction.op, Op::PUSH1 | Op::PUSH4))
            .expect("folded literal push")
            .literal_rep
            != tcl_bytecode::LiteralRep::Word,
        "a folded command result is already a final Tcl value",
    );
}

#[test]
fn folded_dict_payload_is_verbatim_before_verification() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(false, &[], &registry);
    assert!(ctx.try_emit_constant_fold("[dict create {[missing]} {${value}}]"));

    let push = ctx
        .instructions
        .iter()
        .find(|instruction| matches!(instruction.op, Op::PUSH1 | Op::PUSH4))
        .expect("dict payload push");
    assert_eq!(push.literal_rep, tcl_bytecode::LiteralRep::Verbatim);
    assert!(
        ctx.instructions
            .iter()
            .any(|instruction| instruction.op == Op::VERIFY_DICT)
    );
}

// split_array_ref.

#[test]
fn split_array_ref_basic() {
    assert_eq!(split_array_ref("arr(key)"), Some(("arr", "key")));
}

#[test]
fn split_array_ref_no_parens() {
    assert_eq!(split_array_ref("scalar"), None);
}

#[test]
fn split_array_ref_nested() {
    assert_eq!(split_array_ref("arr(${inner})"), Some(("arr", "${inner}")));
}

/// The owner's edge cases hold through this facade:
/// both halves may be empty, and a `(` with nothing closing it is not a
/// reference. A local re-spelling that adds a "base must be non-empty"
/// test would silently demote `set (x) 5` to a scalar.
#[test]
fn split_array_ref_matches_the_owners_edges() {
    assert_eq!(split_array_ref("(x)"), Some(("", "x")));
    assert_eq!(split_array_ref("arr()"), Some(("arr", "")));
    assert_eq!(split_array_ref("()"), Some(("", "")));
    assert_eq!(split_array_ref(")"), None);
    assert_eq!(split_array_ref("a(b"), None);
    assert!(is_array_ref("(x)"));
}

// is_array_ref, is_qualified.

#[test]
fn is_array_ref_yes() {
    assert!(is_array_ref("a(1)"));
}

#[test]
fn is_array_ref_no() {
    assert!(!is_array_ref("x"));
}

#[test]
fn is_qualified_yes() {
    assert!(is_qualified("::foo"));
    assert!(is_qualified("n::foo"));
    assert!(is_qualified("k\0n::foo"));
}

#[test]
fn is_qualified_no() {
    assert!(!is_qualified("foo"));
    assert!(!is_qualified("a(k::z)"));
}

// parse_simple_var_ref.

#[test]
fn parse_simple_var_ref_basic() {
    assert_eq!(
        parse_simple_var_ref("${x}", tcl_dialect::BracedVarStyle::default()),
        Some("x")
    );
}

#[test]
fn parse_simple_var_ref_qualified() {
    assert_eq!(
        parse_simple_var_ref("${::foo}", tcl_dialect::BracedVarStyle::default()),
        Some("::foo")
    );
}

#[test]
fn parse_simple_var_ref_nested() {
    assert_eq!(
        parse_simple_var_ref("${::a(${::a(1)})}", tcl_dialect::BracedVarStyle::default()),
        Some("::a(${::a(1)})")
    );
}

#[test]
fn parse_simple_var_ref_bare_dollar() {
    assert_eq!(
        parse_simple_var_ref("$x", tcl_dialect::BracedVarStyle::default()),
        None
    );
}

#[test]
fn parse_simple_var_ref_no_close() {
    assert_eq!(
        parse_simple_var_ref("${x", tcl_dialect::BracedVarStyle::default()),
        None
    );
}

// The `$={name}` spelling is not a marker.

/// A whole word spelt `$={name}` is the *user's* literal text — `=` is not
/// a name character, so `Tcl_ParseVarName` never starts a substitution
/// there and both tclsh oracles print `$={y}` for `puts $={y}`. Treating
/// it as a "braced scalar" marker would compile it to
/// `push "y"; loadStk`, silently reading a variable.
#[test]
fn dollar_equals_word_is_a_literal_not_a_variable_load() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(true, &["y"], &registry);
    ctx.emit_value("$={y}", true);
    assert_eq!(
        ctx.instructions.iter().map(|i| i.op).collect::<Vec<_>>(),
        vec![Op::PUSH1],
        "`$={{y}}` must be pushed whole, not decoded as a variable load"
    );
    assert!(ctx.literals.entries().iter().any(|l| l == "$={y}"));
    // The real braced form still loads.
    let mut ctx = CodegenCtx::new(true, &["y"], &registry);
    ctx.emit_value("${y}", true);
    assert_eq!(
        ctx.instructions.iter().map(|i| i.op).collect::<Vec<_>>(),
        vec![Op::LOAD_SCALAR1],
    );
}

// is_integer_literal.

#[test]
fn integer_literal_positive() {
    assert!(is_integer_literal("42"));
}

#[test]
fn integer_literal_negative() {
    assert!(is_integer_literal("-7"));
}

#[test]
fn integer_literal_non_integer() {
    assert!(!is_integer_literal("abc"));
    assert!(!is_integer_literal(""));
}

// CodegenCtx value emission.

#[test]
fn push_lit_dedup() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(false, &[], &registry);
    ctx.push_lit("hello");
    ctx.push_lit("hello"); // dedup
    assert_eq!(ctx.literals.len(), 1);
    assert_eq!(ctx.instructions.len(), 2);
    // Both should reference index 0
    assert_eq!(
        ctx.instructions[0].operands[0],
        super::super::Operand::Imm(0)
    );
    assert_eq!(
        ctx.instructions[1].operands[0],
        super::super::Operand::Imm(0)
    );
}

#[test]
fn push_lit_no_dedup_creates_fresh_slot() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(false, &[], &registry);
    ctx.push_lit_no_dedup("x");
    ctx.push_lit_no_dedup("x");
    assert_eq!(ctx.literals.len(), 2); // two distinct slots
}

#[test]
fn original_variable_operands_keep_literal_keys_and_evaluate_bare_indices() {
    let registry = CommandRegistry::build_default();
    for is_proc in [false, true] {
        let mut literal = CodegenCtx::new(is_proc, &[], &registry);
        assert!(literal.emit_variable_reference("${arr($i)}"));
        let mut dynamic = CodegenCtx::new(is_proc, &[], &registry);
        assert!(dynamic.emit_variable_reference("$arr($i)"));
        assert_eq!(
            literal
                .instructions
                .iter()
                .map(|instruction| instruction.op)
                .collect::<Vec<_>>(),
            vec![Op::PUSH1, Op::LOAD_STK]
        );
        assert_eq!(
            dynamic.instructions.last().unwrap().op,
            if is_proc {
                Op::LOAD_ARRAY1
            } else {
                Op::LOAD_ARRAY_STK
            }
        );
        assert!(
            literal
                .literals
                .entries()
                .iter()
                .any(|value| value == "arr($i)")
        );
        assert!(!literal.literals.entries().iter().any(|value| value == "$i"));
        assert!(!dynamic.literals.entries().iter().any(|value| value == "$i"));
        assert!(
            dynamic
                .instructions
                .iter()
                .any(|instruction| matches!(instruction.op, Op::LOAD_SCALAR1 | Op::LOAD_STK))
        );
        let mut compound = CodegenCtx::new(is_proc, &[], &registry);
        assert!(!compound.emit_variable_reference("$arr(k)suffix"));
        assert_eq!(compound.instructions, [] as [tcl_bytecode::Instruction; 0]);
    }
}

#[test]
fn load_var_scalar_proc() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(true, &["x"], &registry);
    ctx.load_var("x");
    assert_eq!(ctx.instructions.len(), 1);
    assert_eq!(ctx.instructions[0].op, Op::LOAD_SCALAR1);
    assert_eq!(
        ctx.instructions[0].operands[0],
        super::super::Operand::Imm(0)
    );
}

#[test]
fn load_var_scalar_toplevel() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(false, &[], &registry);
    ctx.load_var("x");
    assert_eq!(ctx.instructions.len(), 2); // push name + loadStk
    assert_eq!(ctx.instructions[0].op, Op::PUSH1); // push "x"
    assert_eq!(ctx.instructions[1].op, Op::LOAD_STK);
}

#[test]
fn load_var_array_proc() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(true, &[], &registry);
    ctx.load_var("arr(key)");
    // Should intern "arr", push_lit "key", then LOAD_ARRAY1
    assert_eq!(ctx.instructions.last().unwrap().op, Op::LOAD_ARRAY1);
}

#[test]
fn load_var_array_toplevel() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(false, &[], &registry);
    ctx.load_var("arr(key)");
    // push "arr", push "key", LOAD_ARRAY_STK
    assert_eq!(ctx.instructions.last().unwrap().op, Op::LOAD_ARRAY_STK);
}

#[test]
fn load_var_qualified_in_proc() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(true, &[], &registry);
    ctx.load_var("::global_var");
    // Qualified vars always use stack-based ops
    assert_eq!(ctx.instructions.last().unwrap().op, Op::LOAD_STK);
}

#[test]
fn store_var_proc() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(true, &["x"], &registry);
    ctx.store_var("x");
    assert_eq!(ctx.instructions[0].op, Op::STORE_SCALAR1);
}

#[test]
fn store_var_toplevel() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(false, &[], &registry);
    ctx.store_var("x");
    assert_eq!(ctx.instructions[0].op, Op::STORE_STK);
}

#[test]
fn generic_original_operands_share_native_source_channel_and_escape_rules() {
    // Contract: naming.compiler.original-info-commands-literal-resolution
    // Software integration only. The original info-commands native
    // comparison separately observes escaped-NUL versus counted raw NUL.
    // docs/design/analysis/name-resolution-proofs/compiler-original-info-commands-literal-resolution.md
    for version in tcl_dialect::TclVersion::ALL {
        let dialect = tcl_native_model::InvocationDialect::for_version(version);
        let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
        let registry = CommandRegistry::build_default();
        for (source, expected) in [
            (r#"opaque "A\u0000B""#, b"A\xc0\x80B".as_slice()),
            ("opaque \"A\0B\"", b"A\0B".as_slice()),
        ] {
            let image = tcl_lexer::SourceImage::native(source.as_bytes());
            let tokens = increment_tokens(&image, config);
            let mut context = CodegenCtx::new(false, &[], &registry);
            context.ingress_lexer_config = Some(config);
            context.invocation_dialect = Some(dialect);
            context.source_string_protocol = dialect.native_source_string_protocol();
            context.set_source_image(image);
            context.with_invocation_tokens(Some(&tokens), |context| {
                let word = &tokens.words()[1];
                assert!(context.try_emit_original_operand_word(&word.legacy_text(), word));
                assert!(!context.try_emit_original_operand_word("derived", word));
            });
            assert!(
                context
                    .literals
                    .entries()
                    .iter()
                    .any(|literal| literal.bytes() == expected),
                "{version:?}/{source}"
            );
        }
    }
}

#[test]
fn original_lexical_cache_reuses_one_vector_and_restores_nested_scope() {
    // Software lexical-owner control; no Native object cache or execution grant.
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let registry = CommandRegistry::build_default();
    let config = tcl_lexer::LexerConfig::default();
    let image = tcl_lexer::SourceImage::native(b"opaque A B C".as_slice());
    let child_image = tcl_lexer::SourceImage::native(b"opaque CHILD".as_slice());
    let tokens = increment_tokens(&image, config);
    let child = increment_tokens(&child_image, config);
    let mut context = CodegenCtx::new(false, &[], &registry);
    context.ingress_lexer_config = Some(config);
    context.set_source_image(image.clone());
    context.with_invocation_tokens(Some(&tokens), |context| {
        let (outer, _) = context.original_lexical_words().unwrap();
        for word in tokens.words() {
            assert!(context.try_emit_original_operand_word(&word.legacy_text(), word));
            let (cached, _) = context.original_lexical_words().unwrap();
            assert!(Rc::ptr_eq(&outer, &cached));
        }
        context.with_invocation_tokens(Some(&child), |context| {
            assert!(context.original_lexical_words.get_mut().is_none());
            context.set_source_image(child_image.clone());
            let (nested, _) = context.original_lexical_words().unwrap();
            assert!(!Rc::ptr_eq(&outer, &nested));
            assert_eq!(nested.len(), 2);
        });
        context.set_source_image(image.clone());
        let (restored, _) = context.original_lexical_words().unwrap();
        assert!(Rc::ptr_eq(&outer, &restored));
        context.with_invocation_tokens(None, |context| {
            assert!(context.original_lexical_words().is_none());
        });
        assert!(Rc::ptr_eq(
            &outer,
            &context.original_lexical_words().unwrap().0
        ));
    });
    assert!(context.original_lexical_words.get_mut().is_none());
    assert!(context.original_lexical_words().is_none());
}

#[test]
fn original_lexical_cache_keys_allocation_channel_and_full_configuration() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let registry = CommandRegistry::build_default();
    let config = tcl_lexer::LexerConfig::default();
    let image = tcl_lexer::SourceImage::native(b"opaque A".as_slice());
    let tokens = increment_tokens(&image, config);
    let mut context = CodegenCtx::new(false, &[], &registry);
    context.ingress_lexer_config = Some(config);
    context.set_source_image(image.clone());
    context.with_invocation_tokens(Some(&tokens), |context| {
        let (first, _) = context.original_lexical_words().unwrap();
        context.set_source_image(image.clone());
        assert!(Rc::ptr_eq(
            &first,
            &context.original_lexical_words().unwrap().0
        ));
        context
            .ingress_lexer_config
            .as_mut()
            .unwrap()
            .strict_quoting = true;
        let (strict, _) = context.original_lexical_words().unwrap();
        assert!(!Rc::ptr_eq(&first, &strict));
        let equal = tcl_lexer::SourceImage::native(image.bytes());
        assert_eq!(equal, image);
        assert_ne!(equal.bytes().as_ptr(), image.bytes().as_ptr());
        context.set_source_image(equal.clone());
        let (reallocated, _) = context.original_lexical_words().unwrap();
        assert!(!Rc::ptr_eq(&strict, &reallocated));
        let document = tcl_lexer::SourceImage::from_bytes(
            equal.shared_bytes(),
            tcl_lexer::SourceChannel::Document,
        );
        assert_eq!(document.bytes().as_ptr(), equal.bytes().as_ptr());
        context.set_source_image(document);
        let (channel, _) = context.original_lexical_words().unwrap();
        assert!(!Rc::ptr_eq(&reallocated, &channel));
        context.set_source("opaque Q");
        assert!(context.original_lexical_words().is_none());
    });
}

#[test]
fn original_lexical_cache_rechecks_protocol_and_current_entry_issuer() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    use tcl_syntax::native_string::NativeStringProtocol;
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let registry = CommandRegistry::build_default().project_for_profile(profile);
    let (_owner, entry) = crate::test_support::captured_native_entry_with_owner(profile);
    let mut withdrawn = entry.clone();
    withdrawn.source_string_protocol = None;
    let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
    let image = tcl_lexer::SourceImage::native(b"opaque A".as_slice());
    let tokens = increment_tokens(&image, config);
    let mut context = CodegenCtx::new(false, &[], &registry);
    context.ingress_lexer_config = Some(config);
    context.native_entry = Some(&entry);
    context.source_string_protocol = entry.source_string_protocol;
    context.set_source_image(image);
    context.with_invocation_tokens(Some(&tokens), |context| {
        let (first, protocol) = context.original_lexical_words().unwrap();
        assert_eq!(Some(protocol), entry.source_string_protocol);
        context.source_string_protocol = None;
        assert!(context.original_lexical_words().is_none());
        context.source_string_protocol = Some(NativeStringProtocol::Jim084);
        assert!(context.original_lexical_words().is_none());
        context.source_string_protocol = entry.source_string_protocol;
        context.native_entry = Some(&withdrawn);
        assert!(context.original_lexical_words().is_none());
        context.native_entry = Some(&entry);
        assert!(Rc::ptr_eq(
            &first,
            &context.original_lexical_words().unwrap().0
        ));
    });
}

#[test]
fn original_lexical_cache_keeps_the_retained_invocation_source_owner() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let registry = CommandRegistry::build_default();
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
    let source = "opaque A";
    let bindings = tcl_compiler::command_binding::SourceCommandBindings::analyse_with_options(
        source,
        config,
        &registry,
        tcl_compiler::command_binding::SourceAnalysisOptions {
            invocation_dialect: Some(tcl_native_model::InvocationDialect::of_profile(profile)),
            ..Default::default()
        },
    );
    let image = tcl_lexer::SourceImage::document(source);
    let mut tokens = increment_tokens(&image, config);
    bindings.stamp_original_tokens(&mut tokens);
    assert!(
        tokens
            .source_binding
            .as_ref()
            .unwrap()
            .invocation_site()
            .is_some()
    );
    let mut context = CodegenCtx::new(false, &[], &registry);
    context.ingress_lexer_config = Some(config);
    context.set_source_image(image);
    context.with_invocation_tokens(Some(&tokens), |context| {
        let (original, _) = context.original_lexical_words().unwrap();
        context.set_source("other SOURCE");
        assert!(Rc::ptr_eq(
            &original,
            &context.original_lexical_words().unwrap().0
        ));
        assert!(
            tokens
                .source_binding
                .as_ref()
                .unwrap()
                .original_normal_result(&tokens)
                .is_none()
        );
    });
}

fn increment_entry(version: tcl_dialect::TclVersion) -> tcl_runtime_api::NativeCompilationEntry {
    use tcl_runtime_api::native_compilation::{
        NativeCompilationFrame, NativeInterpreterIdentity, NativeVariableObserverPresence,
    };
    let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name()).unwrap();
    let point = tcl_dialect::model::DialectPoint::for_tcl_version(version);
    let dialect = tcl_native_model::InvocationDialect::of_point(point);
    tcl_runtime_api::NativeCompilationEntry {
        interpreter: NativeInterpreterIdentity {
            owner: 1,
            interpreter: 0,
        },
        epoch: 0,
        profile: profile.cache_key(),
        invocation_policy: Some(profile.cache_key()),
        expression_policy:
            tcl_registry::native_expression_program::native_expression_evaluation_policy(
                profile, point,
            ),
        execution_point: Some(point),
        execution_name_policy: None,
        name_protocol: tcl_syntax::naming::NamePolicyProtocol::for_native_point(point),
        compiled_variable_protocol:
            tcl_syntax::naming::NativeCompiledVariableProtocol::for_native_point(point),
        compiled_local_layout: None,
        oo_classes: None,
        ensemble_target_objects: None,
        source_string_protocol: dialect.native_source_string_protocol(),
        lexer_grammar: Some(profile.grammar),
        inline_compilation_disabled: false,
        direct_evaluation: false,
        authored_tmm_static: None,
        namespace_variable_tables: None,
        empty_literal_world: None,
        compiler_pass_environment: None,
        command_resolvers: None,
        variable_observers: NativeVariableObserverPresence::Unknown,
        math_functions: None,
        closed: true,
        commands: vec![].into(),
        namespaces: vec![],
        current_namespace: 0,
        frame: NativeCompilationFrame::Global,
    }
}

fn increment_tokens(
    source: &tcl_lexer::SourceImage,
    config: tcl_lexer::LexerConfig,
) -> tcl_compiler::ir::CommandTokens {
    let segment =
        tcl_source::segment::segment_commands_image_with_offset_and_config(source, 0, config)
            .unwrap()
            .remove(0);
    tcl_compiler::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::from_image(source),
        config,
        &segment,
    )
}

#[test]
fn original_variable_projection_keeps_the_final_substitution_closer() {
    use tcl_syntax::native_variable_words::NativeVariableWordOperand;
    let profile = tcl_dialect::DialectProfile::find("tcl8.4").unwrap();
    let registry = CommandRegistry::build_default().project_for_profile(profile);
    let entry: crate::test_support::CapturedNativeEntry =
        crate::test_support::captured_native_entry(profile);
    for source in [
        "lappend ::events [list $n $i $op]",
        "lappend \"arr($i)\" [list $n]",
        "lappend {arr($i)} [list $n]",
    ] {
        let image = tcl_lexer::SourceImage::native(source.as_bytes());
        let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
        let tokens = increment_tokens(&image, config);
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        ctx.source = image;
        ctx.native_entry = Some(&entry);
        ctx.source_string_protocol = entry.source_string_protocol;
        ctx.invocation_tokens = Some(Box::new(tokens.clone()));
        let original = ctx.original_compiler_words().unwrap();
        assert_eq!(original.words.len(), 3, "{source}");
        let variable = ctx.original_variable_operand(&tokens.words()[1]).unwrap();
        match (source, variable) {
            (
                "lappend ::events [list $n $i $op]",
                NativeVariableWordOperand::Literal { name, index, .. },
            ) => {
                assert_eq!(name, b"::events");
                assert_eq!(index, None);
            }
            (
                "lappend \"arr($i)\" [list $n]",
                NativeVariableWordOperand::CompoundArray { name, index, .. },
            ) => {
                assert_eq!(name, b"arr");
                assert_eq!(index.image().bytes(), original.words[1].image().bytes());
            }
            ("lappend {arr($i)} [list $n]", NativeVariableWordOperand::DynamicWord) => {}
            (source, variable) => panic!("{source}: actual C84 variable layout {variable:?}"),
        }
        let changed = increment_tokens(
            &tcl_lexer::SourceImage::native(b"lappend other [list $n]".as_slice()),
            config,
        );
        assert!(ctx.original_variable_operand(&changed.words()[1]).is_none());
        ctx.source_string_protocol = None;
        assert!(ctx.original_variable_operand(&tokens.words()[1]).is_none());
    }
}

#[test]
fn original_upvar_level_layout_uses_current_compiler_words_and_release() {
    let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
    let registry = CommandRegistry::build_default().project_for_profile(profile);
    let entry = crate::test_support::captured_native_entry(profile);
    for (source, count, expected) in [
        ("upvar {+0} other local", 3, Some(1)),
        ("upvar other local", 2, Some(0)),
        ("upvar 0 other local extra", 4, None),
        ("upvar $level other local", 3, None),
        ("upvar {0\0suffix} other local", 3, None),
    ] {
        let image = tcl_lexer::SourceImage::native(source.as_bytes());
        let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
        let tokens = increment_tokens(&image, config);
        let mut ctx = CodegenCtx::new(true, &[], &registry);
        ctx.source = image;
        ctx.native_entry = Some(&entry);
        ctx.source_string_protocol = entry.source_string_protocol;
        ctx.invocation_tokens = Some(Box::new(tokens));
        assert_eq!(
            ctx.original_upvar_level_width(count),
            expected,
            "{source:?}"
        );
        ctx.native_hook_layout = Some(("captured".to_owned(), 1));
        assert!(ctx.original_upvar_level_width(count).is_none());
        assert_eq!(ctx.instructions, [] as [tcl_bytecode::Instruction; 0]);
        assert!(ctx.lvt.is_empty());
    }
}

#[test]
fn named_unit_dependencies_require_the_original_entry_world() {
    use std::sync::Arc;
    use tcl_runtime_api::native_compilation::{
        NativeCommandCompiler, NativeCommandCompilerPrerequisite, NativeCommandImplementation,
        NativeCompilationBinding, NativeCompilationNamespace, NativeCompilerHookPresence,
        NativeCompilerSelectionPrerequisite, NativeEnsembleCompiler,
    };
    let registry = CommandRegistry::build_default();
    let mut entry = increment_entry(tcl_dialect::TclVersion::V9_0);
    entry.namespaces.push(NativeCompilationNamespace {
        path: tcl_core_types::ByteNamespacePath::root(),
        jim_namespace_object: None,
        token: 0,
        visible: true,
        exports: vec![],
        command_path: vec![],
        unknown_handler: None,
    });
    let compiler = NativeCommandCompiler {
        registry_identity: "info".to_owned(),
        ensemble: Some(NativeEnsembleCompiler {
            namespace_token: 0,
            map: vec![],
            subcommands: None,
            prefixes: true,
            parameters: vec![],
            unknown_handler: None,
        }),
    };
    let binding = NativeCompilationBinding {
        slot: tcl_core_types::NativeByteCommandSlot::new(
            tcl_core_types::ByteNamespacePath::root(),
            "info".into(),
        ),
        namespace_token: 0,
        token: 7,
        implementation_generation: 11,
        implementation: NativeCommandImplementation::Opaque,
        compiler_hook: NativeCompilerHookPresence::Present,
        compiler: Some(compiler.clone()),
        procedure_header: None,
        has_execution_trace: false,
    };
    entry.commands.push(binding.clone());
    let required = NativeCommandCompilerPrerequisite {
        interpreter: entry.interpreter,
        lookup_namespace_token: 0,
        invocation_word: "info".into(),
        slot: binding.slot.clone(),
        namespace_token: 0,
        token: binding.token,
        implementation_generation: binding.implementation_generation,
        compiler,
        selected_worker: None,
        nested_compilers: Vec::new(),
        guard: tcl_runtime_api::CommandBindingGuard::BeforeArguments,
    };
    for changed_axis in 0..6 {
        let mut selected = required.clone();
        match changed_axis {
            0 => {}
            1 => selected.interpreter.owner += 1,
            2 => selected.lookup_namespace_token += 1,
            3 => selected.token += 1,
            4 => selected.implementation_generation += 1,
            5 => selected.compiler.ensemble.as_mut().unwrap().prefixes = false,
            _ => unreachable!(),
        }
        let selected = Arc::new(selected);
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        ctx.native_entry = Some(&entry);
        ctx.retain_entry_named_compiler_prerequisite(&selected);
        if changed_axis == 0 {
            let mut entry_required = selected.as_ref().clone();
            entry_required.guard = tcl_runtime_api::CommandBindingGuard::ChunkEntry;
            assert_eq!(
                ctx.native_compiler_prerequisites,
                vec![NativeCompilerSelectionPrerequisite::Ensemble(Arc::new(
                    entry_required
                ))]
            );
        } else {
            assert!(
                ctx.native_compiler_prerequisites.is_empty(),
                "axis {changed_axis}"
            );
        }
        // Every original selection retains its own temporal guard even
        // when a future or foreign world cannot become a unit dependency.
        assert_eq!(
            selected.guard,
            tcl_runtime_api::CommandBindingGuard::BeforeArguments
        );
    }
    entry.commands[0].has_execution_trace = true;
    let mut ctx = CodegenCtx::new(false, &[], &registry);
    ctx.native_entry = Some(&entry);
    ctx.retain_entry_named_compiler_prerequisite(&Arc::new(required));
    assert_eq!(
        ctx.native_compiler_prerequisites,
        [] as [tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite; 0]
    );
}

#[test]
fn native_command_literal_actions_retain_data_first_and_hide_only_c85_one_word() {
    use tcl_bytecode::{NativeLiteralAction, NativeLiteralAllocation};
    use tcl_runtime_api::native_compilation::NativeCompilationNamespace;
    let registry = CommandRegistry::build_default();
    for version in tcl_dialect::TclVersion::ALL {
        for source in ["missing", "missing argument"] {
            let mut entry = increment_entry(version);
            entry.namespaces.push(NativeCompilationNamespace {
                path: tcl_core_types::ByteNamespacePath::root(),
                jim_namespace_object: None,
                token: 0,
                visible: true,
                exports: vec![],
                command_path: vec![],
                unknown_handler: None,
            });
            let image = tcl_lexer::SourceImage::native(source.as_bytes());
            let config = tcl_lexer::LexerConfig::from_grammar(entry.lexer_grammar.unwrap());
            let tokens = tcl_lexer::Lexer::with_source_image(&image, config)
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            let words = tcl_lexer::group_commands_bytes(&tokens, image.bytes(), config);
            let native_words = words[0]
                .words
                .iter()
                .map(|word| tcl_lexer::NativeWord::from_group(image.clone(), config, &tokens, word))
                .collect::<Result<Vec<_>, _>>()
                .unwrap();
            let mut ctx = CodegenCtx::new(false, &[], &registry);
            ctx.native_entry = Some(&entry);
            let data = ctx.literals.intern_bytes(b"missing");
            let command = ctx
                .intern_original_native_command_literal(&native_words)
                .unwrap();
            assert_eq!(command, data, "{version:?} {source}");
            assert_eq!(
                ctx.literals.entries()[data].allocation(),
                &NativeLiteralAllocation::RegisteredData
            );
            assert_eq!(
                ctx.literals.native_actions(),
                if version == tcl_dialect::TclVersion::V8_5 && source == "missing" {
                    vec![
                        NativeLiteralAction::Register(data),
                        NativeLiteralAction::Hide(data),
                    ]
                } else {
                    vec![NativeLiteralAction::Register(data)]
                }
            );
            assert!(!ctx.native_dependency_refusal);
        }
    }
}

#[test]
fn source_increment_uses_physical_amount_recipe_and_original_array_layout() {
    let registry = CommandRegistry::build_default();
    for version in tcl_dialect::TclVersion::ALL {
        for (source, immediate) in [
            ("incr arr($key) -127", Some(-127)),
            ("incr arr($key) -128", None),
            (
                "incr arr($key) 4294967295",
                (version <= tcl_dialect::TclVersion::V8_5).then_some(-1),
            ),
            (
                "incr arr($key) \\x31",
                (version >= tcl_dialect::TclVersion::V9_1).then_some(1),
            ),
        ] {
            let entry = increment_entry(version);
            let image = tcl_lexer::SourceImage::native(source.as_bytes());
            let config = tcl_lexer::LexerConfig::from_grammar(entry.lexer_grammar.unwrap());
            let tokens = increment_tokens(&image, config);
            let mut ctx = CodegenCtx::new(true, &["key"], &registry);
            ctx.native_entry = Some(&entry);
            ctx.dialect = tcl_dialect::DialectProfile::find("tcl8.4");
            ctx.ingress_lexer_config = Some(config);
            ctx.source_string_protocol = entry.source_string_protocol;
            ctx.compiled_variable_protocol = entry.compiled_variable_protocol;
            ctx.set_source_image(image);
            ctx.with_invocation_tokens(Some(&tokens), |ctx| {
                ctx.emit_incr("arr($key)", false, Some("ignored"), false);
            });
            assert_eq!(
                ctx.instructions.last().unwrap().op,
                if immediate.is_some() {
                    Op::INCR_ARRAY1_IMM
                } else {
                    Op::INCR_ARRAY1
                },
                "{version:?} {source}"
            );
            if let Some(value) = immediate {
                assert_eq!(
                    ctx.instructions.last().unwrap().operands.last(),
                    Some(&Operand::Imm(value))
                );
            }
            assert_eq!(
                ctx.instructions
                    .iter()
                    .filter(|instruction| instruction.op == Op::LOAD_SCALAR1)
                    .count(),
                1
            );
            assert!(!ctx.native_dependency_refusal);
        }
    }
}

#[test]
fn original_scalar_operand_slot_borrows_only_current_existing_layout_names() {
    use tcl_runtime_api::native_compilation::{
        NativeCompiledLocalLayout, NativeCompiledLocalLayoutKind, NativeInterpreterIdentity,
    };
    let registry = CommandRegistry::build_default();
    let entry = increment_entry(tcl_dialect::TclVersion::V9_0);
    let layout = NativeCompiledLocalLayout {
        owner: NativeInterpreterIdentity {
            owner: 1,
            interpreter: 0,
        },
        token: 7,
        epoch: 1,
        kind: NativeCompiledLocalLayoutKind::Procedure,
        names: vec![Some(tcl_runtime_api::NameBytes::from(
            b"retained".as_slice(),
        ))],
    };
    let mut ctx = CodegenCtx::new(false, &[], &registry);
    ctx.native_entry = Some(&entry);
    ctx.source_string_protocol = entry.source_string_protocol;
    ctx.compiled_variable_protocol = entry.compiled_variable_protocol;
    ctx.lvt = tcl_bytecode::LocalVarTable::from_native_slot_names(&layout.names);
    ctx.lvt
        .set_native_protocol(entry.compiled_variable_protocol);
    ctx.borrowed_local_layout = Some(layout.clone());
    let config = tcl_lexer::LexerConfig::from_grammar(entry.lexer_grammar.unwrap());
    ctx.ingress_lexer_config = Some(config);
    for (source, expected) in [
        ("info exists retained", Some(0)),
        ("info exists missing", None),
        ("info exists retained($key)", None),
        ("info exists ret\\x61ined", None),
    ] {
        let image = tcl_lexer::SourceImage::native(source.as_bytes());
        let tokens = increment_tokens(&image, config);
        let word = tokens.words()[2].clone();
        ctx.set_source_image(image);
        let slot = ctx
            .with_invocation_tokens(Some(&tokens), |ctx| ctx.original_scalar_operand_slot(&word));
        assert_eq!(slot, expected, "{source}");
        assert_eq!(ctx.lvt.len(), 1);
    }
    assert_eq!(ctx.required_compiled_local_layout, Some(layout));
}

#[test]
fn unlocated_increment_amount_is_an_evaluated_value() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(true, &["x"], &registry);
    ctx.emit_incr("x", true, Some("${amount}"), false);
    assert_eq!(ctx.instructions.last().unwrap().op, Op::INCR_SCALAR1);
    assert!(ctx.instructions.iter().all(|instruction| !matches!(
        instruction.op,
        Op::LOAD_SCALAR1 | Op::LOAD_SCALAR4 | Op::LOAD_STK
    )));
    assert!(
        ctx.literals
            .entries()
            .iter()
            .any(|literal| literal == "${amount}")
    );
}

#[test]
fn supplied_unknown_compiler_point_cannot_borrow_authoring_increment_rules() {
    let registry = CommandRegistry::build_default();
    let mut entry = increment_entry(tcl_dialect::TclVersion::V9_0);
    entry.execution_point = None;
    let image = tcl_lexer::SourceImage::native(b"incr x 5".as_slice());
    let config = tcl_lexer::LexerConfig::from_grammar(entry.lexer_grammar.unwrap());
    let tokens = increment_tokens(&image, config);
    let mut ctx = CodegenCtx::new(true, &["x"], &registry);
    ctx.native_entry = Some(&entry);
    ctx.source_string_protocol = entry.source_string_protocol;
    ctx.ingress_lexer_config = Some(config);
    ctx.set_source_image(image);
    ctx.with_invocation_tokens(Some(&tokens), |ctx| {
        ctx.emit_incr("x", true, Some("5"), false);
    });
    assert!(ctx.native_dependency_refusal);
    assert_eq!(ctx.instructions.last().unwrap().op, Op::INCR_SCALAR1);
}

#[test]
fn emit_incr_default_proc() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(true, &["x"], &registry);
    ctx.emit_incr("x", true, None, false);
    assert_eq!(ctx.instructions[0].op, Op::INCR_SCALAR1_IMM);
    // operands: slot 0, imm 1
    assert_eq!(
        ctx.instructions[0].operands[1],
        super::super::Operand::Imm(1)
    );
}

#[test]
fn emit_incr_literal_amount_proc() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(true, &["x"], &registry);
    ctx.emit_incr("x", true, Some("5"), false);
    assert_eq!(ctx.instructions[0].op, Op::INCR_SCALAR1_IMM);
    assert_eq!(
        ctx.instructions[0].operands[1],
        super::super::Operand::Imm(5)
    );
}

#[test]
fn emit_incr_large_amount_proc() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(true, &["x"], &registry);
    ctx.emit_incr("x", true, Some("999"), false);
    // Large amount → push_lit + INCR_SCALAR1
    assert_eq!(ctx.instructions[0].op, Op::PUSH1); // push "999"
    assert_eq!(ctx.instructions[1].op, Op::INCR_SCALAR1);
}

#[test]
fn emit_incr_default_toplevel() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(false, &[], &registry);
    ctx.emit_incr("x", true, None, false);
    // push "x" then INCR_STK_IMM
    assert_eq!(ctx.instructions[0].op, Op::PUSH1);
    assert_eq!(ctx.instructions[1].op, Op::INCR_STK_IMM);
}

#[test]
fn emit_incr_large_amount_toplevel() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(false, &[], &registry);
    ctx.emit_incr("x", true, Some("999"), false);
    assert_eq!(
        ctx.instructions
            .iter()
            .map(|instruction| instruction.op)
            .collect::<Vec<_>>(),
        vec![Op::PUSH1, Op::PUSH1, Op::INCR_STK]
    );
    assert_eq!(ctx.literals.entries()[0].bytes(), b"x");
    assert_eq!(ctx.literals.entries()[1].bytes(), b"999");
}

#[test]
fn begin_end_command() {
    let registry = CommandRegistry::build_default();
    let mut ctx = CodegenCtx::new(false, &[], &registry);
    // First command — no startCommand emitted
    ctx.begin_command(1);
    assert!(ctx.start_cmd_end_label.is_none());
    assert_eq!(ctx.cmd_index, 1);
    ctx.end_command();

    // Second command — startCommand emitted
    ctx.begin_command(1);
    assert!(ctx.start_cmd_end_label.is_some());
    assert_eq!(ctx.cmd_index, 2);
    assert_eq!(ctx.instructions[0].op, Op::START_CMD);
    ctx.end_command();
}

#[test]
fn resolved_targets_prepare_slot_and_stack_operands() {
    let registry = CommandRegistry::build_default();
    let mut top = CodegenCtx::new(false, &[], &registry);
    assert!(top.target_needs_stack("x"));
    let mut procedure = CodegenCtx::new(true, &[], &registry);
    assert!(!procedure.target_needs_stack("x"));
    assert!(procedure.target_needs_stack("::x"));
    assert!(procedure.target_needs_stack("a(1)"));
}

#[test]
fn borrowed_native_commands_reuse_cells_without_declaring_slots() {
    use tcl_runtime_api::native_compilation::{
        NativeCompiledLocalLayout, NativeCompiledLocalLayoutKind, NativeInterpreterIdentity,
    };
    let registry = CommandRegistry::build_default();
    for (point, borrows) in [
        (
            tcl_dialect::model::DialectPoint::for_tcl_version(tcl_dialect::TclVersion::V8_4),
            false,
        ),
        (
            tcl_dialect::model::DialectPoint::for_tcl_version(tcl_dialect::TclVersion::V8_5),
            false,
        ),
        (
            tcl_dialect::model::DialectPoint::for_tcl_version(tcl_dialect::TclVersion::V8_6),
            true,
        ),
        (
            tcl_dialect::model::DialectPoint::for_tcl_version(tcl_dialect::TclVersion::V9_0),
            true,
        ),
        (
            tcl_dialect::model::DialectPoint::for_tcl_version(tcl_dialect::TclVersion::V9_1),
            true,
        ),
    ] {
        let protocol =
            tcl_syntax::naming::NativeCompiledVariableProtocol::for_native_point(point).unwrap();
        let names = vec![Some(tcl_runtime_api::NameBytes::from(b"k\0a".as_slice()))];
        let layout = NativeCompiledLocalLayout {
            owner: NativeInterpreterIdentity {
                owner: 23,
                interpreter: 5,
            },
            token: 7,
            epoch: 1,
            kind: NativeCompiledLocalLayoutKind::Procedure,
            names,
        };
        let mut context = CodegenCtx::new(false, &[], &registry);
        context.compiled_variable_protocol = Some(protocol);
        context.lvt = tcl_bytecode::LocalVarTable::from_native_slot_names(&layout.names);
        context.lvt.set_native_protocol(Some(protocol));
        context.borrowed_local_layout = Some(layout.clone());
        assert_eq!(context.target_needs_stack("k\0b"), !borrows);
        if !borrows {
            context.push_var_ref("k\0b", true);
        }
        context.push_lit_exact("ALTER");
        context.store_var("k\0b");
        assert_eq!(
            context.instructions.last().unwrap().op,
            if borrows {
                Op::STORE_SCALAR1
            } else {
                Op::STORE_STK
            }
        );
        assert!(context.target_needs_stack("missing"));
        assert_eq!(context.lvt.len(), 1);
        let assembly = context.into_function_asm("borrowed".into());
        assert_eq!(
            assembly.required_compiled_local_layout,
            borrows.then_some(layout)
        );
    }
}
#[test]
fn full_depth_text_indices_use_the_same_iterative_operand_emitter() {
    let registry = CommandRegistry::build_default();
    let source = include_str!("../../../../tcl-compiler/tests/data/native_deep_array_source.tcl");
    let operand = source.strip_prefix("set a(x) x; list ").unwrap();
    let mut context = CodegenCtx::new(true, &[], &registry);
    assert!(context.emit_variable_reference(operand));
    assert_eq!(
        context
            .instructions
            .iter()
            .filter(|instruction| instruction.op == Op::LOAD_ARRAY1)
            .count(),
        2000,
    );
    assert_eq!(context.lvt.len(), 1);
    assert!(
        context
            .literals
            .entries()
            .iter()
            .any(|entry| entry.bytes() == b"x")
    );
}

#[test]
fn source_local_loads_select_counted_compiler_purposes_before_runtime_names() {
    use tcl_syntax::naming::NativeCompiledVariableProtocol;

    let registry = CommandRegistry::build_default();
    for version in tcl_dialect::TclVersion::ALL {
        let mut context = CodegenCtx::new(true, &[], &registry);
        let protocol = NativeCompiledVariableProtocol::for_native_point(
            tcl_dialect::model::DialectPoint::for_tcl_version(version),
        )
        .unwrap();
        context.compiled_variable_protocol = Some(protocol);
        context.lvt.set_native_protocol(Some(protocol));
        for original in [b"k\0a".as_slice(), b"k\0b".as_slice()] {
            let (operation, operands) = context.prepare_original_variable_load(original, false);
            assert_eq!(operation, Op::LOAD_SCALAR1);
            assert_eq!(operands, vec![Operand::Imm(0)]);
        }
        assert_eq!(context.lvt.len(), 1);
        let (operation, operands) = context.prepare_original_variable_load(b"k\0bb", false);
        assert_eq!(operation, Op::LOAD_SCALAR1);
        assert_eq!(operands, vec![Operand::Imm(1)]);
        for original in [b"k\0z::x".as_slice(), b"a(k::z)".as_slice()] {
            let (operation, operands) = context.prepare_original_variable_load(original, false);
            assert_eq!(operation, Op::LOAD_STK);
            assert_eq!(operands, [] as [tcl_bytecode::Operand; 0]);
            assert!(
                context
                    .literals
                    .entries()
                    .iter()
                    .any(|entry| entry.bytes() == original)
            );
        }
        assert_eq!(context.lvt.len(), 2);
    }
}

/// `[list …]` of literal words is the list object C's `TclCompileListCmd`
/// pushes (and `[list]` its empty string), while another fold whose
/// literal a release would compile apart from its list result is left to
/// the command: on 8.6 `end-2147483649` is such a word.
#[test]
fn a_folded_list_pushes_a_list_and_a_word_read_apart_stays_unfolded() {
    let registry = CommandRegistry::build_default();
    let pushed = |value: &str| {
        let mut ctx = CodegenCtx::new(false, &[], &registry);
        assert!(ctx.try_emit_constant_fold(value), "{value}");
        ctx.instructions
            .iter()
            .find(|i| matches!(i.op, Op::PUSH1 | Op::PUSH4))
            .map(|i| i.literal_rep == tcl_bytecode::LiteralRep::List)
    };
    assert_eq!(pushed("[list a {b c}]"), Some(true));
    assert_eq!(pushed("[list]"), Some(false));
    assert_eq!(pushed("[llength [list a b]]"), Some(false));

    let profile = |name: &str| {
        Some(tcl_registry::model::ingress::resolve_environment(name).analyser_profile())
    };
    let lrange = "[lrange {string range abc 0 end-2147483649} 0 end]";
    let mut v86 = CodegenCtx::new(false, &[], &registry);
    v86.dialect = profile("tcl8.6");
    assert!(!v86.try_emit_constant_fold(lrange));
    assert!(v86.try_emit_constant_fold("[list string range abc 0 end-2147483649]"));
    let mut v90 = CodegenCtx::new(false, &[], &registry);
    v90.dialect = profile("tcl9.0");
    assert!(v90.try_emit_constant_fold(lrange));
}

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

#[test]
fn opaque_native_call_retains_unknown_accesses_without_invented_named_definitions() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let native = crate::ir::native_call_for_test(b"opaque \xff");
    assert!(native.has_opaque_native_accesses());
    assert!(is_clobber(&native, registry, Some(test_context())));
    assert!(statement_has_wildcard_aliasing(
        &native,
        registry,
        Some(test_context())
    ));
    assert_eq!(
        crate::ssa::defs_of_with_registry(&native, Some(registry)),
        [] as [std::string::String; 0]
    );
    let mut scanner =
        crate::var_refs::VarReferenceScanner::new(crate::var_refs::VarScanOptions::default());
    assert_eq!(
        crate::ssa::uses_of(&native, &mut scanner, registry),
        [] as [std::string::String; 0]
    );
    assert!(native.source_edit_span().is_none());
    assert!(crate::gvn::statement_writes_state(
        registry,
        &native,
        registry.profile()
    ));

    let mut context = crate::var_resolve::ResolveContext::for_function("::f");
    assert_eq!(
        crate::place_bridge::read_places(&native, &context, registry),
        [crate::place::unknown_top()]
    );
    assert_eq!(
        crate::place_bridge::statement_mutation_places(&native, &context, registry),
        [crate::place::unknown_top()]
    );
    assert!(!context.dynamic_bindings);
    crate::variable_bindings::transfer_statement(&mut context, &native, registry);
    assert!(context.dynamic_bindings);
    let ssa = make_ssa_with_entry_stmts(vec![native.clone()]);
    let memory = build_memory_ssa(&ssa, registry, Some(test_context()));
    assert!(memory.has_wildcard_aliasing);
    assert_eq!(memory.count_clobbers, 1);
    assert_eq!(memory.count_defs, 0);
    let script = crate::ir::Script {
        statements: vec![native],
        ..crate::ir::Script::default()
    };
    assert!(matches!(
        crate::executable_ir::build_linear_executable_ir(
            registry,
            Some(test_context()),
            crate::executable_ir::ExecutableFunctionId::new(0),
            &script,
        ),
        Err(
            crate::executable_ir::SourceCompatibilityDecline::MissingCommandTokens {
                statement_index: 0
            }
        )
    ));
}

#[test]
fn actual_memory_metadata_keeps_availability_foreign_and_missing_owner_refusal() {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let current = tcl_registry::model::ingress::static_context_for("tcl8.6");
    let registry = current.commands();
    let cu = crate::compilation_unit::CompilationUnit::build_for_dialect(
        "dict create key value",
        registry,
        false,
        "tcl8.6",
    );
    let statement = cu
        .ir_module
        .top_level
        .statements
        .iter()
        .find(|statement| {
            statement.tokens().is_some_and(|tokens| {
                tokens.synthetic.is_none()
                    && tokens.argv_texts.first().is_some_and(|head| head == "dict")
            })
        })
        .unwrap();
    assert!(!is_clobber_with_metadata_context(
        statement,
        registry,
        Some(current.into())
    ));
    let older = tcl_registry::model::ingress::static_context_for("tcl8.4")
        .with_command_store(std::sync::Arc::clone(registry));
    let foreign = tcl_registry::model::ingress::static_context_for("tcl9.1");
    for context in [Some((&older).into()), Some(foreign.into()), None] {
        assert!(is_clobber_with_metadata_context(
            statement, registry, context
        ));
        assert!(statement_has_wildcard_aliasing_with_metadata_context(
            statement, registry, context
        ));
    }
    let ssa = make_ssa_with_entry_stmts(vec![statement.clone()]);
    assert!(
        !build_memory_ssa_with_metadata_context(&ssa, registry, Some(current.into()))
            .has_wildcard_aliasing
    );
    assert!(build_memory_ssa_with_metadata_context(&ssa, registry, None).has_wildcard_aliasing);
    let mut missing = cu;
    missing.ir_module.source_metadata_input = None;
    let annotated = missing.with_memory_ssa(registry, Some(test_context()));
    assert!(
        annotated
            .top_level
            .memory_ssa
            .as_ref()
            .unwrap()
            .has_wildcard_aliasing
    );
}

fn test_context() -> SemanticContext {
    SemanticContext::for_environment("tcl8.6")
}

fn literal_tokens(command: &str, args: &[String]) -> crate::ir::CommandTokens {
    let words: Vec<String> = std::iter::once(command.to_owned())
        .chain(args.iter().cloned())
        .collect();
    let span = tcl_lexer::Span::new(0, 0);
    crate::ir::CommandTokens {
        argv: vec![span; words.len()],
        argv_texts: words.clone(),
        word_exprs: words
            .into_iter()
            .map(|text| crate::ir::WordExpr::Literal {
                text,
                source: crate::ir::SourceSite::source(span),
            })
            .collect(),
        argv_kinds: vec![tcl_lexer::TokenType::Esc; args.len().saturating_add(1)],
        single_token_word: vec![true; args.len().saturating_add(1)],
        all_tokens: vec![span; args.len().saturating_add(1)],
        expand_word: None,
        synthetic: None,
        evaluated_body: None,
        source_binding: None,
        nested_bindings: std::sync::Arc::default(),
        variable_accesses: Vec::new(),
        hosted_taint_context: None,
        source_bpf_command: None,
    }
}

#[test]
fn memory_location_display() {
    let local = MemoryLocation::new(MemoryLocationKind::Local, "x");
    assert_eq!(local.display(), "x");

    let upvar = MemoryLocation::with_qualifier(MemoryLocationKind::Upvar, "local_x", "caller_x");
    assert_eq!(upvar.display(), "upvar(caller_x -> local_x)");

    let global = MemoryLocation::new(MemoryLocationKind::Global, "g");
    assert_eq!(global.display(), "global(g)");

    let ns = MemoryLocation::with_qualifier(MemoryLocationKind::NamespaceVar, "var", "::foo");
    assert_eq!(ns.display(), "ns(::foo::var)");

    let ivar = MemoryLocation::with_qualifier(MemoryLocationKind::InstanceVar, "self_x", "MyClass");
    assert_eq!(ivar.display(), "ivar(MyClass::self_x)");

    let elem = MemoryLocation::with_qualifier(MemoryLocationKind::ArrayElement, "arr", "key");
    assert_eq!(elem.display(), "arr(key)");

    let unk = MemoryLocation::new(MemoryLocationKind::Unknown, "?");
    assert_eq!(unk.display(), "?(?)");
}

#[test]
fn alias_set_may_alias_and_names() {
    let mut locs = HashSet::new();
    locs.insert(MemoryLocation::new(MemoryLocationKind::Local, "a"));
    locs.insert(MemoryLocation::new(MemoryLocationKind::Local, "b"));
    let set = AliasSet::new(locs, "upvar");
    assert!(set.may_alias(&MemoryLocation::new(MemoryLocationKind::Local, "a")));
    assert!(!set.may_alias(&MemoryLocation::new(MemoryLocationKind::Local, "c")));
    assert!(set.contains_name("a"));
    assert!(set.contains_name("b"));
    assert!(!set.contains_name("c"));
    let names = set.names();
    assert!(names.contains("a"));
    assert!(names.contains("b"));
    assert_eq!(names.len(), 2);
}

fn call(cmd: &str, args: &[&str]) -> Statement {
    let args: Vec<String> = args.iter().map(|arg| (*arg).to_owned()).collect();
    Statement::Call {
        span: tcl_lexer::Span::new(0, 0),
        command: cmd.into(),
        canonical_command: None,
        tokens: Some(literal_tokens(cmd, &args)),
        args,
        defs: Vec::new(),
        reads: Vec::new(),
        reads_own_defs: false,
        safe_on_uninit: false,
        foreach_groups: None,
    }
}

#[test]
fn argument_boundary_omits_outer_dispatch_but_retains_nested_and_missing_read_effects() {
    let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
    let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
    for (source, clobbers) in [
        ("unknown-wrapper {[opaque]}", false),
        ("unknown-wrapper [opaque]", true),
        ("unknown-wrapper $x", true),
    ] {
        let segment = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config)
            .pop()
            .unwrap();
        let mut tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            config,
            &segment,
        );
        tokens.synthetic = Some(crate::ir::SyntheticMarker::EvaluatedArguments);
        let mut statement = call(segment.name(), &[]);
        if let Statement::Call {
            tokens: retained,
            args,
            ..
        } = &mut statement
        {
            *retained = Some(tokens);
            *args = segment.args().to_vec();
        }
        assert_eq!(is_clobber(&statement, registry, None), clobbers, "{source}");
        assert_eq!(
            transition_requires_wildcard(&statement, registry, None),
            clobbers,
            "{source}"
        );
    }
}

fn barrier(cmd: &str, args: &[&str]) -> Statement {
    Statement::Barrier {
        span: tcl_lexer::Span::new(0, 0),
        reason: "test".into(),
        command: cmd.into(),
        canonical_command: None,
        args: args.iter().map(|s| (*s).into()).collect(),
        tokens: None,
    }
}

#[test]
fn memory_op_constructors() {
    let loc = MemoryLocation::new(MemoryLocationKind::Local, "x");
    let def = MemoryOp::new_def(loc.clone(), 3, "b", 1);
    assert_eq!(def.kind, MemoryOpKind::Def);
    assert_eq!(def.version, 3);

    let uv = MemoryOp::new_use(loc.clone(), 3, "b", 2);
    assert_eq!(uv.kind, MemoryOpKind::Use);
    assert_eq!(uv.reaching_version, 3);
    assert_eq!(uv.version, 3);

    let phi = MemoryOp::new_phi(loc.clone(), 4, "join");
    assert_eq!(phi.kind, MemoryOpKind::Phi);
    assert_eq!(phi.statement_index, -1);

    let clob = MemoryOp::new_clobber(5, "b", 7);
    assert_eq!(clob.kind, MemoryOpKind::Clobber);
    assert_eq!(clob.location.kind, MemoryLocationKind::Unknown);
    assert_eq!(clob.location.name, "*");
}

#[test]
fn registry_transition_pairs_preserve_alias_layouts() {
    let registry = CommandRegistry::build_default();
    let upvar = transition_alias_pairs(
        &call("upvar", &["1", "a", "la", "b", "lb"]),
        &registry,
        Some(test_context()),
    );
    assert_eq!(upvar.len(), 2);
    assert_eq!(upvar[0].target.kind, MemoryLocationKind::Upvar);
    assert_eq!(upvar[0].target.name, "a");
    assert_eq!(upvar[0].local.name, "la");
    assert_eq!(upvar[1].target.name, "b");
    assert_eq!(upvar[1].local.name, "lb");

    let namespace = transition_alias_pairs(
        &call("namespace", &["upvar", "::scope", "other", "local"]),
        &registry,
        Some(test_context()),
    );
    assert!(matches!(
        namespace.as_slice(),
        [pair]
            if pair.target.kind == MemoryLocationKind::NamespaceVar
                && pair.target.qualifier == "::scope"
                && pair.target.name == "other"
                && pair.local.name == "local"
    ));

    let global = transition_alias_pairs(
        &call("global", &["::pkg::shared"]),
        &registry,
        Some(test_context()),
    );
    assert!(matches!(
        global.as_slice(),
        [pair]
            if pair.target.kind == MemoryLocationKind::Global
                && pair.target.name == "::pkg::shared"
                && pair.local.name == "shared"
    ));
}

#[test]
fn dynamic_or_expanded_transition_operands_require_a_wildcard_clobber() {
    let registry = CommandRegistry::build_default();
    let mut dynamic = call("global", &["name"]);
    let Statement::Call {
        tokens: Some(tokens),
        ..
    } = &mut dynamic
    else {
        panic!("test call has structured tokens");
    };
    tokens.word_exprs[1] = crate::ir::WordExpr::Variable {
        spelling: "$name".to_owned(),
        source: crate::ir::SourceSite::source(tcl_lexer::Span::new(0, 0)),
    };
    assert_eq!(
        transition_alias_pairs(&dynamic, &registry, Some(test_context())),
        [] as [crate::memory_ssa::RegistryAliasPair; 0]
    );
    assert!(transition_requires_wildcard(
        &dynamic,
        &registry,
        Some(test_context())
    ));

    let mut expanded = call("proc", &["p", "args", "body"]);
    let Statement::Call {
        tokens: Some(tokens),
        ..
    } = &mut expanded
    else {
        panic!("test call has structured tokens");
    };
    tokens.word_exprs[2] = crate::ir::WordExpr::Expand {
        source: crate::ir::SourceSite::source(tcl_lexer::Span::new(0, 0)),
        word: Box::new(crate::ir::WordExpr::Literal {
            text: "args".to_owned(),
            source: crate::ir::SourceSite::source(tcl_lexer::Span::new(0, 0)),
        }),
    };
    assert_eq!(
        transition_alias_pairs(&expanded, &registry, Some(test_context())),
        [] as [crate::memory_ssa::RegistryAliasPair; 0]
    );
    assert!(transition_requires_wildcard(
        &expanded,
        &registry,
        Some(test_context())
    ));

    let expanded_ssa = make_ssa_with_entry_stmts(vec![expanded]);
    let memory = build_memory_ssa(&expanded_ssa, &registry, Some(test_context()));
    assert!(memory.has_wildcard_aliasing);
    assert_eq!(memory.count_clobbers, 1);
}

#[test]
fn is_clobber_barrier_and_eval() {
    let reg = CommandRegistry::build_default();
    assert!(is_clobber(
        &barrier("eval", &["x"]),
        &reg,
        Some(test_context())
    ));
    assert!(is_clobber(
        &call("eval", &["script"]),
        &reg,
        Some(test_context())
    ));
    assert!(is_clobber(
        &call("uplevel", &["1", "script"]),
        &reg,
        Some(test_context())
    ));
    assert!(!is_clobber(
        &call("set", &["x", "1"]),
        &reg,
        Some(test_context())
    ));
}

/// A code-evaluating subcommand must be classified as a clobber. Matching
/// a compound spelling such as `"interp eval"` against a bare `command`
/// field never fires; the registry path instead resolves the subcommand
/// word (unique prefixes included) and reads its traits.
#[test]
fn is_clobber_resolves_code_evaluating_subcommands() {
    let reg = CommandRegistry::build_default();
    assert!(is_clobber(
        &call("interp", &["eval", "{}", "script"]),
        &reg,
        Some(test_context())
    ));
    assert!(is_clobber(
        &call("namespace", &["eval", "ns", "script"]),
        &reg,
        Some(test_context())
    ));
    let native_registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
    let native = crate::lowering::lower_to_ir("namespace current", native_registry);
    assert!(!is_clobber(
        &native.top_level.statements[0],
        native_registry,
        Some(test_context())
    ));
    // The ensemble worker is an independent dependency, absent from a
    // manually constructed public-head-only invocation.
    assert!(is_clobber(
        &call("namespace", &["current"]),
        &reg,
        Some(test_context())
    ));
    assert!(!is_clobber(
        &call("interp", &["exists", "x"]),
        &reg,
        Some(test_context())
    ));

    let mut computed_subcommand = call("namespace", &["current"]);
    let Statement::Call {
        tokens: Some(tokens),
        ..
    } = &mut computed_subcommand
    else {
        panic!("test call has structured tokens");
    };
    tokens.word_exprs[1] = crate::ir::WordExpr::Variable {
        spelling: "$operation".to_owned(),
        source: crate::ir::SourceSite::source(tcl_lexer::Span::new(0, 0)),
    };
    assert!(is_clobber(&computed_subcommand, &reg, Some(test_context())));
}

use crate::ssa::{SsaBlock, SsaStatement};

fn make_ssa_with_entry_stmts(stmts: Vec<Statement>) -> SsaFunction {
    let mut ssa_stmts: Vec<SsaStatement> = Vec::new();
    for s in stmts {
        ssa_stmts.push(SsaStatement {
            statement: s,
            uses: HashMap::new(),
            defs: HashMap::new(),
            may_defs: std::collections::HashSet::new(),
            destruction_defs: std::collections::HashSet::new(),
            quoted_uses: std::collections::HashSet::new(),
            name_only_uses: std::collections::HashSet::new(),
        });
    }
    let entry = BlockId(0);
    let mut ssa = SsaFunction::trivial("::test", entry, vec!["entry".into()]);
    ssa.blocks.insert(
        entry,
        SsaBlock {
            name: "entry".into(),
            phis: Vec::new(),
            statements: ssa_stmts,
            entry_versions: HashMap::new(),
            exit_versions: HashMap::new(),
        },
    );
    ssa.dominator_tree.insert(entry, Vec::new());
    ssa
}

#[test]
fn compute_aliases_finds_upvar_set() {
    let ssa = make_ssa_with_entry_stmts(vec![call("upvar", &["1", "caller_x", "local_x"])]);
    let sets = compute_aliases(
        &ssa,
        &CommandRegistry::build_default(),
        Some(test_context()),
    );
    assert_eq!(sets.len(), 1);
    let set = &sets[0];
    assert!(set.contains_name("caller_x"));
    assert!(set.contains_name("local_x"));
    assert!(set.reason.contains("caller-frame"));
}

#[test]
fn compute_aliases_merges_shared_caller_upvars() {
    // `upvar 1 x a` and `upvar 1 x b` alias the *same* caller
    // variable `x`, so `a` and `b` may alias each other. The
    // caller-side node is keyed on `x` alone, letting union-find
    // collapse both declarations into a single alias set.
    let ssa = make_ssa_with_entry_stmts(vec![
        call("upvar", &["1", "x", "a"]),
        call("upvar", &["1", "x", "b"]),
    ]);
    let sets = compute_aliases(
        &ssa,
        &CommandRegistry::build_default(),
        Some(test_context()),
    );
    assert_eq!(sets.len(), 1, "shared-caller upvars must merge");
    let set = &sets[0];
    assert!(set.contains_name("a"));
    assert!(set.contains_name("b"));
    assert!(set.contains_name("x"));
}

#[test]
fn compute_aliases_global_variable_pair() {
    let ssa = make_ssa_with_entry_stmts(vec![call("global", &["shared"])]);
    let sets = compute_aliases(
        &ssa,
        &CommandRegistry::build_default(),
        Some(test_context()),
    );
    assert_eq!(sets.len(), 1);
    let set = &sets[0];
    // Global and Local kinds for the same name merge.
    assert!(
        set.locations
            .iter()
            .any(|l| l.kind == MemoryLocationKind::Global)
    );
    assert!(
        set.locations
            .iter()
            .any(|l| l.kind == MemoryLocationKind::Local)
    );
    assert!(set.reason.contains("global"));
}

#[test]
fn compute_aliases_empty_when_no_aliasing_commands() {
    let ssa = make_ssa_with_entry_stmts(vec![call("set", &["x", "1"])]);
    assert_eq!(
        compute_aliases(
            &ssa,
            &CommandRegistry::build_default(),
            Some(test_context())
        )
        .len(),
        0
    );
    let memory = build_memory_ssa(
        &ssa,
        &CommandRegistry::build_default(),
        Some(test_context()),
    );
    assert!(memory.has_wildcard_aliasing);
    assert_eq!(memory.count_clobbers, 1);
}

#[test]
fn build_memory_ssa_empty_function() {
    let ssa = make_ssa_with_entry_stmts(Vec::new());
    let m = build_memory_ssa(
        &ssa,
        &CommandRegistry::build_default(),
        Some(test_context()),
    );
    assert_eq!(m.alias_sets, [] as [crate::memory_ssa::AliasSet; 0]);
    assert_eq!(m.memory_ops, [] as [crate::memory_ssa::MemoryOp; 0]);
    assert_eq!(m.count_defs, 0);
    assert_eq!(m.count_uses, 0);
    assert_eq!(m.count_clobbers, 0);
}

#[test]
fn build_memory_ssa_emits_clobber_for_eval() {
    let ssa = make_ssa_with_entry_stmts(vec![call("eval", &["foo"])]);
    let m = build_memory_ssa(
        &ssa,
        &CommandRegistry::build_default(),
        Some(test_context()),
    );
    assert_eq!(m.count_clobbers, 1);
    assert_eq!(m.memory_ops[0].kind, MemoryOpKind::Clobber);
    assert_eq!(m.memory_ops[0].location.name, "*");
}

#[test]
fn build_memory_ssa_emits_clobber_for_upframe() {
    // A static-body `uplevel 1 {…}` lowers to `Statement::UpFrame`;
    // its body runs in the caller's frame and clobbers aliased
    // memory just like the dynamic `uplevel` barrier.
    let upframe = Statement::UpFrame {
        span: tcl_lexer::Span::new(0, 0),
        frame_shift: 1,
        absolute: false,
        body: crate::ir::Script::new(),
        tokens: None,
    };
    assert!(is_clobber(
        &upframe,
        &CommandRegistry::build_default(),
        Some(test_context())
    ));
    let ssa = make_ssa_with_entry_stmts(vec![upframe]);
    let m = build_memory_ssa(
        &ssa,
        &CommandRegistry::build_default(),
        Some(test_context()),
    );
    assert_eq!(m.count_clobbers, 1);
    assert_eq!(m.memory_ops[0].kind, MemoryOpKind::Clobber);
}

#[test]
fn build_memory_ssa_tracks_aliased_def_and_use() {
    // global shared; then a statement that uses+defs shared.
    let entry = BlockId(0);
    let mut ssa = SsaFunction::trivial("::test", entry, vec!["entry".into()]);
    let shared = ssa.intern_var("shared");
    let mut stmts: Vec<SsaStatement> = Vec::new();
    stmts.push(SsaStatement {
        statement: call("global", &["shared"]),
        uses: HashMap::new(),
        defs: HashMap::new(),
        may_defs: std::collections::HashSet::new(),
        destruction_defs: std::collections::HashSet::new(),
        quoted_uses: std::collections::HashSet::new(),
        name_only_uses: std::collections::HashSet::new(),
    });
    let mut defs = HashMap::new();
    defs.insert(shared, 1);
    stmts.push(SsaStatement {
        statement: call("set", &["shared", "1"]),
        uses: HashMap::new(),
        defs,
        may_defs: std::collections::HashSet::new(),
        destruction_defs: std::collections::HashSet::new(),
        quoted_uses: std::collections::HashSet::new(),
        name_only_uses: std::collections::HashSet::new(),
    });
    let mut uses = HashMap::new();
    uses.insert(shared, 1);
    stmts.push(SsaStatement {
        statement: call("puts", &["$shared"]),
        uses,
        defs: HashMap::new(),
        may_defs: std::collections::HashSet::new(),
        destruction_defs: std::collections::HashSet::new(),
        quoted_uses: std::collections::HashSet::new(),
        name_only_uses: std::collections::HashSet::new(),
    });

    ssa.blocks.insert(
        entry,
        SsaBlock {
            name: "entry".into(),
            phis: Vec::new(),
            statements: stmts,
            entry_versions: HashMap::new(),
            exit_versions: HashMap::new(),
        },
    );
    ssa.dominator_tree.insert(entry, Vec::new());
    let m = build_memory_ssa(
        &ssa,
        &CommandRegistry::build_default(),
        Some(test_context()),
    );
    assert_eq!(m.count_defs, 1);
    assert_eq!(m.count_uses, 1);
    // Version for def must be > version visible at preceding
    // `global` statement (which emitted no memory op).
    let def_op = m
        .memory_ops
        .iter()
        .find(|o| o.kind == MemoryOpKind::Def)
        .expect("def present");
    let load_op = m
        .memory_ops
        .iter()
        .find(|o| o.kind == MemoryOpKind::Use)
        .expect("use present");
    assert!(
        load_op.reaching_version > def_op.version,
        "an unstamped invocation must preserve its wildcard clobber"
    );
}

#[test]
fn self_referential_aliased_write_reads_incoming_version() {
    // `global shared`; `set shared 0`; then a self-referential
    // `set shared [expr {$shared + 1}]` that both reads and writes `shared`
    // in one statement. The read must carry the version *reaching* the
    // statement, strictly below the version the statement defines — tagging
    // it with the post-def counter would record the write as its own
    // reaching def.
    let entry = BlockId(0);
    let mut ssa = SsaFunction::trivial("::test", entry, vec!["entry".into()]);
    let shared = ssa.intern_var("shared");
    let mut stmts: Vec<SsaStatement> = Vec::new();
    stmts.push(SsaStatement {
        statement: call("global", &["shared"]),
        uses: HashMap::new(),
        defs: HashMap::new(),
        may_defs: std::collections::HashSet::new(),
        destruction_defs: std::collections::HashSet::new(),
        quoted_uses: std::collections::HashSet::new(),
        name_only_uses: std::collections::HashSet::new(),
    });
    let mut defs1 = HashMap::new();
    defs1.insert(shared, 1);
    stmts.push(SsaStatement {
        statement: call("set", &["shared", "0"]),
        uses: HashMap::new(),
        defs: defs1,
        may_defs: std::collections::HashSet::new(),
        destruction_defs: std::collections::HashSet::new(),
        quoted_uses: std::collections::HashSet::new(),
        name_only_uses: std::collections::HashSet::new(),
    });
    let mut uses = HashMap::new();
    uses.insert(shared, 1);
    let mut defs2 = HashMap::new();
    defs2.insert(shared, 2);
    stmts.push(SsaStatement {
        statement: call("set", &["shared", "[expr {$shared + 1}]"]),
        uses,
        defs: defs2,
        may_defs: std::collections::HashSet::new(),
        destruction_defs: std::collections::HashSet::new(),
        quoted_uses: std::collections::HashSet::new(),
        name_only_uses: std::collections::HashSet::new(),
    });
    ssa.blocks.insert(
        entry,
        SsaBlock {
            name: "entry".into(),
            phis: Vec::new(),
            statements: stmts,
            entry_versions: HashMap::new(),
            exit_versions: HashMap::new(),
        },
    );
    ssa.dominator_tree.insert(entry, Vec::new());
    let m = build_memory_ssa(
        &ssa,
        &CommandRegistry::build_default(),
        Some(test_context()),
    );
    // The def and use from the self-referential statement (index 2).
    let self_def = m
        .memory_ops
        .iter()
        .find(|o| o.kind == MemoryOpKind::Def && o.statement_index == 2)
        .expect("self-ref def");
    let self_use = m
        .memory_ops
        .iter()
        .find(|o| o.kind == MemoryOpKind::Use && o.statement_index == 2)
        .expect("self-ref use");
    assert!(
        self_use.reaching_version < self_def.version,
        "use must read the pre-write version (use {}, def {})",
        self_use.reaching_version,
        self_def.version,
    );
}

#[test]
fn canonical_memory_keys_preserve_native_incarnation_and_member_lifetime() {
    use crate::command_binding::SourceNamespaceKey;
    use crate::place::{CellGeneration, CellOwner};
    use tcl_runtime_api::native_compilation::{NativeInterpreterIdentity, NativeNamespaceContext};
    let interpreter = NativeInterpreterIdentity {
        owner: NativeInterpreterIdentity::fresh_owner(),
        interpreter: 0,
    };
    let mut first = crate::place::scalar("x", "::same", false);
    first.cell = Some(crate::place::CellIdentity {
        owner: CellOwner::NamespaceIdentity(Box::new(SourceNamespaceKey::Native(
            NativeNamespaceContext {
                interpreter,
                token: 1,
                path: tcl_core_types::ByteNamespacePath::from_segments(["same"]),
            },
        ))),
        name: "x".into(),
        generation: CellGeneration::Incoming,
        interpreter: None,
        storage_domain: None,
        execution: None,
    });
    let mut recreated = first.clone();
    let CellOwner::NamespaceIdentity(namespace) = &mut recreated.cell.as_mut().unwrap().owner
    else {
        unreachable!();
    };
    let SourceNamespaceKey::Native(context) = namespace.as_mut() else {
        unreachable!();
    };
    context.token = 2;
    let mut later_lifetime = first.clone();
    later_lifetime.cell.as_mut().unwrap().generation = CellGeneration::After(9);
    let locations: HashSet<_> = [&first, &recreated, &later_lifetime]
        .into_iter()
        .map(memory_location_of_place)
        .collect();
    assert_eq!(locations.len(), 3);
    assert_ne!(
        memory_location_of_place(&first).storage_key,
        memory_location_of_place(&recreated).storage_key
    );
    assert_ne!(
        memory_location_of_place(&first).storage_key,
        memory_location_of_place(&later_lifetime).storage_key
    );
    let name = memory_location_of_place(&first).name;
    let aliases = AliasSet::new(locations, "native");
    assert!(
        !aliases.contains_name(&name),
        "presentation cannot query native alias facts"
    );
}

#[test]
fn memory_location_ordering_stable() {
    // Presentation remains ordered independently of the exact key inventory.
    let mut set = HashSet::new();
    set.insert(MemoryLocation::new(MemoryLocationKind::Local, "z"));
    set.insert(MemoryLocation::new(MemoryLocationKind::Local, "a"));
    set.insert(MemoryLocation::new(MemoryLocationKind::Global, "m"));
    let aliases = AliasSet::new(set, "presentation");
    let names: Vec<_> = aliases
        .ordered_locations()
        .iter()
        .map(|l| l.name.clone())
        .collect();
    assert_eq!(
        names,
        vec!["a".to_string(), "z".to_string(), "m".to_string()]
    );
}

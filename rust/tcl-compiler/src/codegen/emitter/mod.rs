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

//! Main emitter loop and public API.
//!
//! Split across multiple files by responsibility:
//!
//! - [`ordering`]  — CFG linearisation and loop body detection
//! - [`terminator`] — CFG terminator emission (goto/branch/return)
//! - [`loop_blocks`] — per-block handlers for foreach/while/for
//! - [`try_blocks`] — try/finally CFG pattern detection
//! - [`generate`] — top-level dispatcher
//! - [`bytecoded`] — registry-backed codegen hook dispatch

#![allow(dead_code)]

pub mod bytecoded;
pub mod generate;
pub mod loop_blocks;
pub mod ordering;
pub mod terminator;
pub mod try_blocks;

use std::collections::HashMap;

use tcl_registry::CommandRegistry;

use crate::cfg::{CfgModule, Function as CfgFunction};
use crate::ir::Module as IrModule;

use super::{CodegenCtx, FunctionAsm, ModuleAsm};

/// Generate bytecode assembly for a single CFG function.
/// Procedure declarations are emitted from retained source statements; module
/// lowering carries their source, namespace context and body inventory.
///
/// When `is_proc` is true, variables are accessed via the LVT
/// (`loadScalar1`/`storeScalar1`). When `false` (top-level scripts),
/// variables are accessed via the stack (`loadStk`/`storeStk`).
/// `registry` is consulted for codegen-hook resolution; pass the
/// same instance the lowering pass used so dialect-loaded specs
/// are visible.
///
/// # Panics
/// Rejects analysis-only completion summaries. Supply a CFG built through
/// [`crate::cfg_builder::build_cfg_codegen_with_registry_and_config`], which
/// retains runtime invocation continuations.
#[must_use]
pub fn codegen_function(
    cfg: &CfgFunction,
    params: &[&str],
    is_proc: bool,
    registry: &CommandRegistry,
) -> FunctionAsm {
    let mut ctx = CodegenCtx::new(is_proc, params, registry);
    generate::generate(&mut ctx, cfg)
}

/// Emission uses the same body inventory selected before source analysis.
pub use tcl_runtime_api::SourceCompilationScope as ModuleEmissionScope;

#[derive(Clone, Copy)]
struct ModuleTarget<'a> {
    parameters: &'a [tcl_runtime_api::NameBytes],
    is_procedure: bool,
    scope: ModuleEmissionScope,
}

/// Registry, original source and grammar shared by one module's functions.
struct ModuleEmit<'a> {
    registry: &'a CommandRegistry,
    source: tcl_lexer::SourceImage,
    line_index: tcl_lexer::LineIndex,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    numbers: tcl_dialect::NumberSyntax,
    escapes: tcl_dialect::EscapeSyntax,
    braced_var: tcl_dialect::BracedVarStyle,
    word_rules: tcl_syntax::word_rules::WordValueRules,
    expr_grammar: Option<tcl_dialect::LexerGrammar>,
    lexer_config: tcl_lexer::LexerConfig,
    invocation_dialect: Option<tcl_registry::InvocationDialect>,
    compiled_variable_protocol: Option<tcl_syntax::naming::NativeCompiledVariableProtocol>,
    compiled_local_layout: Option<tcl_runtime_api::native_compilation::NativeCompiledLocalLayout>,
    source_string_protocol: Option<tcl_syntax::native_string::NativeStringProtocol>,
    native_entry: Option<&'a tcl_runtime_api::NativeCompilationEntry>,
    /// The unit's command-binding summary — see
    /// [`CodegenCtx::command_bindings`](crate::codegen::CodegenCtx::command_bindings).
    /// Scanned once per module, not once per function.
    command_bindings: &'a crate::command_binding::ModuleCommandMutations,
    plain_command_dispatch: bool,
    native_compilation: tcl_registry::native_compilation::NativeCompilationContext,
    source_proofs: std::sync::Arc<crate::command_binding::BodySourceProofs>,
    /// The pack commands' definitions the module's calls were inlined from:
    /// each procedure binding of a function that names one is claimed.
    references: &'a [crate::ir::ReferenceImport],
    /// The spans of the top level's own commands where the release evaluates
    /// them without compiling them ([`IrModule::top_level_evaluation`]).
    direct_commands: Vec<tcl_lexer::Span>,
}

/// Retain original module source so
/// each instruction carries its command's surface text for `errorInfo`, plus
/// the proc body's `base_line` (its `proc` definition line) so the
/// `(procedure … line N)` frame reports a proc-relative line.
#[must_use]
fn configured_codegen_context<'a>(
    params: &[tcl_runtime_api::NameBytes],
    is_proc: bool,
    module: &ModuleEmit<'a>,
    namespace: &tcl_runtime_api::ByteNamespacePath,
) -> CodegenCtx<'a> {
    let mut ctx = CodegenCtx::with_native_parameters(is_proc, params, module.registry);
    ctx.set_resolution_namespace_path(namespace.clone());
    ctx.numbers = module.numbers;
    ctx.escapes = module.escapes;
    ctx.braced_var = module.braced_var;
    ctx.word_rules = module.word_rules;
    ctx.expr_grammar = module.expr_grammar;
    ctx.dialect = module.dialect;
    ctx.ingress_lexer_config = Some(module.lexer_config);
    ctx.invocation_dialect = module.invocation_dialect;
    ctx.compiled_variable_protocol = module.compiled_variable_protocol;
    ctx.source_string_protocol = module.source_string_protocol;
    ctx.native_entry = module.native_entry;
    if !is_proc
        && module.compiled_variable_protocol.is_some_and(|protocol| {
            protocol.supports_environment(
                tcl_syntax::naming::NativeCompiledVariableEnvironment::BorrowFrameSlots,
            )
        })
        && let Some(layout) = module.compiled_local_layout.as_ref()
    {
        ctx.lvt = tcl_bytecode::LocalVarTable::from_native_slot_names(&layout.names);
        ctx.borrowed_local_layout = Some(layout.clone());
    }
    ctx.lvt
        .set_native_protocol(module.compiled_variable_protocol);
    ctx.command_bindings = Some(module.command_bindings);
    ctx.plain_command_dispatch = module.plain_command_dispatch;
    ctx.source_proofs = Some(std::sync::Arc::clone(&module.source_proofs));
    ctx.native_compilation = tcl_registry::native_compilation::NativeCompilationContext {
        mode: if is_proc {
            tcl_registry::native_compilation::NativeCompilationMode::BytecodeObject
        } else {
            module.native_compilation.mode
        },
        frame: if is_proc {
            tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode
        } else {
            module.native_compilation.frame
        },
        ..module.native_compilation
    };
    if !is_proc {
        ctx.direct_commands.clone_from(&module.direct_commands);
    }
    ctx.set_indexed_source(module.source.clone(), module.line_index.clone());
    ctx
}

#[derive(Clone, Copy)]
struct FunctionNamespace<'a> {
    path: &'a tcl_runtime_api::ByteNamespacePath,
    selected_procedure: bool,
}

fn codegen_function_src(
    cfg: &CfgFunction,
    params: &[tcl_runtime_api::NameBytes],
    is_proc: bool,
    module: &ModuleEmit<'_>,
    base_line: u32,
    scope: FunctionNamespace<'_>,
) -> FunctionAsm {
    let mut ctx = configured_codegen_context(params, is_proc, module, scope.path);
    let mut asm = generate::generate(&mut ctx, cfg);
    if let Some(environment) =
        super::native_compiler_pass::replay_environment(&ctx, scope.selected_procedure)
    {
        let mut replay = configured_codegen_context(params, is_proc, module, scope.path);
        replay.lvt = std::mem::take(&mut asm.lvt);
        replay.compact_compiler_pass = true;
        let first_literals = std::mem::take(&mut asm.literals);
        drop(asm);
        asm = generate::generate(&mut replay, cfg);
        asm.literals.retain_discarded_native_pass(first_literals);
        asm.literals.retain_compiler_replay_environment(environment);
    }
    asm.body_base_line = base_line;
    claim_reference_bodies(&mut asm, module.references);
    asm
}

/// Record, beside each procedure binding of `asm` that names a definition
/// copied from a pack, the claim on the pack's facts and on the backing that
/// makes the definition the command ([`crate::inlining::inline_reference_bodies`]).
fn claim_reference_bodies(asm: &mut FunctionAsm, references: &[crate::ir::ReferenceImport]) {
    if references.is_empty() {
        return;
    }
    asm.site_claims.extend(
        asm.procedure_bindings
            .iter()
            .filter_map(|binding| crate::inlining::reference::claim_for(references, binding)),
    );
    asm.site_claims.sort();
    asm.site_claims.dedup();
}

/// Resolve the compile's dialect name to the profile [`ModuleEmit`] carries.
///
/// `map(by_name)`, deliberately — **not** `and_then(DialectProfile::find)`.
/// Every *named* dialect must stay `Some` here. `find` answers `None` for any
/// name without a catalogue entry of its own (`tk`, an additive command
/// surface, and every unrecognised string), while `by_name` sinks those to the
/// permissive plain-Tcl profile and stays `Some`.
///
/// The distinction is load-bearing because the readers branch on `is_some()`,
/// not on which profile came back: `parse_expr_for_profile` in
/// [`control_flow`](crate::codegen::control_flow) and
/// [`cmd_subst`](crate::codegen::cmd_subst) selects the *target* numeral
/// grammar when this is `Some` and the *thread-ambient* one when it is `None`.
/// Resolving with `find` therefore moves a `tk` (or unknown-dialect) compile's
/// re-parsed `expr` bodies onto whatever grammar the host thread happens to
/// have installed — a silent, host-dependent change of meaning for a literal
/// like `010`. `None` here must mean "this compile named no dialect", nothing
/// else. This reproduces the pre-refactor `parse_expr(&str)` boundary.
fn emit_profile(dialect: Option<&str>) -> Option<&'static tcl_dialect::DialectProfile> {
    dialect.map(|name| crate::environment_ingress::resolve_environment(name).analyser_profile())
}

/// Generate bytecode assembly for an entire module.
///
/// # Errors
/// Returns an unsupported compilation when the original top-level source
/// namespace is absent or no longer matches its retained producer.
pub fn codegen_module(
    cfg_module: &CfgModule,
    ir_module: &IrModule,
    registry: &CommandRegistry,
) -> Result<ModuleAsm, tcl_runtime_api::CompileError> {
    let command_mutations =
        crate::command_binding::scan_module_command_mutations(ir_module, registry);
    codegen_module_with_command_mutations(cfg_module, ir_module, registry, &command_mutations)
}

/// Generate bytecode assembly while reusing a whole-module command-mutation
/// summary already owned by the caller's compilation unit.
///
/// [`codegen_module`] remains the compatibility entry point for callers that
/// only retain IR; it computes the same summary once before delegating here.
///
/// # Errors
/// Returns an unsupported compilation when the original source namespace is unavailable.
pub fn codegen_module_with_command_mutations(
    cfg_module: &CfgModule,
    ir_module: &IrModule,
    registry: &CommandRegistry,
    command_mutations: &crate::command_binding::ModuleCommandMutations,
) -> Result<ModuleAsm, tcl_runtime_api::CompileError> {
    codegen_module_with_emission_scope(
        cfg_module,
        ir_module,
        registry,
        command_mutations,
        ModuleEmissionScope::WholeModule,
    )
}

/// Emit a script artifact with an explicit source-body ownership policy.
/// Runtime targets use `EnteredSource`; no nested body is compiled or imported.
///
/// # Errors
/// Returns an unsupported compilation when the source namespace is unavailable.
pub fn codegen_module_with_emission_scope(
    cfg_module: &CfgModule,
    ir_module: &IrModule,
    registry: &CommandRegistry,
    command_mutations: &crate::command_binding::ModuleCommandMutations,
    scope: ModuleEmissionScope,
) -> Result<ModuleAsm, tcl_runtime_api::CompileError> {
    codegen_module_with_top_context(
        cfg_module,
        ir_module,
        ModuleTarget {
            parameters: &[],
            is_procedure: false,
            scope,
        },
        registry,
        command_mutations,
    )
}

/// Generate a bytecode module whose outer function is a procedure body.
/// Nested procedure definitions remain ordinary entries in `procedures`.
///
/// # Errors
/// Returns an unsupported compilation when the original source namespace is unavailable.
pub fn codegen_procedure_module(
    cfg_module: &CfgModule,
    ir_module: &IrModule,
    params: &[&str],
    registry: &CommandRegistry,
) -> Result<ModuleAsm, tcl_runtime_api::CompileError> {
    let command_mutations =
        crate::command_binding::scan_module_command_mutations(ir_module, registry);
    codegen_procedure_module_with_command_mutations(
        cfg_module,
        ir_module,
        params,
        registry,
        &command_mutations,
    )
}

/// Generate a procedure-body bytecode module while reusing the caller's
/// whole-module command-mutation summary.
///
/// # Errors
/// Returns an unsupported compilation when the original source namespace is unavailable.
pub fn codegen_procedure_module_with_command_mutations(
    cfg_module: &CfgModule,
    ir_module: &IrModule,
    params: &[&str],
    registry: &CommandRegistry,
    command_mutations: &crate::command_binding::ModuleCommandMutations,
) -> Result<ModuleAsm, tcl_runtime_api::CompileError> {
    let parameters: Vec<_> = params
        .iter()
        .map(|name| tcl_runtime_api::NameBytes::from(*name))
        .collect();
    codegen_procedure_module_with_native_parameters_and_command_mutations(
        cfg_module,
        ir_module,
        &parameters,
        registry,
        command_mutations,
    )
}

/// Generate a procedure module using the actual bound native formal keys.
/// Every key enters the same LVT owner before instruction emission.
///
/// # Errors
/// Returns an unsupported compilation when the original source namespace is unavailable.
pub fn codegen_procedure_module_with_native_parameters_and_command_mutations(
    cfg_module: &CfgModule,
    ir_module: &IrModule,
    params: &[tcl_runtime_api::NameBytes],
    registry: &CommandRegistry,
    command_mutations: &crate::command_binding::ModuleCommandMutations,
) -> Result<ModuleAsm, tcl_runtime_api::CompileError> {
    codegen_procedure_module_with_emission_scope(
        cfg_module,
        ir_module,
        params,
        registry,
        command_mutations,
        ModuleEmissionScope::WholeModule,
    )
}

/// Emit the actual bound procedure body with an explicit source-body policy.
/// `EnteredSource` leaves nested procedure definitions to runtime activation.
///
/// # Errors
/// Returns an unsupported compilation when the source namespace is unavailable.
pub fn codegen_procedure_module_with_emission_scope(
    cfg_module: &CfgModule,
    ir_module: &IrModule,
    params: &[tcl_runtime_api::NameBytes],
    registry: &CommandRegistry,
    command_mutations: &crate::command_binding::ModuleCommandMutations,
    scope: ModuleEmissionScope,
) -> Result<ModuleAsm, tcl_runtime_api::CompileError> {
    codegen_module_with_top_context(
        cfg_module,
        ir_module,
        ModuleTarget {
            parameters: params,
            is_procedure: true,
            scope,
        },
        registry,
        command_mutations,
    )
}

struct EmittedProcedures {
    functions: HashMap<crate::ir::ProcedureId, FunctionAsm>,
    provenance: HashMap<crate::ir::ProcedureId, tcl_bytecode::ProcedureProvenance>,
}

fn codegen_procedures(
    cfg_module: &CfgModule,
    ir_module: &IrModule,
    module: &ModuleEmit<'_>,
) -> EmittedProcedures {
    let mut functions = HashMap::new();
    let mut provenance = HashMap::new();
    for (qname, cfg_func) in &cfg_module.procedures {
        let Some(procedure) = ir_module
            .procedures
            .get(qname)
            .filter(|_| ir_module.procedure_ids.owns(*qname))
        else {
            continue;
        };
        let ir_proc = Some(procedure);
        // Skip procs defined inside namespace eval — tclsh compiles
        // them lazily at runtime, not at compile time.
        if let Some(p) = ir_proc
            && p.namespace_scoped
        {
            continue;
        }
        let params: Vec<tcl_runtime_api::NameBytes> = ir_proc
            .map(|p| {
                p.params
                    .iter()
                    .map(tcl_runtime_api::NameBytes::from)
                    .collect()
            })
            .unwrap_or_default();
        // The proc's definition line drives proc-relative `errorInfo` lines.
        let base_line = ir_proc.map_or(0, |p| {
            module
                .line_index
                .position_at(p.span.start())
                .line
                .saturating_add(1)
        });
        let namespace_context =
            ir_proc.and_then(|procedure| procedure.body.namespace_context.as_deref());
        let Some(procedure_namespace) = namespace_context
            .and_then(crate::command_binding::SourceNamespaceKey::source_artifact_path)
        else {
            continue;
        };
        let mut asm = codegen_function_src(
            cfg_func,
            &params,
            true,
            module,
            base_line,
            FunctionNamespace {
                path: &procedure_namespace,
                selected_procedure: false,
            },
        );
        // Retain the original body word value for runtime body correspondence.
        // Braced backslash-newline continuations fold under this module's rules.
        asm.proc_body_src = ir_proc
            .and_then(|p| p.body_source.as_deref())
            .map(|body| module.word_rules.collapse_braced_word(body).into_owned());
        functions.insert(*qname, asm);
        if let Some(proc) = ir_proc
            && let Some(body) = &proc.body_source
        {
            provenance.insert(
                *qname,
                tcl_bytecode::ProcedureProvenance {
                    name: proc.qualified_name.clone(),
                    namespace_context: namespace_context
                        .and_then(crate::command_binding::SourceNamespaceKey::to_compiled_context),
                    parameters: proc.params_raw.clone(),
                    body: body.clone(),
                },
            );
        }
    }
    EmittedProcedures {
        functions,
        provenance,
    }
}
fn module_source_proofs(
    ir_module: &IrModule,
    scope: ModuleEmissionScope,
) -> crate::command_binding::BodySourceProofs {
    let mut proofs = crate::command_binding::BodySourceProofs::from_body(&ir_module.top_level);
    if scope == ModuleEmissionScope::WholeModule {
        for procedure in ir_module.procedures.values() {
            proofs.tokens.extend(
                crate::command_binding::BodySourceProofs::from_body(&procedure.body).tokens,
            );
        }
    }
    proofs
}

fn module_emission_context<'a>(
    ir_module: &'a IrModule,
    scope: ModuleEmissionScope,
    registry: &'a CommandRegistry,
    command_mutations: &'a crate::command_binding::ModuleCommandMutations,
) -> ModuleEmit<'a> {
    let src = &ir_module.source;
    let source = src.clone();
    let line_index = tcl_lexer::LineIndex::from_bytes(source.bytes());
    // The compile's target release: a named dialect's own numeric grammar, else
    // the permissive 9.x default.
    // One grammar, resolved once from the name, and every axis read off it
    // — so the numerals codegen emits and the numerals it re-parses `expr`
    // bodies under are the same value by construction.
    let profile = ir_module.resolved_profile();
    let base_grammar = ir_module.source_entry.invocation_dialect.map_or_else(
        || {
            profile.map_or_else(
                || tcl_dialect::grammar_of_dialect_name(None),
                |profile| profile.grammar,
            )
        },
        |native| native.lexer_grammar,
    );
    let grammar = ir_module.native_lexer_config().grammar_over(base_grammar);
    let numbers = grammar.numbers;
    let escapes = grammar.escapes;
    let braced_var = grammar.braced_var;
    let word_rules = tcl_syntax::word_rules::WordValueRules::from_grammar(&grammar);
    let expr_grammar = Some(grammar);
    let source_proofs = module_source_proofs(ir_module, scope);
    ModuleEmit {
        registry,
        source,
        line_index,
        dialect: profile,
        numbers,
        escapes,
        braced_var,
        word_rules,
        expr_grammar,
        lexer_config: ir_module.native_lexer_config(),
        invocation_dialect: ir_module.source_entry.invocation_dialect,
        compiled_variable_protocol: ir_module
            .source_entry
            .options()
            .compiled_variable_protocol(),
        compiled_local_layout: ir_module
            .source_entry
            .native_entry
            .as_ref()
            .and_then(|entry| entry.compiled_local_layout.clone()),
        source_string_protocol: match ir_module.source_entry.native_entry.as_ref() {
            Some(entry) => entry.source_string_protocol,
            None => ir_module
                .source_entry
                .invocation_dialect
                .and_then(tcl_registry::InvocationDialect::native_source_string_protocol),
        },
        native_entry: ir_module.source_entry.native_entry.as_deref(),
        command_bindings: command_mutations,
        plain_command_dispatch: ir_module.plain_command_dispatch,
        native_compilation: ir_module.source_entry.native_compilation,
        source_proofs: std::sync::Arc::new(source_proofs),
        references: &ir_module.reference_bodies.imports,
        direct_commands: if ir_module.top_level_evaluation == crate::ir::CommandEvaluation::Direct {
            ir_module
                .top_level
                .statements
                .iter()
                .map(crate::ir::Statement::span)
                .collect()
        } else {
            Vec::new()
        },
    }
}

fn codegen_module_with_top_context(
    cfg_module: &CfgModule,
    ir_module: &IrModule,
    target: ModuleTarget<'_>,
    registry: &CommandRegistry,
    command_mutations: &crate::command_binding::ModuleCommandMutations,
) -> Result<ModuleAsm, tcl_runtime_api::CompileError> {
    let module = module_emission_context(ir_module, target.scope, registry, command_mutations);
    let profile = module.dialect;
    let namespace = match ir_module.retained_source_bindings.as_deref() {
        Some(owner) => owner
            .original_top_namespace_for_module(ir_module, registry)
            .and_then(crate::command_binding::SourceNamespaceKey::source_artifact_path),
        None => ir_module
            .original_byte_source_namespace_for_codegen(registry)
            .cloned(),
    }
    .ok_or_else(|| {
        tcl_runtime_api::CompileError::Unsupported(
            "original top-level source namespace is unavailable".to_owned(),
        )
    })?;
    let top = codegen_function_src(
        &cfg_module.top_level,
        target.parameters,
        target.is_procedure,
        &module,
        0,
        FunctionNamespace {
            path: &namespace,
            selected_procedure: target.is_procedure,
        },
    );
    // The same top level as a *procedure body*. A body compiled at run time
    // (`proc` on a cache miss, an `apply` lambda, a method) reaches the
    // compiler as a bare script, so without this it would run script-shaped and
    // lose every `is_proc` specialisation its AOT-compiled twin gets — see
    // [`ModuleAsm::top_level_body`]. A procedure-target compile already has
    // that shape, including its seeded parameter slots.
    let top_body = if target.is_procedure {
        top.clone()
    } else {
        codegen_function_src(
            &cfg_module.top_level,
            target.parameters,
            true,
            &module,
            0,
            FunctionNamespace {
                path: &namespace,
                selected_procedure: false,
            },
        )
    };
    let procedures = match target.scope {
        ModuleEmissionScope::WholeModule => codegen_procedures(cfg_module, ir_module, &module),
        ModuleEmissionScope::EnteredSource => EmittedProcedures {
            functions: HashMap::new(),
            provenance: HashMap::new(),
        },
    };
    let mut asm = ModuleAsm {
        profile: profile.unwrap_or_else(tcl_dialect::DialectProfile::plain_tcl),
        source: ir_module.own_source(),
        source_namespace: namespace,
        plain_command_dispatch: ir_module.plain_command_dispatch,
        top_level: top,
        top_level_body: top_body,
        procedures: procedures.functions,
        procedure_provenance: procedures.provenance,
        manifest: None,
    };
    asm.manifest = Some(std::sync::Arc::new(module_manifest(&asm)));
    Ok(asm)
}

/// What `module` says about the world it was compiled for: the context of the
/// profile it carries, the pack facts its sites claim, and this build's
/// intrinsic table. The runtime's pin states the same thing in the same
/// shape (`tcl_runtime_api::RuntimeContext::identity`), so the two compare.
pub(crate) fn module_manifest(module: &ModuleAsm) -> tcl_runtime_api::ArtefactIdentityManifest {
    tcl_registry::model::runtime_context_for_profile(module.profile).identity(
        &module.claimed_packs(),
        tcl_registry::intrinsic_table_hash(),
    )
}

#[cfg(test)]
mod tests {
    use crate::interprocedural::test_body_identity::fixture_id;

    use super::emit_profile;

    /// Only an *unnamed* compile may reach the readers as `None`.
    ///
    /// `None` selects the thread-ambient numeral grammar over the compile's
    /// target grammar, so any named dialect answering `None` here silently
    /// re-reads literals like `010` under whatever the host thread installed.
    #[test]
    fn every_named_dialect_resolves_to_some_profile() {
        assert!(
            emit_profile(None).is_none(),
            "a compile that named no dialect stays `None`"
        );

        for name in [
            "tcl8.4",
            "tcl8.6",
            "tcl9.0",
            "f5-irules",
            "expect",
            // The regression cases: `tk` is an additive command surface with
            // no catalogue entry, and an unrecognised name is not a licence to
            // fall back to the ambient grammar either. `DialectProfile::find`
            // answers `None` for both.
            "tk",
            "not-a-real-dialect",
        ] {
            assert!(
                emit_profile(Some(name)).is_some(),
                "named dialect {name:?} must resolve to a profile, not to the \
                 thread-ambient grammar"
            );
        }
    }

    #[test]
    fn procedure_emission_keeps_body_geometry_separate_from_its_display_name() {
        use crate::command_binding::{
            AllocationIncarnation, CommandAllocationSite, SourceNamespaceKey, SourceOriginId,
        };
        use crate::ir::{Script, Statement};
        use std::sync::Arc;
        use tcl_core_types::ByteNamespacePath;

        let registry = tcl_registry::CommandRegistry::build_default();
        let mut paths = Vec::new();
        for path in [
            ByteNamespacePath::from_segments(["a:", "b"]),
            ByteNamespacePath::from_segments(["a", ":b"]),
        ] {
            // A geometry-only artifact transport control supplies no native
            // token, original dispatch or compiler-admission receipt.
            let source = "proc p {} {return}";
            let mut ir = crate::lowering::lower_to_ir(source, &registry);
            let mut procedure = {
                let id = fixture_id(&ir.procedures, "::p");
                ir.procedures.remove(&id)
            }
            .unwrap();
            procedure.qualified_name = "::a:::b::p".to_owned();
            procedure.body = Script::new();
            procedure.body.namespace_context = Some(Box::new(SourceNamespaceKey::Allocated {
                site: CommandAllocationSite {
                    source: Arc::new(SourceOriginId::authored(&Arc::from(source))),
                    offset: 0,
                },
                incarnation: AllocationIncarnation::First,
                path: path.clone(),
            }));
            procedure.body.statements.push(Statement::Return {
                span: tcl_lexer::Span::new(11, 17),
                tokens: None,
                value: None,
                value_word: None,
                expr: None,
                expr_base: None,
                command_binding: None,
                braced: true,
            });
            let procedure_id = ir.retain_procedure(procedure);
            let cfg = crate::cfg_builder::build_cfg_codegen_with_registry(&ir, false, &registry);
            let module =
                super::codegen_module(&cfg, &ir, &registry).expect("original source namespace");
            let context = module.procedure_provenance[&procedure_id]
                .namespace_context
                .as_ref()
                .unwrap();
            assert_eq!(context.path(), &path);
            assert!(matches!(
                context,
                tcl_runtime_api::CompiledNamespaceContext::ConstructedPath(_)
            ));
            let instructions = &module
                .procedures
                .get(&fixture_id(&module.procedures, "::a:::b::p"))
                .expect("authored fixture body")
                .instructions;
            let source_instructions: Vec<_> = instructions
                .iter()
                .filter(|instruction| !instruction.source_cmd_text.is_empty())
                .collect();
            assert!(!source_instructions.is_empty());
            assert!(source_instructions.iter().all(|instruction| {
                instruction.source_cmd_text.bytes() == b"return"
                    && instruction.source_command_namespace == path
            }));
            paths.push(context.path().clone());
        }
        assert_ne!(paths[0], paths[1]);
    }
    /// A name `find` rejects still picks the compile's *target* numeral
    /// grammar, not the ambient one — and the target is the name's **own**
    /// point, not a permissive fallback.
    ///
    /// `tk` has no catalogue row; its grammar is the `tk` environment's
    /// 8.6 core, so a `tk` compile reads `010` as octal 8 — under 8.6's
    /// numerals — whatever grammar happens to be installed on the thread.
    /// `emit_profile("tk")` is still the anonymous fallback profile *by
    /// design* (the cache-key and help-filter reasons on
    /// `DocumentEnvironment::analyser_profile`), which is exactly why the
    /// compile does not take its numerals from that profile: it takes them,
    /// and its `expr` re-parse grammar, from `grammar_of_dialect_name`, so
    /// the two cannot disagree inside one compile as they did for `tk`.
    #[test]
    fn an_uncatalogued_dialect_keeps_the_target_numeral_grammar() {
        use tcl_dialect::NumberSyntax;

        // Thread-local, but `cargo test` may run this thread again for another
        // test, so restore it the way `number.rs`'s own ambient tests do.
        let restore = tcl_syntax::number::runtime_syntax();
        tcl_syntax::number::set_runtime_syntax(NumberSyntax::Tcl84);
        assert_eq!(
            tcl_syntax::number::runtime_syntax(),
            NumberSyntax::Tcl84,
            "the ambient grammar must actually be installed for this to pin \
             anything"
        );

        let grammar = tcl_dialect::grammar_of_dialect_name(Some("tk"));
        let profile = emit_profile(Some("tk")).expect("`tk` resolves to a profile");
        tcl_syntax::number::set_runtime_syntax(restore);

        assert_eq!(
            grammar.numbers,
            NumberSyntax::Tcl85,
            "a `tk` compile reads numerals under its own 8.6 core, not the \
             thread-ambient grammar and not the 9.x fallback"
        );
        assert!(
            std::ptr::eq(profile, tcl_dialect::DialectProfile::plain_tcl()),
            "the analyser profile for `tk` is deliberately the anonymous fallback; \
             the compile must not take its grammar from it"
        );
        assert_eq!(
            grammar,
            tcl_dialect::model::DialectPoint::of_dialect_name(Some("tk"))
                .expect("tk has a core")
                .grammar(),
            "the name resolves to the environment's point"
        );
    }
    #[test]
    fn unavailable_procedure_namespace_keeps_the_original_declaration_call() {
        let registry = tcl_registry::CommandRegistry::build_default();
        let source = "proc p {} {return VALUE}";
        let mut ir = crate::lowering::lower_to_ir(source, &registry);
        let id = fixture_id(&ir.procedures, "::p");
        let tokens = ir
            .top_level
            .statements
            .iter()
            .find_map(|statement| match statement {
                crate::ir::Statement::Call { tokens, .. } => tokens.as_ref(),
                _ => None,
            })
            .expect("actual original declaration call");
        assert_eq!(tokens.words().len(), 4);
        let written: Vec<_> = tokens
            .words()
            .iter()
            .map(|word| {
                let span = word.original_written_span(ir.source.bytes()).unwrap();
                ir.source.bytes()[span.as_range()].to_vec()
            })
            .collect();
        assert_eq!(
            written,
            [
                b"proc".to_vec(),
                b"p".to_vec(),
                b"{}".to_vec(),
                b"{return VALUE}".to_vec()
            ]
        );
        ir.procedures.get_mut(&id).unwrap().body.namespace_context = None;
        let cfg = crate::cfg_builder::build_cfg_codegen_with_registry(&ir, false, &registry);
        let asm = super::codegen_module(&cfg, &ir, &registry).expect("original source namespace");
        assert!(!asm.procedures.contains_key(&id));
        assert!(!asm.procedure_provenance.contains_key(&id));
        assert!(
            asm.top_level
                .instructions
                .iter()
                .any(|instruction| instruction.source_cmd_text.bytes() == source.as_bytes())
        );
    }
}

#[cfg(test)]
mod original_top_namespace_tests {
    use super::*;

    #[test]
    fn top_artifact_namespace_uses_original_entry_and_ignores_reporting_labels() {
        let registry = CommandRegistry::build_default();
        let source = "proc helper {} {return VALUE}; helper";
        let ir = crate::lowering::lower_to_ir_for_bytecode(source, &registry);
        let cfg = crate::cfg_builder::build_cfg_codegen_with_registry(&ir, false, &registry);
        let original = codegen_module(&cfg, &ir, &registry).unwrap();
        let mut reported = ir.clone();
        reported.top_level_namespace = "::unrelated::report".to_owned();
        let changed = codegen_module(&cfg, &reported, &registry).unwrap();
        assert_eq!(original.source_namespace, changed.source_namespace);
        assert!(original.source_namespace.is_root());
        assert_eq!(original.source.bytes(), source.as_bytes());
    }

    #[test]
    fn explicit_script_target_keeps_its_typed_source_namespace() {
        let registry = CommandRegistry::build_default();
        let source = "puts VALUE";
        let ir = crate::lowering::lower_script_module_for_bytecode(
            source,
            "actual::scope",
            &registry,
            tcl_lexer::LexerConfig::default(),
            None,
            true,
        );
        let cfg = crate::cfg_builder::build_cfg_codegen_with_registry(&ir, false, &registry);
        let asm = codegen_module(&cfg, &ir, &registry).unwrap();
        let expected = tcl_core_types::ByteNamespacePath::from_segments(["actual", "scope"]);
        assert_eq!(asm.source_namespace, expected);
        assert_eq!(asm.source.bytes(), source.as_bytes());
    }

    #[test]
    fn missing_top_namespace_refuses_the_artifact_and_preserves_original_source() {
        let registry = CommandRegistry::build_default();
        let source = "proc helper {} {return VALUE}; helper";
        let original = crate::lowering::lower_to_ir_for_bytecode(source, &registry);
        let cfg = crate::cfg_builder::build_cfg_codegen_with_registry(&original, false, &registry);
        for mode in 0..3 {
            let mut changed = original.clone();
            match mode {
                0 => changed.top_level_namespace_context = None,
                1 => changed.retained_source_bindings = None,
                _ => {
                    changed.top_level_namespace_context = Some(
                        crate::command_binding::SourceNamespaceKey::authored("::donated"),
                    );
                }
            }
            assert!(matches!(
                codegen_module(&cfg, &changed, &registry),
                Err(tcl_runtime_api::CompileError::Unsupported(_))
            ));
            assert_eq!(changed.own_source().bytes(), source.as_bytes());
            assert_eq!(changed.top_level.statements, original.top_level.statements);
        }
    }
}

#[cfg(test)]
mod original_byte_top_namespace_tests {
    use super::*;

    #[test]
    fn opaque_byte_target_retains_its_original_path_and_rejects_donated_geometry() {
        let registry = CommandRegistry::build_default();
        let source = tcl_lexer::SourceImage::native(b"puts {\xff}".as_slice());
        let namespace = tcl_core_types::ByteNamespacePath::from_segments([b"N\xff".as_slice()]);
        let build = || {
            crate::lowering::lower_script_bytes_module_for_bytecode_with_options(
                tcl_runtime_api::ScriptCompileTargetBytes {
                    source: &source,
                    namespace: &namespace,
                },
                &registry,
                tcl_lexer::LexerConfig::default(),
                None,
                true,
                None,
            )
            .unwrap()
        };
        let ir = build();
        assert!(ir.retained_source_bindings.is_none());
        assert!(ir.original_byte_source_namespace.is_some());
        let cfg = crate::cfg_builder::build_cfg_codegen_with_registry(&ir, false, &registry);
        assert_eq!(
            codegen_module(&cfg, &ir, &registry)
                .unwrap()
                .source_namespace,
            namespace
        );
        let mut changed = ir.clone();
        changed.native_namespace = Some(tcl_core_types::ByteNamespacePath::root());
        assert!(codegen_module(&cfg, &changed, &registry).is_err());
        let mut foreign = build();
        foreign.original_byte_source_namespace = ir.original_byte_source_namespace.clone();
        assert!(codegen_module(&cfg, &foreign, &registry).is_err());
    }
}

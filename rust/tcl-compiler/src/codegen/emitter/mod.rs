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
//! - [`proc_defs`] — interleaved proc definition emission
//! - [`loop_blocks`] — per-block handlers for foreach/while/for
//! - [`try_blocks`] — try/finally CFG pattern detection
//! - [`generate`] — top-level dispatcher
//! - [`bytecoded`] — registry-backed codegen hook dispatch

#![allow(dead_code)]

pub mod bytecoded;
pub mod generate;
pub mod loop_blocks;
pub mod ordering;
pub mod proc_defs;
pub mod terminator;
pub mod try_blocks;

use std::collections::HashMap;

use tcl_registry::CommandRegistry;

use crate::cfg::{CfgModule, Function as CfgFunction};
use crate::ir::{Module as IrModule, Procedure as IrProcedure};

use super::{CodegenCtx, FunctionAsm, ModuleAsm};

/// Generate bytecode assembly for a single CFG function.
///
/// When `is_proc` is true, variables are accessed via the LVT
/// (`loadScalar1`/`storeScalar1`). When `false` (top-level scripts),
/// variables are accessed via the stack (`loadStk`/`storeStk`).
/// `registry` is consulted for codegen-hook resolution; pass the
/// same instance the lowering pass used so dialect-loaded specs
/// are visible.
#[must_use]
pub fn codegen_function(
    cfg: &CfgFunction,
    params: &[&str],
    is_proc: bool,
    registry: &CommandRegistry,
) -> FunctionAsm {
    codegen_function_with_procs(cfg, params, is_proc, &[], registry)
}

/// Generate bytecode assembly for a CFG function, with pending proc defs.
///
/// Used by `codegen_module` to interleave proc definitions at their
/// source positions within the top-level script.
#[must_use]
pub fn codegen_function_with_procs(
    cfg: &CfgFunction,
    params: &[&str],
    is_proc: bool,
    proc_defs: &[IrProcedure],
    registry: &CommandRegistry,
) -> FunctionAsm {
    let mut ctx = CodegenCtx::new(is_proc, params, registry);
    generate::generate(&mut ctx, cfg, proc_defs)
}

/// The per-module facts every function emission shares: the registry, the
/// module source (for `errorInfo` surface text) and the release being compiled
/// for (its dialect name, and the numeral and backslash-escape grammars that
/// name resolves to). Bundled rather than threaded as parallel parameters —
/// the argument list is already at `clippy::too_many_arguments`'s ceiling, and
/// these always travel together.
struct ModuleEmit<'a> {
    registry: &'a CommandRegistry,
    source: std::rc::Rc<str>,
    line_index: tcl_lexer::LineIndex,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    numbers: tcl_dialect::NumberSyntax,
    escapes: tcl_dialect::EscapeSyntax,
    braced_var: tcl_dialect::BracedVarStyle,
    word_rules: tcl_syntax::word_rules::WordValueRules,
    expr_grammar: Option<tcl_dialect::LexerGrammar>,
    /// The unit's command-binding summary — see
    /// [`CodegenCtx::command_bindings`](crate::codegen::CodegenCtx::command_bindings).
    /// Scanned once per module, not once per function.
    command_bindings: &'a crate::command_binding::ModuleCommandMutations,
}

/// Like [`codegen_function_with_procs`] but threading the module source text so
/// each instruction carries its command's surface text for `errorInfo`, plus
/// the proc body's `base_line` (its `proc` definition line) so the
/// `(procedure … line N)` frame reports a proc-relative line.
#[must_use]
fn codegen_function_src(
    cfg: &CfgFunction,
    params: &[&str],
    is_proc: bool,
    proc_defs: &[IrProcedure],
    module: &ModuleEmit<'_>,
    base_line: u32,
) -> FunctionAsm {
    let mut ctx = CodegenCtx::new(is_proc, params, module.registry);
    ctx.numbers = module.numbers;
    ctx.escapes = module.escapes;
    ctx.braced_var = module.braced_var;
    ctx.word_rules = module.word_rules;
    ctx.expr_grammar = module.expr_grammar;
    ctx.dialect = module.dialect;
    ctx.command_bindings = Some(module.command_bindings);
    ctx.set_indexed_source(
        std::rc::Rc::clone(&module.source),
        module.line_index.clone(),
    );
    let mut asm = generate::generate(&mut ctx, cfg, proc_defs);
    asm.body_base_line = base_line;
    asm
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
#[must_use]
pub fn codegen_module(
    cfg_module: &CfgModule,
    ir_module: &IrModule,
    registry: &CommandRegistry,
) -> ModuleAsm {
    let source: std::rc::Rc<str> = ir_module.source.as_str().into();
    let line_index = tcl_lexer::LineIndex::new(&source);
    // The compile's target release: a named dialect's own numeric grammar, else
    // the permissive 9.x default.
    let dialect = ir_module.dialect.as_deref();
    // One grammar, resolved once from the name, and every axis read off it
    // — so the numerals codegen emits and the numerals it re-parses `expr`
    // bodies under are the same value by construction.
    let grammar = tcl_dialect::grammar_of_dialect_name(dialect);
    let numbers = grammar.numbers;
    let escapes = grammar.escapes;
    let braced_var = grammar.braced_var;
    let word_rules = tcl_syntax::word_rules::WordValueRules::from_grammar(&grammar);
    let expr_grammar = dialect.map(|_| grammar);
    // Which builtins this unit leaves alone (issue #1585). Scanned from the IR
    // — the top-level script plus every proc / method body — so a `rename` or
    // shadowing `proc` *anywhere* in the unit is seen before the first
    // instruction is emitted, whatever order the bodies are lowered in.
    let command_bindings =
        crate::command_binding::scan_module_command_mutations(ir_module, registry);
    let module = ModuleEmit {
        registry,
        source,
        line_index,
        dialect: emit_profile(dialect),
        numbers,
        escapes,
        braced_var,
        word_rules,
        expr_grammar,
        command_bindings: &command_bindings,
    };
    let top = codegen_function_src(&cfg_module.top_level, &[], false, &[], &module, 0);
    // The same top level as a *procedure body*. A body compiled at run time
    // (`proc` on a cache miss, an `apply` lambda, a method) reaches the
    // compiler as a bare script, so without this it would run script-shaped and
    // lose every `is_proc` specialisation its AOT-compiled twin gets — see
    // [`ModuleAsm::top_level_body`].
    let top_body = codegen_function_src(&cfg_module.top_level, &[], true, &[], &module, 0);
    let mut procs: HashMap<String, FunctionAsm> = HashMap::new();
    for (qname, cfg_func) in &cfg_module.procedures {
        let ir_proc = ir_module.procedures.get(qname);
        // Skip procs defined inside namespace eval — tclsh compiles
        // them lazily at runtime, not at compile time.
        if let Some(p) = ir_proc
            && p.namespace_scoped
        {
            continue;
        }
        let params: Vec<&str> = ir_proc
            .map(|p| p.params.iter().map(String::as_str).collect())
            .unwrap_or_default();
        // The proc's definition line drives proc-relative `errorInfo` lines.
        let base_line = ir_proc.map_or(0, |p| {
            module
                .line_index
                .position_at(p.span.start())
                .line
                .saturating_add(1)
        });
        let mut asm = codegen_function_src(cfg_func, &params, true, &[], &module, base_line);
        // The body word this assembly was compiled from, so a runtime consumer
        // keyed by name can tell it apart from another `proc` of the same name
        // (see `FunctionAsm::proc_body_src`). Recorded as the word *value*, not
        // as the source text: lowering keeps the written word, but the value a
        // runtime `proc` is handed has had the one substitution braces permit
        // applied — a `\<newline>` continuation folded to a space — and the
        // comparison is against that. Without the fold, every body holding a
        // continuation missed.
        asm.proc_body_src = ir_proc
            .and_then(|p| p.body_source.as_deref())
            .map(|body| module.word_rules.collapse_braced_word(body).into_owned());
        procs.insert(qname.clone(), asm);
    }
    ModuleAsm {
        profile: emit_profile(dialect).unwrap_or_else(tcl_dialect::DialectProfile::plain_tcl),
        top_level: top,
        top_level_body: top_body,
        procedures: procs,
    }
}

#[cfg(test)]
mod tests {
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

    /// A name `find` rejects still picks the compile's *target* numeral
    /// grammar, not the ambient one — and the target is the name's **own**
    /// point, not the fallback it used to sink to.
    ///
    /// `tk` has no catalogue row; its grammar is the `tk` environment's
    /// 8.6 core, so a `tk` compile reads `010` as octal 8 — under 8.6's
    /// numerals — whatever grammar happens to be installed on the thread.
    /// `emit_profile("tk")` is still the anonymous fallback profile *by
    /// design* (the cache-key and help-filter reasons on
    /// `DocumentEnvironment::analyser_profile`), which is exactly why the
    /// compile no longer takes its numerals from that profile: it takes them,
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
}

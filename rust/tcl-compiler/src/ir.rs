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

//! Intermediate representation (IR) for Tcl analysis.
//!
//! This is a structured IR front-end that keeps source [`Span`]s on all
//! nodes. Later passes lower this to CFG + SSA.
//!
//! **Architectural note:** every IR node carries a [`Span`] (two `u32`
//! byte offsets) rather than inline `(line, character, offset)` pairs.
//! Text and positions are resolved on demand via a
//! [`SourceMap`](tcl_lexer::SourceMap). This mirrors the span-first
//! design established in the lexer crate.

#[cfg(test)]
use tcl_lexer::{LexerConfig, SourceMap, Span};
/// A [`TryHandler`]'s selection vocabulary, the registry's clause-row fact.
pub use tcl_registry::value_transfer::HandlerMatch;
pub use tcl_runtime_api::CommandEvaluation;

#[cfg(test)]
use crate::expr_ast::ExprNode;

mod definition_script;
pub(crate) use definition_script::DefinitionScriptLayout;
pub use definition_script::OriginalDefinitionScriptSource;
mod byte_source_namespace;
pub use byte_source_namespace::OriginalByteSourceNamespace;
mod method_identity;
pub use method_identity::{MethodCoordinate, MethodInventory};
mod source_bpf_command;
pub use source_bpf_command::{SourceBpfClosedEffects, SourceBpfCommand, SourceBpfModuleContext};
mod procedure_identity;
pub use procedure_identity::{
    CarriedInstalledLowerings, ProcedureBodyKind, ProcedureId, ProcedureIdIssuer,
    ProcedureInventory,
};

/// Nested [`Script`] levels a derived traversal may stack on one native
/// stack before it continues on a fresh segment. One level costs about
/// 30 KiB in a `dev`-profile build.
const STRUCTURED_IR_LEVELS_PER_STACK: u32 = 8;

/// Native stack of one continuation segment.
const STRUCTURED_IR_SEGMENT_STACK: usize = 8 * 1024 * 1024;

std::thread_local! {
    static STRUCTURED_IR_LEVEL: std::cell::Cell<u32> = const { std::cell::Cell::new(0) };
}

mod command_tokens;
mod definitions;
mod module;
mod provenance;
mod script;
mod statement;
mod synthetic_marker;
mod word_expr;

pub use command_tokens::CommandTokens;
pub use definitions::{
    ClassRelation, MethodDef, MethodKind, OoDefinitionEvidence, Procedure, SourceIrulesEventBody,
};
pub use module::{
    DeclaredFrameEffects, DeferredWrites, Module, NO_DEFERRED_WRITES, ReferenceBodies,
    ReferenceImport, TopLevelKind, when_event_name,
};
pub use provenance::{DerivedKind, NodeId, Provenance, SourceSite};
pub use script::{
    CommandBindingSite, CommandBindingSites, ExecutionNamespace, ProcedureBindingRequirements,
    Script, StatementResultUse, for_each_script_mut, for_each_statement,
};
pub(crate) use statement::statements_have_opaque_native_accesses;
pub use statement::{ForeachIterator, IfClause, Statement, SwitchArm, SwitchMode, TryHandler};
pub use synthetic_marker::{OriginalArgvInventoryDisposition, SyntheticMarker, WordEffectsHost};
pub use word_expr::{WordExpr, WordOpacity, WordPart};

/// Run one nested-script step of a recursive IR traversal, moving to a fresh
/// stack segment after [`STRUCTURED_IR_LEVELS_PER_STACK`] levels.
fn with_structured_ir_stack<T: Send>(work: impl FnOnce() -> T + Send) -> T {
    struct Restore(u32);
    impl Drop for Restore {
        fn drop(&mut self) {
            STRUCTURED_IR_LEVEL.with(|level| level.set(self.0));
        }
    }
    let level = STRUCTURED_IR_LEVEL.with(std::cell::Cell::get);
    if level >= STRUCTURED_IR_LEVELS_PER_STACK {
        return std::thread::scope(|scope| {
            std::thread::Builder::new()
                .name("structured-ir".into())
                .stack_size(STRUCTURED_IR_SEGMENT_STACK)
                .spawn_scoped(scope, work)
                .expect("spawn a structured IR stack segment")
                .join()
                .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
        });
    }
    let _restore = Restore(level);
    STRUCTURED_IR_LEVEL.with(|current| current.set(level + 1));
    work()
}

#[cfg(any(test, feature = "test-support"))]
#[doc(hidden)]
#[must_use]
pub fn native_call_for_test(bytes: &[u8]) -> Statement {
    let image = tcl_lexer::SourceImage::native(bytes);
    // dialect-drift-ok: a test-support fixture constructor
    let config = tcl_lexer::LexerConfig::default();
    let tokens = tcl_lexer::Lexer::with_source_image(&image, config)
        .tokenise_all()
        .expect("complete original native words");
    let groups = tcl_lexer::group_commands_bytes(&tokens, image.bytes(), config);
    assert_eq!(groups.len(), 1, "one native invocation");
    let words: Vec<_> = groups[0]
        .words
        .iter()
        .map(|word| {
            tcl_lexer::word_parts::NativeWord::from_group(image.clone(), config, &tokens, word)
                .expect("original token word extent")
        })
        .collect();
    let first = words.first().expect("command head");
    let last = words.last().expect("final operand");
    Statement::NativeCall {
        span: Span::new(first.span().start(), last.span().end()),
        words,
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_source_procedure_inventory_tests;

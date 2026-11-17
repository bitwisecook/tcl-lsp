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

//! Bytecode artifact types for the Tcl 9 bytecode ("TCLVM") backend.
//!
//! The lean, `forbid(unsafe)` leaf crate that owns the shared bytecode
//! *artifact*: the opcode set ([`Op`], matching Tcl 9.0.2), the instruction
//! and assembly types ([`Instruction`], [`FunctionAsm`], [`ModuleAsm`]), the
//! interning tables ([`LiteralTable`], [`LocalVarTable`]), plus instruction
//! [`layout`] and disassembly [`format`].
//!
//! The compiler's emitter (`tcl_codegen::codegen`) produces these types and
//! re-exports them; the bytecode VM consumes them. Keeping them in a leaf crate
//! (whose only dependency is `tcl-syntax`, for [`BinOp`]/[`UnaryOp`]) lets the
//! VM depend on the artifact without pulling in the whole compiler.

pub mod format;
pub mod layout;

use std::collections::HashMap;

mod instruction;
mod literals;
mod local_vars;
mod op;

pub use instruction::{
    CommandStartHost, EnteredCommandSite, ErrorInfoCommand, ErrorOrigin, ErrorStackContext,
    Instruction, LiteralRep, NativeCompilerSelectionSite, NativeEachAuxiliary,
    NativeOperationSelectionSite, NativeStringIndexPurpose, Operand, ReleaseBodyEntry,
    ReleaseCompiled, SourceCommandBoundary,
};
pub use literals::{
    LiteralTable, NativeExpressionNumberLiteral, NativeLiteralAction, NativeLiteralAllocation,
    NativeStringLiteral,
};
pub use local_vars::LocalVarTable;
pub use op::Op;

/// Index sentinel for "end"-based Tcl index notation.
///
/// `end` → `INDEX_END`, `end-N` → `INDEX_END - N`.
pub const INDEX_END: i32 = -(1 << 30);

/// A selected physical local slot or an original dynamic variable name.
/// Slot targets retain compiler allocation and never repeat name resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompiledVariableTarget {
    /// An indexed cell in the actual activation's compiled-local layout.
    Slot(usize),
    /// A complete counted name for an authored dynamic backend operation.
    Name(tcl_runtime_api::NameBytes),
}

/// Convert a non-negative count or index to an `i32` bytecode operand.
///
/// Bytecode `Imm` operands are `i32`; call-site usage always comes from
/// `usize` counts (argument lists, indices, slot numbers) that cannot
/// plausibly exceed `i32::MAX` — a compiler invariant. A program that
/// does exceed this limit is malformed and panicking is the correct
/// outcome rather than silent truncation.
#[inline]
#[must_use]
pub fn bytecode_imm(n: usize) -> i32 {
    i32::try_from(n).expect("bytecode operand exceeds i32::MAX")
}

/// How far from [`INDEX_END`] an index immediate still reads as `end`-relative:
/// the VM reads one within the band as `end` plus its distance from
/// `INDEX_END`, and any other as the index it is.
pub const INDEX_END_BAND: i32 = 1 << 29;

/// The index immediate for `end` plus `offset`, `None` outside
/// [`INDEX_END_BAND`].
#[must_use]
pub fn end_relative_imm(offset: i64) -> Option<i32> {
    let offset = i32::try_from(offset)
        .ok()
        .filter(|offset| offset.unsigned_abs() < INDEX_END_BAND.unsigned_abs())?;
    Some(INDEX_END + offset)
}

/// Parse a Tcl index string to an integer suitable for IMM instructions.
///
/// Plain integers compile directly. `end`-based indices are encoded
/// relative to [`INDEX_END`] so the VM can resolve them at runtime.
#[must_use]
pub fn parse_tcl_index(s: &str) -> Option<i32> {
    let s = s.trim();
    if s == "end" {
        return Some(INDEX_END);
    }
    if let Some(rest) = s.strip_prefix("end-") {
        return rest.parse::<i32>().ok().map(|n| INDEX_END - n);
    }
    if let Some(rest) = s.strip_prefix("end+") {
        return rest.parse::<i32>().ok().map(|n| INDEX_END + n);
    }
    s.parse::<i32>().ok()
}

/// `string is` class name → numeric index.
#[must_use]
pub fn str_class_id(name: &str) -> Option<u8> {
    Some(match name {
        "alnum" => 0,
        "alpha" => 1,
        "ascii" => 2,
        "control" => 3,
        "digit" => 4,
        "graph" => 5,
        "lower" => 6,
        "print" => 7,
        "punct" => 8,
        "space" => 9,
        "upper" => 10,
        "wordchar" => 11,
        "xdigit" => 12,
        _ => return None,
    })
}

/// Numeric index → `string is` class name.
#[must_use]
pub fn str_class_name(id: u8) -> Option<&'static str> {
    Some(match id {
        0 => "alnum",
        1 => "alpha",
        2 => "ascii",
        3 => "control",
        4 => "digit",
        5 => "graph",
        6 => "lower",
        7 => "print",
        8 => "punct",
        9 => "space",
        10 => "upper",
        11 => "wordchar",
        12 => "xdigit",
        _ => return None,
    })
}

/// Complete assembly for one CFG function.
///
/// `Default` yields an empty function (no instructions) — used as the placeholder
/// asm for a scanner-driven activation like the VM's `subst` frame, which never
/// executes bytecode.
#[derive(Debug, Clone, Default)]
pub struct FunctionAsm {
    /// Function name.
    pub name: String,
    /// Proved native compiler rejection at this function's actual entry.
    /// The executor validates handler and raw compiler prerequisites before presenting this error
    /// and admits it before evaluating arguments or binding procedure formals.
    /// This metadata never moves into a caller through executable inlining.
    pub native_compilation_failure: Option<tcl_runtime_api::NativeCompilationError>,
    /// Remaining entry-time native compiler obligation. A missing error
    /// presentation never turns a definite compiler failure into executable code.
    pub native_compilation_preflight: tcl_runtime_api::NativeCompilationPreflight,
    /// Actual fixed math registrations required to reuse native compiler metadata.
    pub native_math_table_prerequisite:
        Option<tcl_runtime_api::native_compilation::NativeMathFunctionPrerequisite>,
    /// Exact raw compiler registrations consumed by failure or preparation.
    /// These are checked once at admission, separately from handler dependencies.
    pub native_compiler_prerequisites:
        Vec<tcl_runtime_api::native_compilation::NativeCompilerSelectionPrerequisite>,
    /// Exact reusable compiled-local cache whose existing indices this body borrows.
    /// No activation identifiers or variable cells participate in this prerequisite.
    pub required_compiled_local_layout:
        Option<tcl_runtime_api::native_compilation::NativeCompiledLocalLayout>,
    /// Literal constant pool.
    pub literals: LiteralTable,
    /// Local variable table.
    pub lvt: LocalVarTable,
    /// Instruction stream.
    pub instructions: Vec<Instruction>,
    /// Label → byte offset (populated by the layout pass).
    pub labels: HashMap<String, usize>,
    /// For each instruction inside an inline loop body (by index), the loop's
    /// `(break, continue)` jump targets as **byte offsets**. Lets the executor
    /// catch a `break`/`continue` *returned by a command* (`if {…} $z`, `eval
    /// break`) — which the inline `JUMP` only covers for a *literal*
    /// `break`/`continue` — and jump to the loop's exit/continue point, as C
    /// Tcl's exception ranges do. Either target may be `None` (a `for`-step's
    /// `continue` propagates). Sparse: only loop-body instructions appear.
    pub loop_targets: HashMap<usize, (Option<i32>, Option<i32>)>,
    /// For a procedure body, the 1-based source line of its `proc` definition
    /// (the body's line 1). `errorInfo`'s `(procedure "name" line N)` reports
    /// `N = instruction_line − body_base_line + 1`, so the line is relative to
    /// the proc, not the whole module. `0` for the top level / hand-built asm.
    pub body_base_line: u32,
    /// For an entry in [`ModuleAsm::procedures`], the `proc` command's body
    /// **word value** — the source text with the one substitution braces permit
    /// applied, so it compares equal to the value a runtime `proc` is handed.
    /// `None` for every other function (both top-level shapes, hand-built
    /// assembly, a synthetic proc with no source).
    ///
    /// A runtime consumer keyed by *name* cannot assume the body it is handed
    /// is the body that was compiled: a name is reachable from more than one
    /// `proc` command (a redefinition, opposite arms of an `if`, a second
    /// `eval` after a `rename`), and a compiled unit records only the first.
    /// Comparing this against the body word actually supplied turns that
    /// ambiguity into a cache miss instead of a wrong body.
    ///
    /// A body the script *builds* (`proc p {} "return $x"`) never matches — the
    /// compiler records the unsubstituted word — so it misses and recompiles.
    pub proc_body_src: Option<String>,
    /// Inline command-body regions (`eval {…}` / `while`/`for`/`foreach` bodies)
    /// folded into this function's instruction stream. As an error unwinds past
    /// a covering region the executor synthesises the body frame the *uncompiled*
    /// command would add to `errorInfo` (`("eval" body line N)` + `invoked from
    /// within "eval {…}"`), so an inlined body matches C's `CmdFrame` trace
    /// without de-inlining. Empty when the function holds no inlined bodies.
    pub error_regions: Vec<ErrorRegion>,
    /// Whether this function was compiled with every command invocation kept
    /// on the ordinary runtime-dispatch path. This explicit stamp lets a
    /// runtime distinguish a deliberately de-optimised function from an
    /// optimised function that merely happens to have no binding requirements.
    pub plain_command_dispatch: bool,
    /// Runtime command bindings whose registry implementations this function's
    /// specialised operations or native compilation rejection assume. A
    /// plain-dispatch function can still retain compiler-entry dependencies.
    pub command_bindings: Vec<tcl_runtime_api::CommandBindingIdentity>,
    /// Exact user-procedure bindings whose bodies were copied into this
    /// function by an executable inlining transform.
    pub procedure_bindings: Vec<tcl_runtime_api::ProcedureBindingIdentity>,
    /// The spec-pack facts this function's specialised sites rest on
    /// ([`tcl_runtime_api::SiteClaim`]): admission requires every claim's
    /// stamp among the pack facts the VM holds. Empty for a function no pack
    /// fact reached, which admits on its bindings alone.
    pub site_claims: Vec<tcl_runtime_api::SiteClaim>,
}

impl FunctionAsm {
    /// The rungs this function's sites rest on, read off what it records.
    ///
    /// Generic dispatch, rung 0, is always among them: every function's words
    /// were decoded under the release it was compiled for. A pack-fact claim
    /// is rung 1, a builtin-alias claim rung 2, a reference-body claim or an
    /// inlined procedure body rung 3, and a specialisation resting on a
    /// shipped implementation's identity — every command binding — rung 4.
    #[must_use]
    pub fn rungs(&self) -> tcl_runtime_api::RungSet {
        use tcl_runtime_api::{Rung, SiteClaim};

        let mut rungs = tcl_runtime_api::RungSet::of(Rung::Generic);
        for claim in &self.site_claims {
            rungs = rungs.with(match claim {
                SiteClaim::PackFacts(_) => Rung::PackFacts,
                SiteClaim::BuiltinAlias { .. } => Rung::BuiltinAlias,
                SiteClaim::ReferenceBody { .. } => Rung::ReferenceBody,
            });
        }
        if !self.procedure_bindings.is_empty() {
            rungs = rungs.with(Rung::ReferenceBody);
        }
        if !self.command_bindings.is_empty() {
            rungs = rungs.with(Rung::ShippedBacking);
        }
        rungs
    }
}

impl FunctionAsm {
    /// Validate native compiler metadata before entering this function.
    ///
    /// This returns a host admission error, never a Tcl catch-visible error.
    /// Hosts resolve a remaining obligation through their genuine compiler
    /// provider before evaluating any source word or binding procedure formals.
    pub fn validate_native_compilation_entry(
        &self,
    ) -> Result<(), tcl_runtime_api::NativeCompilationAdmissionError> {
        use tcl_runtime_api::{
            NativeCompilationAdmissionError as Error, NativeCompilationPreflight,
        };
        if self.native_compilation_preflight != NativeCompilationPreflight::NotRequired {
            return Err(Error::NativePreflightRequired);
        }
        if self
            .native_compiler_prerequisites
            .iter()
            .any(|required| required.guard() != tcl_runtime_api::CommandBindingGuard::ChunkEntry)
        {
            return Err(Error::NativePreflightRequired);
        }
        if self
            .native_compilation_failure
            .as_ref()
            .is_some_and(|error| error.error_info_for_procedure(Some(&self.name)).is_none())
        {
            return Err(Error::InvalidErrorPresentation);
        }
        if !self.native_operation_sites_valid() || !self.native_fixed_math_calls_valid() {
            return Err(Error::NativePreflightRequired);
        }
        if self.instructions.iter().any(|instruction| {
            instruction
                .source_command_namespace_context
                .as_ref()
                .is_some_and(|context| context.path() != &instruction.source_command_namespace)
        }) {
            return Err(Error::NativePreflightRequired);
        }
        Ok(())
    }
    fn native_fixed_math_calls_valid(&self) -> bool {
        use tcl_runtime_api::native_compilation::NativeMathFunctionResolution;

        self.instructions.iter().all(|instruction| {
            if instruction.op != Op::CALL_FUNC1 {
                return instruction.native_fixed_math_call.is_none();
            }
            let Some(binding) = instruction.native_fixed_math_call.as_ref() else {
                return false;
            };
            let [Operand::Imm(argc)] = instruction.operands.as_slice() else {
                return false;
            };
            let Ok(argc) = u8::try_from(*argc) else {
                return false;
            };
            binding.arity == Some(usize::from(argc))
                && binding.registry_identity.is_some()
                && self
                    .native_math_table_prerequisite
                    .as_ref()
                    .is_some_and(|required| {
                        matches!(required.table.lookup_bytes(binding.name.as_bytes()),
                        NativeMathFunctionResolution::Present(actual) if actual == binding)
                    })
        })
    }
    fn native_operation_sites_valid(&self) -> bool {
        let byte_end = self.instructions.last().and_then(|instruction| {
            usize::try_from(instruction.offset)
                .ok()?
                .checked_add(usize::from(instruction.op.size()))
        });
        self.instructions.iter().all(|instruction| {
            instruction.native_operation_selections.iter().all(|site| {
                let Some(&end) = self.labels.get(&site.end) else {
                    return false;
                };
                let Some(start) = usize::try_from(instruction.offset).ok() else {
                    return false;
                };
                end > start
                    && site
                        .namespace_context
                        .as_ref()
                        .is_none_or(|context| context.path() == &site.namespace)
                    && (Some(end) == byte_end
                        || self
                            .instructions
                            .iter()
                            .any(|next| usize::try_from(next.offset).ok() == Some(end)))
                    && (site.compiler_prerequisite.is_some() || !site.requirements.is_empty())
                    && site.compiler_prerequisite.as_ref().is_none_or(|required| {
                        required.guard == site.guard
                            && (required.compiler.ensemble.is_some()
                                || required.selected_worker.is_none())
                    })
                    && !site.source.is_empty()
                    && usize::try_from(site.span.len()).ok() == Some(site.source.len())
                    && (site.guard != tcl_runtime_api::CommandBindingGuard::ChunkEntry
                        || site.requirements.iter().all(|binding| {
                            binding.guard == tcl_runtime_api::CommandBindingGuard::ChunkEntry
                        }))
            })
        })
    }
}

/// An inlined command body's instruction range and the `errorInfo` frame the
/// enclosing command (`eval`, a loop, `if`, `switch`) would add when its body
/// errors — the compiled analogue of C's per-command `CmdFrame`. Populated by
/// codegen when it folds a literal body inline (so the LSP keeps its inlined
/// view), consumed by the executor as an error unwinds (see
/// `FunctionAsm::error_regions`).
#[derive(Debug, Clone)]
pub struct ErrorRegion {
    /// Start byte offset of the enclosing command's source span (inclusive). An
    /// instruction is covered when its `source_span` lies within `[start, end)`,
    /// so coverage is matched by source containment — robust to instruction
    /// reordering and exact across interleaved (non-body) instructions.
    pub start: u32,
    /// End byte offset of the enclosing command's source span (exclusive).
    pub end: u32,
    /// The frame the body adds before the command is logged.
    pub frame: ErrorRegionFrame,
    /// The enclosing command's surface text, for its `invoked from within "…"`
    /// frame (truncated to 150 bytes by the logger, as in C).
    pub cmd_text: tcl_lexer::SourceImage,
    /// One less than the body's first source line: a covered instruction's
    /// body-relative line is `instruction_line − line_base` (so the body frame
    /// reports a line relative to the body, not the whole module).
    pub line_base: u32,
    /// The enclosing command's own source line — the line its `invoked from
    /// within` frame contributes to any further-out (proc) frame.
    pub cmd_line: u32,
    /// Whether the release compiles the command in a procedure body, so the
    /// region adds no frame where its script runs as part of one.
    pub compiled_in_procedure: bool,
}

/// The `errorInfo` frame an [`ErrorRegion`] adds, `N` the failing command's
/// line within the body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorRegionFrame {
    /// `("LABEL" body line N)`: `eval`, `foreach`, `while`, `for`, `lmap`.
    Body(String),
    /// `(TEXT line N)`: a `switch` arm's `"PATTERN" arm`.
    Line(String),
    /// `(TEXT)`: `"for" initial command`.
    Fixed(String),
    /// None: the command is logged alone (`if`).
    None,
}

/// Assembly for an entire module.
#[derive(Debug, Clone)]
pub struct ModuleAsm {
    /// Exact interned dialect profile selected during lowering/codegen.  A VM
    /// must reject an AOT module compiled for another profile rather than
    /// treating its specialised opcodes as belonging to its current surface.
    pub profile: &'static tcl_dialect::DialectProfile,
    /// Original module source, retained so a reusable optimised artifact can
    /// be recompiled through the plain-dispatch capability after invalidation.
    pub source: tcl_lexer::SourceImage,
    /// Canonical unrooted constructed namespace in which the top-level
    /// assembly was specialised.  This is part of the artifact's executable
    /// provenance: an unqualified command may resolve to a different binding
    /// when the same source is entered from another namespace.
    pub source_namespace: tcl_runtime_api::ByteNamespacePath,
    /// Whether lowering suppressed every registry-driven specialised command
    /// path. Runtimes validate this explicit capability stamp instead of
    /// inferring plain dispatch from an accidentally-empty dependency list.
    pub plain_command_dispatch: bool,
    /// Top-level script assembly — the unit entered as a *script*.
    pub top_level: FunctionAsm,
    /// The same top level emitted as a **procedure body**: LVT variable forms
    /// (`STORE_SCALAR1`, `INCR_SCALAR1`, `LAPPEND_SCALAR1`, the compiled
    /// `unset`) and the proc return protocol, exactly as an entry in
    /// [`Self::procedures`] is emitted.
    ///
    /// A body compiled at run time — `proc`'s body when the pre-compiled entry
    /// misses, an `apply` lambda, a `TclOO` method — is the *whole* source the
    /// compiler was handed, so it arrives here as a top level rather than as a
    /// procedure. Running that as a script would deny every dynamically
    /// compiled body the `is_proc` specialisation an AOT-compiled one gets, and
    /// the two would diverge on any semantics the specialised opcodes carry
    /// (C's `INST_LAPPEND_SCALAR` omits `TCL_TRACE_READS`; `INST_UNSET_ARRAY`
    /// reports a two-part access). Both shapes are emitted so the consumer
    /// picks by how it will *enter* the code, not by how it was compiled.
    ///
    /// The local variable table is not pre-seeded with parameter names (the
    /// compiler is handed a body, never its parameter list), so slot *order*
    /// can differ from the AOT form. Slots resolve by name at run time, so this
    /// is a disassembly difference only.
    pub top_level_body: FunctionAsm,
    /// Procedure assemblies joined by their originating module body ID.
    pub procedures: HashMap<tcl_runtime_api::ProcedureId, FunctionAsm>,
    /// Source identity for each statically emitted procedure body. Runtimes
    /// use this to admit a fast precompiled body only for the exact `proc`
    /// definition that produced it, never by command name alone.
    pub procedure_provenance: HashMap<tcl_runtime_api::ProcedureId, ProcedureProvenance>,
    /// What this artefact says about the world it was compiled for: the
    /// environment, release and build of [`Self::profile`], the package
    /// floors, the pack facts any function's sites rest on, and the ABI,
    /// intrinsic table and embedded library it assumed. A VM refuses the
    /// rungs a disagreeing field rests on. Shared, because every unit made
    /// from the module keeps it. `None` for assembly no compiler produced,
    /// which is admitted on its profile and bindings alone.
    pub manifest: Option<std::sync::Arc<tcl_runtime_api::ArtefactIdentityManifest>>,
}

impl ModuleAsm {
    /// The pack facts any function of this module rests on: every site
    /// claim's stamp, sorted and without repeats.
    #[must_use]
    pub fn claimed_packs(&self) -> Vec<tcl_runtime_api::PackFactStamp> {
        let mut packs: Vec<tcl_runtime_api::PackFactStamp> =
            [&self.top_level, &self.top_level_body]
                .into_iter()
                .chain(self.procedures.values())
                .flat_map(|function| function.site_claims.iter())
                .map(|claim| claim.facts().clone())
                .collect();
        packs.sort();
        packs.dedup();
        packs
    }
}

/// Exact source provenance for a compiler-emitted procedure body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcedureProvenance {
    /// Reporting name, checked against the joined assembly's name.
    /// Retained namespace geometry independently distinguishes colliding labels.
    pub name: String,
    /// Original body lookup context, separate from the displayed declaration name.
    /// A constructed path preserves geometry without attesting a native owner.
    /// Missing context cannot authorise native procedure-artifact cache reuse.
    pub namespace_context: Option<tcl_runtime_api::CompiledNamespaceContext>,
    /// Raw formal-parameter list value, including defaults.
    pub parameters: String,
    /// Raw procedure body value.
    pub body: String,
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod native_local_tests;

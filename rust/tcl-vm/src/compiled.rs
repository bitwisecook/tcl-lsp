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

use std::rc::Rc;
use std::sync::Arc;

use tcl_bytecode::FunctionAsm;
use tcl_runtime_api::{ArtefactIdentityManifest, FatalTail};

/// How bytecode entered this VM's compilation domain.
///
/// A public [`tcl_bytecode::ModuleAsm`] is an embedder-owned artifact. Admitting
/// it for one execution must not claim that the VM's current compile service
/// produced it; reusable source-bearing children are therefore recompiled on
/// first use when a service is installed. A source-less VM may execute a still-
/// valid admitted child as supplied without changing this marker. VM compilation
/// paths record the service generation that really produced their assembly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CompilerProvenance {
    /// Genuine Jim Script activation, independent of compiler admission.
    NativeScript,
    /// A plain source control plan, with no native compiled cache or literal registrations.
    NativeDirect(u64),
    CurrentService(u64),
    AdmittedForeign(u64),
}

impl CompilerProvenance {
    pub(crate) fn generation(self) -> u64 {
        match self {
            Self::NativeScript => 0,
            Self::NativeDirect(generation)
            | Self::CurrentService(generation)
            | Self::AdmittedForeign(generation) => generation,
        }
    }

    pub(crate) fn is_current_service(self, generation: u64) -> bool {
        self == Self::CurrentService(generation)
    }

    pub(crate) fn is_current_foreign_admission(self, generation: u64) -> bool {
        self == Self::AdmittedForeign(generation)
    }
}

/// Actual native compiler-cache generation, separate from lookup/guard epochs.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct NativeCompilerCacheEpoch(pub(crate) u64);

/// Exact physical compiler engine and logical invocation policy of an artifact.
/// Scoped host activation changes this receipt without changing source grammar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NativeCompilerPolicy {
    pub(crate) expression_evaluation:
        Option<tcl_runtime_api::expression_policy::ExpressionEvaluationPolicy>,
    pub(crate) eval_object_provider:
        Option<tcl_registry::native_eval_object::LogicalEvalObjectProvider>,
    pub(crate) source_word_provider:
        Option<tcl_registry::invocation_words::LogicalSourceWordProvider>,
    pub(crate) expression_provider:
        Option<tcl_registry::invocation_words::LogicalExpressionParseProvider>,
    pub(crate) name_provider: Option<tcl_syntax::naming::NamePolicyProtocol>,
    pub(crate) compiled_variable_provider:
        Option<tcl_registry::native_compiled_variables::LogicalCompiledVariableProvider>,
    pub(crate) engine: tcl_dialect::DialectProfileKey,
    pub(crate) invocation: tcl_dialect::DialectProfileKey,
    pub(crate) quote_provider:
        Option<tcl_registry::invocation_words::LogicalExpressionQuoteProvider>,
    pub(crate) numeric_provider:
        Option<tcl_syntax::logical_numeric_simulation::AuthoredLogicalNumericSimulation>,
}

/// Native interpreter and namespace resolver epochs at cache admission.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NativeCompilerCacheStamp {
    pub(crate) policy: NativeCompilerPolicy,
    pub(crate) interpreter_epoch: NativeCompilerCacheEpoch,
    pub(crate) namespace: tcl_runtime_api::NsId,
    pub(crate) resolver_epoch: u64,
}

/// Bytecode assembly and the complete VM-local provenance that validated it.
#[derive(Clone)]
pub(crate) struct CompiledUnit {
    pub(crate) native_local_names:
        Rc<std::cell::RefCell<Option<Rc<crate::literal_pool::NativeLocalNameTable>>>>,
    /// Retained owner of this body's declaration layout, shared across activations.
    pub(crate) compiled_local_layout:
        Option<tcl_runtime_api::native_compilation::NativeCompiledLocalLayout>,
    pub(crate) jim_script: Option<crate::native_jim_script::NativeJimScriptEntry>,
    pub(crate) asm: Rc<FunctionAsm>,
    /// Original fixed builtin handlers selected before argument evaluation.
    pub(crate) fixed_math_calls:
        Option<Rc<std::collections::HashMap<usize, crate::interp::NativeFixedMathCall>>>,
    pub(crate) literal_pool: crate::literal_pool::NativeLiteralPoolReceipt,
    pub(crate) direct_source_operands: Option<crate::literal_pool::NativeDirectSourceOperands>,
    pub(crate) source_location:
        Option<tcl_runtime_api::script_source_location::ScriptSourceLocation>,
    pub(crate) source_namespace: tcl_core_types::ByteNamespacePath,
    pub(crate) profile_generation: u64,
    pub(crate) command_epoch: u64,
    pub(crate) native_cache: Option<NativeCompilerCacheStamp>,
    /// Interpreter that supplied the compilation command/namespace world.
    pub(crate) interpreter: tcl_runtime_api::native_compilation::NativeInterpreterIdentity,
    pub(crate) compiler: CompilerProvenance,
    /// The parse error to raise once this unit's commands have run, for a body
    /// whose *later* commands do not parse.
    ///
    /// C compiles a procedure body when the procedure is **called**, so a
    /// malformed body neither refuses the definition nor runs with the lenient
    /// lowering's invented meaning: the clean prefix runs on entry and this is
    /// raised after it (#1829).  `None` for every body that parses whole.
    pub(crate) fatal_tail: Option<FatalTail>,
    /// What the module this unit came from says about the world it was
    /// compiled for, so the rungs a disagreeing field rests on are refused
    /// again whenever the unit is re-checked. `None` for a unit no compiler
    /// stated a manifest for: plain-dispatch children, scanner activations.
    pub(crate) manifest: Option<Arc<ArtefactIdentityManifest>>,
}

impl CompiledUnit {
    pub(crate) fn native_script(
        entry: crate::native_jim_script::NativeJimScriptEntry,
        source_namespace: tcl_core_types::ByteNamespacePath,
        profile_generation: u64,
        interpreter: tcl_runtime_api::native_compilation::NativeInterpreterIdentity,
    ) -> Self {
        Self {
            native_local_names: Rc::new(std::cell::RefCell::new(None)),
            compiled_local_layout: None,
            jim_script: Some(entry),
            asm: Rc::new(FunctionAsm::default()),
            fixed_math_calls: None,
            literal_pool: Err(crate::literal_pool::NativeLiteralUnavailable::uninitialized()),
            direct_source_operands: None,
            source_location: None,
            source_namespace,
            profile_generation,
            command_epoch: 0,
            native_cache: None,
            interpreter,
            compiler: CompilerProvenance::NativeScript,
            fatal_tail: None,
            manifest: None,
        }
    }
    /// Move a worker-produced original script into its actual deferred Jim
    /// activation. Other compiled units retain their independent source path.
    pub(crate) fn with_deferred_script_original(
        self,
        original: crate::Value,
    ) -> Result<Self, tcl_syntax::value::ValueError> {
        if let Some(entry) = &self.jim_script {
            entry.retain_activation_original(original)?;
        }
        Ok(self)
    }

    pub(crate) fn new(
        asm: Rc<FunctionAsm>,
        source_namespace: tcl_core_types::ByteNamespacePath,
        profile_generation: u64,
        command_epoch: u64,
        native_cache: Option<&NativeCompilerCacheStamp>,
        interpreter: tcl_runtime_api::native_compilation::NativeInterpreterIdentity,
        compiler: CompilerProvenance,
    ) -> Self {
        static NEXT_LAYOUT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        let token = tcl_runtime_api::checked_counter::allocate(&NEXT_LAYOUT)
            .expect("compiled layout identity space exhausted");
        let compiled_local_layout =
            tcl_runtime_api::native_compilation::NativeCompiledLocalLayout {
                owner: interpreter,
                token,
                epoch: profile_generation,
                kind: tcl_runtime_api::native_compilation::NativeCompiledLocalLayoutKind::Procedure,
                names: asm.lvt.native_slot_names(),
            };
        Self {
            native_local_names: Rc::new(std::cell::RefCell::new(None)),
            compiled_local_layout: Some(compiled_local_layout),
            jim_script: None,
            asm,
            fixed_math_calls: None,
            literal_pool: Err(crate::literal_pool::NativeLiteralUnavailable::uninitialized()),
            direct_source_operands: None,
            source_location: None,
            source_namespace,
            profile_generation,
            command_epoch,
            native_cache: native_cache.copied(),
            interpreter,
            compiler,
            fatal_tail: None,
            manifest: None,
        }
    }

    pub(crate) fn with_literal_pool(
        mut self,
        pool: crate::literal_pool::NativeLiteralPoolReceipt,
    ) -> Self {
        self.literal_pool = pool;
        self
    }

    pub(crate) fn with_source_location(
        mut self,
        location: Option<tcl_runtime_api::script_source_location::ScriptSourceLocation>,
    ) -> Self {
        self.source_location = location;
        self
    }

    /// Carry the manifest of the module this unit came from.
    pub(crate) fn with_manifest(mut self, manifest: Option<Arc<ArtefactIdentityManifest>>) -> Self {
        self.manifest = manifest;
        self
    }

    /// Carry the parse error this unit raises once its clean prefix has run.
    pub(crate) fn with_fatal_tail(mut self, fatal_tail: Option<FatalTail>) -> Self {
        self.fatal_tail = fatal_tail;
        self
    }
}

// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Live interpreter entry facts supplied to runtime compilation.

use tcl_core_types::{ByteNamespacePath, NameBytes, NativeByteCommandSlot};

/// A never-reused interpreter identity within one runtime owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NativeInterpreterIdentity {
    /// Process-unique owner identity.
    pub owner: u64,
    /// Never-reused interpreter arena slot.
    pub interpreter: u64,
}

impl NativeInterpreterIdentity {
    /// Allocate a never-reused runtime owner across all concrete engine adapters.
    /// Interpreter slots remain an independently managed arena within this owner.
    ///
    /// # Panics
    /// Panics if the process exhausts the owner identity space.
    #[must_use]
    pub fn fresh_owner() -> u64 {
        use std::sync::atomic::AtomicU64;
        static NEXT_OWNER: AtomicU64 = AtomicU64::new(1);
        crate::checked_counter::allocate(&NEXT_OWNER)
            .expect("native runtime owner identity space exhausted")
    }
}

/// Retained owner of a reusable compiled-local index layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeCompiledLocalLayoutKind {
    /// The retained procedure body owns the compiler declaration slots.
    Procedure,
    /// A retained native local cache owns slots borrowed by an entered script.
    RetainedLocalCache,
}

/// Reusable compiled-local layout, independent of any activation's variables.
/// Dynamic hash bindings never contribute entries or index permissions.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativeCompiledLocalLayout {
    /// Interpreter that owns the retained layout.
    pub owner: NativeInterpreterIdentity,
    /// Never-reused retained layout token within this owner.
    pub token: u64,
    /// Layout incarnation; replacement invalidates earlier receipts.
    pub epoch: u64,
    /// Actual retained layout owner.
    pub kind: NativeCompiledLocalLayoutKind,
    /// Native index order, including temporary slots without source names.
    pub names: Vec<Option<NameBytes>>,
}

/// Namespace in which an actual alias callable resolves its target prefix.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeAliasTargetLookup {
    /// Lookup begins in the target interpreter's global namespace.
    Global,
    /// Lookup begins in the current caller namespace.
    CallerNamespace,
}

/// Implementation installed at one live command-table slot.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NativeCommandImplementation {
    /// An actual stock handler, independently identified by the runtime.
    Registry {
        /// Stable registry implementation identity, retained through moves.
        identity: String,
        /// The token carries the native command-specific compiler hook.
        compiler_hook: bool,
    },
    /// A known command with an implementation the compiler does not interpret.
    Opaque,
    /// A name-following command prefix, with its exact target interpreter.
    Alias {
        /// Interpreter in which the prefix resolves.
        interpreter: NativeInterpreterIdentity,
        /// Actual callable's target namespace lookup rule.
        target_lookup: NativeAliasTargetLookup,
        /// Target command followed by frozen prefix arguments.
        words: Vec<NameBytes>,
    },
    /// An alias prefix whose stored value objects have not all acquired bytes.
    /// The positions retain exact argc without forcing lazy string conversion.
    PartialAlias {
        /// Interpreter in which the command prefix resolves.
        interpreter: NativeInterpreterIdentity,
        /// Actual callable's target namespace lookup rule.
        target_lookup: NativeAliasTargetLookup,
        /// Existing string bytes only. Missing entries are value objects whose
        /// string representation depends on later execution context.
        words: Vec<Option<NameBytes>>,
    },
    /// A namespace import whose lifecycle refers to an exact source token.
    Imported {
        /// Source interpreter (normally the containing interpreter).
        interpreter: NativeInterpreterIdentity,
        /// Source token identity; never re-resolve it by a display spelling.
        token: u64,
        /// Current source slot, when the token remains visible.
        slot: Option<NativeByteCommandSlot>,
    },
}

/// Presence of a command-specific native compiler hook, independently of
/// whether the compiler knows the handler's semantics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeCompilerHookPresence {
    /// Actual registration has no command-specific compiler hook.
    Absent,
    /// Actual registration supplies a command-specific compiler hook.
    Present,
    /// The runtime cannot certify compiler-hook registration.
    Unknown,
}

/// Independently stamped native compiler registration. This does not identify
/// the runtime handler, which may remain opaque after an ensemble map changes.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativeCommandCompiler {
    /// Registry descriptor of the actual installed compiler implementation.
    pub registry_identity: String,
    /// Actual ensemble configuration consumed by that compiler, when applicable.
    pub ensemble: Option<NativeEnsembleCompiler>,
}

/// Actual mutable ensemble operands at a compiler boundary. Stored value
/// objects with no string bytes retain `None`; admission never forces them.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativeEnsembleCompiler {
    /// Native namespace incarnation that supplies unmapped/exported members.
    pub namespace_token: u64,
    /// Insertion-ordered member to target-command-prefix mappings.
    pub map: Vec<(NameBytes, Vec<Option<NameBytes>>)>,
    /// Explicit member roster, or namespace exports when absent.
    pub subcommands: Option<Vec<NameBytes>>,
    /// Whether unambiguous member abbreviations are admitted.
    pub prefixes: bool,
    /// Leading ensemble parameter names before the member operand.
    pub parameters: Vec<NameBytes>,
    /// Actual unresolved-member callback prefix, when configured.
    pub unknown_handler: Option<Vec<Option<NameBytes>>>,
}

/// Exact ensemble compiler and mutable inputs selected before argument
/// execution. This prerequisite supplies no runtime handler identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativeCommandCompilerPrerequisite {
    /// Runtime owner and interpreter that supplied the compiler registration.
    pub interpreter: NativeInterpreterIdentity,
    /// Namespace incarnation from which the written public command resolves.
    pub lookup_namespace_token: u64,
    /// Written public command word selected from that namespace.
    pub invocation_word: NameBytes,
    /// Actual public command slot, independent of display spelling.
    pub slot: NativeByteCommandSlot,
    /// Namespace incarnation containing the public command token.
    pub namespace_token: u64,
    /// Actual public command token.
    pub token: u64,
    /// Implementation incarnation of that token.
    pub implementation_generation: u64,
    /// Actual compiler registration and exact mutable configuration.
    pub compiler: NativeCommandCompiler,
    /// Actual mapped worker at compilation admission. This is retained proof,
    /// not a live invocation guard: non-invalidating worker replacement keeps
    /// a cached captured-name invocation, whose handler resolves after arguments.
    pub selected_worker: Option<NativeCompilationBinding>,
    /// Original compiler registrations reached through nested ensemble maps.
    /// These preserve each intermediate configuration independently of the
    /// final worker's late runtime handler lookup.
    pub nested_compilers: Vec<NativeCommandCompilerPrerequisite>,
    /// Boundary at which compilation selection is retained for the invocation.
    pub guard: crate::CommandBindingGuard,
}

impl NativeCommandCompilerPrerequisite {
    /// Validate every retained original compiler registration before operands run.
    /// The lookup consumes original written map prefixes, never reported slots.
    ///
    /// # Errors
    /// Preserves an unavailable lookup from the actual registration provider.
    pub fn matches_registration_with<E>(
        &self,
        mut lookup: impl FnMut(u64, &NameBytes) -> Result<Option<NativeCompilationBinding>, E>,
    ) -> Result<bool, E> {
        let mut pending = vec![self];
        while let Some(required) = pending.pop() {
            if required.interpreter != self.interpreter {
                return Ok(false);
            }
            let Some(binding) = lookup(required.lookup_namespace_token, &required.invocation_word)?
            else {
                return Ok(false);
            };
            if binding.slot != required.slot
                || binding.namespace_token != required.namespace_token
                || binding.token != required.token
                || binding.implementation_generation != required.implementation_generation
                || binding.compiler_hook != NativeCompilerHookPresence::Present
                || binding.compiler.as_ref() != Some(&required.compiler)
                || binding.has_execution_trace
            {
                return Ok(false);
            }
            pending.extend(&required.nested_compilers);
        }
        Ok(true)
    }
}

/// An ensemble uses the same exact original compiler-registration receipt as
/// an ordinary command, with its mutable configuration and selected worker.
pub type NativeEnsembleCompilerPrerequisite = NativeCommandCompilerPrerequisite;

/// Actual compiler operation selected once for an invocation's instruction range.
/// The operation's implementation and configuration are separate from late dispatch.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NativeCompilerSelectionPrerequisite {
    /// Actual ordinary command compiler, independently of its runtime handler.
    Command(std::sync::Arc<NativeCommandCompilerPrerequisite>),
    /// Actual procedure-header compiler and callable incarnation.
    ProcedureHeader(Box<NativeProcedureHeaderPrerequisite>),
    /// Actual ensemble compiler with its captured mutable configuration.
    Ensemble(std::sync::Arc<NativeEnsembleCompilerPrerequisite>),
}

impl NativeCompilerSelectionPrerequisite {
    /// Classify a retained original registration by its actual compiler payload.
    /// This preserves its namespace, token, configuration and worker receipt;
    /// it grants no authority from the callable handler or command spelling.
    #[must_use]
    pub fn from_command_registration(
        required: std::sync::Arc<NativeCommandCompilerPrerequisite>,
    ) -> Self {
        if required.compiler.ensemble.is_some() {
            Self::Ensemble(required)
        } else {
            Self::Command(required)
        }
    }

    /// Validation boundary retained by the native compiler owner.
    #[must_use]
    pub fn guard(&self) -> crate::CommandBindingGuard {
        match self {
            Self::Command(required) | Self::Ensemble(required) => required.guard,
            Self::ProcedureHeader(required) => required.guard,
        }
    }
}

/// One visible command binding at compilation entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativeCompilationBinding {
    /// Injective constructed slot; display strings are not identities.
    pub slot: NativeByteCommandSlot,
    /// Namespace token owning this slot, distinct from a same-named recreation.
    pub namespace_token: u64,
    /// Stable command token identity in this interpreter.
    pub token: u64,
    /// Implementation incarnation independent of the token's current slot.
    pub implementation_generation: u64,
    /// Actual selected implementation or explicitly opaque implementation.
    pub implementation: NativeCommandImplementation,
    /// Actual compiler-hook registration; absence does not grant handler facts.
    pub compiler_hook: NativeCompilerHookPresence,
    /// Actual compiler implementation and mutable inputs, independent of handler identity.
    pub compiler: Option<NativeCommandCompiler>,
    /// Audited procedure header compiler, when this is an actual procedure.
    pub procedure_header: Option<tcl_dialect::NativeProcedureHeaderCompilation>,
    /// An execution trace prevents native command-specific compilation.
    pub has_execution_trace: bool,
}

/// Exact actual procedure header selected by a native command compiler.
/// This prerequisite confers no registry handler identity or body semantics.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativeProcedureHeaderPrerequisite {
    /// Runtime owner and interpreter that supplied the callable token.
    pub interpreter: NativeInterpreterIdentity,
    /// Actual namespace incarnation from which the written head resolves.
    pub lookup_namespace_token: u64,
    /// Literal command word resolved from that namespace at selection time.
    pub invocation_word: NameBytes,
    /// Constructed command slot, independent of its display spelling.
    pub slot: NativeByteCommandSlot,
    /// Owning namespace incarnation.
    pub namespace_token: u64,
    /// Actual callable token.
    pub token: u64,
    /// Implementation incarnation of that callable.
    pub implementation_generation: u64,
    /// Independently certified native procedure header compiler.
    pub header: tcl_dialect::NativeProcedureHeaderCompilation,
    /// Boundary at which the selected token becomes frozen for this invocation.
    pub guard: crate::CommandBindingGuard,
}

/// Exact namespace context retained by an actual native compilation entry.
/// The token denotes an incarnation within its interpreter; component paths
/// retain lookup geometry independently of non-injective display spellings.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NativeNamespaceContext {
    /// Actual runtime owner and interpreter.
    pub interpreter: NativeInterpreterIdentity,
    /// Stable actual namespace incarnation token.
    pub token: u64,
    /// Exact constructed component boundaries.
    pub path: ByteNamespacePath,
}

/// Complete root-name inventory of one actual namespace variable table.
/// This capability is captured independently of the command table. Names
/// include undefined and linked entries; no value getter, alias traversal,
/// variable representation or observer absence is implied.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativeNamespaceVariableTable {
    /// Actual interpreter and namespace incarnation owning the observed table.
    pub namespace: NativeNamespaceContext,
    /// Original table keys, including undefined cells and link entries.
    pub roots: Vec<NameBytes>,
}

/// Namespace lookup state at compilation entry.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativeCompilationNamespace {
    /// Constructed namespace path, preserving literal colon segments.
    pub path: ByteNamespacePath,
    /// Actual Jim namespace-object spelling. A constructed C path or its
    /// display spelling cannot establish this independent flat-name context.
    /// C providers use `None`; missing Jim evidence remains unknown.
    pub jim_namespace_object: Option<NameBytes>,
    /// Stable namespace token identity.
    pub token: u64,
    /// Whether the namespace token is reachable in the live namespace tree.
    /// A deleted token may remain usable only by its retained activation.
    pub visible: bool,
    /// Actual namespace export patterns.
    pub exports: Vec<NameBytes>,
    /// Ordered command search path as stable namespace tokens.
    pub command_path: Vec<u64>,
    /// Namespace unknown prefix, when explicitly installed. The outer option
    /// distinguishes absence; each inner option retains already materialised
    /// bytes without converting a stored value object at compiler admission.
    pub unknown_handler: Option<Vec<Option<NameBytes>>>,
}

/// Actual variable activation kind; cell contents/history remain unknown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeCompilationFrame {
    /// The root global activation.
    Global,
    /// A namespace-evaluation activation.
    Namespace,
    /// A procedure activation (including an eval inside that procedure).
    Procedure,
    /// The runtime cannot provide a closed activation kind.
    Unknown,
}

/// Actual variable-observer surface, independent of variable contents and links.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeVariableObserverPresence {
    /// The complete interpreter table contains no variable observers.
    Absent,
    /// Only engine-owned hooks with a bounded contract and no Tcl or opaque
    /// callback entry are registered. Variable contents remain unknown.
    BoundedNativeOnly,
    /// Script callbacks, opaque native observers, or retained active trace
    /// firing may enter arbitrary Tcl commands.
    Present,
    /// The provider cannot close the observer table.
    Unknown,
}

impl NativeVariableObserverPresence {
    /// Whether actual registrations prove that a variable access cannot enter
    /// an arbitrary callback. This supplies no variable-value or link proof.
    #[must_use]
    pub const fn permits_no_callbacks(self) -> bool {
        matches!(self, Self::Absent | Self::BoundedNativeOnly)
    }
}

/// Observed original target primary at an actual ensemble compiler boundary.
/// These records retain no object, callable or command-node ownership.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum NativeEnsembleTargetPrimary {
    /// An original NULL or native String primary with callback-free retirement.
    StockString,
    /// The same original target has an actual command-name primary.
    CommandName {
        /// Original descriptor release, including unresolved C8 descriptors.
        origin: tcl_dialect::TclVersion,
        /// Actual cached node, absent for an unresolved descriptor.
        cache: Option<crate::native_command_name::NativeCommandNameCache>,
        /// Passive current observations for that cached node and lookup context.
        lookup: Option<crate::native_command_name::NativeCommandNameLookupState>,
    },
    /// Its primary/getter/retirement boundary remains unproved.
    Unavailable,
}

/// Passive observation of one original configured ensemble prefix element.
/// Compiler configuration equality deliberately excludes this object state.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativeEnsembleTargetObservation {
    /// Actual public ensemble command node supplying the original map.
    pub ensemble_token: u64,
    /// Canonical actual map member, independent of an abbreviated source word.
    pub member: NameBytes,
    /// Position in the original mapped prefix.
    pub prefix_index: usize,
    /// Opaque identity of the same retained original target object.
    pub object_identity: u64,
    /// Existing original bytes; no updater is run to obtain this observation.
    pub resident_name: Option<NameBytes>,
    /// Original primary and independently observed current command world.
    pub primary: NativeEnsembleTargetPrimary,
}

/// Closed live command/namespace world at one runtime compilation boundary.
///
/// Missing rows denote absent bindings only when `closed` is true. This does
/// not assert fresh variables, filesystem state, package providers or traces.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativeCompilationEntry {
    /// Runtime owner/interpreter identity.
    pub interpreter: NativeInterpreterIdentity,
    /// Runtime mutation epoch; retained in addition to structural equality.
    pub epoch: u64,
    /// Exact selected profile, independent of assistance catalogue identity.
    pub profile: tcl_dialect::DialectProfileKey,
    /// Exact logical native handler policy, including an active host override.
    /// This is independent of source grammar and physical compiler provenance.
    /// Absence retains unknown handler/value/frame policies; the physical engine
    /// point cannot fill this slot.
    pub invocation_policy: Option<tcl_dialect::DialectProfileKey>,
    /// Actual native engine point, also when the assistance profile is permissive.
    pub execution_point: Option<tcl_dialect::model::DialectPoint>,
    /// Audited native name-input issuer. This remains independent of logical
    /// handler simulation and source grammar; missing evidence stays unknown.
    pub name_protocol: Option<tcl_syntax::naming::NamePolicyProtocol>,
    /// Independently selected compiler-local recipe. A missing live policy
    /// remains unknown, even when the physical host compiler point is known.
    pub compiled_variable_protocol: Option<tcl_syntax::naming::NativeCompiledVariableProtocol>,
    /// Independently selected source-word byte recipe. Logical source providers
    /// do not authenticate physical string representations or compiler names.
    pub source_string_protocol: Option<tcl_syntax::native_string::NativeStringProtocol>,
    /// Actual retained compiled-slot layout available to the active frame.
    pub compiled_local_layout: Option<NativeCompiledLocalLayout>,
    /// Actual lexical grammar, including independently configured engine axes.
    /// Absence preserves the compile driver's supplied grammar; an engine point
    /// alone does not prove that custom lexical axes use canonical defaults.
    pub lexer_grammar: Option<tcl_dialect::LexerGrammar>,
    /// Actual interpreter flag disabling command compiler hooks. Parsing,
    /// argument evaluation and bytecode-object compilation remain enabled.
    pub inline_compilation_disabled: bool,
    /// Actual fixed math-function compiler table, when the runtime can export
    /// one. Command catalogue names do not establish this separate table.
    pub math_functions: Option<NativeMathFunctionTable>,
    /// Whether the visible table/namespace rows are complete.
    pub closed: bool,
    /// Actual visible command slots, including opaque custom commands.
    pub commands: Vec<NativeCompilationBinding>,
    /// Same-entry original map target observations, independent of logical map
    /// bytes and of configuration/compiled-prerequisite equality. Missing rows
    /// or inventory do not imply a fresh String primary.
    pub ensemble_target_objects: Option<Vec<NativeEnsembleTargetObservation>>,
    /// Actual namespace lookup state.
    pub namespaces: Vec<NativeCompilationNamespace>,
    /// Stable active namespace token, including a retained deleted token.
    pub current_namespace: u64,
    /// Actual variable observer surface. Closure is independent of command
    /// table closure and does not imply fresh variable contents or links.
    pub variable_observers: NativeVariableObserverPresence,
    /// Independently captured complete variable-root tables. Missing tables
    /// remain open; command or observer closure cannot fill this capability.
    pub namespace_variable_tables: Option<Vec<NativeNamespaceVariableTable>>,
    /// Independently installed authored worker publication contract. Missing
    /// concrete TMM providers do not acquire this from a dialect/profile.
    pub authored_tmm_static: Option<crate::authored_tmm::AuthoredTmmStaticCompilationContext>,
    /// Actual active variable frame kind; its cells are not presumed fresh.
    pub frame: NativeCompilationFrame,
}

/// A command lookup that the retained native entry cannot settle.
/// This is a capability residual, not a Tcl missing-command result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeCommandLookupUnavailable {
    /// The snapshot does not describe every candidate slot.
    OpenTable,
    /// No audited naming policy was retained.
    NamePolicy,
    /// A required namespace token is missing or conflicts with another row.
    Namespace,
    /// The snapshot has no retained parent edges for a deleted subtree.
    RetainedDescendant,
    /// More than one binding occupies the same namespace incarnation and slot.
    ConflictingBinding,
}

/// An exact candidate slot in an actual namespace incarnation. Its path is
/// retained component data, not a written or displayed command address.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativeCommandLookupCandidate {
    /// Actual namespace incarnation owning the candidate command table.
    pub namespace_token: u64,
    /// Original-input projection under the selected native naming policy.
    pub slot: NativeByteCommandSlot,
}

#[derive(Debug, Clone, Copy)]
enum NativeLookupBase {
    Token(u64),
    Root,
}

/// Stop-aware native lookup traversal over an immutable original namespace
/// context. Advancing validates only the candidate now reached by lookup.
pub struct NativeCommandLookupCursor<'a> {
    entry: &'a NativeCompilationEntry,
    original: &'a [u8],
    protocol: tcl_syntax::naming::NativeNameProtocol,
    bases: std::vec::IntoIter<NativeLookupBase>,
    jim_keys: Option<std::vec::IntoIter<NameBytes>>,
    visited: Vec<u64>,
}

impl NativeCommandLookupCursor<'_> {
    /// Visit the next native candidate after the preceding candidate was absent
    /// in the consumer's current binding state.
    ///
    /// # Errors
    /// Returns an error for missing, conflicting or unavailable namespace
    /// provenance at this reached fallback step.
    pub fn next_candidate(
        &mut self,
    ) -> Result<Option<NativeCommandLookupCandidate>, NativeCommandLookupUnavailable> {
        use tcl_syntax::naming::NativeNameContext;
        if let Some(keys) = &mut self.jim_keys {
            let Some(key) = keys.next() else {
                return Ok(None);
            };
            let root = self
                .entry
                .visible_namespace_by_path(&ByteNamespacePath::root())?
                .ok_or(NativeCommandLookupUnavailable::Namespace)?;
            return Ok(Some(NativeCommandLookupCandidate {
                namespace_token: root.token,
                slot: NativeByteCommandSlot::new(ByteNamespacePath::root(), key),
            }));
        }
        for base in self.bases.by_ref() {
            let namespace = match base {
                NativeLookupBase::Token(token) => self.entry.namespace_by_token(token)?,
                NativeLookupBase::Root => self
                    .entry
                    .visible_namespace_by_path(&ByteNamespacePath::root())?
                    .ok_or(NativeCommandLookupUnavailable::Namespace)?,
            };
            if self.visited.contains(&namespace.token) {
                continue;
            }
            self.visited.push(namespace.token);
            let slot = self
                .protocol
                .command_lookup_slot(NativeNameContext::new(&namespace.path), self.original)
                .map_err(|_| NativeCommandLookupUnavailable::NamePolicy)?;
            let selected = if slot.namespace == namespace.path {
                Some(namespace)
            } else {
                if !namespace.visible {
                    return Err(NativeCommandLookupUnavailable::RetainedDescendant);
                }
                self.entry.visible_namespace_by_path(&slot.namespace)?
            };
            if let Some(selected) = selected {
                return Ok(Some(NativeCommandLookupCandidate {
                    namespace_token: selected.token,
                    slot,
                }));
            }
        }
        Ok(None)
    }
}

impl NativeCompilationEntry {
    /// Compare actual compilation world for reuse of an already retained artifact.
    /// Original map primary observations affect a new compiler lookup, but
    /// harmless target shimmering does not invalidate existing native bytecode.
    /// Compiler epochs, registrations, configuration and context still match.
    /// Independent variable-table inventories also match for source proofs
    /// depending on incoming absence. This is a cache-input comparison, not
    /// running-frame or native Bytecode freshness after a legitimate store.
    #[must_use]
    pub fn same_compilation_world(&self, other: &Self) -> bool {
        self.interpreter == other.interpreter
            && self.epoch == other.epoch
            && self.profile == other.profile
            && self.invocation_policy == other.invocation_policy
            && self.execution_point == other.execution_point
            && self.name_protocol == other.name_protocol
            && self.compiled_variable_protocol == other.compiled_variable_protocol
            && self.source_string_protocol == other.source_string_protocol
            && self.compiled_local_layout == other.compiled_local_layout
            && self.lexer_grammar == other.lexer_grammar
            && self.inline_compilation_disabled == other.inline_compilation_disabled
            && self.math_functions == other.math_functions
            && self.closed == other.closed
            && self.commands == other.commands
            && self.namespaces == other.namespaces
            && self.current_namespace == other.current_namespace
            && self.variable_observers == other.variable_observers
            && self.namespace_variable_tables == other.namespace_variable_tables
            && self.authored_tmm_static == other.authored_tmm_static
            && self.frame == other.frame
    }

    /// Resolve an original byte operand in this immutable compilation snapshot.
    ///
    /// Namespace tokens select incarnations; structured paths select descendants
    /// only in the live tree. C lookup uses the selected `CString` extent and
    /// ordered namespace-path/global candidates. Jim uses the separately retained
    /// namespace object and its native flat keys. The returned registration grants
    /// no handler semantics, compiler permission or future dispatch currentness.
    /// `Ok(None)` means closed absence; an incomplete or conflicting snapshot is
    /// an explicit error, never a fallback to a same-spelled generation.
    ///
    /// # Errors
    /// Returns the missing snapshot capability or conflicting observation.
    pub fn lookup_command_bytes(
        &self,
        lookup_namespace: u64,
        original: &[u8],
    ) -> Result<Option<&NativeCompilationBinding>, NativeCommandLookupUnavailable> {
        let mut cursor = self.command_lookup_cursor(lookup_namespace, original)?;
        while let Some(candidate) = cursor.next_candidate()? {
            if let Some(binding) =
                self.binding_at_slot(candidate.namespace_token, &candidate.slot)?
            {
                return Ok(Some(binding));
            }
        }
        Ok(None)
    }

    /// Retain a checked namespace incarnation without reconstructing its path
    /// from the namespace's display spelling.
    ///
    /// # Errors
    /// Returns an error for a missing or conflicting token row.
    pub fn namespace_context(
        &self,
        token: u64,
    ) -> Result<&NativeCompilationNamespace, NativeCommandLookupUnavailable> {
        self.namespace_by_token(token)
    }

    /// Retain an actual namespace identity and its original component geometry.
    ///
    /// # Errors
    /// Returns an error for a missing or conflicting token row.
    pub fn retained_namespace_context(
        &self,
        token: u64,
    ) -> Result<NativeNamespaceContext, NativeCommandLookupUnavailable> {
        let row = self.namespace_by_token(token)?;
        Ok(NativeNamespaceContext {
            interpreter: self.interpreter,
            token: row.token,
            path: row.path.clone(),
        })
    }

    /// Start original-word lookup without visiting later fallback contexts.
    /// Consumers must request the next candidate only when their current
    /// binding state permits native lookup to continue.
    ///
    /// # Errors
    /// Returns an error for an open snapshot, missing naming policy or invalid
    /// initial namespace context. Later candidate failures remain lazy.
    pub fn command_lookup_cursor<'a>(
        &'a self,
        lookup_namespace: u64,
        original: &'a [u8],
    ) -> Result<NativeCommandLookupCursor<'a>, NativeCommandLookupUnavailable> {
        use tcl_syntax::naming::{NativeNameContext, NativeNameProtocol, NativeNameQualification};
        if !self.closed {
            return Err(NativeCommandLookupUnavailable::OpenTable);
        }
        let protocol = self
            .name_protocol
            .ok_or(NativeCommandLookupUnavailable::NamePolicy)?
            .recipe();
        let context = self.namespace_by_token(lookup_namespace)?;
        let (bases, jim_keys) = if protocol.is_jim084() {
            let object = context
                .jim_namespace_object
                .as_ref()
                .ok_or(NativeCommandLookupUnavailable::Namespace)?;
            let keys = protocol
                .jim_command_lookup_keys(
                    NativeNameContext::with_jim_namespace(&context.path, object.as_bytes()),
                    original,
                )
                .map_err(|_| NativeCommandLookupUnavailable::NamePolicy)?;
            (Vec::new(), Some(keys.into_iter()))
        } else {
            let projection = protocol
                .command_lookup_input(NativeNameContext::new(&context.path), original)
                .map_err(|_| NativeCommandLookupUnavailable::NamePolicy)?;
            let absolute = projection.qualification() == NativeNameQualification::Absolute;
            let mut bases = Vec::new();
            if !absolute {
                bases.push(NativeLookupBase::Token(context.token));
                if matches!(protocol, NativeNameProtocol::C(version) if version.has_namespace_path())
                {
                    bases.extend(
                        context
                            .command_path
                            .iter()
                            .copied()
                            .map(NativeLookupBase::Token),
                    );
                }
            }
            // The root row is checked only after all earlier candidates permit
            // fallback; a malformed later row cannot spoil an earlier hit.
            bases.push(NativeLookupBase::Root);
            (bases, None)
        };
        Ok(NativeCommandLookupCursor {
            entry: self,
            original,
            protocol,
            bases: bases.into_iter(),
            jim_keys,
            visited: Vec::new(),
        })
    }

    fn namespace_by_token(
        &self,
        token: u64,
    ) -> Result<&NativeCompilationNamespace, NativeCommandLookupUnavailable> {
        let mut rows = self
            .namespaces
            .iter()
            .filter(|namespace| namespace.token == token);
        let first = rows
            .next()
            .ok_or(NativeCommandLookupUnavailable::Namespace)?;
        if rows.next().is_some() {
            return Err(NativeCommandLookupUnavailable::Namespace);
        }
        Ok(first)
    }

    fn visible_namespace_by_path(
        &self,
        path: &ByteNamespacePath,
    ) -> Result<Option<&NativeCompilationNamespace>, NativeCommandLookupUnavailable> {
        let mut rows = self
            .namespaces
            .iter()
            .filter(|namespace| namespace.visible && &namespace.path == path);
        let first = rows.next();
        if rows.next().is_some() {
            return Err(NativeCommandLookupUnavailable::Namespace);
        }
        Ok(first)
    }

    fn binding_at_slot(
        &self,
        namespace_token: u64,
        slot: &NativeByteCommandSlot,
    ) -> Result<Option<&NativeCompilationBinding>, NativeCommandLookupUnavailable> {
        let mut rows = self
            .commands
            .iter()
            .filter(|binding| binding.namespace_token == namespace_token && &binding.slot == slot);
        let first = rows.next();
        if rows.next().is_some() {
            return Err(NativeCommandLookupUnavailable::ConflictingBinding);
        }
        Ok(first)
    }
}

/// Native fixed-function registration at an observed compiler boundary.
/// This table is independent of Tcl command and namespace bindings.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativeMathFunctionTable {
    /// Missing rows prove absence only in a complete registration table.
    pub closed: bool,
    /// Registration generation, including replacement of an existing row.
    pub generation: u64,
    /// Actual registrations supplied by the runtime provider.
    pub functions: Vec<NativeMathFunctionBinding>,
}

/// Actual interpreter/table prerequisite for a reused native compilation result.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativeMathFunctionPrerequisite {
    /// Owner and interpreter that supplied the registration table.
    pub interpreter: NativeInterpreterIdentity,
    /// Complete observed registration identities and compiler contracts.
    pub table: NativeMathFunctionTable,
}

/// Compiler-visible fixed-function identity and argument-count contract.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativeMathFunctionBinding {
    /// Name consumed by the selected native expression compiler.
    pub name: NameBytes,
    /// Stable registration identity within the owning interpreter.
    pub token: u64,
    /// Implementation incarnation; replacing a function changes this value.
    pub implementation_generation: u64,
    /// Actual stock implementation contract independently stamped at native
    /// registration. Opaque installed functions retain `None` even when their
    /// name and arity happen to match a catalogue row.
    pub registry_identity: Option<String>,
    /// Exact fixed arity, or an opaque installed registration whose contract
    /// requires a genuine provider. Opaque presence never proves absence.
    pub arity: Option<usize>,
}

/// Outcome of lookup in actual fixed-function registration evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeMathFunctionResolution<'a> {
    /// All represented registrations agree on this function and its contract.
    Present(&'a NativeMathFunctionBinding),
    /// A complete table proves the name is absent.
    Absent,
    /// Missing or conflicting registration evidence prevents a proof.
    Unknown,
}

impl NativeMathFunctionTable {
    /// Resolve without choosing an arbitrary first conflicting row. A known
    /// row remains usable in an open table; only absence requires closure.
    #[must_use]
    pub fn lookup(&self, name: &str) -> NativeMathFunctionResolution<'_> {
        self.lookup_bytes(name.as_bytes())
    }

    /// Resolve the exact measured registration key without Unicode projection.
    #[must_use]
    pub fn lookup_bytes(&self, name: &[u8]) -> NativeMathFunctionResolution<'_> {
        let mut candidates = self
            .functions
            .iter()
            .filter(|row| row.name.as_bytes() == name);
        let Some(first) = candidates.next() else {
            return if self.closed {
                NativeMathFunctionResolution::Absent
            } else {
                NativeMathFunctionResolution::Unknown
            };
        };
        if candidates.any(|row| row != first) {
            NativeMathFunctionResolution::Unknown
        } else {
            NativeMathFunctionResolution::Present(first)
        }
    }
}

#[cfg(test)]
mod math_function_tests {
    use super::*;

    #[test]
    fn an_opaque_installed_function_is_present_without_claiming_an_arity() {
        let table = NativeMathFunctionTable {
            closed: true,
            generation: 3,
            functions: vec![NativeMathFunctionBinding {
                name: "host".into(),
                token: 4,
                implementation_generation: 3,
                registry_identity: None,
                arity: None,
            }],
        };
        let NativeMathFunctionResolution::Present(row) = table.lookup("host") else {
            panic!("opaque row must remain present")
        };
        assert!(row.arity.is_none());
        assert_eq!(table.lookup("absent"), NativeMathFunctionResolution::Absent);
    }

    #[test]
    fn registration_lookup_preserves_closure_and_conflicting_incarnations() {
        let row = NativeMathFunctionBinding {
            name: "abs".into(),
            token: 7,
            implementation_generation: 3,
            registry_identity: Some("::tcl::mathfunc::abs".into()),
            arity: Some(1),
        };
        let mut table = NativeMathFunctionTable {
            closed: false,
            generation: 3,
            functions: vec![row.clone()],
        };
        assert_eq!(
            table.lookup("abs"),
            NativeMathFunctionResolution::Present(&row)
        );
        assert_eq!(
            table.lookup("missing"),
            NativeMathFunctionResolution::Unknown
        );
        table.closed = true;
        assert_eq!(
            table.lookup("missing"),
            NativeMathFunctionResolution::Absent
        );
        let mut replacement = row;
        replacement.implementation_generation += 1;
        table.functions.push(replacement);
        assert_eq!(table.lookup("abs"), NativeMathFunctionResolution::Unknown);
    }
}

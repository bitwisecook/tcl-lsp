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

//! The top-level module: its procedures, deferred writes, reference bodies and the queries over them.

use super::byte_source_namespace::OriginalByteSourceNamespace;
use super::definition_script::OriginalDefinitionScriptSource;
use super::definitions::{
    ClassRelation, MethodDef, OoDefinitionEvidence, Procedure, SourceIrulesEventBody,
};
use super::method_identity::MethodInventory;
use super::procedure_identity::{
    CarriedInstalledLowerings, ProcedureBodyKind, ProcedureId, ProcedureIdIssuer,
    ProcedureInventory,
};
use super::script::{ExecutionNamespace, Script};
use tcl_lexer::Span;
use tcl_runtime_api::CommandEvaluation;

/// What a [`Module`]'s top level holds.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TopLevelKind {
    /// A script's global level — code outside any procedure.
    #[default]
    Script,
    /// A single `proc` or `TclOO` method body, compiled as its own module by
    /// the runtime. It has a local variable table; a script's global level
    /// does not.
    ProcedureBody,
}

/// The variables the scripts a module stores as callbacks write, destroy or
/// bind.
///
/// A script a command stores to run after it returns — an `after` or
/// `fileevent` handler, a `bind`ing, a variable trace's callback, everything
/// the registry states as a callback
/// ([`tcl_registry::CommandRegistry::callback_script_indices`]) — runs at the
/// global level, or in the frame of the code that fires it. A plain name in
/// it can denote a variable the registering code holds, and nothing in that
/// code's text shows the write. Whole-module and position-independent, like
/// [`Module::traced_variables`]: a name in [`Self::names`] is externally
/// mutable in every function, and the solver never refines it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct DeferredWrites {
    /// The literal names a callback script writes, destroys or binds
    /// (`global`, `upvar`, `variable`), the leading `::` stripped and an
    /// element's array beside it.
    pub names: std::collections::BTreeSet<String>,
    /// A callback writes, destroys or binds a name computed at run time, so
    /// any variable may change at any time.
    pub any: bool,
    /// The original invocation sites of the statements that alone raised
    /// [`Self::any`], when every contribution has one: a function whose
    /// solve proves such a statement's block never runs may leave it out
    /// ([`Self::any_beyond`]). Empty when [`Self::any`] has a contribution no
    /// site names.
    pub any_sites: std::collections::BTreeSet<crate::command_binding::CommandAllocationSite>,
    /// The literal names a callback script reads, kept as [`Self::names`]
    /// keeps them. A store the code that registers the callback makes to one
    /// is observed when the callback runs (`after 0 {puts $x}`; `update`).
    pub reads: std::collections::BTreeSet<String>,
    /// A callback reads a name the scan cannot enumerate.
    pub reads_any: bool,
}

impl DeferredWrites {
    /// Whether no callback script of the module writes anything.
    #[must_use]
    pub fn is_clear(&self) -> bool {
        self.names.is_empty() && !self.any
    }

    /// Whether a callback may read the global `name` (no leading `::`).
    #[must_use]
    pub fn may_read(&self, name: &str) -> bool {
        self.reads_any || self.reads.contains(name)
    }

    /// [`Self::any`] once the statements at `excluded` sites are known never
    /// to run.
    #[must_use]
    pub fn any_beyond(
        &self,
        excluded: impl Fn(&crate::command_binding::CommandAllocationSite) -> bool,
    ) -> bool {
        self.any && (self.any_sites.is_empty() || self.any_sites.iter().any(|site| !excluded(site)))
    }
}

/// The fact of a build with no module to scan: no callback writes anything.
pub static NO_DEFERRED_WRITES: DeferredWrites = DeferredWrites {
    names: std::collections::BTreeSet::new(),
    any: false,
    any_sites: std::collections::BTreeSet::new(),
    reads: std::collections::BTreeSet::new(),
    reads_any: false,
};

/// A top-level module: procedures + top-level script.
///
/// This is the only mutable IR type — it accumulates procedures and
/// methods during lowering.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Module {
    /// Exact source-word grammar used at ingress. A profile name cannot
    /// reconstruct custom or independently selected grammar axes.
    pub lexer_config: tcl_lexer::LexerConfig,
    /// Resolved profile retained by ingress; presentation names cannot recreate it.
    pub dialect_profile: Option<&'static tcl_dialect::DialectProfile>,
    /// Exact command registry snapshot which produced this module.
    pub registry_snapshot: Option<tcl_registry::RegistrySnapshot>,
    /// Original compiler metadata ingress, including the complete availability
    /// generation and word grammar. This supplies no runtime entry or frame.
    pub source_metadata_input: Option<crate::analyser::ResolvedAnalysisInput>,
    /// Execution entry and selected provider provenance shared by source,
    /// CFG, and invocation consumers. It is part of the module cache input.
    pub source_entry: crate::command_binding::SourceAnalysisEntry,
    /// Original source interpretation shared by lowering and command-effect queries.
    /// Its owner checks source, grammar, entry, frame and registry before reuse.
    pub retained_source_bindings:
        Option<std::sync::Arc<crate::command_binding::RetainedSourceModuleBindings>>,
    /// Possible future callers, separate from actual normal document entry.
    /// This inventory only retracts caller assumptions; it grants no execution.
    pub future_call_sites: Vec<crate::command_binding::SourceFutureCallSite>,
    /// Analysis-only body units recovered from actual native installations.
    /// Full allocation/source identity groups their retention limits; neither
    /// the tag nor the body unit installs a callable or licenses compilation.
    pub installed_procedure_body_units:
        std::collections::BTreeMap<ProcedureId, crate::command_binding::CommandAllocation>,
    /// Installed lowerings carried by readonly authored units of
    /// [`Self::body_units`], each still tagged in
    /// [`Self::installed_procedure_body_units`] under its own body ID.
    /// [`Self::body_lowerings`] reaches them; they are not analysis units.
    pub carried_installed_lowerings: CarriedInstalledLowerings,
    /// Analysis-only units for original procedure declarations, independent
    /// of current installation or actual entry. Allocation, original source
    /// and namespace context retain their identity independently of the ID.
    pub original_declaration_body_units:
        std::collections::BTreeMap<ProcedureId, crate::command_binding::CommandAllocation>,
    /// Native-name source headers and read-flow cards retain original bodies
    /// without implementation allocation, publication or an entered frame.
    pub original_native_declaration_body_units: std::collections::BTreeMap<
        ProcedureId,
        std::sync::Arc<crate::command_binding::OriginalNativeProcedureBody>,
    >,
    /// Each named procedure whose commands the walk bound in a caller's
    /// activation, lowered again in its own declared frame: the body its
    /// transfer summary describes. Keyed by the procedure's own ID; never a
    /// body unit, so no other consumer sees a second copy.
    pub declared_frame_bodies: std::collections::BTreeMap<ProcedureId, Procedure>,
    /// Original Logical source cards retain analysis-only bodies without a
    /// successful command allocation, namespace publication or entered frame.
    pub original_logical_declaration_body_units: std::collections::BTreeMap<
        ProcedureId,
        std::sync::Arc<crate::command_binding::OriginalLogicalProcedureBody>,
    >,
    /// Standalone source declaration cards retain their actual original
    /// grammar and body without publication, physical frame or Native entry.
    pub original_authored_declaration_body_units: std::collections::BTreeMap<
        ProcedureId,
        std::sync::Arc<crate::registry_invocation::OriginalAuthoredProcedureBody>,
    >,
    /// Hosted source declarations retain their actual rule-loader owner,
    /// independently of Native callable publication or Logical source scopes.
    pub original_hosted_declaration_body_units: std::collections::BTreeMap<
        ProcedureId,
        std::sync::Arc<crate::command_binding::OriginalHostedProcedureBody>,
    >,
    /// Original body metadata for exact procedure implementations, including
    /// retired incarnations. This does not select a callable or grant entry,
    /// completion, compilation or an executable replacement.
    pub procedure_implementation_bodies:
        std::sync::Arc<[crate::command_binding::SourceInstalledProcedureBody]>,
    /// The original source text this module was lowered from. Spans on the
    /// lowered statements index into it, so codegen can recover each command's
    /// surface text for `errorInfo` (`while executing "…"`). Empty when not
    /// supplied (older construction paths / hand-built test modules).
    pub source: tcl_lexer::SourceImage,
    /// Retained native namespace path for executable compilation. Symbolic
    /// analysis names never reconstruct an opaque native namespace.
    pub native_namespace: Option<tcl_core_types::ByteNamespacePath>,
    /// Explicit byte target namespace retained by the original source driver.
    /// This supplies artifact geometry, never a live namespace incarnation.
    pub original_byte_source_namespace: Option<std::sync::Arc<OriginalByteSourceNamespace>>,
    /// Rooted constructed namespace in which [`Self::top_level`] resolves
    /// command names. This is `"::"` for an ordinary source module and the
    /// procedure's defining namespace for a runtime procedure-body compile.
    pub top_level_namespace: String,
    /// Exact analysis namespace identity. Printed names never reconstruct native
    /// component boundaries or select an actual namespace incarnation.
    pub top_level_namespace_context: Option<crate::command_binding::SourceNamespaceKey>,
    /// The dialect this module was lowered for (`"tcl8.6"`, `"f5-irules"`, …),
    /// `None` for an unnamed/permissive compile.
    ///
    /// The dialect is a top-level property of the compile, threaded from the
    /// entry point down to codegen so the backend emits code *for* a release
    /// rather than deferring release-dependent decisions to run time — a
    /// numeric literal is a case in point: `0755` is 493 under 8.6 and 755
    /// under 9.0, and which one it is settled here.
    pub dialect: Option<String>,
    /// Whether lowering deliberately preserved ordinary command dispatch for
    /// every invocation (trace/invalidation recovery compilation).
    pub plain_command_dispatch: bool,
    /// How the emulated release evaluates the commands of [`Self::top_level`]
    /// itself; the bodies those commands evaluate it compiles all the same.
    pub top_level_evaluation: CommandEvaluation,
    /// What [`Self::top_level`] actually is.
    ///
    /// The runtime compiles a `proc` or `TclOO` method body as its own module
    /// whose top level *is* that body, so the name alone cannot tell the two
    /// apart — both are `::top`. Anything that depends on the body having a
    /// local variable table has to ask this instead (#2207: a `catch` is only
    /// inlined where its result variable can have a slot).
    pub top_level_kind: TopLevelKind,
    /// Top-level script (code outside any procedure).
    pub top_level: Script,
    /// Named procedures.
    pub procedures: ProcedureInventory,
    /// Shared body-ID owner for declarations and analysis-only units.
    pub procedure_ids: ProcedureIdIssuer,
    /// Issued definition-script source coordinates, independent of runtime frames.
    pub original_definition_scripts:
        std::collections::BTreeMap<ProcedureId, std::sync::Arc<OriginalDefinitionScriptSource>>,
    /// Sealed actual event-declaration bodies. Internal procedure labels do
    /// not create an event frame or select a hosted execution context.
    pub irules_event_bodies:
        std::collections::HashMap<ProcedureId, std::sync::Arc<SourceIrulesEventBody>>,
    /// Every method definition under its independently issued body ID.
    pub methods: MethodInventory,
    /// Analysis bodies of lambdas, namespace scripts and retained declarations.
    /// Their IDs identify bodies; these rows do not publish callable commands.
    pub body_units: ProcedureInventory,
    /// The subset of [`Self::body_units`] that are **lambda** frames — an
    /// `apply` literal's body, whose parameter list binds real locals exactly
    /// as a `proc`'s does.
    ///
    /// A `namespace eval` body unit is *not* one: its variables are the
    /// namespace's, shared with every other body that opens the same
    /// namespace, so a name it reads without writing is routinely defined
    /// elsewhere.  The read-before-set family (`W210`) therefore runs over
    /// this subset only — a lambda body is a closed frame and an unwritten
    /// read in it is a genuine error.
    pub lambda_body_units: std::collections::BTreeSet<ProcedureId>,
    /// Procedure names that were defined more than once.
    pub redefined_procedures: std::collections::HashSet<String>,
    /// Conservative replacement groups of actual retained body IDs.
    /// Grouping is source metadata; no receiver or method token is selected.
    pub redefined_methods: std::collections::HashMap<ProcedureId, Vec<ProcedureId>>,
    /// Class qnames with a member definition the lowering could not
    /// statically model — a dynamic member name (`method $n …`) or a
    /// dynamic member body (`method m {} $body`) inside an `oo::class` /
    /// `oo::define` block. Any method of such a class may have been
    /// (re)defined with an unknown body, so per-class analysis
    /// (purity, the method-dispatch propagation barrier) must abstain
    /// for the whole class.
    pub oo_unanalysed_classes: std::collections::HashSet<String>,
    /// Whole-module OO-definition evidence flags — see
    /// [`OoDefinitionEvidence`].
    pub oo_evidence: OoDefinitionEvidence,
    /// Original selected class-relation members, retaining both full source
    /// vectors. The method barrier joins current class allocations and their
    /// retained inheritance candidates; missing targets widen its MAY graph.
    pub class_relations: Vec<ClassRelation>,
    /// Literal selected import patterns as `(context_namespace, pattern)`
    /// dependency pairs in source order. These supply no resolved binding;
    /// the source command graph independently owns import resolution.
    pub namespace_imports: Vec<(String, String)>,
    /// Literal selected export patterns as `(context_namespace, pattern)`
    /// dependency pairs. Export filtering and binding selection retain their
    /// independent typed source and Native owners.
    pub namespace_exports: Vec<(String, String)>,
    /// Literal command names that have an execution trace
    /// registered (`trace add execution NAME enter|leave HANDLER`).
    /// GVN consults this set to gate purity (a traced call is
    /// never pure because the trace handler composes side effects
    /// in).
    pub traced_commands: std::collections::BTreeSet<String>,
    /// `true` when a `trace add execution` was seen with a
    /// non-literal command target (`trace add execution $cmd ...`).
    /// Forces GVN / partial-redundancy / loop-invariant passes to
    /// treat *every* call as potentially traced.
    pub has_dynamic_trace: bool,
    /// Literal variable names targeted by an active *variable* trace
    /// (`trace add variable NAME ops HANDLER`, or the deprecated
    /// `trace variable`/`vdelete` spellings) anywhere in the module —
    /// top level or any procedure body, regardless of whether the
    /// installing call is reachable before or after a given use.  A
    /// read of a traced variable can run arbitrary handler code, and a
    /// write handler can rewrite the value being stored, so the
    /// propagation optimiser (`O102` load-forwarding) and dead-store
    /// elimination must never treat a use of one of these names as
    /// equivalent to its last literal assignment. Whole-module and
    /// position-independent by design — mirrors [`Self::traced_commands`]
    /// but for `ArgRole::VarWrite`-role variable-trace targets
    /// (`Traits::ESTABLISHES_VARIABLE_TRACE`) rather than execution-trace
    /// command names.
    pub traced_variables: std::collections::BTreeSet<String>,
    /// `true` when a variable-trace install/remove call was seen with a
    /// non-literal variable-name target (`trace add variable $name ...`).
    /// Forces the propagation optimiser to treat *every* variable as
    /// potentially traced.
    pub has_dynamic_variable_trace: bool,
    /// The variables the scripts the module stores as callbacks write — see
    /// [`DeferredWrites`]. Populated after lowering, beside
    /// [`Self::traced_variables`].
    pub deferred_writes: DeferredWrites,
    /// The definitions of pack commands this module's calls were inlined
    /// from, and where [`Self::source`] holds their text — see
    /// [`ReferenceBodies`].
    pub reference_bodies: ReferenceBodies,
    /// The commands the document declares (`# tcl-lsp: stub`) as plain
    /// calls, which the catalogue does not hold — see
    /// [`DeclaredFrameEffects`]. Empty for a module lowered without the
    /// document's declarations.
    pub declared_frame_effects: DeclaredFrameEffects,
}

/// The commands a document declares whose declarations state their frame
/// effect and nothing a reader of the catalogue alone would miss
/// ([`tcl_registry::model::DeclaredCommand::plain_call_frame_effect`]), by
/// unambiguous descriptor metadata coordinate, with the effect each states: `None` for one
/// that crosses no frame (`-frame own`, `-frame none`), `argparse`'s
/// caller-frame effect for `-frame caller`.
///
/// The module's command table holds them as it holds a catalogue command
/// (`ModuleCommandBindings`), so a call to one is a call to a command the
/// module can name, and it brings the stated effect where a catalogue
/// command's call brings the registry's ([`crate::dynamic_names`]). A
/// declaration that states no frame effect leaves its command out, and a call
/// to it stays a call to code the module cannot see; a name the catalogue
/// holds keeps the catalogue's answer, which is what the flow graph reads.
pub type DeclaredFrameEffects =
    std::collections::BTreeMap<String, Option<tcl_registry::FrameEffectSpec>>;

/// The reference bodies a module's calls were inlined from.
///
/// A command a pack declares `TclBody`-backed has its definition in the pack,
/// not in the module that calls it. The inliner lowers that definition on its
/// own and shifts its spans past the end of [`Module::source`], then appends
/// the definition's text there, so every span in the module — the module's own
/// and the borrowed — indexes one text and codegen slices each as it always
/// did. The appended text is no part of the module: [`Module::own_source`]
/// is what an artefact carries as its source, and what a plain recompile
/// reads.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReferenceBodies {
    /// The byte offset in [`Module::source`] at which the appended
    /// definitions start; `None` when nothing was appended.
    pub appendix_start: Option<usize>,
    /// The definitions appended, by what identifies each to the runtime.
    pub imports: Vec<ReferenceImport>,
    /// Prefixes of the names a splice gave the locals of a body it inlined,
    /// none of which the module's own source spells: codegen gives each such
    /// name an anonymous slot, so the caller's frame holds no variable of that
    /// name for anything to list, reach or collide with.
    pub hidden_local_prefixes: Vec<String>,
    /// The calls a live compile spliced, each with what codegen needs to emit
    /// the call the splice falls back to.
    pub splices: Vec<SpliceSite>,
}

/// A call whose body a live compile spliced in its place. Codegen brackets the
/// body's instructions as a range and follows it with the call itself, made
/// from the words the splice evaluated, which the runtime runs instead when
/// anything inside the range leaves abnormally.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpliceSite {
    /// The call's own extent in the caller.
    pub call_span: Span,
    /// The extent of the spliced body's statements in the module's source.
    pub body_region: Span,
    /// The hidden locals holding the call's evaluated words, the command word
    /// first.
    pub words: Vec<String>,
    /// Whether the call's value is left on the stack after it.
    pub resumes_with_value: bool,
}

/// One pack command's definition, as copied into a module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReferenceImport {
    /// The rooted name the definition creates (`::vendor::double`).
    pub name: String,
    /// The definition's raw parameter list.
    pub parameters: String,
    /// The definition's body text, as the live procedure would hold it.
    pub body: String,
    /// The kind of backing the definition was declared with.
    pub backing: tcl_runtime_api::BackingKind,
    /// The pack facts that declared the backing the definition came from.
    pub facts: tcl_runtime_api::PackFactStamp,
}

impl Module {
    /// Selected original procedure formal topology for one Module-owned body.
    /// This is source argument structure only and grants no activation/frame.
    pub(crate) fn original_source_formal_topology(
        &self,
        body_id: ProcedureId,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<crate::signature_scan::formal_parameters::SourceProcedureFormalTopology> {
        self.retained_source_bindings
            .as_ref()?
            .original_procedure_formals_for_module(self, body_id, registry)
    }

    /// Original procedure syntax under this Module issuer, including retained
    /// declaration bodies. Lambdas and installed-only units are separate purposes.
    #[doc(hidden)]
    pub fn original_source_procedure_bodies(
        &self,
    ) -> impl Iterator<Item = (&ProcedureId, &Procedure)> {
        self.procedures
            .iter()
            .filter(|(id, _)| self.procedure_ids.owns(**id))
            .chain(
                self.original_declaration_body_units
                    .iter()
                    .filter_map(|(id, allocation)| {
                        let procedure = self.body_units.get(id)?;
                        (self.procedure_ids.owns(*id)
                            && procedure.body_kind == ProcedureBodyKind::OriginalDeclaration
                            && procedure.declaration_site.as_ref() == Some(&allocation.site)
                            && allocation.matches_source_declaration_image(
                                &self.source,
                                allocation.site.offset,
                            ))
                        .then_some((id, procedure))
                    }),
            )
            .chain(
                self.original_native_declaration_body_units
                    .iter()
                    .filter_map(|(id, card)| {
                        let procedure = self.body_units.get(id)?;
                        (self.procedure_ids.owns(*id) && card.matches_procedure(self, procedure))
                            .then_some((id, procedure))
                    }),
            )
            .chain(
                self.original_logical_declaration_body_units
                    .iter()
                    .filter_map(|(id, card)| {
                        let procedure = self.body_units.get(id)?;
                        (self.procedure_ids.owns(*id) && card.matches_procedure(self, procedure))
                            .then_some((id, procedure))
                    }),
            )
            .chain(
                self.original_authored_declaration_body_units
                    .iter()
                    .filter_map(|(id, card)| {
                        let procedure = self.body_units.get(id)?;
                        (self.procedure_ids.owns(*id) && card.matches_procedure(self, procedure))
                            .then_some((id, procedure))
                    }),
            )
            .chain(
                self.original_hosted_declaration_body_units
                    .iter()
                    .filter_map(|(id, card)| {
                        let procedure = self.body_units.get(id)?;
                        (self.procedure_ids.owns(*id)
                            && card.matches_procedure(self, procedure, self.resolved_registry()))
                        .then_some((id, procedure))
                    }),
            )
    }

    /// Bodies available for compiling a definition-time runtime entry. Original
    /// Native cards retain source syntax only: the allocating invocation still
    /// performs publication, and direct-call/frame authority is independent.
    #[doc(hidden)]
    pub fn native_source_procedure_bodies_for_compilation(
        &self,
    ) -> impl Iterator<Item = (&ProcedureId, &Procedure)> {
        self.procedures
            .iter()
            .filter(|(id, _)| self.procedure_ids.owns(**id))
            .chain(
                self.original_native_declaration_body_units
                    .iter()
                    .filter_map(|(id, card)| {
                        let procedure = self.body_units.get(id)?;
                        (self.procedure_ids.owns(*id) && card.matches_procedure(self, procedure))
                            .then_some((id, procedure))
                    }),
            )
    }

    /// Select only the exact allocating source instruction before validating
    /// its retained Native body, so unrelated bodies need no deep comparison.
    #[doc(hidden)]
    pub fn native_source_procedure_bodies_for_compilation_at_declaration<'a>(
        &'a self,
        site: &'a crate::command_binding::CommandAllocationSite,
        span: Span,
    ) -> impl Iterator<Item = (&'a ProcedureId, &'a Procedure)> {
        self.procedures
            .at_declaration(site)
            .filter(move |(id, procedure)| self.procedure_ids.owns(**id) && procedure.span == span)
            .chain(
                self.original_native_declaration_body_units
                    .iter()
                    .filter_map(move |(id, card)| {
                        let procedure = self.body_units.get(id)?;
                        (procedure.declaration_site.as_ref() == Some(site)
                            && card.source_declaration_span() == Some(span)
                            && self.procedure_ids.owns(*id)
                            && card.matches_procedure(self, procedure))
                        .then_some((id, procedure))
                    }),
            )
    }

    /// Procedure bodies the source binding owner recovered from an evaluated
    /// script (`namespace eval ::n [list proc writer {} {...}]`): an installed
    /// or original-declaration unit whose declaration is not in this Module's
    /// own source image. Each keeps its own issued ID.
    pub fn recovered_procedure_bodies(&self) -> impl Iterator<Item = (&ProcedureId, &Procedure)> {
        self.installed_procedure_body_units
            .iter()
            .chain(self.original_declaration_body_units.iter())
            .filter_map(|(id, allocation)| {
                let procedure = self.body_units.get(id)?;
                (self.procedure_ids.owns(*id)
                    && procedure.declaration_site.as_ref() == Some(&allocation.site)
                    && !allocation
                        .matches_source_declaration_image(&self.source, allocation.site.offset))
                .then_some((id, procedure))
            })
    }

    /// Every body with an issued summary coordinate: the original source
    /// procedures and the recovered procedure bodies.
    pub fn summary_procedure_bodies(&self) -> impl Iterator<Item = (&ProcedureId, &Procedure)> {
        self.original_source_procedure_bodies()
            .chain(self.recovered_procedure_bodies())
    }

    /// The recovered body a procedure target's exact implementation
    /// allocation selects. An allocation shared by more than one body declines.
    pub(crate) fn recovered_procedure_for_target(
        &self,
        target: &crate::command_binding::SourceCommandTarget,
    ) -> Option<(ProcedureId, &Procedure)> {
        if target.registry_backed || target.kind != crate::command_binding::BindingKind::Proc {
            return None;
        }
        let allocation = target.implementation_allocation.as_ref()?;
        let mut matching = self.recovered_procedure_bodies().filter(|(id, _)| {
            self.installed_procedure_body_units
                .get(id)
                .or_else(|| self.original_declaration_body_units.get(id))
                == Some(allocation)
        });
        let (id, procedure) = matching.next()?;
        matching.next().is_none().then_some((*id, procedure))
    }

    /// Report an original body's source label after its actual issued ID and
    /// retained Module source owner agree. Equal labels retain separate IDs;
    /// this projection selects no callable, binding, namespace or Native entry.
    #[must_use]
    pub fn original_procedure_report_label(
        &self,
        body: ProcedureId,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<&str> {
        self.procedure_ids.owns(body).then_some(())?;
        let retained = self.retained_source_bindings.as_ref()?;
        retained.positioned_for_module(self, registry)?;
        let mut matches = self
            .original_source_procedure_bodies()
            .filter(|(id, _)| **id == body);
        let (_, procedure) = matches.next()?;
        matches.next().is_none().then_some(())?;
        retained
            .matches_original_procedure_summary(body, procedure)
            .then_some(())?;
        if let Some(card) = self.original_authored_declaration_body_units.get(&body) {
            return card.reporting_label(self, procedure);
        }
        Some(&procedure.qualified_name)
    }

    /// Conditional source allocation lookup; no callable publication or Native
    /// entry follows from an original declaration retained only for analysis.
    pub(crate) fn original_source_procedure_for_target(
        &self,
        target: &crate::command_binding::SourceCommandTarget,
    ) -> Option<(ProcedureId, &Procedure)> {
        if target.registry_backed || target.kind != crate::command_binding::BindingKind::Proc {
            return None;
        }
        let allocation = target.implementation_allocation.as_ref()?;
        let mut matching = self
            .original_source_procedure_bodies()
            .filter(|(_, procedure)| {
                procedure.declaration_site.as_ref() == Some(&allocation.site)
                    && allocation
                        .matches_source_declaration_image(&self.source, allocation.site.offset)
                    && target
                        .matches_authored_implementation_image(&self.source, allocation.site.offset)
            });
        let (id, procedure) = matching.next()?;
        self.procedure_ids.owns(*id).then_some(())?;
        matching.next().is_none().then_some((*id, procedure))
    }

    pub(crate) fn source_procedure_for_target(
        &self,
        target: &crate::command_binding::SourceCommandTarget,
    ) -> Option<(ProcedureId, &Procedure)> {
        if target.registry_backed || target.kind != crate::command_binding::BindingKind::Proc {
            return None;
        }
        let allocation = target.implementation_allocation.as_ref()?;
        let (id, procedure) = self.source_procedure_for_allocation(allocation)?;
        target
            .matches_authored_implementation_image(&self.source, procedure.span.start())
            .then_some((id, procedure))
    }

    pub(crate) fn source_procedure_for_allocation(
        &self,
        allocation: &crate::command_binding::CommandAllocation,
    ) -> Option<(ProcedureId, &Procedure)> {
        let mut matching =
            self.procedures
                .at_declaration(&allocation.site)
                .filter(|(_, procedure)| {
                    allocation
                        .matches_source_declaration_image(&self.source, procedure.span.start())
                });
        let (id, procedure) = matching.next()?;
        self.procedure_ids.owns(*id).then_some(())?;
        matching.next().is_none().then_some((*id, procedure))
    }

    /// Issue a fresh body ID and retain this declaration without name-key overwrites.
    pub fn retain_procedure(&mut self, procedure: Procedure) -> ProcedureId {
        let id = self.procedure_ids.allocate();
        self.procedures.insert(id, procedure);
        id
    }

    /// Issue one method-body ID and retain every definition independently.
    /// Structured metadata groups protective may-body facts, never Native tokens.
    pub fn retain_method(&mut self, method: MethodDef) -> ProcedureId {
        let coordinate = method.coordinate();
        let primary = self
            .methods
            .iter()
            .find_map(|(id, existing)| (existing.coordinate() == coordinate).then_some(*id));
        let id = self.procedure_ids.allocate();
        self.methods.insert(id, method);
        if let Some(primary) = primary {
            self.redefined_methods.entry(primary).or_default().push(id);
        }
        id
    }

    /// All retained bodies conservatively associated with this definition.
    /// Repeated labels cannot omit a body or identify its runtime receiver.
    pub fn method_definition_bodies(
        &self,
        id: ProcedureId,
    ) -> impl Iterator<Item = (ProcedureId, &MethodDef)> {
        let primary = self
            .redefined_methods
            .iter()
            .find_map(|(primary, replacements)| {
                (*primary == id || replacements.contains(&id)).then_some(*primary)
            })
            .unwrap_or(id);
        std::iter::once(primary)
            .chain(
                self.redefined_methods
                    .get(&primary)
                    .into_iter()
                    .flatten()
                    .copied(),
            )
            .filter_map(|id| self.methods.get(&id).map(|body| (id, body)))
    }

    /// Retain an independent executable body under the same Module issuer.
    pub fn retain_body_unit(&mut self, procedure: Procedure) -> ProcedureId {
        let id = self.procedure_ids.allocate();
        self.body_units.insert(id, procedure);
        id
    }

    /// One body lowering: a body unit, or an Installed lowering a unit carries.
    #[must_use]
    pub fn body_lowering(&self, id: &ProcedureId) -> Option<&Procedure> {
        self.body_units
            .get(id)
            .or_else(|| self.carried_installed_lowerings.get(id))
    }

    /// Every body lowering in issuance order: each body unit and each
    /// Installed lowering a unit carries. [`Self::body_units`] alone holds
    /// the analysis units.
    pub fn body_lowerings(&self) -> impl Iterator<Item = (&ProcedureId, &Procedure)> {
        // Both stores are ordered by ID; merge them without collecting.
        let mut units = self.body_units.iter().peekable();
        let mut carried = self.carried_installed_lowerings.lowerings().peekable();
        std::iter::from_fn(move || {
            let carried_first = match (units.peek(), carried.peek()) {
                (Some((unit, _)), Some((lowering, _))) => lowering < unit,
                (None, Some(_)) => true,
                _ => false,
            };
            if carried_first {
                carried.next()
            } else {
                units.next()
            }
        })
    }

    /// Checked analytical namespace key. Native component boundaries must
    /// round-trip through the constructed-key owner before advice can use it.
    #[must_use]
    pub fn advisory_top_level_namespace(&self) -> Option<String> {
        if let Some(path) = &self.native_namespace {
            return tcl_syntax::naming::checked_namespace_path_key(path);
        }
        Some(if self.top_level_namespace.is_empty() {
            String::from("::")
        } else {
            self.top_level_namespace.clone()
        })
    }

    /// Retained resolved profile, with presentation-name compatibility only for
    /// manually constructed modules which predate the explicit ingress carrier.
    #[must_use]
    pub fn resolved_profile(&self) -> Option<&'static tcl_dialect::DialectProfile> {
        self.dialect_profile.or_else(|| {
            self.dialect.as_deref().map(|name| {
                crate::environment_ingress::resolve_environment(name).analyser_profile()
            })
        })
    }

    /// Exact ingress registry, including workspace and custom command metadata.
    /// Compatibility modules fall back to their resolved catalogue profile.
    #[must_use]
    pub fn resolved_registry(&self) -> &tcl_registry::CommandRegistry {
        self.registry_snapshot.as_ref().map_or_else(
            || {
                self.resolved_profile().map_or_else(
                    || tcl_registry::model::ingress::static_context_for("tcl").commands(),
                    |profile| {
                        tcl_registry::model::ingress::static_context_for_profile(profile).commands()
                    },
                )
            },
            tcl_registry::RegistrySnapshot::registry,
        )
    }

    /// Actual source variable/word grammar, independent of assistance metadata.
    /// Preserve source coordinates and parser-mode knobs while selecting native
    /// lexical axes; syntax alone supplies no execution or storage proof.
    #[must_use]
    pub fn native_lexer_config(&self) -> tcl_lexer::LexerConfig {
        self.source_entry
            .options()
            .native_lexer_config(self.lexer_config)
    }

    /// Native numeral ingress retained by this module. Actual invocation axes
    /// precede the assistance profile and its legacy display-name adapter.
    #[must_use]
    pub fn number_syntax(&self) -> tcl_dialect::NumberSyntax {
        self.source_entry.invocation_dialect.map_or_else(
            || tcl_dialect::NumberSyntax::of_profile(self.resolved_profile()),
            |dialect| dialect.numbers,
        )
    }

    /// Native word/list value rules, independent of catalogue display names.
    #[must_use]
    pub fn word_values(&self) -> tcl_syntax::word_rules::WordValueRules {
        self.source_entry.invocation_dialect.map_or_else(
            || tcl_syntax::word_rules::WordValueRules::from_config(&self.lexer_config),
            |dialect| dialect.word_values,
        )
    }

    /// Native parameter activation grammar. Unknown execution axes abstain;
    /// assistance list syntax cannot select a procedure's binding protocol.
    #[must_use]
    pub fn parameter_grammar(&self) -> Option<tcl_dialect::ParameterGrammar> {
        if self.source_entry.native_entry.is_some() {
            return self
                .source_entry
                .options()
                .logical_invocation_dialect()?
                .parameter_grammar();
        }
        self.source_entry.invocation_dialect.map_or_else(
            || {
                self.resolved_profile().and_then(|profile| {
                    tcl_native_model::InvocationDialect::of_profile(profile).parameter_grammar()
                })
            },
            tcl_native_model::InvocationDialect::parameter_grammar,
        )
    }

    /// The counted original source before appended reference definitions.
    #[must_use]
    pub fn own_source(&self) -> tcl_lexer::SourceImage {
        let bytes = self
            .reference_bodies
            .appendix_start
            .and_then(|end| self.source.bytes().get(..end))
            .unwrap_or_else(|| self.source.bytes());
        tcl_lexer::SourceImage::from_bytes(bytes, self.source.channel())
    }

    /// Executable roots whose invocation is independent of an enclosing
    /// statement: the module load script, retained procedures and methods,
    /// and retained replacement methods.
    ///
    /// Synthetic [`Self::body_units`] are deliberately absent. An `apply` or
    /// `namespace eval` body executes only through its owning invocation, in
    /// that invocation's program-order position; treating the same body as an
    /// independently callable fixpoint root both invents an execution path and
    /// repeatedly replays nested body suffixes. Whole-module static analyses
    /// that need every body in isolation should use
    /// [`Self::executable_script_roots`] instead.
    #[must_use]
    pub(crate) fn independent_executable_script_roots(&self) -> Vec<(&Script, ExecutionNamespace)> {
        let mut roots = vec![(
            &self.top_level,
            self.top_level_namespace_context.as_ref().map_or_else(
                || {
                    self.advisory_top_level_namespace().map_or(
                        ExecutionNamespace::RuntimeSelected,
                        ExecutionNamespace::exact,
                    )
                },
                |context| match context {
                    crate::command_binding::SourceNamespaceKey::Authored(namespace) => {
                        ExecutionNamespace::exact(namespace)
                    }
                    _ => ExecutionNamespace::SourceContext(context.clone()),
                },
            ),
        )];

        let mut procedures: Vec<_> = self.procedures.iter().collect();
        procedures.sort_by(|a, b| a.0.cmp(b.0));
        roots.extend(procedures.into_iter().map(|(_, procedure)| {
            (
                &procedure.body,
                procedure
                    .body
                    .execution_namespace(ExecutionNamespace::RuntimeSelected),
            )
        }));

        let mut methods: Vec<_> = self.methods.iter().collect();
        methods.sort_by(|a, b| a.0.cmp(b.0));
        roots.extend(methods.into_iter().map(|(_, method)| {
            (
                &method.body,
                method
                    .body
                    .execution_namespace(method.execution_namespace.clone()),
            )
        }));

        roots
    }

    /// Every retained executable script root and the namespace in which its
    /// command heads resolve.
    ///
    /// Whole-module semantic scans use this inventory instead of each keeping
    /// a partial copy of the procedure/method/body-unit map traversal. The
    /// ordering follows the module's body allocation order so monotone state
    /// replays and memo inputs stay deterministic. Executable units retain
    /// their namespace; analysis-only declarations and installations stay in
    /// their separate body inventories.
    #[must_use]
    pub fn executable_script_roots(&self) -> Vec<(&Script, ExecutionNamespace)> {
        let mut roots = self.independent_executable_script_roots();

        let mut body_units: Vec<_> = self.body_units.iter().collect();
        body_units.sort_by(|a, b| a.0.cmp(b.0));
        roots.extend(
            body_units
                .into_iter()
                .filter(|(qname, _)| {
                    !self.installed_procedure_body_units.contains_key(*qname)
                        && !self.original_declaration_body_units.contains_key(*qname)
                        && !self
                            .original_native_declaration_body_units
                            .contains_key(*qname)
                        && !self
                            .original_logical_declaration_body_units
                            .contains_key(*qname)
                        && !self
                            .original_hosted_declaration_body_units
                            .contains_key(*qname)
                        && !self
                            .original_authored_declaration_body_units
                            .contains_key(*qname)
                })
                .map(|(_, unit)| {
                    (
                        &unit.body,
                        unit.body
                            .execution_namespace(ExecutionNamespace::RuntimeSelected),
                    )
                }),
        );

        roots
    }
}

/// Extract the event name from a `::when::` qualified name.
///
/// Handles both `::when::HTTP_REQUEST` and indexed forms like
/// `::when::HTTP_REQUEST#1`.
#[must_use]
pub fn when_event_name(qualified_name: &str) -> &str {
    let bare = qualified_name
        .strip_prefix("::when::")
        .unwrap_or(qualified_name);
    match bare.find('#') {
        Some(idx) => &bare[..idx],
        None => bare,
    }
}

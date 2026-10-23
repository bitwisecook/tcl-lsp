// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original finite source continuation plans owned by the existing kernel.

mod region;
pub(super) use region::SourceConditionalRegionEntry;
pub use region::{
    OriginalSourceConditionalOutcome, OriginalSourceConditionalRegion,
    OriginalSourceConditionalRegionSet,
};

use super::{
    Arc, CommandAllocationSite, ModuleCommandBindings, SourceExecutionContext, SourceNamespaceKey,
    SourceOutcomes,
};
use std::collections::BTreeMap;
use tcl_lexer::SourceImage;

#[derive(Clone, PartialEq, Eq)]
pub(super) struct SourceConditionalNamespaceExit {
    pub(super) delegation:
        Arc<super::original_namespace_body::OriginalConditionalNamespaceDelegation>,
    pub(super) body: super::ExecutedScriptSource,
    pub(super) namespace: SourceNamespaceKey,
    pub(super) entered: crate::var_resolve::VariableExecutionFrame,
    pub(super) parent: crate::var_resolve::VariableExecutionFrame,
    pub(super) parent_variables: Arc<crate::var_resolve::ResolveContext>,
}

#[derive(Clone, PartialEq, Eq)]
struct SourceConditionalContext {
    source_purpose: tcl_lexer::SourceSegmentationPurpose,
    realm: tcl_dialect::model::InvocationRealm,
    namespace: String,
    namespace_key: SourceNamespaceKey,
    config: tcl_lexer::LexerConfig,
    frame: crate::var_resolve::VariableExecutionFrame,
    depth: u32,
    invocation_offset: u32,
}

impl SourceConditionalContext {
    fn borrowed<'a>(
        &'a self,
        registry: &'a tcl_registry::CommandRegistry,
    ) -> SourceExecutionContext<'a> {
        SourceExecutionContext {
            source_purpose: self.source_purpose,
            body_entry: super::SourceBodyEntryPurpose::Conditional,
            realm: self.realm,
            compilation: Default::default(),
            compilation_snapshot: None,
            selected_compilation: None,
            original_variable_compilation: None,
            namespace: &self.namespace,
            namespace_key: Some(&self.namespace_key),
            config: self.config,
            registry,
            depth: self.depth,
            frame: &self.frame,
            invocation_offset: self.invocation_offset,
            variable_read_owner: None,
            original_written_projection: None,
            written_arguments: None,
            written_values: None,
            written_name_values: None,
            written_representations: None,
            written_objects: None,
            written_method_prefixes: None,
            written_variable_reads: None,
            expression_source: None,
        }
    }
}

#[derive(Clone, PartialEq, Eq)]
struct SourceConditionalScriptFrame {
    source: Arc<super::SourceOriginId>,
    image: SourceImage,
    base: u32,
    commands: Arc<[crate::segmenter::SegmentedCommand]>,
    next: usize,
    ordinal_start: usize,
    context: SourceConditionalContext,
    exit: Option<SourceConditionalNamespaceExit>,
}

#[derive(Clone, PartialEq, Eq)]
struct SourceConditionalPlan {
    frames: Arc<[SourceConditionalScriptFrame]>,
}

/// Derived immutable original plans stay separate from actual analysis owners.
/// A public carrier must rejoin the live analysis attachment at issuance.
#[derive(Clone, Default, PartialEq, Eq)]
pub(super) struct SourceConditionalKernel {
    frames: Vec<Option<SourceConditionalScriptFrame>>,
    plans: BTreeMap<CommandAllocationSite, Option<Arc<SourceConditionalPlan>>>,
    pub(super) pending_namespace: Option<SourceConditionalNamespaceExit>,
    pub(super) root_outcomes: Option<Arc<SourceOutcomes>>,
    resume: Option<SourceConditionalScriptFrame>,
}
impl std::fmt::Debug for SourceConditionalKernel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SourceConditionalKernel")
            .field("plan_count", &self.plans.len())
            .field("has_root_outcomes", &self.root_outcomes.is_some())
            .finish_non_exhaustive()
    }
}
impl SourceConditionalKernel {
    pub(super) fn enter_script(
        &mut self,
        image: &SourceImage,
        base: u32,
        commands: &[crate::segmenter::SegmentedCommand],
        state: &ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
    ) {
        let exit = self.pending_namespace.take();
        if let Some(resume) = self.resume.take() {
            let matches = resume.image == *image
                && resume.base == base
                && resume.commands.get(resume.ordinal_start..) == Some(commands)
                && state.current_source_origin.as_ref() == Some(&resume.source)
                && resume.context.frame == *context.frame
                && resume.context.namespace_key == context.namespace_identity()
                && resume.context.config == context.config;
            self.frames.push(matches.then_some(resume));
            return;
        }
        let Some(origin) = state.current_source_origin.as_ref() else {
            self.frames.push(None);
            return;
        };
        let supported = if self.frames.is_empty() {
            base == 0 && context.source_purpose == tcl_lexer::SourceSegmentationPurpose::WholeInput
        } else {
            self.frames.iter().all(Option::is_some)
                && exit.as_ref().is_some_and(|exit| {
                    exit.body.base() == base
                        && exit.body.text == *image
                        && exit.entered == *context.frame
                        && exit.namespace == context.namespace_identity()
                })
        };
        self.frames
            .push(supported.then(|| SourceConditionalScriptFrame {
                source: Arc::clone(origin),
                image: image.clone(),
                base,
                commands: Arc::from(commands),
                next: 0,
                ordinal_start: 0,
                context: SourceConditionalContext {
                    source_purpose: context.source_purpose,
                    realm: context.realm,
                    namespace: context.namespace.to_owned(),
                    namespace_key: context.namespace_identity(),
                    config: context.config,
                    frame: context.frame.clone(),
                    depth: context.depth,
                    invocation_offset: context.invocation_offset,
                },
                exit,
            }));
    }

    pub(super) fn retain_command_plan(
        &mut self,
        index: usize,
        command: &crate::segmenter::SegmentedCommand,
        state: &ModuleCommandBindings,
        context: &SourceExecutionContext<'_>,
        prefix_complete: bool,
    ) {
        let Some(origin) = state.current_source_origin.as_ref() else {
            return;
        };
        let site = CommandAllocationSite {
            source: Arc::clone(origin),
            offset: command.span.start(),
        };
        // Every observation participates, including unavailable/rejected ones.
        // A later positive cannot renew a withdrawn same-site continuation.
        if self.plans.contains_key(&site) {
            self.plans.insert(site, None);
            return;
        }
        self.plans.insert(site.clone(), None);
        let Some(Some(current)) = self.frames.last_mut() else {
            return;
        };
        current.next = current.ordinal_start + index + 1;
        if !prefix_complete
            || context.expression_source.is_some()
            || context.source_purpose != current.context.source_purpose
            || origin != &current.source
            || state.has_opaque_domain()
            || state.unclosed_command_world
            || state.source_step_observed()
            || !state.command_observers.is_quiet()
        {
            return;
        }
        let Some(frames) = self.frames.iter().cloned().collect::<Option<Vec<_>>>() else {
            return;
        };
        self.plans.insert(
            site,
            Some(Arc::new(SourceConditionalPlan {
                frames: Arc::from(frames),
            })),
        );
    }

    pub(super) fn leave_script(&mut self) {
        self.frames.pop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn analysed(source: &str) -> super::super::SourceCommandBindings {
        let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        analysis
            .retained_command_realm()
            .unwrap()
            .source_bindings_ref()
            .clone()
    }
    fn plan<'a>(
        bindings: &'a super::super::SourceCommandBindings,
        source: &str,
        text: &str,
    ) -> Option<&'a SourceConditionalPlan> {
        let offset = u32::try_from(source.find(text)?).ok()?;
        bindings
            .conditional_source
            .plans
            .get(&CommandAllocationSite {
                source: Arc::clone(bindings.source_origin()?),
                offset,
            })?
            .as_deref()
    }

    #[test]
    fn original_finite_plan_keeps_the_real_script_suffix_and_prior_outcome() {
        let source = "source one.tcl; set after ONE; source two.tcl";
        let bindings = analysed(source);
        let retained = plan(&bindings, source, "source one.tcl").unwrap();
        assert_eq!(retained.frames.len(), 1);
        let root = &retained.frames[0];
        assert_eq!(root.base, 0);
        assert_eq!(root.image, tcl_lexer::SourceImage::document(source));
        assert_eq!(root.next, 1);
        assert_eq!(root.commands.len(), 3);
        assert!(root.exit.is_none());
        assert!(plan(&bindings, source, "source two.tcl").is_none());
        assert!(bindings.conditional_source.root_outcomes.is_some());
        assert_eq!(bindings.conditional_source.frames.len(), 0);
    }

    #[test]
    fn namespace_plan_retains_inner_then_outer_suffix_and_actual_restore_owner() {
        let source = "namespace eval app {source one.tcl; set inner ONE}; set outer TWO";
        let bindings = analysed(source);
        let retained = plan(&bindings, source, "source one.tcl").unwrap();
        assert_eq!(retained.frames.len(), 2);
        let outer = &retained.frames[0];
        let inner = &retained.frames[1];
        assert_eq!(outer.next, 1);
        assert_eq!(outer.commands.len(), 2);
        assert_eq!(inner.next, 1);
        assert_eq!(inner.commands.len(), 2);
        let exit = inner.exit.as_ref().unwrap();
        assert_eq!(inner.context.namespace_key, exit.namespace);
        assert_eq!(inner.context.frame, exit.entered);
        assert_eq!(outer.context.frame, exit.parent);
        assert_eq!(bindings.conditional_source.frames.len(), 0);
    }

    #[test]
    fn refused_same_site_observation_withdraws_and_cannot_renew_a_prior_plan() {
        let source = "source one.tcl; set after ONE";
        let bindings = analysed(source);
        let original = plan(&bindings, source, "source one.tcl").unwrap();
        let frame = original.frames[0].clone();
        let state = &bindings.dispatch_points_at(0).next().unwrap().state;
        let registry = bindings
            .source_metadata_owner()
            .source_analysis_input()
            .unwrap()
            .borrowed_context_registry()
            .commands();
        let context = frame.context.borrowed(registry);
        let command = &frame.commands[0];
        for prefix_complete in [false, true] {
            let mut kernel = SourceConditionalKernel {
                frames: vec![Some(frame.clone())],
                ..Default::default()
            };
            kernel.retain_command_plan(0, command, state, &context, true);
            assert!(kernel.plans.values().next().unwrap().is_some());
            if prefix_complete {
                kernel.frames = vec![None];
            }
            kernel.retain_command_plan(0, command, state, &context, prefix_complete);
            assert!(kernel.plans.values().next().unwrap().is_none());
            kernel.frames = vec![Some(frame.clone())];
            kernel.retain_command_plan(0, command, state, &context, true);
            assert!(kernel.plans.values().next().unwrap().is_none());
        }
    }

    #[test]
    fn unsupported_control_descent_and_unknown_prefix_cannot_issue_finite_plans() {
        for source in [
            "while {$condition} {source one.tcl}",
            "if {$condition} {source one.tcl}",
            "proc p {} {source one.tcl}",
            "unknown_writer; source one.tcl",
            "set result [source one.tcl]",
        ] {
            let bindings = analysed(source);
            assert!(
                plan(&bindings, source, "source one.tcl").is_none(),
                "{source}"
            );
        }
    }
}

/// The same genuine parent point, finite original plan and child normal route.
/// Applicability remains conditional; no physical file evaluation is attested.
#[derive(Clone)]
pub struct OriginalSourceConditionalContinuation(Arc<SourceConditionalContinuationPayload>);

struct SourceConditionalContinuationPayload {
    point: super::OriginalSourcePointEntry,
    plan: Arc<SourceConditionalPlan>,
    child_owner: Arc<super::OriginalSourceAnalysisOwner>,
    child_origin: Arc<super::SourceOriginId>,
    child_input: crate::analyser::ResolvedAnalysisInput,
    child_outcomes: Arc<SourceOutcomes>,
    conditional_outcome: Option<OriginalSourceConditionalOutcome>,
    child_graph: Arc<super::source_transition_advice::OriginalSourcePointAdviceGraph>,
    dialect: tcl_registry::InvocationDialect,
    grammar: tcl_registry::source_file::SourceFileGrammar,
}
impl std::fmt::Debug for OriginalSourceConditionalContinuation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OriginalSourceConditionalContinuation")
            .field("point", &self.0.point)
            .field("frame_count", &self.0.plan.frames.len())
            .finish_non_exhaustive()
    }
}
impl PartialEq for OriginalSourceConditionalContinuation {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}
impl Eq for OriginalSourceConditionalContinuation {}
impl SourceConditionalContinuationPayload {
    fn same_request(&self, other: &Self) -> bool {
        self.point == other.point
            && Arc::ptr_eq(&self.plan, &other.plan)
            && Arc::ptr_eq(&self.child_owner, &other.child_owner)
            && self.child_origin == other.child_origin
            && self.child_input == other.child_input
            && self.conditional_outcome == other.conditional_outcome
            && self.dialect == other.dialect
            && self.grammar == other.grammar
    }
}
impl OriginalSourceConditionalContinuation {
    /// Actual compiler source attachment; host publication retirement is separate.
    #[must_use]
    pub fn matches_parent_analysis(&self, parent: &crate::analyser::AnalysisResult) -> bool {
        self.0.point.matches_parent_analysis(parent)
    }

    /// Same child source view and complete input, never a byte-equal substitute.
    #[must_use]
    pub fn matches_child_analysis(&self, child: &crate::analyser::AnalysisResult) -> bool {
        self.0.point.matches_child_analysis(child)
            && child.retained_command_realm().is_some_and(|realm| {
                let bindings = realm.source_bindings_ref();
                bindings.original_analysis_owner().is_some_and(|owner| {
                    Arc::ptr_eq(owner, &self.0.child_owner)
                        && owner.matches(bindings, &self.0.child_input)
                }) && bindings.source_origin() == Some(&self.0.child_origin)
            })
    }
}

impl super::SourceCommandBindings {
    pub(crate) fn conditional_source_continuation(
        &self,
        point: super::OriginalSourcePointEntry,
        parent: &crate::analyser::AnalysisResult,
        child: &crate::analyser::AnalysisResult,
    ) -> Option<OriginalSourceConditionalContinuation> {
        self.conditional_source_continuation_with_outcome(point, parent, child, None)
    }

    pub(crate) fn conditional_source_continuation_with_outcome(
        &self,
        point: super::OriginalSourcePointEntry,
        parent: &crate::analyser::AnalysisResult,
        child: &crate::analyser::AnalysisResult,
        conditional_outcome: Option<&OriginalSourceConditionalOutcome>,
    ) -> Option<OriginalSourceConditionalContinuation> {
        use std::hash::{Hash, Hasher};
        use std::sync::{Mutex, OnceLock, Weak};
        point.matches_parent_analysis(parent).then_some(())?;
        point.matches_child_analysis(child).then_some(())?;
        let plan = self
            .conditional_source
            .plans
            .get(point.allocation_site())?
            .as_ref()?;
        let dialect = point.source_file_dialect()?;
        let grammar = dialect.source_file_grammar()?;
        let child_bindings = child.retained_command_realm()?.source_bindings_ref();
        let owner = child_bindings.original_analysis_owner()?;
        let input = child.resolved_input.as_ref()?;
        owner.matches(child_bindings, input).then_some(())?;
        let outcomes = match conditional_outcome {
            Some(outcome) => outcome.outcomes_for(child)?,
            None => Arc::clone(child_bindings.conditional_source.root_outcomes.as_ref()?),
        };
        // This first finite issuer supports a closed normal child route only.
        // Abrupt/unknown alternatives require their own selected route join.
        if outcomes.normal_completion.is_none()
            || outcomes.normal.is_none()
            || !outcomes.abrupt.is_empty()
        {
            return None;
        }
        let graph = match conditional_outcome {
            Some(outcome) => outcome.graph_for(child)?,
            None => child_bindings
                .original_source_transition_seed()?
                .normal_source_outcome_graph(input.borrowed_context_registry(), child_bindings)?,
        };
        let payload = SourceConditionalContinuationPayload {
            point,
            plan: Arc::clone(plan),
            child_owner: Arc::clone(owner),
            child_origin: Arc::clone(child_bindings.source_origin()?),
            child_input: input.clone(),
            child_outcomes: outcomes,
            conditional_outcome: conditional_outcome.cloned(),
            child_graph: graph,
            dialect,
            grammar,
        };
        type Memo = std::collections::HashMap<u64, Vec<Weak<SourceConditionalContinuationPayload>>>;
        static MEMO: OnceLock<Mutex<Memo>> = OnceLock::new();
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        payload.point.hash(&mut hasher);
        let mut memo = MEMO
            .get_or_init(|| Mutex::new(Memo::new()))
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let bucket = memo.entry(hasher.finish()).or_default();
        bucket.retain(|candidate| candidate.strong_count() != 0);
        if let Some(retained) = bucket
            .iter()
            .filter_map(Weak::upgrade)
            .find(|candidate| candidate.same_request(&payload))
        {
            return Some(OriginalSourceConditionalContinuation(retained));
        }
        let payload = Arc::new(payload);
        bucket.push(Arc::downgrade(&payload));
        Some(OriginalSourceConditionalContinuation(payload))
    }
}

#[cfg(test)]
mod continuation_controls {

    #[test]
    fn finite_continuation_joins_same_parent_child_and_reuses_only_live_actual_owners() {
        let source = "source child.tcl; set after ONE; source next.tcl";
        let parent = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        let load = crate::registry_invocation::source_structure::original_project_source_load_at(
            source, &parent, 0,
        )
        .unwrap();
        let point = load.source_point_entry(&parent).unwrap();
        let child = crate::analyser::Analyser::new()
            .analyse_with_source_point_entry("", "tcl8.6", &point)
            .unwrap();
        let continuation = load.conditional_continuation(&parent, &child).unwrap();
        assert_eq!(
            load.conditional_continuation(&parent, &child),
            Some(continuation.clone())
        );
        assert!(continuation.matches_parent_analysis(&parent.clone()));
        assert!(continuation.matches_child_analysis(&child.clone()));
        let foreign_child = crate::analyser::Analyser::new()
            .analyse_with_source_point_entry("", "tcl8.6", &point)
            .unwrap();
        assert!(!continuation.matches_child_analysis(&foreign_child));
        assert_ne!(
            load.conditional_continuation(&parent, &foreign_child),
            Some(continuation)
        );
        let foreign_parent = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        assert!(
            load.conditional_continuation(&foreign_parent, &child)
                .is_none()
        );
        for unknown in ["unknown_writer", "error STOP", "return VALUE"] {
            let child = crate::analyser::Analyser::new()
                .analyse_with_source_point_entry(unknown, "tcl8.6", &point)
                .unwrap();
            assert!(
                load.conditional_continuation(&parent, &child).is_none(),
                "{unknown}"
            );
        }
    }
}

/// An actual finite kernel plan at its still-current original source point.
#[derive(Clone)]
pub(crate) struct OriginalSourceFiniteCut {
    point: super::OriginalSourcePointEntry,
    plan: Arc<SourceConditionalPlan>,
}
impl std::fmt::Debug for OriginalSourceFiniteCut {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OriginalSourceFiniteCut")
            .field("point", &self.point)
            .finish_non_exhaustive()
    }
}
impl PartialEq for OriginalSourceFiniteCut {
    fn eq(&self, other: &Self) -> bool {
        self.point == other.point && Arc::ptr_eq(&self.plan, &other.plan)
    }
}
impl Eq for OriginalSourceFiniteCut {}
impl OriginalSourceFiniteCut {
    pub(crate) fn point(&self) -> &super::OriginalSourcePointEntry {
        &self.point
    }
    pub(crate) fn matches_parent_analysis(&self, parent: &crate::analyser::AnalysisResult) -> bool {
        self.point.matches_parent_analysis(parent)
            && parent.retained_command_realm().is_some_and(|realm| {
                realm
                    .source_bindings_ref()
                    .source_frontier_parent_view_matches(parent)
                    && realm
                        .source_bindings_ref()
                        .conditional_source
                        .plans
                        .get(self.point.allocation_site())
                        .and_then(Option::as_ref)
                        .is_some_and(|plan| Arc::ptr_eq(plan, &self.plan))
            })
    }
}
impl super::SourceCommandBindings {
    /// A missing result marker cannot convert a retained partial entry to a
    /// whole-file analysis. Full inherited children have no region payload.
    pub(crate) fn has_conditional_source_region(&self) -> bool {
        self.original_inherited_source_entry()
            .is_some_and(|entry| entry.conditional_region().is_some())
    }

    pub(crate) fn source_frontier_parent_view_matches(
        &self,
        parent: &crate::analyser::AnalysisResult,
    ) -> bool {
        let partial = self.has_conditional_source_region();
        match parent.original_source_region.as_ref() {
            Some(region) => partial && region.matches_region_analysis(parent),
            None => !partial,
        }
    }

    pub(crate) fn original_source_finite_cut(
        &self,
        point: super::OriginalSourcePointEntry,
        parent: &crate::analyser::AnalysisResult,
    ) -> Option<OriginalSourceFiniteCut> {
        point.matches_parent_analysis(parent).then_some(())?;
        point.source_file_dialect()?.source_file_grammar()?;
        let plan = self
            .conditional_source
            .plans
            .get(point.allocation_site())?
            .as_ref()?;
        (!plan.frames.is_empty()).then_some(())?;
        let cut = OriginalSourceFiniteCut {
            point,
            plan: Arc::clone(plan),
        };
        cut.matches_parent_analysis(parent).then_some(cut)
    }

    pub(crate) fn source_association_has_conflicting_entry(
        &self,
        parent: &super::CommandAllocationSite,
        argument: usize,
        body: &crate::registry_invocation::OriginalSourceScriptBody,
    ) -> bool {
        let content = body.content_span();
        let bytes = parent.source.source_image().bytes().get(content.as_range());
        self.entered_scripts
            .get(parent)
            .and_then(|arguments| arguments.get(&argument))
            .is_some_and(|observations| {
                observations.iter().any(|observation| {
                    observation.frame.is_some()
                        || &observation.source.origin != &parent.source
                        || observation.source.base() != content.start()
                        || bytes != Some(observation.source.text.bytes())
                })
            })
    }
}

#[cfg(test)]
mod finite_cut_controls {
    #[test]
    fn a_genuine_point_cannot_replace_a_withdrawn_finite_plan() {
        let source = "source child.tcl";
        let parent = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        let load = crate::registry_invocation::source_structure::original_project_source_load_at(
            source, &parent, 0,
        )
        .unwrap();
        let point = load.source_point_entry(&parent).unwrap();
        let bindings = parent
            .retained_command_realm()
            .unwrap()
            .source_bindings_ref();
        let cut = bindings
            .original_source_finite_cut(point.clone(), &parent)
            .unwrap();
        assert!(cut.matches_parent_analysis(&parent));
        let mut withdrawn = bindings.clone();
        assert!(
            withdrawn
                .conditional_source
                .plans
                .remove(point.allocation_site())
                .is_some()
        );
        assert!(point.matches_parent_analysis(&parent));
        assert!(
            withdrawn
                .original_source_finite_cut(point, &parent)
                .is_none()
        );
    }
}

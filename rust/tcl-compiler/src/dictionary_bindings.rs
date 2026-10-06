// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Dictionary scope entry and completion-sensitive physical-cell writeback.

use crate::place::{Place, PlaceKind};
use crate::var_resolve::{
    ContentsPresence, ResolveContext, VariableProofRelocation, resolve_literal_access,
};
use tcl_registry::dictionary_scope::{
    DictionaryMissingKey, DictionaryScopeBindings, DictionaryScopePlan,
};
use tcl_registry::{CommandRegistry, InvocationArguments};

/// One wrapper source instruction in its actual variable activation.
/// Equal byte offsets in derived sources or sibling frames are not equal scopes.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DictionaryScopeId {
    /// Typed source instance and wrapper offset.
    pub site: crate::command_binding::CommandAllocationSite,
    /// Actual selected activation; global frames are distinguished by source site.
    pub activation: Option<String>,
}

impl DictionaryScopeId {
    /// Relocate exact store sites and activation allocations without erasing source identity.
    #[must_use]
    pub fn relocated(&self, relocation: &VariableProofRelocation) -> Self {
        let mut id = self.clone();
        id.site.offset = relocation.source_offset(id.site.offset);
        if let Some(activation) = &mut id.activation
            && let Some(replacement) = relocation.activations.get(activation)
        {
            activation.clone_from(replacement);
        }
        id
    }
}

/// Recover a scope identity from the original wrapper's actual source provenance.
#[must_use]
pub fn scope_id(
    tokens: &crate::ir::CommandTokens,
    state: &ResolveContext,
) -> Option<DictionaryScopeId> {
    let region = tokens.evaluated_body()?;
    let original = &region.scope.as_ref()?.invocation;
    Some(DictionaryScopeId {
        site: crate::command_binding::CommandAllocationSite {
            source: std::sync::Arc::clone(original.source_binding.as_ref()?.source_origin()?),
            offset: region.source.span.start(),
        },
        activation: state.activation.clone(),
    })
}

/// Inputs retained across body execution; addresses are selected afresh at each phase.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DictionaryScopeActivation {
    /// Authored mapping, dialect and completion contract.
    pub plan: DictionaryScopePlan,
    /// Frozen dictionary variable NAME, independently of its current binding.
    pub dictionary_name: String,
    /// Frozen subdictionary path.
    pub path: Vec<String>,
    /// Original key/name mappings; unknown dict-with keysets remain absent.
    pub bindings: Option<Vec<(String, String)>>,
    /// Physical cell read on entering the wrapper.
    pub entry_read: Place,
    /// Physical entry stores, removals and reads, in selected scope state.
    pub entry_effects: DictionaryScopeEffects,
}

/// Physical effects of one reached dictionary phase; uncertain stores are clobbers.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct DictionaryScopeEffects {
    /// Cells read at this phase, independently of original argv evaluation.
    pub reads: Vec<Place>,
    /// Proven successful contents stores.
    pub writes: Vec<Place>,
    /// Cells destroyed without producing a contents value.
    pub destructions: Vec<Place>,
    /// May stores or callback effects, never a strong contents definition.
    pub clobbers: Vec<Place>,
}

impl DictionaryScopeActivation {
    /// Relocate physical input proof while retaining frozen value names.
    #[must_use]
    pub fn relocated(&self, relocation: &VariableProofRelocation) -> Self {
        let mut relocated = self.clone();
        relocated.entry_read = relocation.place(&self.entry_read);
        for places in [
            &mut relocated.entry_effects.reads,
            &mut relocated.entry_effects.writes,
            &mut relocated.entry_effects.destructions,
            &mut relocated.entry_effects.clobbers,
        ] {
            for place in places {
                *place = relocation.place(place);
            }
        }
        relocated
    }
}

/// Selection includes actual dictionary shape and reached entry stores.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DictionaryScopeEntry {
    /// Mapping is active; value uncertainty stays in the physical state.
    Entered(Box<DictionaryScopeActivation>),
    /// Known missing or malformed input prevents body entry.
    Invalid,
    /// Frozen operands or observer effects are not representable.
    Unknown,
}

/// Writeback outcome, independently of the body's completion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DictionaryScopeFinish {
    /// This completion does not reach writeback, or the dictionary was removed.
    Skipped,
    /// Successful completion wrote these selected physical cells.
    Written(Vec<Place>),
    /// Known malformed current dictionary/path prevents successful writeback.
    Invalid,
    /// Observer or address uncertainty requires the actual provider.
    Unknown,
}

type Pairs = Vec<(String, String)>;

fn parse_dictionary(value: &str, dialect: tcl_registry::InvocationDialect) -> Option<Pairs> {
    let items: Vec<String> = match dialect.lexer_grammar.list_parse {
        tcl_dialect::ListParse::Strict => tcl_syntax::list::split_list(value)
            .ok()?
            .into_iter()
            .map(std::borrow::Cow::into_owned)
            .collect(),
        tcl_dialect::ListParse::Lenient => tcl_syntax::list::split_list_jim(value)
            .into_iter()
            .map(std::borrow::Cow::into_owned)
            .collect(),
    };
    if !items.len().is_multiple_of(2) {
        return None;
    }
    let keys: Vec<_> = items.iter().step_by(2).collect();
    Some(
        tcl_syntax::value::canonical_dict_slots(keys)
            .into_iter()
            .map(|(first, last)| (items[first * 2].clone(), items[last * 2 + 1].clone()))
            .collect(),
    )
}

fn selected_dictionary(
    value: &str,
    path: &[String],
    dialect: tcl_registry::InvocationDialect,
) -> Option<Pairs> {
    let mut dictionary = parse_dictionary(value, dialect)?;
    for key in path {
        let value = dictionary
            .iter()
            .find(|(candidate, _)| candidate == key)?
            .1
            .clone();
        dictionary = parse_dictionary(&value, dialect)?;
    }
    Some(dictionary)
}

/// Enter an already selected dictionary wrapper on its body-entry continuation.
/// Literal dictionary bytes establish the original keys; unknown values never
/// manufacture a dict-with keyset. Stores/unsets resolve in native order and
/// unknown observers withdraw later binding evidence.
pub fn enter_dictionary_scope(
    state: &mut ResolveContext,
    plan: &DictionaryScopePlan,
    arguments: InvocationArguments<'_>,
    registry: &CommandRegistry,
    source: u32,
) -> DictionaryScopeEntry {
    let Some(name) = arguments.literal_at(plan.dictionary_argument) else {
        return DictionaryScopeEntry::Unknown;
    };
    let path = match &plan.bindings {
        DictionaryScopeBindings::Pairs(_) => Vec::new(),
        DictionaryScopeBindings::AllKeys(indices) => {
            let Some(path) = indices
                .iter()
                .map(|&index| arguments.literal_at(index).map(str::to_owned))
                .collect::<Option<Vec<_>>>()
            else {
                return DictionaryScopeEntry::Unknown;
            };
            path
        }
    };
    let entry_read = resolve_literal_access(
        name,
        state,
        false,
        registry,
        tcl_registry::TraceOperation::Read,
    );
    if entry_read.observed || entry_read.kind == PlaceKind::Unknown {
        state.widen();
        return DictionaryScopeEntry::Unknown;
    }
    if state.read_completion(&entry_read) == crate::var_resolve::VariableReadCompletion::Error {
        return DictionaryScopeEntry::Invalid;
    }
    let dictionary = match state.literal_contents_at(&entry_read, registry) {
        Some(value) => match selected_dictionary(value, &path, plan.dialect) {
            Some(dictionary) => Some(dictionary),
            None => return DictionaryScopeEntry::Invalid,
        },
        None => None,
    };
    let bindings = match &plan.bindings {
        DictionaryScopeBindings::Pairs(indices) => {
            let Some(bindings) = indices
                .iter()
                .map(|&(key, name)| {
                    Some((
                        arguments.literal_at(key)?.to_owned(),
                        arguments.literal_at(name)?.to_owned(),
                    ))
                })
                .collect::<Option<Vec<_>>>()
            else {
                return DictionaryScopeEntry::Unknown;
            };
            Some(bindings)
        }
        DictionaryScopeBindings::AllKeys(_) => dictionary.as_ref().map(|pairs| {
            pairs
                .iter()
                .map(|(key, _)| (key.clone(), key.clone()))
                .collect()
        }),
    };
    let mut activation = DictionaryScopeActivation {
        plan: plan.clone(),
        dictionary_name: name.to_owned(),
        path,
        bindings,
        entry_effects: DictionaryScopeEffects {
            reads: vec![entry_read.clone()],
            ..DictionaryScopeEffects::default()
        },
        entry_read,
    };
    let Some(effects) =
        apply_entry_bindings(state, &activation, dictionary.as_ref(), registry, source)
    else {
        return DictionaryScopeEntry::Invalid;
    };
    activation.entry_effects = effects;
    DictionaryScopeEntry::Entered(Box::new(activation))
}

fn apply_entry_bindings(
    state: &mut ResolveContext,
    activation: &DictionaryScopeActivation,
    dictionary: Option<&Pairs>,
    registry: &CommandRegistry,
    source: u32,
) -> Option<DictionaryScopeEffects> {
    let mut effects = activation.entry_effects.clone();
    let Some(bindings) = &activation.bindings else {
        state.record_contents_write(&crate::place::unknown_top(), source, true);
        if state.dynamic_traces || !state.traced.is_empty() {
            state.widen();
        }
        effects.clobbers.push(crate::place::unknown_top());
        return Some(effects);
    };
    for (key, name) in bindings {
        let value = dictionary.and_then(|pairs| {
            pairs
                .iter()
                .find(|(candidate, _)| candidate == key)
                .map(|(_, value)| value)
        });
        let missing_unset = dictionary.is_some()
            && value.is_none()
            && activation.plan.missing_key == DictionaryMissingKey::Unset;
        let target = resolve_literal_access(
            name,
            state,
            false,
            registry,
            if missing_unset {
                tcl_registry::TraceOperation::Unset
            } else {
                tcl_registry::TraceOperation::Write
            },
        );
        if target.observed || target.kind == PlaceKind::Unknown {
            effects.clobbers.push(crate::place::unknown_top());
            state.widen();
            continue;
        }
        if value.is_some() && state.store_would_error(&target) {
            return None;
        }
        if let Some(dictionary) = dictionary {
            if let Some((_, value)) = dictionary.iter().find(|(candidate, _)| candidate == key) {
                effects.writes.push(target.clone());
                state.record_contents_write(&target, source, false);
                state.define_literal(name, value, registry);
            } else if activation.plan.missing_key == DictionaryMissingKey::Unset {
                effects.destructions.push(resolve_literal_access(
                    name,
                    state,
                    false,
                    registry,
                    tcl_registry::TraceOperation::Unset,
                ));
                crate::variable_bindings::destroy_literal_binding(state, name, source, registry);
            }
        } else if state.store_would_error(&target) {
            if activation.plan.missing_key == DictionaryMissingKey::Unset {
                effects.destructions.push(target);
                crate::variable_bindings::destroy_literal_binding(state, name, source, registry);
            }
        } else {
            effects.clobbers.push(target.clone());
            let mut defined = state.clone();
            defined.define_unknown_contents(name, registry);
            defined.record_contents_write(&target, source, false);
            if activation.plan.missing_key == DictionaryMissingKey::Unset {
                crate::variable_bindings::destroy_literal_binding(state, name, source, registry);
            }
            state.join(&defined);
        }
    }
    Some(effects)
}

fn update_pair(dictionary: &mut Pairs, key: &str, value: Option<String>) {
    if let Some(index) = dictionary
        .iter()
        .position(|(candidate, _)| candidate == key)
    {
        if let Some(value) = value {
            dictionary[index].1 = value;
        } else {
            dictionary.remove(index);
        }
    } else if let Some(value) = value {
        dictionary.push((key.to_owned(), value));
    }
}

fn write_path(
    value: &str,
    path: &[String],
    updates: &[(String, Option<String>)],
    dialect: tcl_registry::InvocationDialect,
) -> Option<String> {
    let mut dictionary = parse_dictionary(value, dialect)?;
    if let Some((key, rest)) = path.split_first() {
        let index = dictionary
            .iter()
            .position(|(candidate, _)| candidate == key)?;
        dictionary[index].1 = write_path(&dictionary[index].1, rest, updates, dialect)?;
    } else {
        for (key, value) in updates {
            update_pair(&mut dictionary, key, value.clone());
        }
    }
    Some(tcl_syntax::list::join_list(
        dictionary.into_iter().flat_map(|(key, value)| [key, value]),
    ))
}

/// Run a reached epilogue against live bindings and live dictionary contents.
/// The original keyset is retained, but the dictionary address, mapped cell
/// addresses and path contents are selected after the body. A removed root
/// skips writeback; unknown stored values retain an exact root write footprint.
pub fn finish_dictionary_scope(
    state: &mut ResolveContext,
    activation: &DictionaryScopeActivation,
    route: tcl_registry::completion_route::InvocationCompletionRoute,
    registry: &CommandRegistry,
    source: u32,
) -> DictionaryScopeFinish {
    if !activation.plan.writeback_routes(route).captured {
        return DictionaryScopeFinish::Skipped;
    }
    let read = resolve_literal_access(
        &activation.dictionary_name,
        state,
        false,
        registry,
        tcl_registry::TraceOperation::Read,
    );
    if read.observed || read.kind == PlaceKind::Unknown {
        state.widen();
        return DictionaryScopeFinish::Unknown;
    }
    if state.contents_presence(&read) == ContentsPresence::Undefined {
        return DictionaryScopeFinish::Skipped;
    }
    let current = state
        .literal_contents_at(&read, registry)
        .map(str::to_owned);
    let updates = read_mapped_contents(state, activation, registry);
    if state.dynamic_bindings {
        return DictionaryScopeFinish::Unknown;
    }
    let value = match (current.as_deref(), updates) {
        (Some(current), Some(updates)) => {
            match write_path(current, &activation.path, &updates, activation.plan.dialect) {
                Some(value) => Some(value),
                None => return DictionaryScopeFinish::Invalid,
            }
        }
        _ => None,
    };
    let target = resolve_literal_access(
        &activation.dictionary_name,
        state,
        false,
        registry,
        tcl_registry::TraceOperation::Write,
    );
    if target.observed || target.kind == PlaceKind::Unknown {
        state.widen();
        return DictionaryScopeFinish::Unknown;
    }
    if let Some(value) = value {
        state.record_contents_write(&target, source, false);
        state.define_literal(&activation.dictionary_name, &value, registry);
    } else {
        state.define_unknown_contents(&activation.dictionary_name, registry);
        state.record_contents_write(&target, source, false);
    }
    DictionaryScopeFinish::Written(vec![target])
}

fn read_mapped_contents(
    state: &mut ResolveContext,
    activation: &DictionaryScopeActivation,
    registry: &CommandRegistry,
) -> Option<Vec<(String, Option<String>)>> {
    let mut updates = Vec::new();
    for (key, name) in activation.bindings.as_ref()? {
        let read = resolve_literal_access(
            name,
            state,
            false,
            registry,
            tcl_registry::TraceOperation::Read,
        );
        if read.observed || read.kind == PlaceKind::Unknown {
            state.widen();
            return None;
        }
        let value = match state.contents_presence(&read) {
            ContentsPresence::Defined => {
                Some(state.literal_contents_at(&read, registry)?.to_owned())
            }
            ContentsPresence::Undefined => None,
            ContentsPresence::Unknown | ContentsPresence::DefinedOrUndefined => return None,
        };
        updates.push((key.clone(), value));
    }
    Some(updates)
}

/// Apply a typed dictionary phase without reevaluating wrapper argv or body.
/// The activation site is the original wrapper source, so its keyset survives
/// body mutations and is intersected on physical-context joins.
pub fn transfer_scope_marker(
    state: &mut ResolveContext,
    statement: &crate::ir::Statement,
    registry: &CommandRegistry,
) -> bool {
    let Some(tokens) = statement.tokens() else {
        return false;
    };
    let Some(marker) = tokens.synthetic else {
        return false;
    };
    if !matches!(
        marker,
        crate::ir::SyntheticMarker::DictionaryScopeEntry
            | crate::ir::SyntheticMarker::DictionaryScopeWriteback(_)
    ) {
        return false;
    }
    let Some(region) = tokens
        .evaluated_body()
        .and_then(|region| region.scope.as_deref())
    else {
        state.widen();
        return true;
    };
    let source = tokens.evaluated_body().unwrap().source.span.start();
    let Some(id) = scope_id(tokens, state) else {
        state.record_contents_write(&crate::place::unknown_top(), source, true);
        return true;
    };
    match marker {
        crate::ir::SyntheticMarker::DictionaryScopeEntry => {
            let words: Vec<_> = region
                .arguments
                .iter()
                .map(|value| {
                    value.as_deref().map_or(
                        tcl_registry::InvocationWord::Dynamic,
                        tcl_registry::InvocationWord::Literal,
                    )
                })
                .collect();
            let arguments =
                InvocationArguments::structured(&words).with_dialect(region.plan.dialect);
            match enter_dictionary_scope(state, &region.plan, arguments, registry, source) {
                DictionaryScopeEntry::Entered(activation) => {
                    state
                        .dictionary_scopes
                        .insert(id.clone(), std::sync::Arc::new(*activation));
                }
                DictionaryScopeEntry::Invalid | DictionaryScopeEntry::Unknown => {
                    state.dictionary_scopes.remove(&id);
                    state.record_contents_write(&crate::place::unknown_top(), source, true);
                }
            }
        }
        crate::ir::SyntheticMarker::DictionaryScopeWriteback(route) => {
            if let Some(activation) = state.dictionary_scopes.remove(&id) {
                finish_dictionary_scope(state, &activation, route, registry, source);
            } else {
                state.record_contents_write(&crate::place::unknown_top(), source, true);
            }
        }
        _ => {}
    }
    true
}

/// Project a typed dictionary phase through the same state transition owner.
/// Non-scope statements return no projection; unresolved scope input preserves
/// a may clobber rather than claiming native dispatch or a contents definition.
#[must_use]
pub fn scope_marker_effects(
    statement: &crate::ir::Statement,
    state: &ResolveContext,
    registry: &CommandRegistry,
) -> Option<DictionaryScopeEffects> {
    let tokens = statement.tokens()?;
    let marker = tokens.synthetic?;
    if !matches!(
        marker,
        crate::ir::SyntheticMarker::DictionaryScopeEntry
            | crate::ir::SyntheticMarker::DictionaryScopeWriteback(_)
    ) {
        return None;
    }
    let region = tokens.evaluated_body()?;
    let scope = region.scope.as_deref()?;
    let source = region.source.span.start();
    let mut state = state.clone();
    if marker == crate::ir::SyntheticMarker::DictionaryScopeEntry {
        let words: Vec<_> = scope
            .arguments
            .iter()
            .map(|value| {
                value.as_deref().map_or(
                    tcl_registry::InvocationWord::Dynamic,
                    tcl_registry::InvocationWord::Literal,
                )
            })
            .collect();
        return Some(
            match enter_dictionary_scope(
                &mut state,
                &scope.plan,
                InvocationArguments::structured(&words).with_dialect(scope.plan.dialect),
                registry,
                source,
            ) {
                DictionaryScopeEntry::Entered(activation) => activation.entry_effects,
                DictionaryScopeEntry::Invalid => DictionaryScopeEffects::default(),
                DictionaryScopeEntry::Unknown => DictionaryScopeEffects {
                    clobbers: vec![crate::place::unknown_top()],
                    ..DictionaryScopeEffects::default()
                },
            },
        );
    }
    let Some(activation) = scope_id(tokens, &state)
        .and_then(|id| state.dictionary_scopes.get(&id))
        .cloned()
    else {
        return Some(DictionaryScopeEffects {
            clobbers: vec![crate::place::unknown_top()],
            ..DictionaryScopeEffects::default()
        });
    };
    let crate::ir::SyntheticMarker::DictionaryScopeWriteback(route) = marker else {
        return None;
    };
    if !activation.plan.writeback_routes(route).captured {
        return Some(DictionaryScopeEffects::default());
    }
    Some(writeback_effects(
        &mut state,
        &activation,
        route,
        registry,
        source,
    ))
}

fn writeback_effects(
    state: &mut ResolveContext,
    activation: &DictionaryScopeActivation,
    route: tcl_registry::completion_route::InvocationCompletionRoute,
    registry: &CommandRegistry,
    source: u32,
) -> DictionaryScopeEffects {
    let mut effects = DictionaryScopeEffects::default();
    effects.reads.push(resolve_literal_access(
        &activation.dictionary_name,
        state,
        false,
        registry,
        tcl_registry::TraceOperation::Read,
    ));
    if state.contents_presence(effects.reads.first().unwrap()) != ContentsPresence::Undefined
        && let Some(bindings) = &activation.bindings
    {
        for (_, name) in bindings {
            effects.reads.push(resolve_literal_access(
                name,
                state,
                false,
                registry,
                tcl_registry::TraceOperation::Read,
            ));
        }
    }
    match finish_dictionary_scope(state, activation, route, registry, source) {
        DictionaryScopeFinish::Written(writes) => effects.writes = writes,
        DictionaryScopeFinish::Skipped | DictionaryScopeFinish::Invalid => {}
        DictionaryScopeFinish::Unknown => effects.clobbers.push(crate::place::unknown_top()),
    }
    effects
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_registry::completion::CompletionCode;
    use tcl_registry::completion_route::InvocationCompletionRoute;
    use tcl_registry::dictionary_scope::{DictionaryScopeSelection, DictionaryScopeSpec};
    fn enter(
        state: &mut ResolveContext,
        spec: DictionaryScopeSpec,
        words: &[&str],
        registry: &CommandRegistry,
    ) -> DictionaryScopeActivation {
        let dialect = tcl_registry::InvocationDialect::of_profile(registry.profile().unwrap());
        state.invocation_dialect = Some(dialect);
        let arguments = InvocationArguments::literals(words).with_dialect(dialect);
        let DictionaryScopeSelection::Selected(plan) = spec.select(arguments, 0) else {
            panic!("known scope grammar");
        };
        let DictionaryScopeEntry::Entered(activation) =
            enter_dictionary_scope(state, &plan, arguments, registry, 10)
        else {
            panic!("defined scope input");
        };
        *activation
    }
    #[test]
    fn update_entry_and_live_writeback_share_physical_binding_owner() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        let mut state = ResolveContext::for_function("::p");
        state.define_literal("d", "key ORIGINAL", registry);
        state.define_literal("missing", "OLD", registry);
        let activation = enter(
            &mut state,
            DictionaryScopeSpec::Update,
            &["d", "key", "v", "absent", "missing", "body"],
            registry,
        );
        assert_eq!(state.literal_value("v", registry), Some("ORIGINAL"));
        let missing = resolve_literal_access(
            "missing",
            &state,
            false,
            registry,
            tcl_registry::TraceOperation::Read,
        );
        assert_eq!(
            state.contents_presence(&missing),
            ContentsPresence::Undefined
        );
        state.define_literal("d", "extra KEPT key CHANGED", registry);
        state.define_literal("other", "NEW", registry);
        crate::variable_bindings::destroy_literal_binding(&mut state, "v", 15, registry);
        let other = resolve_literal_access(
            "other",
            &state,
            false,
            registry,
            tcl_registry::TraceOperation::Write,
        );
        state.alias_bindings.insert("v", other);
        assert!(matches!(
            finish_dictionary_scope(
                &mut state,
                &activation,
                InvocationCompletionRoute::Tcl(CompletionCode::Ok),
                registry,
                20
            ),
            DictionaryScopeFinish::Written(_)
        ));
        assert_eq!(
            state.literal_value("d", registry),
            Some("extra KEPT key NEW")
        );
    }
    #[test]
    fn with_keeps_original_keys_and_skips_removed_dictionary() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        let mut state = ResolveContext::for_function("::p");
        state.define_literal("d", "x OLD", registry);
        let activation = enter(
            &mut state,
            DictionaryScopeSpec::With,
            &["d", "body"],
            registry,
        );
        state.define_literal("x", "NEW", registry);
        state.define_literal("unmapped", "UNRELATED", registry);
        assert!(matches!(
            finish_dictionary_scope(
                &mut state,
                &activation,
                InvocationCompletionRoute::Tcl(CompletionCode::Error),
                registry,
                20
            ),
            DictionaryScopeFinish::Written(_)
        ));
        assert_eq!(state.literal_value("d", registry), Some("x NEW"));
        crate::variable_bindings::destroy_literal_binding(&mut state, "d", 30, registry);
        assert_eq!(
            finish_dictionary_scope(
                &mut state,
                &activation,
                InvocationCompletionRoute::Tcl(CompletionCode::Ok),
                registry,
                40
            ),
            DictionaryScopeFinish::Skipped
        );
    }
    #[test]
    fn jim_retains_missing_update_keys_and_skips_abrupt_with_writeback() {
        let registry = tcl_registry::model::ingress::static_context_for("jim").commands();
        let mut state = ResolveContext::for_function("::p");
        state.define_literal("d", "", registry);
        state.define_literal("v", "OLD", registry);
        let activation = enter(
            &mut state,
            DictionaryScopeSpec::Update,
            &["d", "absent", "v", "body"],
            registry,
        );
        assert_eq!(state.literal_value("v", registry), Some("OLD"));
        assert!(matches!(
            finish_dictionary_scope(
                &mut state,
                &activation,
                InvocationCompletionRoute::Tcl(CompletionCode::Error),
                registry,
                20,
            ),
            DictionaryScopeFinish::Written(_)
        ));
        assert_eq!(state.literal_value("d", registry), Some("absent OLD"));
        state.define_literal("d", "x OLD", registry);
        let activation = enter(
            &mut state,
            DictionaryScopeSpec::With,
            &["d", "body"],
            registry,
        );
        state.define_literal("x", "NEW", registry);
        let route = InvocationCompletionRoute::Tcl(CompletionCode::Error);
        assert_eq!(
            finish_dictionary_scope(&mut state, &activation, route, registry, 30,),
            DictionaryScopeFinish::Skipped
        );
        assert_eq!(state.literal_value("d", registry), Some("x OLD"));
    }
    #[test]
    fn scope_keysets_join_and_relocate_without_losing_physical_dependencies() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        let mut state = ResolveContext::for_function("::p");
        state.define_literal("d", "x OLD", registry);
        let activation = enter(
            &mut state,
            DictionaryScopeSpec::With,
            &["d", "body"],
            registry,
        );
        let id = DictionaryScopeId {
            site: crate::command_binding::CommandAllocationSite {
                source: std::sync::Arc::new(crate::command_binding::SourceOriginId::authored(
                    &"dict-body".into(),
                )),
                offset: 10,
            },
            activation: state.activation.clone(),
        };
        state
            .dictionary_scopes
            .insert(id.clone(), std::sync::Arc::new(activation));
        let mut relocation = VariableProofRelocation::default();
        relocation.source_offsets.insert(10, 110);
        let relocated = state.relocated(&relocation);
        assert!(
            relocated
                .dictionary_scopes
                .contains_key(&id.relocated(&relocation))
        );
        assert_eq!(relocated.relocated(&relocation.inverse().unwrap()), state);
        let mut other = state.clone();
        other.dictionary_scopes.clear();
        state.join(&other);
        assert!(state.dictionary_scopes.is_empty());
    }

    #[test]
    fn entry_array_conflict_preserves_only_preceding_mapping_stores() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let mut state = ResolveContext::for_function("::p");
        state.invocation_dialect = Some(tcl_registry::InvocationDialect::of_profile(
            registry.profile().unwrap(),
        ));
        state.define_literal("d", "first NEW second OTHER", registry);
        state.define_literal("blocked(k)", "OLD", registry);
        let words = ["d", "first", "ok", "second", "blocked", "body"];
        let arguments = InvocationArguments::literals(&words).with_profile(registry.profile());
        let DictionaryScopeSelection::Selected(plan) =
            DictionaryScopeSpec::Update.select(arguments, 0)
        else {
            panic!("native update grammar");
        };
        assert!(matches!(
            enter_dictionary_scope(&mut state, &plan, arguments, registry, 10),
            DictionaryScopeEntry::Invalid
        ));
        assert_eq!(state.literal_value("ok", registry), Some("NEW"));
        assert_eq!(state.literal_value("blocked(k)", registry), Some("OLD"));
        assert_eq!(state.literal_value("blocked", registry), None);
    }
}

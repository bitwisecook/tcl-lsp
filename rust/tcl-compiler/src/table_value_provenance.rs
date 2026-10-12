// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Navigation-only literal ancestry of an actually read physical container.

use crate::{
    compilation_unit::{CompilationUnit, FunctionUnit},
    ir::{CommandTokens, Provenance, WordExpr, WordPart},
    place::Place,
    registry_invocation::{
        InvocationMetadataContext, normal_representation_invocation_with_metadata_context,
        normal_transfer_invocation_with_metadata_context,
    },
    value_provenance::ValueContributor,
    var_resolve::{ContentsOrigin, ContentsPresence, ResolveContext},
};
use std::collections::{BTreeMap, HashSet};
use tcl_registry::{CommandRegistry, VarElementsEffect};

const VALUE_BUDGET: usize = 64;

#[derive(Clone)]
enum Stored {
    Literal(ValueContributor),
    Dictionary(BTreeMap<String, Stored>),
    List(Vec<Stored>),
    Alternatives(Vec<Stored>),
    Unknown,
}

/// Editable contributors retain whether their bytes are a whole prefix or an
/// already separated constructor operand. A List operand must not be split again.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TableValueContributor {
    Prefix(ValueContributor),
    ListHead(ValueContributor),
    SingletonList(ValueContributor),
}

impl TableValueContributor {
    fn literal(&self) -> &ValueContributor {
        match self {
            Self::Prefix(value) | Self::ListHead(value) | Self::SingletonList(value) => value,
        }
    }
}

/// Original literal table values reaching this captured read. Command lookup
/// belongs to the consuming invocation; this query grants no executable proof.
pub(crate) fn table_values(
    compilation: &CompilationUnit,
    unit: &FunctionUnit,
    tokens: &CommandTokens,
    word: &WordExpr,
    keys: Option<&[Option<String>]>,
    source: &str,
    registry: &CommandRegistry,
) -> Option<Vec<TableValueContributor>> {
    let units: Vec<_> = compilation.all_body_function_units().collect();
    if !units.iter().any(|candidate| std::ptr::eq(*candidate, unit)) {
        return None;
    }
    let (_, site) = word.sole_variable_substitution()?;
    let access = tokens.variable_access_for_site(site)?;
    if access.context_residual() != crate::command_binding::SourceVariableReadResidual::Closed {
        return None;
    }
    let syntax = access.variable_context.invocation_dialect?.word_values.list;
    let mut query = Query {
        unit,
        units,
        module: &compilation.ir_module,
        source,
        registry,
        syntax,
        active: HashSet::new(),
    };
    query.invocation_metadata(tokens)?;
    let mut result = Vec::new();
    for context in access.context_alternatives() {
        if context.invocation_dialect?.word_values.list != syntax {
            return None;
        }
        let target = access.place_in_context(context, registry);
        if target.dynamic && target.index.is_none() || target.observed || target.cell.is_none() {
            return None;
        }
        if target
            .index
            .as_ref()
            .is_some_and(|index| index.kind != crate::place::IndexKind::Literal)
            && keys.is_none()
        {
            let root = target.base();
            let crate::array_destruction::ArrayContentsInventory::Known(members) =
                context.array_contents_inventory(&root)
            else {
                return None;
            };
            for member in members {
                if target
                    .index
                    .as_ref()
                    .is_some_and(|index| index.kind == crate::place::IndexKind::Literal)
                    && member.index != target.index
                {
                    continue;
                }
                result.extend(
                    query
                        .values_at(&member, context)?
                        .into_iter()
                        .flat_map(contributors),
                );
            }
        } else {
            for stored in query.values_at(&target, context)? {
                result.extend(select(stored, keys.unwrap_or(&[]), syntax)?);
            }
        }
    }
    result.sort_by_key(|value| {
        (
            value.literal().literal_span.map(tcl_lexer::Span::start),
            value.literal().value.clone(),
        )
    });
    result.dedup();
    (result.len() <= VALUE_BUDGET).then_some(result)
}

fn contributors(value: Stored) -> Vec<TableValueContributor> {
    match value {
        Stored::Literal(value) => vec![TableValueContributor::Prefix(value)],
        Stored::List(values) => {
            let singleton = values.len() == 1;
            values
                .into_iter()
                .next()
                .map_or_else(Vec::new, |head| list_heads(head, singleton))
        }
        Stored::Dictionary(_) | Stored::Unknown => Vec::new(),
        Stored::Alternatives(values) => values.into_iter().flat_map(contributors).collect(),
    }
}

fn list_heads(value: Stored, singleton: bool) -> Vec<TableValueContributor> {
    match value {
        Stored::Literal(value) => vec![if singleton {
            TableValueContributor::SingletonList(value)
        } else {
            TableValueContributor::ListHead(value)
        }],
        Stored::Alternatives(values) => values
            .into_iter()
            .flat_map(|value| list_heads(value, singleton))
            .collect(),
        Stored::List(_) | Stored::Dictionary(_) | Stored::Unknown => Vec::new(),
    }
}

fn dictionary_entries(
    value: Stored,
    syntax: tcl_dialect::ListParse,
) -> Option<BTreeMap<String, Stored>> {
    match value {
        Stored::Dictionary(entries) => Some(entries),
        Stored::Literal(value) => literal_dictionary_value(&value, syntax),
        Stored::List(values) => {
            if values.len() % 2 != 0 {
                return None;
            }
            let mut entries = BTreeMap::new();
            let mut values = values.into_iter();
            while let Some(key) = values.next() {
                let Stored::Literal(key) = key else {
                    return None;
                };
                entries.insert(key.value, values.next()?);
            }
            Some(entries)
        }
        Stored::Alternatives(_) | Stored::Unknown => None,
    }
}

fn select(
    value: Stored,
    keys: &[Option<String>],
    syntax: tcl_dialect::ListParse,
) -> Option<Vec<TableValueContributor>> {
    if keys.len() > VALUE_BUDGET {
        return None;
    }
    if let Stored::Alternatives(values) = value {
        let mut result = Vec::new();
        for value in values {
            result.extend(select(value, keys, syntax)?);
        }
        return (result.len() <= VALUE_BUDGET).then_some(result);
    }
    if matches!(value, Stored::Unknown) {
        return None;
    }
    if keys.is_empty() {
        return Some(contributors(value));
    }
    let entries = dictionary_entries(value, syntax)?;
    let mut result = Vec::new();
    for (key, value) in entries {
        if keys[0].as_ref().is_none_or(|wanted| *wanted == key) {
            result.extend(select(value, &keys[1..], syntax)?);
        }
    }
    Some(result)
}

struct Query<'a> {
    unit: &'a FunctionUnit,
    units: Vec<&'a FunctionUnit>,
    module: &'a crate::ir::Module,
    source: &'a str,
    registry: &'a CommandRegistry,
    syntax: tcl_dialect::ListParse,
    active: HashSet<(*const FunctionUnit, crate::cfg::BlockId, usize, Place)>,
}

type StorePoint<'a> = (
    &'a FunctionUnit,
    crate::cfg::BlockId,
    usize,
    &'a CommandTokens,
);

impl<'a> Query<'a> {
    /// Keep whole-Module currency separate from FU/point availability. Only an
    /// independently retained Standalone entry can use the compatibility route.
    fn invocation_metadata<'b>(
        &self,
        tokens: &'b CommandTokens,
    ) -> Option<Option<InvocationMetadataContext<'b>>> {
        let owner = self.module.retained_source_bindings.as_deref()?;
        if !owner.matches_module(self.module, self.registry) {
            return None;
        }
        let binding = tokens.source_binding.as_ref()?;
        if self.module.source_metadata_input.is_some() {
            self.unit
                .invocation_metadata_context_for_module(self.registry, self.module)?;
            return binding
                .original_invocation_metadata_for_module(tokens, self.module, self.registry)
                .map(Some);
        }
        if !self.module.source_entry.metadata_context.is_standalone()
            || !owner.owns_original_tokens(tokens)
        {
            return None;
        }
        binding.original_invocation_metadata_for_function(tokens, self.unit, self.registry)
    }

    fn values_at(&mut self, target: &Place, context: &ResolveContext) -> Option<Vec<Stored>> {
        if !context.contents_have_authored_source(target, self.source)
            || target.observed
            || target.dynamic
            || context.contents_presence(target) != ContentsPresence::Defined
        {
            return None;
        }
        let offsets = match context.read_contents_origin(target, self.registry) {
            ContentsOrigin::WrittenAt(offset) => vec![offset],
            ContentsOrigin::Alternatives {
                incoming: false,
                writes,
            } => writes,
            _ => return None,
        };
        let mut result = Vec::new();
        for offset in offsets {
            let producer_points = self.represented_stores_at(offset, target, context)?;
            let matched = !producer_points.is_empty();
            for (unit, block, index, tokens) in producer_points {
                let key = (std::ptr::from_ref(unit), block, index, target.clone());
                if !self.active.insert(key.clone()) {
                    return None;
                }
                #[cfg(test)]
                if std::env::var_os("TCL_TABLE_PROOF_DEBUG").is_some() {
                    eprintln!(
                        "table represented writer {offset}: {:?}",
                        tokens
                            .words()
                            .iter()
                            .map(WordExpr::legacy_text)
                            .collect::<Vec<_>>()
                    );
                }
                let original_unit = self.unit;
                self.unit = unit;
                let stored = self.values_from_store(tokens, target);
                self.unit = original_unit;
                self.active.remove(&key);
                result.extend(stored?);
                if result.len() > VALUE_BUDGET {
                    return None;
                }
            }
            if !matched {
                return None;
            }
        }
        Some(result)
    }

    fn represented_stores_at(
        &self,
        offset: u32,
        target: &Place,
        context: &ResolveContext,
    ) -> Option<Vec<StorePoint<'a>>> {
        let mut stores = Vec::new();
        for &unit in &self.units {
            let Some(points) = unit.ssa.point_contexts.as_ref() else {
                continue;
            };
            for (&block, body) in &unit.ssa.blocks {
                for (index, statement) in body.statements.iter().enumerate() {
                    if unit.abs_span(statement.statement.span()).start() != offset {
                        continue;
                    }
                    let before = points.context_before(block, index)?;
                    let after = points.after_statement(block, index);
                    let writes = crate::place_bridge::def_places_with_continuation(
                        &statement.statement,
                        before,
                        after,
                        self.registry,
                    );
                    if writes.iter().any(|write| {
                        write.cell == target.cell && crate::place::overlap(write, target)
                    }) {
                        let tokens = points.source_tokens_at(block, index)?;
                        let source = tokens.source_binding.as_ref()?.source_origin()?;
                        if context.contents_have_source(target, source) {
                            stores.push((unit, block, index, tokens));
                        }
                    }
                }
            }
        }
        Some(stores)
    }

    fn values_from_store(&mut self, tokens: &CommandTokens, target: &Place) -> Option<Vec<Stored>> {
        if !self.authored_carrier(tokens) {
            return None;
        }
        let metadata = self.invocation_metadata(tokens)?;
        let normal =
            normal_transfer_invocation_with_metadata_context(self.registry, metadata, tokens)?;
        let context = &tokens.source_binding.as_ref()?.variable_context;
        if let Some(word) = normal.stored_value_word(context, self.registry) {
            return self.word_value(word, tokens);
        }
        let write = normal.container_element_write(context, self.registry)?;
        match write.effect {
            VarElementsEffect::SetsArrayElementsFromList { values_at } => {
                let word = write
                    .words
                    .get(write.argument_offset + usize::from(values_at))?;
                let dictionary = self.literal_dictionary(word)?;
                let index = target.index.as_ref()?;
                if index.kind != crate::place::IndexKind::Literal {
                    return None;
                }
                Some(vec![dictionary.get(index.value.try_utf8().ok()?)?.clone()])
            }
            VarElementsEffect::SetsDictValue => {
                let keys = write.dictionary_keys()?;
                let value = self.stored_word(write.words.last()?, tokens);
                let mut result = if context.contents_presence(target) == ContentsPresence::Undefined
                {
                    vec![Stored::Dictionary(BTreeMap::new())]
                } else {
                    self.values_at(target, context)?
                };
                for stored in &mut result {
                    if matches!(stored, Stored::Literal(_) | Stored::List(_)) {
                        *stored =
                            Stored::Dictionary(dictionary_entries(stored.clone(), self.syntax)?);
                    }
                    put(stored, keys, value.clone(), self.syntax)?;
                }
                Some(result)
            }
            _ => None,
        }
    }

    fn authored_carrier(&self, tokens: &CommandTokens) -> bool {
        tokens.source_binding.as_ref().and_then(|binding| binding.source_origin()).is_some_and(|origin| {
            matches!(origin.kind(), crate::command_binding::SourceOriginKind::Authored(text) if text.bytes() == self.source.as_bytes())
        })
    }

    fn stored_word(&mut self, word: &WordExpr, tokens: &CommandTokens) -> Stored {
        match self.word_value(word, tokens) {
            Some(mut values) if values.len() == 1 => values.remove(0),
            Some(values) if !values.is_empty() => Stored::Alternatives(values),
            _ => Stored::Unknown,
        }
    }

    fn word_value(&mut self, word: &WordExpr, tokens: &CommandTokens) -> Option<Vec<Stored>> {
        if !self.authored_carrier(tokens) {
            return None;
        }
        if word.sole_variable_substitution().is_some() {
            let (_, site) = word.sole_variable_substitution()?;
            let access = tokens.variable_access_for_site(site)?;
            if access.context_residual()
                != crate::command_binding::SourceVariableReadResidual::Closed
            {
                return None;
            }
            let mut values = Vec::new();
            for context in access.context_alternatives() {
                if context.invocation_dialect?.word_values.list != self.syntax {
                    return None;
                }
                let place = access.place_in_context(context, self.registry);
                values.extend(self.values_at(&place, context)?);
                if values.len() > VALUE_BUDGET {
                    return None;
                }
            }
            return (!values.is_empty()).then_some(values);
        }
        if let Some(value) = self.literal(word) {
            return Some(vec![Stored::Literal(value)]);
        }
        let metadata = self.invocation_metadata(tokens)?;
        let config = self.unit.source_lexer_config();
        let calls = if let Some(actual) =
            metadata.filter(|actual| actual.source_analysis_input().is_some())
        {
            crate::word_subst::checked_original_lifted_calls_with_metadata_context(
                tokens,
                config,
                self.registry,
                actual,
            )?
        } else {
            // The invocation metadata issuer has already proved explicit
            // Standalone ownership and the original complete source vector.
            crate::word_subst::checked_lifted_calls(tokens, config)?
        };
        let mut nested = crate::word_subst::whole_word_command_tokens(word, config)?;
        nested.inherit_nested_bindings(tokens);
        if !calls
            .iter()
            .any(|call| call.tokens.as_ref() == Some(&nested))
        {
            return None;
        }
        let metadata = self.invocation_metadata(&nested)?;
        let normal = normal_representation_invocation_with_metadata_context(
            self.registry,
            metadata,
            &nested,
        )?;
        if let Some(words) = normal.list_constructor_words() {
            if words.len() > VALUE_BUDGET {
                return None;
            }
            let values = words
                .iter()
                .map(|word| self.stored_word(word, &nested))
                .collect();
            return Some(vec![Stored::List(values)]);
        }
        let words = normal.dictionary_constructor_words()?;
        if words.len() % 2 != 0 || words.len() > VALUE_BUDGET * 2 {
            return None;
        }
        let mut dictionary = BTreeMap::new();
        for pair in words.as_chunks::<2>().0 {
            let key = self.literal(&pair[0])?.value;
            let value = self.stored_word(&pair[1], &nested);
            dictionary.insert(key, value);
        }
        Some(vec![Stored::Dictionary(dictionary)])
    }

    fn literal(&self, word: &WordExpr) -> Option<ValueContributor> {
        let (text, site, offset) = match word {
            WordExpr::Literal { text, source } => (text, source, 0),
            WordExpr::BracedLiteral { text, source } => (text, source, 1),
            WordExpr::Template { parts, .. } => match parts.as_slice() {
                [WordPart::Text { text, source }] if !text.contains('\\') => (text, source, 0),
                _ => return None,
            },
            _ => return None,
        };
        if site.provenance != Provenance::Source || text.contains("\\\n") {
            return None;
        }
        let start = site.span.start().checked_add(offset)?;
        let span = self.unit.abs_span(tcl_lexer::Span::new(
            start,
            start.checked_add(u32::try_from(text.len()).ok()?)?,
        ));
        (self
            .source
            .get(span.start() as usize..span.end() as usize)?
            == text)
            .then(|| ValueContributor {
                value: text.clone(),
                literal_span: Some(span),
            })
    }

    fn literal_dictionary(&self, word: &WordExpr) -> Option<BTreeMap<String, Stored>> {
        let literal = self.literal(word)?;
        literal_dictionary_value(&literal, self.syntax)
    }
}

fn literal_dictionary_value(
    literal: &ValueContributor,
    syntax: tcl_dialect::ListParse,
) -> Option<BTreeMap<String, Stored>> {
    let span = literal.literal_span?;
    let mut elements = Vec::new();
    let mut at = 0;
    while let Some(element) =
        tcl_syntax::list::find_element_with_syntax(&literal.value, at, syntax).ok()?
    {
        if !element.literal {
            return None;
        }
        let value = literal.value.get(element.value.clone())?.to_owned();
        let start = span
            .start()
            .checked_add(u32::try_from(element.value.start).ok()?)?;
        let end = span
            .start()
            .checked_add(u32::try_from(element.value.end).ok()?)?;
        elements.push(ValueContributor {
            value,
            literal_span: Some(tcl_lexer::Span::new(start, end)),
        });
        if elements.len() > VALUE_BUDGET * 2 {
            return None;
        }
        at = element.next;
    }
    if elements.len() % 2 != 0 {
        return None;
    }
    let mut dictionary = BTreeMap::new();
    for pair in elements.as_chunks::<2>().0 {
        dictionary.insert(pair[0].value.clone(), Stored::Literal(pair[1].clone()));
    }
    Some(dictionary)
}

fn put(
    value: &mut Stored,
    keys: &[String],
    stored: Stored,
    syntax: tcl_dialect::ListParse,
) -> Option<()> {
    if keys.len() > VALUE_BUDGET {
        return None;
    }
    if let Stored::Alternatives(values) = value {
        for value in values {
            put(value, keys, stored.clone(), syntax)?;
        }
        return Some(());
    }
    if matches!(value, Stored::Literal(_) | Stored::List(_)) {
        *value = Stored::Dictionary(dictionary_entries(value.clone(), syntax)?);
    }
    let (key, remaining) = keys.split_first()?;
    let Stored::Dictionary(entries) = value else {
        return None;
    };
    if remaining.is_empty() {
        entries.insert(key.clone(), stored);
    } else {
        put(
            entries
                .entry(key.clone())
                .or_insert_with(|| Stored::Dictionary(BTreeMap::new())),
            remaining,
            stored,
            syntax,
        )?;
    }
    (entries.len() <= VALUE_BUDGET).then_some(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyser::ResolvedAnalysisInput;
    use crate::compilation_unit::UnitBuildOptions;
    use std::sync::Arc;

    fn read_values(
        unit: &CompilationUnit,
        registry: &CommandRegistry,
    ) -> Option<Vec<TableValueContributor>> {
        let script = &unit.ir_module.top_level;
        let tokens = script.retained_source_tokens_for_statement(script.statements.last()?)?;
        table_values(
            unit,
            &unit.top_level,
            tokens,
            tokens.words().get(1)?,
            Some(&[Some("alpha".into())]),
            &unit.source,
            registry,
        )
    }

    #[test]
    fn original_table_values_keep_actual_constructor_selection_and_whole_module_owner() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Original literal ancestry of a separately admitted cell/read and
        // construction handler; neither context nor ancestry grants execution.
        let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        let context =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        let config = tcl_lexer::LexerConfig::for_profile(Some(profile));
        let input = ResolvedAnalysisInput::new(profile, profile, Arc::clone(&context), config);
        let (_owner, native) =
            crate::environment_ingress::captured_native_entry_with_owner(profile);
        let entry = crate::command_binding::SourceAnalysisEntry {
            metadata_context:
                crate::registry_invocation::OwnedInvocationMetadataContext::SuppliedSource(
                    Box::new(input.clone()),
                ),
            native_entry: Some(Arc::new(native)),
            ..Default::default()
        };
        for (source, expected) in [
            (
                "set table [dict create alpha {helper baked}]; puts $table",
                Some("helper baked"),
            ),
            (
                "rename dict nativeDict; set table [nativeDict create alpha {helper baked}]; puts $table",
                Some("helper baked"),
            ),
            (
                "proc dict {args} {return {alpha {helper baked}}}; set table [dict create alpha {helper baked}]; puts $table",
                None,
            ),
        ] {
            let unit = CompilationUnit::build_with_analysis_input(
                source,
                UnitBuildOptions {
                    registry: context.commands(),
                    defer_top_level: false,
                    config,
                    dialect: Some(profile),
                    external_call_sites: None,
                    declared_commands: None,
                },
                Some(&entry),
                &input,
            );
            let values = read_values(&unit, context.commands());
            assert_eq!(
                values
                    .as_ref()
                    .and_then(|values| values.first())
                    .map(|value| value.literal().value.as_str()),
                expected,
                "{source}"
            );
            if let Some(values) = values {
                assert_eq!(values.len(), 1);
                assert_eq!(
                    source.get(values[0].literal().literal_span.unwrap().as_range()),
                    expected
                );
            }
            let mut changed = unit.clone();
            changed.ir_module.top_level_namespace = "::changed".into();
            assert!(read_values(&changed, context.commands()).is_none());
            let mut missing = unit.clone();
            missing.top_level.source_metadata_input = None;
            assert!(read_values(&missing, context.commands()).is_none());
            let mut unavailable = unit.clone();
            let older = Arc::new(
                tcl_registry::model::ingress::static_context_for("tcl8.4")
                    .with_command_store(Arc::clone(context.commands())),
            );
            unavailable.top_level.source_metadata_input =
                Some(ResolvedAnalysisInput::new(profile, profile, older, config));
            assert!(read_values(&unavailable, context.commands()).is_none());
        }
    }
}

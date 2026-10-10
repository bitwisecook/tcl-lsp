// SPDX-License-Identifier: AGPL-3.0-or-later
//! Namespace edits from authentic original operands and retained byte geometry.

use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::signature_scan::scope::{SignatureNamespaceScope, SignatureSourceNameInput};
use tcl_core_types::NameBytes;
use tcl_lexer::{LineIndex, SourceImage, Span};
use tcl_syntax::naming::{NativeNameContext, NativeNamespaceOperandRename};

use crate::namespace_symbol::OriginalNamespaceSymbol;
use crate::rename::TextEdit;
use crate::rename_safety::RenameRefusal;

struct OriginalNamespaceEdits<'a> {
    source: &'a str,
    image: SourceImage,
    analysis: &'a AnalysisResult,
    symbol: &'a OriginalNamespaceSymbol,
    new_tail: &'a str,
    tail_units: Vec<u8>,
    requests: Vec<(SignatureSourceNameInput, NameBytes)>,
    roots: Vec<(Span, String)>,
    considered: Vec<Span>,
}

impl OriginalNamespaceEdits<'_> {
    fn refusal(&self, span: Option<Span>, reason: &str) -> RenameRefusal {
        RenameRefusal::at(
            reason.to_owned(),
            self.source,
            &LineIndex::new(self.source),
            span,
        )
    }

    fn record_alias_target(
        &mut self,
        input: &SignatureSourceNameInput,
    ) -> Result<(), RenameRefusal> {
        let wanted =
            self.symbol.scope().context().ok_or_else(|| {
                self.refusal(None, "the selected namespace has no native geometry")
            })?;
        let slot = self
            .symbol
            .policy()
            .recipe()
            .command_lookup_slot(NativeNameContext::root(), input.bytes())
            .map_err(|_| self.refusal(None, "the alias target has no command naming purpose"))?;
        if tcl_syntax::naming::native_command_slot_is_under_namespace(
            self.symbol.policy().recipe(),
            &slot,
            wanted,
        ) == Some(false)
        {
            return Ok(());
        }
        let projection = tcl_syntax::naming::native_namespace_operand_rename(
            self.symbol.policy().recipe(),
            NativeNameContext::root(),
            input.bytes(),
            wanted,
            &self.tail_units,
        )
        .ok_or_else(|| self.refusal(None, "the alias target has no selected namespace purpose"))?;
        if matches!(projection, NativeNamespaceOperandRename::Unchanged) {
            return Ok(());
        }
        let span = input_source_span(input).ok_or_else(|| {
            self.refusal(
                None,
                "the alias target names this namespace through a readonly computed value",
            )
        })?;
        self.record_command(input, NativeNameContext::root(), span)
    }

    fn record_command(
        &mut self,
        input: &SignatureSourceNameInput,
        current: NativeNameContext<'_>,
        span: Span,
    ) -> Result<(), RenameRefusal> {
        let slot = input
            .policy()
            .recipe()
            .command_lookup_slot(current, input.bytes())
            .map_err(|_| {
                self.refusal(
                    Some(span),
                    "the command operand has no native lookup geometry",
                )
            })?;
        let wanted = self.symbol.scope().context().ok_or_else(|| {
            self.refusal(Some(span), "the selected namespace has no native geometry")
        })?;
        match tcl_syntax::naming::native_command_slot_is_under_namespace(
            input.policy().recipe(),
            &slot,
            wanted,
        ) {
            Some(false) => {
                self.considered.push(span);
                Ok(())
            }
            Some(true) => self.record(input, current, span),
            None => Err(self.refusal(
                Some(span),
                "the command namespace qualifier has no supported exact geometry",
            )),
        }
    }

    fn record_variable(
        &mut self,
        input: &SignatureSourceNameInput,
        current: NativeNameContext<'_>,
        span: Span,
    ) -> Result<(), RenameRefusal> {
        let recipe = input.policy().recipe();
        let combined;
        let root = match input {
            SignatureSourceNameInput::OriginalVariableRoot(root)
                if root.is_separate_array_root() =>
            {
                input.bytes()
            }
            _ => {
                combined = recipe.combined_variable_input(input.bytes());
                combined.root().selected()
            }
        };
        let wanted = self.symbol.scope().context().ok_or_else(|| {
            self.refusal(Some(span), "the selected namespace has no native geometry")
        })?;
        match tcl_syntax::naming::native_variable_root_is_under_namespace(
            recipe, current, root, wanted,
        ) {
            Some(false) => {
                self.considered.push(span);
                Ok(())
            }
            Some(true) => self.record(input, current, span),
            None => Err(self.refusal(
                Some(span),
                "the variable namespace qualifier has no supported exact geometry",
            )),
        }
    }

    fn record(
        &mut self,
        input: &SignatureSourceNameInput,
        current: NativeNameContext<'_>,
        span: Span,
    ) -> Result<(), RenameRefusal> {
        if input.policy() != self.symbol.policy() {
            return Ok(());
        }
        let wanted = self.symbol.scope().context().ok_or_else(|| {
            self.refusal(
                Some(span),
                "the selected namespace has no original native geometry",
            )
        })?;
        let projection = tcl_syntax::naming::native_namespace_operand_rename(
            self.symbol.policy().recipe(), current, input.bytes(), wanted, &self.tail_units,
        ).ok_or_else(|| self.refusal(Some(span), "the namespace operand or replacement does not round-trip through its original naming purpose"))?;
        if !crate::original_name_edit::original_input_matches_source(
            self.source,
            self.analysis,
            input,
            span,
        ) {
            return Err(self.refusal(
                Some(span),
                "the namespace operand no longer matches its original source owner",
            ));
        }
        self.considered.push(span);
        let NativeNamespaceOperandRename::Replace(bytes) = projection else {
            return Ok(());
        };
        if bytes == input.bytes() {
            return Ok(());
        }
        if let SignatureSourceNameInput::OriginalVariableRoot(root) = input {
            let name = root.name_span().ok_or_else(|| {
                self.refusal(
                    Some(span),
                    "the lexical variable has no authentic original name extent",
                )
            })?;
            let member = tcl_syntax::naming::native_written_namespace_member_extent(
                self.symbol.policy().recipe(),
                current,
                root.bytes(),
                wanted,
            )
            .ok_or_else(|| {
                self.refusal(
                    Some(span),
                    "the lexical variable does not write the selected namespace component",
                )
            })?;
            let raw = self.image.bytes().get(name.as_range()).ok_or_else(|| {
                self.refusal(
                    Some(span),
                    "the lexical variable name is outside its source image",
                )
            })?;
            let extent = tcl_syntax::backslash::native_source_literal_extent(
                raw,
                self.image.channel(),
                root.policy().string_protocol(),
                member,
            )
            .ok_or_else(|| {
                self.refusal(
                    Some(span),
                    "the namespace component crosses a native source unit boundary",
                )
            })?;
            let mut after = std::str::from_utf8(raw)
                .map_err(|_| self.refusal(Some(span), "the editor variable source is not Unicode"))?
                .to_owned();
            after.replace_range(extent, self.new_tail);
            let produced = tcl_syntax::backslash::native_source_literal_bytes(
                after.as_bytes(),
                self.image.channel(),
                root.policy().string_protocol(),
            )
            .map_err(|_| {
                self.refusal(
                    Some(span),
                    "the replacement variable name has no source channel recipe",
                )
            })?;
            if produced.as_ref() != bytes {
                return Err(self.refusal(
                    Some(span),
                    "the replacement changes unrelated lexical variable units",
                ));
            }
            let edit = crate::original_name_edit::original_variable_root_name_edit(&self.image,root,&after)
                .ok_or_else(|| self.refusal(Some(span), "the replacement changes lexical variable delimiters or its root/index boundary"))?;
            self.roots.push((edit.span(), edit.text().to_owned()));
        } else {
            if input.original_static_list_container().is_none() {
                return Err(self.refusal(Some(span), "the namespace is supplied by a readonly computed value with no original editable container"));
            }
            self.requests.push((input.clone(), bytes.into()));
        }
        Ok(())
    }
}

/// Rename a namespace selected from an authentic original operand. Inputs,
/// exact namespace geometry and source currency precede every edit. Computed
/// names without an original static container and unsupported callback homes
/// refuse atomically. This supplies source edits, not execution/lifetime proof.
///
/// # Errors
/// Returns a refusal when any required operand, source mapping or coverage
/// obligation cannot be independently established.
pub fn original_namespace_rename_edits(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    analysis: &AnalysisResult,
    symbol: &OriginalNamespaceSymbol,
    new_tail: &str,
) -> Result<Vec<TextEdit>, RenameRefusal> {
    if analysis.resolved_profile() != Some(dialect) {
        return Err(RenameRefusal::at(
            "the requested profile differs from the retained analysis input".to_owned(),
            source,
            &LineIndex::new(source),
            None,
        ));
    }
    let image = SourceImage::document(source);
    let tail_units = tcl_syntax::backslash::native_source_literal_bytes(
        new_tail.as_bytes(),
        image.channel(),
        symbol.policy().string_protocol(),
    )
    .map_err(|_| {
        RenameRefusal::at(
            "the replacement namespace has no selected source channel recipe".to_owned(),
            source,
            &LineIndex::new(source),
            None,
        )
    })?
    .into_owned();
    let mut plan = OriginalNamespaceEdits {
        source,
        image,
        analysis,
        symbol,
        new_tail,
        tail_units,
        requests: Vec::new(),
        roots: Vec::new(),
        considered: Vec::new(),
    };
    let realm = analysis
        .retained_command_realm()
        .ok_or_else(|| plan.refusal(None, "the document has no retained original command owner"))?;
    let config = analysis
        .body_lexer_config
        .ok_or_else(|| plan.refusal(None, "the document has no retained complete grammar"))?;
    if !analysis.matches_original_source_image(&plan.image, config) {
        return Err(plan.refusal(
            None,
            "the document no longer matches its complete original analysis/source owner",
        ));
    }
    for row in &analysis.namespace_refs {
        let (Some(input), Some(current), Some(selected)) = (
            row.original_name_input.as_ref(),
            row.source_context.as_ref(),
            row.source_namespace.as_ref(),
        ) else {
            continue;
        };
        if row.name_policy != Some(symbol.policy()) {
            continue;
        }
        if selected == symbol.scope()
            || symbol
                .scope()
                .is_strict_ancestor_of(selected, symbol.policy())
                == Some(true)
        {
            plan.record(
                input,
                current.context().ok_or_else(|| {
                    plan.refusal(
                        Some(row.span),
                        "the namespace occurrence has no native naming context",
                    )
                })?,
                row.span,
            )?;
        }
    }
    for row in &analysis.qualified_var_refs {
        let (Some(input), Some(current)) = (
            row.original_name_input.as_ref(),
            row.original_namespace.as_ref(),
        ) else {
            continue;
        };
        if input.policy() != symbol.policy() {
            continue;
        }
        plan.record_variable(
            input,
            current.context().ok_or_else(|| {
                plan.refusal(
                    Some(row.span),
                    "the variable occurrence has no original native namespace",
                )
            })?,
            row.span,
        )?;
    }
    for pattern in analysis.original_namespace_patterns() {
        plan.record(
            pattern.name_input(),
            pattern.context().context().ok_or_else(|| {
                plan.refusal(
                    Some(pattern.span()),
                    "the pattern has no original naming context",
                )
            })?,
            pattern.span(),
        )?;
    }
    for (input, site, publication) in analysis
        .original_procedure_declarations()
        .map(|row| (row.name_input(), row.declaration_site(), row.name()))
        .chain(
            analysis
                .original_class_declarations()
                .map(|row| (row.name_input(), row.declaration_site(), row.name())),
        )
    {
        if input.policy() != symbol.policy() {
            continue;
        }
        if !slot_may_be_under(publication.slot(), symbol) {
            plan.considered.push(input.span());
            continue;
        }
        let current = analysis
            .original_namespace_scope_at(site.offset)
            .ok_or_else(|| {
                plan.refusal(
                    Some(input.span()),
                    "the declaration has no retained original namespace",
                )
            })?;
        use tcl_compiler::signature_scan::scope::{
            SignatureSourceCommand, SourceCommandPublication,
        };
        let reissued = match publication.publication() {
            SourceCommandPublication::NamedCommand => {
                SignatureSourceCommand::procedure_from_key(&current, input)
            }
            SourceCommandPublication::TclOoObject => {
                SignatureSourceCommand::object_from_key(&current, input)
            }
            SourceCommandPublication::ProviderAdvice => {
                SignatureSourceCommand::provider_advice_from_key(&current, input)
            }
        };
        if reissued.as_ref() != Some(publication) {
            return Err(plan.refusal(
                Some(input.span()),
                "the declaration publication and original naming context disagree",
            ));
        }
        plan.record(
            &SignatureSourceNameInput::OriginalWord(input.clone()),
            current.context().ok_or_else(|| {
                plan.refusal(
                    Some(input.span()),
                    "the declaration namespace has no native geometry",
                )
            })?,
            input.span(),
        )?;
    }
    for invocation in &analysis.command_invocations {
        let Some(input) = invocation.original_name_input.as_ref() else {
            continue;
        };
        if input.policy() != symbol.policy() {
            continue;
        }
        let Some(lookup) = invocation.original_lookup.as_ref() else {
            // Original executable heads (including static expansions) have
            // direct absolute namespace geometry. Mere catalogue roles and
            // callback children need their independent receiver purpose.
            if input.bytes().starts_with(b"::") {
                if let Some(reference) = invocation.resolved_command_reference.as_ref() {
                    let slot = reference.original_slot().ok_or_else(|| {
                        plan.refusal(
                            Some(invocation.range),
                            "the consumed command has no exact original slot",
                        )
                    })?;
                    if reference.original_name_policy() != Some(input.policy())
                        || input
                            .policy()
                            .recipe()
                            .command_lookup_slot(NativeNameContext::root(), input.bytes())
                            .ok()
                            .as_ref()
                            != Some(slot)
                    {
                        return Err(plan.refusal(
                            Some(invocation.range),
                            "the consumed command's original input and called slot disagree",
                        ));
                    }
                    plan.record_command(input, NativeNameContext::root(), invocation.range)?;
                } else if invocation.lookup.is_execution_site()
                    && input.original_static_list_container().is_some()
                {
                    plan.record_command(input, NativeNameContext::root(), invocation.range)?;
                }
            }
            continue;
        };
        if lookup.name_input() != input || lookup.site().source.source_image() != &plan.image {
            return Err(plan.refusal(
                Some(invocation.range),
                "the command lookup belongs to a different original operand",
            ));
        }
        let current = lookup
            .original_naming_scope()
            .and_then(SignatureNamespaceScope::context);
        let context = if let Some(reference) = invocation.resolved_command_reference.as_ref() {
            let slot = reference.original_slot().ok_or_else(|| {
                plan.refusal(
                    Some(invocation.range),
                    "the called command has no exact original slot",
                )
            })?;
            if reference.original_name_policy() != Some(input.policy()) {
                return Err(plan.refusal(
                    Some(invocation.range),
                    "the called command has another naming policy",
                ));
            }
            let Some(current) = current.or_else(|| {
                input
                    .bytes()
                    .starts_with(b"::")
                    .then(NativeNameContext::root)
            }) else {
                if slot_may_be_under(slot, symbol)
                    && input.bytes().windows(2).any(|pair| pair == b"::")
                {
                    return Err(plan.refusal(
                        Some(invocation.range),
                        "the called namespace component has no independently retained lookup home",
                    ));
                }
                continue;
            };
            tcl_syntax::naming::native_command_lookup_context_for_slot(
                input.policy().recipe(),
                current,
                input.bytes(),
                slot,
            )
            .ok_or_else(|| {
                plan.refusal(
                    Some(invocation.range),
                    "the written command and called slot disagree",
                )
            })?
        } else if input.bytes().starts_with(b"::") {
            NativeNameContext::root()
        } else {
            if input.bytes().windows(2).any(|pair| pair == b"::")
                && lookup
                    .candidates()
                    .iter()
                    .flatten()
                    .any(|slot| slot_may_be_under(slot, symbol))
            {
                return Err(plan.refusal(Some(invocation.range), "the relative command's ordered alternatives do not select one called namespace"));
            }
            plan.considered.push(invocation.range);
            continue;
        };
        plan.record_command(input, context, invocation.range)?;
    }
    if let Some(world) = analysis.original_completed_command_world() {
        if world.source_image() != &plan.image || world.lexer_config() != config {
            return Err(plan.refusal(
                None,
                "the completed command world belongs to another source or grammar",
            ));
        }
        for publication in world.declarations() {
            if publication.policy() != symbol.policy() {
                continue;
            }
            if let Some(target) = publication.alias_target() {
                match target.lookup() {
                    tcl_registry::AliasTargetLookup::Global => {
                        plan.record_alias_target(target.name_input())?
                    }
                    tcl_registry::AliasTargetLookup::CallerNamespace => {
                        if target.name_input().bytes().starts_with(b"::") {
                            plan.record_alias_target(target.name_input())?;
                        }
                    }
                }
            }
        }
    }
    if let Some(span) = analysis.namespace_path_computed.first() {
        return Err(plan.refusal(
            Some(*span),
            "a computed namespace path has no complete original list container",
        ));
    }
    let coverage = original_coverage(source, &plan.image, analysis, config).ok_or_else(|| {
        plan.refusal(
            None,
            "the original lexical coverage cannot retain every command/body region",
        )
    })?;
    for span in &analysis.namespace_name_unknowns {
        let word = coverage.word_at(*span).ok_or_else(|| {
            plan.refusal(
                Some(*span),
                "the unknown namespace-role operand has no complete original lexical owner",
            )
        })?;
        if word_may_select_namespace(word, symbol, config) {
            return Err(plan.refusal(
                Some(*span),
                "a namespace naming word computed at run time may still select this namespace",
            ));
        }
        plan.considered.push(*span);
    }
    let mut hazard = None;
    for command in &coverage.commands {
        if hazard.is_some() {
            break;
        }
        for token in &command.argv {
            if plan
                .considered
                .iter()
                .any(|span| super::spans_overlap(*span, token.span))
                || coverage.bodies.iter().any(|(start, end)| {
                    (token.span.start() as usize) < *end && *start < token.span.end() as usize
                })
            {
                continue;
            }
            let may_name = if let Some(input) =
                realm.original_written_name_input_at_span_in_source(&plan.image, token.span, config)
            {
                input.policy() == symbol.policy()
                    && literal_may_name_namespace(input.bytes(), symbol, config)
            } else {
                // Lexical coverage does not mint a value key or lookup. It
                // checks the same original arena for unsupported embedded
                // references that the positioned value producer declined.
                let word = coverage.word_at(token.span);
                word.is_none_or(|word| word_may_contain_namespace(word, symbol, config))
            };
            if may_name {
                hazard = Some(token.span);
                break;
            }
        }
    }
    if let Some(span) = hazard {
        return Err(plan.refusal(
            Some(span),
            "an original value may name this namespace in an unsupported callback or data position",
        ));
    }
    let edits = crate::original_name_edit::original_name_input_edits(&plan.image, &plan.requests)
        .ok_or_else(|| plan.refusal(None, "the namespace edit containers conflict or cannot preserve every unrelated original value"))?;
    let mut edits = edits
        .into_iter()
        .map(|edit| (edit.span(), edit.text().to_owned()))
        .chain(plan.roots.iter().cloned())
        .collect::<Vec<_>>();
    edits.sort_by_key(|(span, _)| (span.start(), span.end()));
    edits.dedup();
    if edits
        .windows(2)
        .any(|pair| pair[0].0.end() > pair[1].0.start())
    {
        return Err(plan.refusal(
            None,
            "namespace edits overlap different original producer domains",
        ));
    }
    let index = LineIndex::new(source);
    Ok(edits
        .into_iter()
        .map(|(span, new_text)| TextEdit {
            range: crate::definition::span_to_range(source, &index, span),
            new_text,
        })
        .collect())
}

fn literal_may_name_namespace(
    value: &[u8],
    symbol: &OriginalNamespaceSymbol,
    config: tcl_lexer::LexerConfig,
) -> bool {
    let Some(wanted) = symbol.scope().context() else {
        return true;
    };
    let mut pending = vec![(value.to_vec(), 0usize)];
    let mut budget = 128usize;
    while let Some((value, depth)) = pending.pop() {
        if budget == 0 {
            return true;
        }
        budget -= 1;
        if value.starts_with(b"::")
            && tcl_syntax::naming::native_written_namespace_prefix_extent(
                symbol.policy().recipe(),
                NativeNameContext::root(),
                &value,
                wanted,
            )
            .is_some()
        {
            return true;
        }
        if depth == 4 {
            return true;
        }
        if let Ok(elements) =
            tcl_syntax::list::split_native_list_bytes(&value, symbol.policy().string_protocol())
        {
            for element in elements {
                if element.as_ref() != value {
                    pending.push((element.to_vec(), depth + 1));
                }
            }
        }
        let image = SourceImage::native(value.as_slice());
        let Ok(length) = u32::try_from(value.len()) else {
            return true;
        };
        if let Ok(script) = tcl_lexer::native_script_words_in(image, Span::new(0, length), config) {
            for command in script.commands {
                for word in &command.words {
                    if lexical_roots_may_name_namespace(word, symbol) {
                        return true;
                    }
                }
                if let Ok(words) = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
                    &command.words,
                    symbol.policy().string_protocol(),
                ) {
                    for ordinal in 0..command.words.len() {
                        if let Some(word) = words.literal(ordinal) {
                            if word != value {
                                pending.push((word.to_vec(), depth + 1));
                            }
                        }
                    }
                }
            }
        }
    }
    false
}

fn input_source_span(input: &SignatureSourceNameInput) -> Option<Span> {
    let container = input.original_static_list_container()?;
    Some(container.parent_word().tokens().first()?.span)
}

fn slot_may_be_under(
    slot: &tcl_core_types::ByteCommandSlot,
    symbol: &OriginalNamespaceSymbol,
) -> bool {
    symbol.scope().context().is_none_or(|wanted| {
        tcl_syntax::naming::native_command_slot_is_under_namespace(
            symbol.policy().recipe(),
            slot,
            wanted,
        ) != Some(false)
    })
}

struct OriginalNamespaceCoverage {
    commands: Vec<tcl_compiler::segmenter::SegmentedCommand>,
    words: std::collections::HashMap<Span, tcl_lexer::NativeWord>,
    bodies: Vec<(usize, usize)>,
}

impl OriginalNamespaceCoverage {
    fn word_at(&self, span: Span) -> Option<&tcl_lexer::NativeWord> {
        self.words.get(&span)
    }
}

// A lexical May inspection uses the authentic whole image/full grammar and
// independently retained declaration body regions. It does not reconstruct
// command bindings or issue value keys, lookup or editable source authority.
fn original_coverage(
    source: &str,
    image: &SourceImage,
    analysis: &AnalysisResult,
    config: tcl_lexer::LexerConfig,
) -> Option<OriginalNamespaceCoverage> {
    // Existing analyses retain the actual source availability/hosted contexts.
    // The shared scanner supplies only body/command geometry, not a new frame
    // or complete execution/name-link coverage receipt.
    if !analysis.matches_original_source_image(image, config) {
        return None;
    }
    let structure =
        crate::source_structure::SourceStructure::capture(source, Some(analysis), config)?;
    if structure.scripts.iter().any(|(_, depth)| {
        crate::references::MAX_DISPATCH_SCAN_DEPTH.exceeded(depth.saturating_add(1))
    }) {
        return None;
    }
    let mut bodies = structure
        .bodies
        .iter()
        .map(|span| (span.start() as usize, span.end() as usize))
        .collect::<Vec<_>>();
    bodies.extend(structure.scripts.iter().filter_map(|(span, depth)| {
        (*depth > 0).then_some((span.start() as usize, span.end() as usize))
    }));
    bodies.sort_unstable();
    bodies.dedup();
    let mut words = std::collections::HashMap::new();
    for (region, _) in &structure.scripts {
        let original = tcl_lexer::native_script_words_in(image.clone(), *region, config).ok()?;
        if original.fatal_tail.is_some() {
            return None;
        }
    }
    for command in &structure.commands {
        let original = tcl_lexer::native_script_words_in(
            image.clone(),
            command.execution_span(source),
            config,
        )
        .ok()?;
        if original.fatal_tail.is_some() || original.commands.len() != 1 {
            return None;
        }
        for word in original
            .commands
            .into_iter()
            .flat_map(|command| command.words)
        {
            if let Some(token) = word.tokens().first() {
                words.insert(token.span, word.clone());
            }
            words.insert(word.span(), word);
        }
    }
    Some(OriginalNamespaceCoverage {
        commands: structure.commands,
        words,
        bodies,
    })
}

fn lexical_roots_may_name_namespace(
    word: &tcl_lexer::NativeWord,
    symbol: &OriginalNamespaceSymbol,
) -> bool {
    let Some(wanted) = symbol.scope().context() else {
        return true;
    };
    let arena = word.executable_parts();
    arena.all_parts().any(|part| {
        let tcl_lexer::ExecutablePart::Variable { name, .. } = part.part else {
            return false;
        };
        let Some(raw) = arena.bytes(name) else {
            return true;
        };
        let Ok(bytes) = tcl_syntax::backslash::native_source_literal_bytes(
            raw,
            arena.image().channel(),
            symbol.policy().string_protocol(),
        ) else {
            return true;
        };
        if !bytes.starts_with(b"::") {
            return false;
        }
        let combined;
        let root = if matches!(
            part.part,
            tcl_lexer::ExecutablePart::Variable { index: Some(_), .. }
        ) {
            bytes.as_ref()
        } else {
            combined = symbol.policy().recipe().combined_variable_input(&bytes);
            combined.root().selected()
        };
        tcl_syntax::naming::native_variable_root_is_under_namespace(
            symbol.policy().recipe(),
            NativeNameContext::root(),
            root,
            wanted,
        ) != Some(false)
    })
}

fn word_may_contain_namespace(
    word: &tcl_lexer::NativeWord,
    symbol: &OriginalNamespaceSymbol,
    config: tcl_lexer::LexerConfig,
) -> bool {
    if lexical_roots_may_name_namespace(word, symbol) {
        return true;
    }
    if let Ok(words) = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
        std::slice::from_ref(word),
        symbol.policy().string_protocol(),
    ) {
        if let Some(value) = words.literal(0) {
            return literal_may_name_namespace(value, symbol, config);
        }
    }
    word.executable_parts()
        .all_parts()
        .any(|part| match &part.part {
            tcl_lexer::ExecutablePart::Text(_) => tcl_syntax::backslash::native_arena_text(
                word.executable_parts(),
                part,
                config.escapes,
                symbol.policy().string_protocol(),
            )
            .is_ok_and(|bytes| literal_may_name_namespace(&bytes, symbol, config)),
            tcl_lexer::ExecutablePart::Command { body } => word
                .image()
                .bytes()
                .get(body.as_range())
                .is_none_or(|bytes| literal_may_name_namespace(bytes, symbol, config)),
            _ => false,
        })
}

fn word_may_select_namespace(
    word: &tcl_lexer::NativeWord,
    symbol: &OriginalNamespaceSymbol,
    config: tcl_lexer::LexerConfig,
) -> bool {
    if let Ok(words) = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
        std::slice::from_ref(word),
        symbol.policy().string_protocol(),
    ) {
        if let Some(value) = words.literal(0) {
            let Some(wanted) = symbol.scope().context() else {
                return true;
            };
            if !value.starts_with(b"::") {
                return true;
            }
            return tcl_syntax::naming::native_written_namespace_prefix_extent(
                symbol.policy().recipe(),
                NativeNameContext::root(),
                value,
                wanted,
            )
            .is_some();
        }
    }
    let arena = word.executable_parts();
    let mut prefix = Vec::new();
    for part in arena.list(arena.root()) {
        if !matches!(part.part, tcl_lexer::ExecutablePart::Text(_)) {
            break;
        }
        let Ok(bytes) = tcl_syntax::backslash::native_arena_text(
            arena,
            part,
            config.escapes,
            symbol.policy().string_protocol(),
        ) else {
            return true;
        };
        prefix.extend_from_slice(&bytes);
    }
    symbol.scope().context().is_none_or(|wanted| {
        tcl_syntax::naming::native_namespace_literal_prefix_may_select(
            symbol.policy().recipe(),
            &prefix,
            wanted,
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn analyse(source: &str) -> AnalysisResult {
        Analyser::new().analyse(source, "tcl8.6").clone()
    }

    fn symbol(source: &str, analysis: &AnalysisResult, written: &str) -> OriginalNamespaceSymbol {
        let cursor = u32::try_from(source.find(written).unwrap()).unwrap();
        crate::namespace_symbol::original_namespace_at_offset(source, analysis, cursor).unwrap()
    }

    fn apply(
        source: &str,
        analysis: &AnalysisResult,
        symbol: &OriginalNamespaceSymbol,
        new_tail: &str,
    ) -> Result<String, RenameRefusal> {
        let dialect =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
        let edits = original_namespace_rename_edits(source, dialect, analysis, symbol, new_tail)?;
        let index = LineIndex::new(source);
        let mut edits = edits
            .into_iter()
            .map(|edit| {
                let start = crate::definition::byte_offset_at(
                    &index,
                    source,
                    edit.range.start_line,
                    edit.range.start_character,
                ) as usize;
                let end = crate::definition::byte_offset_at(
                    &index,
                    source,
                    edit.range.end_line,
                    edit.range.end_character,
                ) as usize;
                (start, end, edit.new_text)
            })
            .collect::<Vec<_>>();
        edits.sort_by_key(|(start, _, _)| std::cmp::Reverse(*start));
        let mut after = source.to_owned();
        for (start, end, text) in edits {
            after.replace_range(start..end, &text);
        }
        Ok(after)
    }

    #[test]
    fn original_namespace_coverage_reuses_analysis_body_sources_and_rejects_foreign_images() {
        // Implementation contract: naming.source.original-editor-body-structure
        // docs/design/analysis/name-resolution-proofs/original-editor-body-structure.md
        let source = "namespace eval ::old {}; proc p {} {if {1} {list [namespace exists ::old]}}";
        let analysis = analyse(source);
        let image = SourceImage::document(source);
        let config = analysis.body_lexer_config.unwrap();
        let coverage = original_coverage(source, &image, &analysis, config).unwrap();
        let offset = u32::try_from(source.find("namespace exists").unwrap()).unwrap();
        assert!(coverage.commands.iter().any(|command| {
            command
                .argv
                .first()
                .is_some_and(|token| token.span.start() == offset)
        }));
        assert!(coverage.word_at(Span::new(offset, offset + 9)).is_some());
        let changed = source.replace("::old", "::new");
        assert!(
            original_coverage(
                &changed,
                &SourceImage::document(&changed),
                &analysis,
                config
            )
            .is_none()
        );
        let mut unavailable = analysis.clone();
        unavailable.resolved_input = None;
        // Lexical command substitutions remain May geometry; no Registry body
        // role can be recaptured from a fresh standalone context.
        let unresolved = original_coverage(source, &image, &unavailable, config);
        assert!(
            unresolved.is_none()
                || unresolved.unwrap().commands.iter().all(|command| command
                    .argv
                    .first()
                    .is_none_or(|token| token.span.start() != offset))
        );
    }

    #[test]
    fn original_namespace_rename_keeps_opaque_siblings_distinct_with_ui_maps_cleared() {
        let source =
            r"namespace eval ::n\uD800 {}; namespace eval ::n\uD801 {}; namespace exists ::n\uD800";
        let mut analysis = analyse(source);
        let selected = symbol(source, &analysis, r"::n\uD800");
        analysis.all_procs.clear();
        analysis.all_classes.clear();
        let after = apply(source, &analysis, &selected, "new").unwrap();
        let result = analyse(&after);
        let values = result
            .namespace_refs
            .iter()
            .filter_map(|row| row.original_name_input.as_ref())
            .map(|input| input.bytes().to_vec())
            .collect::<Vec<_>>();
        assert_eq!(
            values,
            [
                b"::new".to_vec(),
                b"::n\xed\xa0\x81".to_vec(),
                b"::new".to_vec()
            ]
        );
        assert!(after.contains(r"::n\uD801"));
    }

    #[test]
    fn original_namespace_rename_batches_path_children_and_preserves_other_native_values() {
        let source = r"namespace eval ::old {}; namespace eval ::old::inner {}; namespace path {::old ::other\uD800 ::old::inner}";
        let analysis = analyse(source);
        let selected = symbol(source, &analysis, "::old");
        let after = apply(source, &analysis, &selected, "new").unwrap();
        let result = analyse(&after);
        let path = result
            .namespace_refs
            .iter()
            .filter_map(|row| row.original_name_input.as_ref())
            .filter(|input| {
                input
                    .original_static_list_container()
                    .is_some_and(|container| !container.ordinals().is_empty())
            })
            .map(|input| input.bytes().to_vec())
            .collect::<Vec<_>>();
        assert_eq!(
            path,
            [
                b"::new".to_vec(),
                b"::other\xed\xa0\x80".to_vec(),
                b"::new::inner".to_vec()
            ]
        );
    }

    #[test]
    fn original_namespace_rename_preserves_braced_and_array_variable_boundaries() {
        let source = "namespace eval ::old {}; set ::old::v 1; set ::old::a(k) 2; list ${::old::v} $::old::a(k)";
        let analysis = analyse(source);
        let selected = symbol(source, &analysis, "::old");
        let after = apply(source, &analysis, &selected, "new").unwrap();
        assert!(after.contains("${::new::v}"));
        assert!(after.contains("$::new::a(k)"));
        let result = analyse(&after);
        let roots = result
            .qualified_var_refs
            .iter()
            .filter_map(|row| row.original_name_input.as_ref())
            .filter_map(|input| match input {
                SignatureSourceNameInput::OriginalVariableRoot(root) => Some(root),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert!(roots.iter().any(|root| root.bytes() == b"::new::v"));
        assert!(
            roots
                .iter()
                .any(|root| root.bytes() == b"::new::a" && root.is_separate_array_root())
        );
    }

    #[test]
    fn original_namespace_rename_unknown_operands_refuse_only_unruled_out_prefixes() {
        for (operand, refuses) in [("$ns", true), ("::old::$ns", true), ("::other::$ns", false)] {
            let source = format!(
                "namespace eval ::old {{}}; proc mk {{ns}} {{namespace eval {operand} {{}}}}"
            );
            let analysis = analyse(&source);
            let selected = symbol(&source, &analysis, "::old");
            let result = apply(&source, &analysis, &selected, "new");
            assert_eq!(result.is_err(), refuses, "{operand}: {result:?}");
            if refuses {
                assert!(result.unwrap_err().reason.contains("computed at run time"));
            }
        }
    }

    #[test]
    fn original_namespace_rename_refuses_readonly_computed_and_stale_sources() {
        let source = "namespace eval ::old {}; set ns ::old; namespace exists $ns";
        let analysis = analyse(source);
        let selected = symbol(source, &analysis, "::old");
        assert!(apply(source, &analysis, &selected, "new").is_err());
        assert!(
            apply(
                &source.replace("::old", "::other"),
                &analysis,
                &selected,
                "new"
            )
            .is_err()
        );
    }

    #[test]
    fn original_namespace_rename_coverage_detects_escaped_names_in_unpositioned_data() {
        let source = r"namespace eval ::old {}; set dispatch {\u003A\u003Aold::tick}; set script {list ${::old::v}}";
        let analysis = analyse(source);
        let selected = symbol(source, &analysis, "::old");
        assert!(apply(source, &analysis, &selected, "new").is_err());
    }
}

#[cfg(test)]
mod namespace_tail_purpose_tests {
    #[test]
    fn original_namespace_rename_keeps_equal_global_command_and_variable_tails() {
        // naming.server.original-namespace-rename-plans
        // docs/design/analysis/name-resolution-proofs/server-original-namespace-rename-plans.md
        let source = "namespace eval ::old {}; proc ::old {} {}; set ::old 1; list $::old";
        let mut analyser = tcl_compiler::analyser::Analyser::new();
        let analysis = analyser.analyse(source, "tcl8.6");
        let cursor = u32::try_from(source.find("::old").unwrap()).unwrap();
        let symbol =
            crate::namespace_symbol::original_namespace_at_offset(source, &analysis, cursor)
                .unwrap();
        let dialect =
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile();
        let edits =
            super::original_namespace_rename_edits(source, dialect, &analysis, &symbol, "new")
                .unwrap();
        assert_eq!(
            edits.len(),
            1,
            "global command/variable simple names are separate from the namespace"
        );
        for environment in ["tcl9.1", "f5-irules"] {
            let foreign =
                tcl_registry::model::ingress::resolve_environment(environment).analyser_profile();
            assert!(
                super::original_namespace_rename_edits(source, foreign, &analysis, &symbol, "new")
                    .is_err()
            );
        }
        let index = tcl_lexer::LineIndex::new(source);
        let edit = &edits[0];
        let start = crate::definition::byte_offset_at(
            &index,
            source,
            edit.range.start_line,
            edit.range.start_character,
        ) as usize;
        let end = crate::definition::byte_offset_at(
            &index,
            source,
            edit.range.end_line,
            edit.range.end_character,
        ) as usize;
        let mut after = source.to_owned();
        after.replace_range(start..end, &edit.new_text);
        assert_eq!(
            after,
            "namespace eval ::new {}; proc ::old {} {}; set ::old 1; list $::old"
        );
    }
}

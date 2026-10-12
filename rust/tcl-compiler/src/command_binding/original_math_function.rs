// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original expression identifiers and their distinct function lookup purpose.

use super::declaration_layout::{DeclarationLayoutObservation, original_declaration_layouts};
use super::{
    Arc, CommandAllocationSite, SourceCommandBindings, SourceCommandReference,
    SourceInvocationBinding,
};
use tcl_lexer::{LexerConfig, SourceImage, Span};
use tcl_registry::mathfunc::NativeMathFunctionDispatch;

/// Readonly function identifier issued by the checked original expression.
/// This is neither a command-head word nor an expression evaluation, normal
/// result, function registration or native compiler capability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalMathFunctionOccurrence {
    source: Arc<super::SourceOriginId>,
    config: LexerConfig,
    policy: tcl_syntax::naming::NamePolicyProtocol,
    registry: tcl_registry::RegistrySemanticKey,
    expression: super::SourceConditionalExpressionEvaluation,
    ordinal: usize,
    span: Span,
    function: String,
    arity: usize,
    dispatch: NativeMathFunctionDispatch,
    reference: Option<SourceCommandReference>,
    registry_identity: Option<String>,
    original_lookup: Option<OriginalMathFunctionLookup>,
    fixed_lookup: Option<OriginalFixedMathFunctionLookup>,
}

/// Readonly lookup of a checked expression identifier in its original source
/// table. It retains no command-head word, actual function token, Normal result
/// or native body/compiler permission.
#[derive(Debug, Clone, PartialEq, Eq)]
struct OriginalMathFunctionLookup {
    name: tcl_registry::mathfunc::NativeExpressionFunctionCommandName,
    policy: tcl_syntax::naming::NamePolicyProtocol,
    observations: Vec<(Arc<super::SourceLookupSnapshot>, super::SourceNamespaceKey)>,
}

/// A fixed-table identifier retains its separate original snapshot purpose.
/// The counted ASCII name is owned by the checked original expression; no
/// implicit command name, mutable command-slot reference or runtime token exists.
#[derive(Debug, Clone, PartialEq, Eq)]
struct OriginalFixedMathFunctionLookup {
    function: tcl_core_types::NameBytes,
    dialect: tcl_registry::InvocationDialect,
    observations: Vec<Arc<super::SourceLookupSnapshot>>,
}

impl OriginalFixedMathFunctionLookup {
    fn diagnostic_presence(&self) -> super::SourceCommandSlotPresence {
        use super::SourceCommandSlotPresence as Presence;
        use tcl_runtime_api::native_compilation::NativeMathFunctionResolution as Resolution;
        let Ok(function) = std::str::from_utf8(self.function.as_bytes()) else {
            return Presence::Unknown;
        };
        let mut unanimous = None;
        for snapshot in &self.observations {
            let state = &snapshot.state;
            let current = if let Some(entry) = &state.baseline.native_entry {
                // A supplied actual entry is terminal. Its missing or unknown
                // table cannot borrow the separately authored fresh roster.
                match entry
                    .math_functions
                    .as_ref()
                    .map(|table| table.lookup(function))
                {
                    Some(Resolution::Present(_)) => Presence::Present,
                    Some(Resolution::Absent) => Presence::Absent,
                    Some(Resolution::Unknown) | None => Presence::Unknown,
                }
            } else {
                tcl_registry::mathfunc::fresh_fixed_function_presence(self.dialect, function)
                    .map_or(Presence::Unknown, |present| {
                        if present {
                            Presence::Present
                        } else {
                            Presence::Absent
                        }
                    })
            };
            if unanimous.is_some_and(|previous| previous != current) {
                return Presence::Unknown;
            }
            unanimous = Some(current);
        }
        unanimous.unwrap_or(Presence::Unknown)
    }
}

impl OriginalMathFunctionLookup {
    fn diagnostic_presence(&self) -> super::SourceCommandSlotPresence {
        let mut unanimous = None;
        for (snapshot, namespace) in &self.observations {
            let current = snapshot.state.original_function_diagnostic_presence(
                namespace,
                &self.name,
                self.policy,
            );
            if unanimous.is_some_and(|previous| previous != current) {
                return super::SourceCommandSlotPresence::Unknown;
            }
            unanimous = Some(current);
        }
        unanimous.unwrap_or(super::SourceCommandSlotPresence::Unknown)
    }
}

impl OriginalMathFunctionOccurrence {
    /// Exact source instance and input channel retaining this expression.
    #[must_use]
    pub fn source_image(&self) -> &SourceImage {
        self.source.source_image()
    }

    /// Complete lexer configuration retained with the original expression.
    #[must_use]
    pub const fn lexer_config(&self) -> LexerConfig {
        self.config
    }

    /// Independently selected original naming policy, including its authority.
    #[must_use]
    pub const fn name_policy(&self) -> tcl_syntax::naming::NamePolicyProtocol {
        self.policy
    }

    /// Original expression namespace identity; no current lookup is implied.
    #[must_use]
    pub fn namespace(&self) -> &super::SourceNamespaceKey {
        self.expression.namespace_context()
    }

    /// Conditional original expression frame, without entered-frame authority.
    #[must_use]
    pub fn frame(&self) -> &crate::var_resolve::VariableExecutionFrame {
        self.expression.frame()
    }

    /// Full independently selected expression parser context.
    #[must_use]
    pub fn expression_context(&self) -> &tcl_syntax::expr::parser::ExprParseContext {
        self.expression.parse_context()
    }

    /// Exact identifier extent in the original expression's source instance.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }

    /// Checked function ordinal within its own complete expression tree.
    #[must_use]
    pub const fn ordinal(&self) -> usize {
        self.ordinal
    }

    /// Original identifier bytes. The current projection admits ASCII native
    /// identifiers only; it never reencodes a display string as a native name.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        self.function.as_bytes()
    }

    /// Parser-retained presentation of the admitted original identifier.
    #[must_use]
    pub fn function(&self) -> &str {
        &self.function
    }

    /// Checked argument topology, independently of target arity or completion.
    #[must_use]
    pub const fn argument_count(&self) -> usize {
        self.arity
    }

    /// Independently selected fixed registration or Tcl command-table purpose.
    #[must_use]
    pub const fn dispatch(&self) -> NativeMathFunctionDispatch {
        self.dispatch
    }

    /// Unanimous command-slot navigation after independently closed operand
    /// effects. Fixed-table functions never acquire a command reference.
    #[must_use]
    pub const fn command_reference(&self) -> Option<&SourceCommandReference> {
        self.reference.as_ref()
    }

    /// Original source lookup advice after independently closed literal operands.
    /// Actual reached observations remain the diagnostic owner's first choice;
    /// this conditional value cannot supply runtime dispatch or a command word.
    #[must_use]
    pub fn diagnostic_presence(&self) -> super::SourceCommandSlotPresence {
        match self.dispatch {
            NativeMathFunctionDispatch::CommandTable => self.original_lookup.as_ref().map_or(
                super::SourceCommandSlotPresence::Unknown,
                OriginalMathFunctionLookup::diagnostic_presence,
            ),
            NativeMathFunctionDispatch::FixedTable => self.fixed_lookup.as_ref().map_or(
                super::SourceCommandSlotPresence::Unknown,
                OriginalFixedMathFunctionLookup::diagnostic_presence,
            ),
        }
    }

    /// Checked native function-name value retaining its original expression.
    /// This cannot be consumed as a written head or actual runtime operand.
    #[must_use]
    pub fn original_function_command_name(
        &self,
    ) -> Option<&tcl_registry::mathfunc::NativeExpressionFunctionCommandName> {
        Some(&self.original_lookup.as_ref()?.name)
    }

    /// Selected builtin metadata identity. For fixed tables this is either the
    /// authentic native row or the separately retained fresh authored roster.
    /// It supplies no result, numeric coercion or physical function token.
    #[must_use]
    pub fn registry_identity(&self) -> Option<&str> {
        self.registry_identity.as_deref()
    }

    /// Metadata for this independently selected function identity in its exact
    /// retained Registry. This does not reselect a target from the function's
    /// display or the consumer's profile.
    #[must_use]
    pub fn selected_registry_spec<'a>(
        &self,
        registry: &'a tcl_registry::CommandRegistry,
    ) -> Option<&'a tcl_registry::CommandSpec> {
        (self.registry == registry.snapshot().semantic_key()).then_some(())?;
        registry.selected_math_function_spec(self.registry_identity()?, self.dispatch)
    }

    /// Full original source, lexer and Registry correspondence. Equal display,
    /// offsets or grammar compatibility cannot reuse a function occurrence.
    #[must_use]
    pub fn matches_source(
        &self,
        image: &SourceImage,
        config: LexerConfig,
        registry: &tcl_registry::CommandRegistry,
    ) -> bool {
        self.source.source_image() == image
            && self.config == config
            && self.registry == registry.snapshot().semantic_key()
            && self.expression.source().origin == self.source
    }
}

impl SourceCommandBindings {
    /// Checked original math identifiers in a source region. Complete retained
    /// expression/word owners select syntax; reached lookup or closed original
    /// argument effects separately select navigation. No new script is walked.
    #[must_use]
    pub fn original_math_functions_in_source(
        &self,
        registry: &tcl_registry::CommandRegistry,
        image: &SourceImage,
        config: LexerConfig,
        region: Span,
    ) -> Vec<OriginalMathFunctionOccurrence> {
        if !self.matches_original_source_image(image, config) {
            return Vec::new();
        }
        let mut occurrences: Vec<OriginalMathFunctionOccurrence> = Vec::new();
        for (site, retained) in &self.declaration_layouts {
            if site.offset >= region.end() || site.source.source_image() != image {
                continue;
            }
            let Some(rows) = original_declaration_layouts(retained) else {
                continue;
            };
            let rows: Vec<_> = rows.collect();
            let Some(first) = rows.first() else {
                continue;
            };
            if first
                .words
                .last()
                .is_none_or(|word| word.source().span.end() <= region.start())
            {
                continue;
            }
            if rows.iter().any(|row| {
                row.config != config
                    || row.snapshot.state.baseline.registry_snapshot.as_ref()
                        != Some(&registry.snapshot().semantic_key())
            }) {
                continue;
            }
            let Some(mut tokens) = super::declaration_preview::declaration_tokens(site, first)
            else {
                continue;
            };
            self.stamp_original_tokens(&mut tokens);
            for (written, word) in first.words.iter().enumerate().skip(1) {
                if word.source().span.end() <= region.start()
                    || word.source().span.start() >= region.end()
                {
                    continue;
                }
                let Some(expression) = tokens.source_binding.as_ref().and_then(|binding| {
                    binding.conditional_expression_evaluation_for_original_word(
                        registry, &tokens, written,
                    )
                }) else {
                    continue;
                };
                if expression.source().origin != site.source {
                    continue;
                }
                for occurrence in self.original_expression_function_occurrences(
                    registry,
                    site,
                    &expression,
                    &rows,
                    region,
                    config,
                ) {
                    if let Some(previous) = occurrences
                        .iter_mut()
                        .find(|previous| previous.span == occurrence.span)
                    {
                        if previous != &occurrence {
                            previous.reference = None;
                            previous.registry_identity = None;
                            previous.original_lookup = None;
                            previous.fixed_lookup = None;
                        }
                    } else {
                        occurrences.push(occurrence);
                    }
                }
            }
        }
        occurrences.sort_by_key(|occurrence| (occurrence.span.start(), occurrence.span.end()));
        occurrences
    }
    fn original_expression_function_occurrences(
        &self,
        registry: &tcl_registry::CommandRegistry,
        site: &CommandAllocationSite,
        expression: &super::SourceConditionalExpressionEvaluation,
        rows: &[&DeclarationLayoutObservation],
        region: Span,
        config: LexerConfig,
    ) -> Vec<OriginalMathFunctionOccurrence> {
        let image = site.source.source_image();
        let mut occurrences = Vec::new();
        for (ordinal, (function, start, arity)) in
            expression.tree().function_calls().into_iter().enumerate()
        {
            let Some(start) = expression.source().base().checked_add(start) else {
                continue;
            };
            let Some(end) = u32::try_from(function.len())
                .ok()
                .and_then(|length| start.checked_add(length))
            else {
                continue;
            };
            if end <= region.start()
                || start >= region.end()
                || !function.is_ascii()
                || image.bytes().get(Span::new(start, end).as_range()) != Some(function.as_bytes())
            {
                continue;
            }
            let Some(dialect) = selected_function_dialect(expression, rows) else {
                continue;
            };
            let Some(policy) = selected_function_name_policy(rows, dialect) else {
                continue;
            };
            let Some(dispatch) = tcl_registry::mathfunc::native_function_dispatch(dialect) else {
                continue;
            };
            let function_site = CommandAllocationSite {
                source: Arc::clone(&site.source),
                offset: start,
            };
            let (reference, registry_identity) = match dispatch {
                NativeMathFunctionDispatch::FixedTable => (
                    None,
                    fixed_metadata(self, &function_site, function, dialect, rows),
                ),
                NativeMathFunctionDispatch::CommandTable => {
                    command_metadata(self, &function_site, function, ordinal, expression, rows)
                }
            };
            let original_lookup =
                original_function_lookup(self, &function_site, ordinal, expression, rows, dialect);
            let fixed_lookup = original_fixed_function_lookup(
                self,
                &function_site,
                function,
                ordinal,
                expression,
                rows,
                dialect,
            );
            let occurrence = OriginalMathFunctionOccurrence {
                source: Arc::clone(&site.source),
                config,
                policy,
                registry: registry.snapshot().semantic_key(),
                expression: expression.clone(),
                ordinal,
                span: Span::new(start, end),
                function: function.to_owned(),
                arity,
                dispatch,
                reference,
                registry_identity,
                original_lookup,
                fixed_lookup,
            };
            occurrences.push(occurrence);
        }
        occurrences
    }
}

fn selected_function_name_policy(
    rows: &[&DeclarationLayoutObservation],
    dialect: tcl_registry::InvocationDialect,
) -> Option<tcl_syntax::naming::NamePolicyProtocol> {
    let policy = rows
        .first()?
        .snapshot
        .state
        .baseline
        .execution_name_policy?
        .native_recipe()?;
    if dialect.native_name_protocol()? != policy.recipe() {
        return None;
    }
    rows.iter()
        .all(|row| {
            row.snapshot
                .state
                .baseline
                .execution_name_policy
                .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
                == Some(policy)
        })
        .then_some(policy)
}

fn selected_function_dialect(
    expression: &super::SourceConditionalExpressionEvaluation,
    rows: &[&DeclarationLayoutObservation],
) -> Option<tcl_registry::InvocationDialect> {
    let first = rows.first()?;
    let dialect = first.snapshot.state.baseline.dialect?;
    for row in rows {
        let current = row.snapshot.state.baseline.dialect?;
        if !current.has_same_execution_policy(dialect)
            || row.namespace != *expression.namespace_context()
            || row.snapshot.state.variable_frame != *expression.frame()
        {
            return None;
        }
        if let Some(entry) = &row.snapshot.state.baseline.native_entry
            && tcl_registry::native_expression_program::compilation_expression_function_dispatch(
                entry,
            ) != tcl_registry::mathfunc::native_function_dispatch(dialect)
        {
            return None;
        }
    }
    (dialect.expression_parse_context(None).native_syntax
        == expression.parse_context().native_syntax)
        .then_some(dialect)
}

fn fixed_metadata(
    bindings: &SourceCommandBindings,
    site: &CommandAllocationSite,
    function: &str,
    dialect: tcl_registry::InvocationDialect,
    rows: &[&DeclarationLayoutObservation],
) -> Option<String> {
    use tcl_runtime_api::native_compilation::NativeMathFunctionResolution;
    if let Some(observations) = bindings.implicit_fixed_math_invocations.get(&(
        Arc::clone(&site.source),
        site.offset,
        function.to_owned(),
    )) {
        let mut actual = observations
            .iter()
            .filter(|observation| !observation.declaration_preview);
        if let Some(first) = actual.next() {
            if !actual.all(|other| other == first) {
                return None;
            }
            if let Some(proof) = &first.prerequisite {
                let NativeMathFunctionResolution::Present(row) = proof.table.lookup(function)
                else {
                    return None;
                };
                return row.registry_identity.clone();
            }
        }
    }
    rows.iter()
        .all(|row| {
            row.snapshot.state.baseline.native_entry.is_none()
                && !row.snapshot.state.baseline.unknown_entry
                && !row.snapshot.state.has_opaque_domain()
        })
        .then_some(())?;
    (tcl_registry::mathfunc::fresh_fixed_function_presence(dialect, function) == Some(true))
        .then(|| function.to_owned())
}

fn command_metadata(
    bindings: &SourceCommandBindings,
    site: &CommandAllocationSite,
    function: &str,
    ordinal: usize,
    expression: &super::SourceConditionalExpressionEvaluation,
    rows: &[&DeclarationLayoutObservation],
) -> (Option<SourceCommandReference>, Option<String>) {
    let relative = tcl_registry::mathfunc::qualified_name(function)
        .trim_start_matches("::")
        .to_owned();
    if bindings.implicit_math_invocations.contains_key(&(
        Arc::clone(&site.source),
        site.offset,
        function.to_owned(),
    )) {
        let binding = bindings.implicit_math_invocation_at(&site.source, site.offset, function);
        if binding.runtime_reachability() != super::SourceRuntimeReachability::Conditional {
            return binding_metadata(&binding, &relative);
        }
    }
    let crate::expr_ast::ExprNode::Call {
        args: arguments, ..
    } = expression.tree()
    else {
        return (None, None);
    };
    if ordinal != 0 {
        return (None, None);
    }
    let mut unanimous = None;
    for row in rows {
        if row.snapshot.state.has_opaque_domain()
            || row.snapshot.state.source_step_observed()
            || !row
                .snapshot
                .state
                .ordinary_literal_pool
                .as_ref()
                .is_some_and(super::literal_object_pool::SourceOrdinaryLiteralPool::effects_current)
            || !arguments.iter().all(|argument| {
                constant_argument(argument, expression.parse_context().lexer_grammar.numbers)
            })
        {
            return (None, None);
        }
        let binding = super::source_binding(&row.snapshot.state, &relative, &row.namespace);
        let current = binding_metadata(&binding, &relative);
        if unanimous
            .as_ref()
            .is_some_and(|previous| previous != &current)
        {
            return (None, None);
        }
        unanimous = Some(current);
    }
    unanimous.unwrap_or((None, None))
}

fn original_fixed_function_lookup(
    bindings: &SourceCommandBindings,
    site: &CommandAllocationSite,
    function: &str,
    ordinal: usize,
    expression: &super::SourceConditionalExpressionEvaluation,
    rows: &[&DeclarationLayoutObservation],
    dialect: tcl_registry::InvocationDialect,
) -> Option<OriginalFixedMathFunctionLookup> {
    // naming.expression.original-fixed-function-source-presence
    // docs/design/analysis/name-resolution-proofs/original-fixed-function-source-presence.md
    if tcl_registry::mathfunc::native_function_dispatch(dialect)
        != Some(NativeMathFunctionDispatch::FixedTable)
        || ordinal != 0
        || !expression.semantic_lookup_closed()
    {
        return None;
    }
    let policy = selected_function_name_policy(rows, dialect)?;
    let crate::expr_ast::ExprNode::Call {
        function: original_function,
        args,
        ..
    } = expression.tree()
    else {
        return None;
    };
    if original_function != function
        || !function.is_ascii()
        || !args.iter().all(|argument| {
            constant_argument(argument, expression.parse_context().lexer_grammar.numbers)
        })
    {
        return None;
    }
    let mut observations = Vec::new();
    for row in rows {
        if row.snapshot.state.baseline.unknown_entry
            || !original_fixed_function_snapshot_is_closed(&row.snapshot, policy)
        {
            return None;
        }
        observations.push(Arc::clone(&row.snapshot));
    }
    if observations.is_empty() {
        return None;
    }
    let lookup = OriginalFixedMathFunctionLookup {
        function: tcl_core_types::NameBytes::from(function.as_bytes()),
        dialect,
        observations,
    };
    if let Some(reached) = bindings.fixed_math_diagnostic_presence.get(&(
        Arc::clone(&site.source),
        site.offset,
        function.to_owned(),
    )) && (*reached == super::SourceCommandSlotPresence::Unknown
        || *reached != lookup.diagnostic_presence())
    {
        return None;
    }
    Some(lookup)
}

fn original_function_lookup(
    bindings: &SourceCommandBindings,
    site: &CommandAllocationSite,
    ordinal: usize,
    expression: &super::SourceConditionalExpressionEvaluation,
    rows: &[&DeclarationLayoutObservation],
    dialect: tcl_registry::InvocationDialect,
) -> Option<OriginalMathFunctionLookup> {
    let policy = selected_function_name_policy(rows, dialect)?;
    let source = expression.source().try_text().ok()?;
    let checked = tcl_registry::conditional_expression::ConditionalExpressionEvaluation::prepare(
        source,
        expression.parse_context(),
    )?;
    if checked.tree() != expression.tree() {
        return None;
    }
    let name = checked.original_function_command_name(source, ordinal, dialect)?;
    let crate::expr_ast::ExprNode::Call { args, .. } = expression.tree() else {
        return None;
    };
    if ordinal != 0
        || !args
            .iter()
            .all(|arg| constant_argument(arg, expression.parse_context().lexer_grammar.numbers))
    {
        return None;
    }
    // Reached observations own the post-operand point. Retain that exact
    // snapshot for a proved lookup; an actual unknown cannot borrow the
    // independently closed declaration preview.
    if let Some(reached) = bindings.implicit_math_invocations.get(&(
        Arc::clone(&site.source),
        site.offset,
        std::str::from_utf8(name.function_bytes()).ok()?.to_owned(),
    )) && reached.iter().any(|binding| {
        binding.runtime_reachability() != super::SourceRuntimeReachability::Conditional
    }) {
        let mut observations = Vec::new();
        for binding in reached.iter().filter(|binding| {
            binding.runtime_reachability() != super::SourceRuntimeReachability::Conditional
        }) {
            // An implicit function has no original command-head word. Join
            // its retained lookup spelling to this checked name producer and
            // query the actual snapshot through the function-name purpose.
            if binding.lookup_word.as_deref().map(str::as_bytes) != Some(name.command_bytes()) {
                return None;
            }
            let snapshot = binding.lookup_state.as_ref()?;
            if !original_function_snapshot_is_closed(snapshot, policy)
                || snapshot.state.original_function_diagnostic_presence(
                    &binding.lookup_namespace_key,
                    &name,
                    policy,
                ) == super::SourceCommandSlotPresence::Unknown
            {
                return None;
            }
            observations.push((Arc::clone(snapshot), binding.lookup_namespace_key.clone()));
        }
        return (!observations.is_empty()).then_some(OriginalMathFunctionLookup {
            name,
            policy,
            observations,
        });
    }
    let mut observations = Vec::new();
    for row in rows {
        if !original_function_snapshot_is_closed(&row.snapshot, policy) {
            return None;
        }
        observations.push((Arc::clone(&row.snapshot), row.namespace.clone()));
    }
    (!observations.is_empty()).then_some(OriginalMathFunctionLookup {
        name,
        policy,
        observations,
    })
}

fn original_function_snapshot_has_quiet_context(
    snapshot: &super::SourceLookupSnapshot,
    policy: tcl_syntax::naming::NamePolicyProtocol,
) -> bool {
    let state = &snapshot.state;
    state
        .baseline
        .execution_name_policy
        .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
        == Some(policy)
        && !state.has_opaque_domain()
        && !state.source_step_observed()
        && state.command_observers.is_quiet()
        && super::original_compiler_effects::lookup_has_no_callbacks(state)
}

fn original_function_snapshot_is_closed(
    snapshot: &super::SourceLookupSnapshot,
    policy: tcl_syntax::naming::NamePolicyProtocol,
) -> bool {
    original_function_snapshot_has_quiet_context(snapshot, policy)
        && snapshot
            .state
            .ordinary_literal_pool
            .as_ref()
            .is_some_and(super::literal_object_pool::SourceOrdinaryLiteralPool::effects_current)
}

fn original_fixed_function_snapshot_is_closed(
    snapshot: &super::SourceLookupSnapshot,
    policy: tcl_syntax::naming::NamePolicyProtocol,
) -> bool {
    // The checked fixed-table identifier reads original parser text. Jim's
    // text provenance grants neither C pooled objects nor commandName effects.
    original_function_snapshot_has_quiet_context(snapshot, policy)
        && snapshot.state.ordinary_literal_pool.as_ref().is_some_and(
            super::literal_object_pool::SourceOrdinaryLiteralPool::text_effects_current,
        )
}

fn binding_metadata(
    binding: &SourceInvocationBinding,
    relative: &str,
) -> (Option<SourceCommandReference>, Option<String>) {
    let Some(state) = binding
        .lookup_state
        .as_ref()
        .map(|snapshot| &snapshot.state)
    else {
        return (None, None);
    };
    if state.has_opaque_domain() || state.source_step_observed() {
        return (None, None);
    }
    let Some(target) = binding.proved_target() else {
        return (None, None);
    };
    if state.source_execution_observed(target.identity.as_ref()) {
        return (None, None);
    }
    let reference = binding.command_reference(relative);
    let registry =
        (target.registry_backed && target.prepended.is_empty()).then(|| target.command.clone());
    (reference, registry)
}

fn constant_argument(tree: &crate::expr_ast::ExprNode, syntax: tcl_dialect::NumberSyntax) -> bool {
    use crate::expr_ast::ExprNode;
    let mut pending = vec![tree];
    while let Some(node) = pending.pop() {
        match node {
            ExprNode::Literal { text, .. } => {
                if tcl_syntax::number::parse_whole_with(
                    text,
                    tcl_syntax::number::ParseFlags::for_syntax(syntax),
                )
                .is_none()
                {
                    return false;
                }
            }
            ExprNode::Unary { operand, .. } => pending.push(operand),
            ExprNode::Binary { left, right, .. } => {
                pending.push(left);
                pending.push(right);
            }
            ExprNode::Ternary {
                condition,
                true_branch,
                false_branch,
                ..
            } => {
                pending.extend([
                    condition.as_ref(),
                    true_branch.as_ref(),
                    false_branch.as_ref(),
                ]);
            }
            _ => return false,
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn functions(
        source: &str,
        name: &str,
    ) -> (
        SourceCommandBindings,
        tcl_registry::CommandRegistry,
        SourceImage,
        LexerConfig,
        Vec<OriginalMathFunctionOccurrence>,
    ) {
        let point = tcl_dialect::model::DialectPoint::of_dialect_name(Some(name)).unwrap();
        let dialect = tcl_registry::InvocationDialect::of_point(point);
        let profile = tcl_registry::model::ingress::resolve_environment(name).analyser_profile();
        let registry = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        let config = LexerConfig::from_grammar(dialect.lexer_grammar);
        let image = SourceImage::document(source);
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            config,
            &registry,
            super::super::SourceAnalysisOptions {
                invocation_dialect: Some(dialect),
                ..Default::default()
            },
        );
        let result = bindings.original_math_functions_in_source(
            &registry,
            &image,
            config,
            Span::new(0, u32::try_from(source.len()).unwrap()),
        );
        (bindings, registry, image, config, result)
    }

    #[test]
    fn original_fixed_function_absence_retains_source_snapshot_without_command_name() {
        // naming.expression.original-fixed-function-source-presence
        // docs/design/analysis/name-resolution-proofs/original-fixed-function-source-presence.md
        // Related observed engine purpose: naming.expression.original-implicit-function-slot-presence
        // docs/design/analysis/name-resolution-proofs/original-implicit-function-slot-presence.md
        // Jim Pi() is a native syntax error, not a command-table call. This
        // source-only absence projection claims no executed operand topology.
        use super::super::SourceCommandSlotPresence as Presence;
        for environment in ["tcl8.4", "jim"] {
            let (mut bindings, registry, image, config, calls) =
                functions("expr {Pi()}", environment);
            let [call] = calls.as_slice() else {
                panic!("{environment}: original checked occurrence")
            };
            assert_eq!(call.dispatch(), NativeMathFunctionDispatch::FixedTable);
            assert_eq!(call.bytes(), b"Pi");
            assert_eq!(call.diagnostic_presence(), Presence::Absent);
            assert!(call.original_function_command_name().is_none());
            assert!(call.command_reference().is_none());
            assert!(call.registry_identity().is_none());
            assert!(
                bindings
                    .original_math_functions_in_source(
                        &registry,
                        &SourceImage::document("expr {Pi()} "),
                        config,
                        Span::new(6, 8),
                    )
                    .is_empty()
            );
            let root = Arc::clone(bindings.source_origin().unwrap());
            bindings
                .fixed_math_diagnostic_presence
                .insert((root, 6, "Pi".to_owned()), Presence::Unknown);
            let calls = bindings.original_math_functions_in_source(
                &registry,
                &image,
                config,
                Span::new(6, 8),
            );
            assert_eq!(calls[0].diagnostic_presence(), Presence::Unknown);
            assert_eq!(
                bindings.diagnostic_math_function_presence_at(&registry, "Pi", 6),
                Presence::Unknown
            );
            for source in [
                "expr {Pi([unknown])}",
                "unknownfuture; expr {Pi()}",
                "proc expr args {return ignored}; expr {Pi()}",
            ] {
                let (_, _, _, _, calls) = functions(source, environment);
                assert!(
                    calls
                        .iter()
                        .all(|call| call.diagnostic_presence() == Presence::Unknown),
                    "{environment}: {source}"
                );
            }
            let (_, _, _, _, calls) = functions("expr {abs(-3)}", environment);
            assert_eq!(calls[0].diagnostic_presence(), Presence::Present);
            assert!(calls[0].original_function_command_name().is_none());
        }
    }

    #[test]
    fn original_function_lookup_keeps_absence_and_unknown_separate() {
        // naming.expression.original-function-navigation
        // docs/design/analysis/name-resolution-proofs/original-function-navigation.md
        // Native dispatch control: naming.expression.original-implicit-function-slot-presence
        // docs/design/analysis/name-resolution-proofs/original-implicit-function-slot-presence.md
        use super::super::SourceCommandSlotPresence as Presence;
        for name in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let (mut bindings, registry, image, config, calls) = functions("expr {Pi()}", name);
            let [call] = calls.as_slice() else {
                panic!("{name}: original function");
            };
            let lookup = call
                .original_function_command_name()
                .expect("sealed function-name value");
            assert_eq!(lookup.function_bytes(), b"Pi");
            assert_eq!(lookup.source(), "Pi()");
            assert_eq!(lookup.command_bytes(), b"tcl::mathfunc::Pi");
            assert_eq!(call.diagnostic_presence(), Presence::Absent);
            assert!(call.command_reference().is_none());
            assert!(call.registry_identity().is_none());
            assert_eq!(
                bindings.diagnostic_math_function_presence_at(&registry, "Pi", 6),
                Presence::Absent
            );
            let root = Arc::clone(bindings.source_origin().unwrap());
            bindings.implicit_math_invocations.insert(
                (root, 6, "Pi".to_owned()),
                vec![SourceInvocationBinding::unknown()],
            );
            assert_eq!(
                bindings.diagnostic_math_function_presence_at(&registry, "Pi", 6),
                Presence::Unknown
            );
            let calls = bindings.original_math_functions_in_source(
                &registry,
                &image,
                config,
                Span::new(6, 8),
            );
            assert!(calls[0].original_function_command_name().is_none());
            for source in [
                "expr {Pi([unknown])}",
                "unknownfuture; expr {Pi()}",
                "rename expr old; proc expr args {return ignored}; expr {Pi()}",
            ] {
                let (_, _, _, _, calls) = functions(source, name);
                assert!(
                    calls
                        .iter()
                        .all(|call| call.original_function_command_name().is_none()),
                    "{name}: {source}"
                );
            }
        }
    }

    #[test]
    fn original_math_policy_requires_the_selected_execution_name_recipe() {
        // naming.diagnostic.original-math-function-subject
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-math-function-subject.md
        let (bindings, _, _, _, calls) = functions("expr {abs(1)}", "tcl8.6");
        let rows: Vec<_> = bindings
            .declaration_layouts
            .values()
            .flat_map(|retained| original_declaration_layouts(retained).into_iter().flatten())
            .collect();
        let dialect = tcl_registry::InvocationDialect::of_point(
            tcl_dialect::model::DialectPoint::of_dialect_name(Some("tcl8.6")).unwrap(),
        );
        assert_eq!(
            selected_function_name_policy(&rows, dialect),
            Some(calls[0].name_policy())
        );
        for foreign in ["tcl8.4", "tcl9.1", "jim"] {
            let dialect = tcl_registry::InvocationDialect::of_point(
                tcl_dialect::model::DialectPoint::of_dialect_name(Some(foreign)).unwrap(),
            );
            assert!(selected_function_name_policy(&rows, dialect).is_none());
        }
        let mut mismatched = dialect;
        mismatched.tcl_version = Some(tcl_dialect::TclVersion::V9_1);
        assert!(selected_function_name_policy(&rows, mismatched).is_none());
    }

    #[test]
    fn original_math_function_purposes_separate_fixed_tables_and_command_slots() {
        // Implementation contract: naming.expression.original-function-navigation
        // docs/design/analysis/name-resolution-proofs/original-function-navigation.md
        for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let (_, registry, image, config, calls) = functions("expr {abs(-3)}", name);
            let [call] = calls.as_slice() else {
                panic!("{name}: checked original function");
            };
            assert_eq!(call.bytes(), b"abs");
            assert_eq!(call.span(), Span::new(6, 9));
            assert_eq!(call.ordinal(), 0);
            assert_eq!(call.argument_count(), 1);
            assert!(call.matches_source(&image, config, &registry));
            assert!(call.selected_registry_spec(&registry).is_some());
            if matches!(name, "tcl8.4" | "jim") {
                assert_eq!(call.dispatch(), NativeMathFunctionDispatch::FixedTable);
                assert!(call.command_reference().is_none());
                assert_eq!(call.registry_identity(), Some("abs"));
            } else {
                assert_eq!(call.dispatch(), NativeMathFunctionDispatch::CommandTable);
                assert!(call.command_reference().is_some());
                assert_eq!(call.registry_identity(), Some("::tcl::mathfunc::abs"));
            }
        }
    }

    #[test]
    fn original_math_navigation_retains_installed_allocation_and_operand_barriers() {
        // Implementation contract: naming.expression.original-function-navigation
        // docs/design/analysis/name-resolution-proofs/original-function-navigation.md
        for name in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let source = "proc ::tcl::mathfunc::custom {x} {return $x}; expr {custom(3)}";
            let (_, _, _, _, calls) = functions(source, name);
            let [call] = calls.as_slice() else {
                panic!("{name}: installed custom function");
            };
            let reference = call
                .command_reference()
                .expect("actual current function slot");
            let definition = reference
                .definition()
                .expect("installed function procedure");
            assert_eq!(
                definition.kind(),
                super::super::SourceCommandDefinitionKind::Procedure
            );
            assert_eq!(definition.allocation().site.offset, 0);
            assert!(call.registry_identity().is_none());
            let (_, _, _, _, calls) = functions("expr {abs([unknown])}", name);
            assert_eq!(calls.len(), 1);
            assert!(calls[0].command_reference().is_none());
            assert!(calls[0].registry_identity().is_none());
        }
    }

    #[test]
    fn original_math_topology_is_exact_without_fabricating_head_words() {
        // Implementation contract: naming.expression.original-function-navigation
        // docs/design/analysis/name-resolution-proofs/original-function-navigation.md
        let (bindings, registry, image, config, calls) =
            functions("expr {max(abs(1), abs(2))}", "tcl8.6");
        assert_eq!(
            calls
                .iter()
                .map(|call| (call.ordinal(), call.bytes(), call.span()))
                .collect::<Vec<_>>(),
            vec![
                (0, b"max".as_slice(), Span::new(6, 9)),
                (1, b"abs".as_slice(), Span::new(10, 13)),
                (2, b"abs".as_slice(), Span::new(18, 21)),
            ]
        );
        let foreign = SourceImage::native(image.bytes());
        assert!(
            bindings
                .original_math_functions_in_source(&registry, &foreign, config, Span::new(0, 25))
                .is_empty()
        );
        let changed = LexerConfig {
            strict_quoting: !config.strict_quoting,
            ..config
        };
        assert!(
            bindings
                .original_math_functions_in_source(&registry, &image, changed, Span::new(0, 25))
                .is_empty()
        );
        let wrong_registry =
            registry.project_for_profile(tcl_dialect::DialectProfile::find("tcl9.1").unwrap());
        assert!(
            bindings
                .original_math_functions_in_source(
                    &wrong_registry,
                    &image,
                    config,
                    Span::new(0, 25)
                )
                .is_empty()
        );
        assert!(calls[0].selected_registry_spec(&wrong_registry).is_none());
        assert!(functions("expr {abs([bad}", "tcl8.6").4.is_empty());
        for source in [
            "if {abs(-3)} {set x 1}",
            "while {abs(-3)} {break}",
            "for {} {abs(-3)} {} {break}",
        ] {
            let (_, _, _, _, calls) = functions(source, "tcl8.6");
            assert_eq!(
                calls.len(),
                1,
                "{source}: selected original expression role"
            );
            assert_eq!(calls[0].bytes(), b"abs");
        }
    }
}

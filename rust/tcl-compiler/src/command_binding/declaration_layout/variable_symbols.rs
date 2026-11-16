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

//! The namespace, local, global, alias or formal an original variable span resolves to.

use super::super::{ModuleCommandBindings, SourceCommandBindings};
use super::{
    DeclarationLayoutObservation, OriginalDiagnosticFrameEntry, SourceOriginalVariableFrame,
    original_declaration_layouts,
};

fn original_alias_place_matches(
    row: &DeclarationLayoutObservation,
    input: &crate::signature_scan::scope::SignatureSourceNameInput,
    alias: &crate::signature_scan::variable_symbol::OriginalVariableAliasReceipt,
    place: &crate::place::Place,
) -> bool {
    use crate::signature_scan::variable_symbol::SignatureSourceVariableSlot;
    let Some(cell) = place.cell.as_ref() else {
        return false;
    };
    let crate::place::CellOwner::NamespaceIdentity(owner) = &cell.owner else {
        return false;
    };
    if place.dynamic || place.kind == crate::place::PlaceKind::Unknown {
        return false;
    }

    match alias.target().slot() {
        SignatureSourceVariableSlot::C { namespace, simple } => {
            &cell.name == simple
                && row
                    .snapshot
                    .state
                    .original_namespace_geometry(owner, input.policy())
                    == Some(crate::signature_scan::scope::SignatureNamespaceScope::C(
                        namespace.clone(),
                    ))
        }
        SignatureSourceVariableSlot::Jim(key) => &cell.name == key,
        SignatureSourceVariableSlot::Local { .. } => false,
    }
}

pub(super) fn original_local_primary_is_byte_unique(
    state: &ModuleCommandBindings,
    policy: tcl_syntax::naming::NamePolicyProtocol,
    name: &[u8],
) -> bool {
    let options = super::super::SourceAnalysisOptions {
        native_entry: state.baseline.native_entry.as_deref(),
        invocation_dialect: state.baseline.dialect,
        compiled_variable_provider: state.baseline.compiled_variable_provider,
        ..Default::default()
    };
    if options.execution_name_policy()
        != Some(tcl_syntax::naming::ExecutionNamePolicy::NativeRecipe(
            policy,
        ))
    {
        return false;
    }
    if policy.recipe().is_jim084() {
        // Jim installs formals through its selected general setter. It has no
        // C compiled-local primary to donate; the caller still checks the
        // exact direct scalar installation and any current alias separately.
        return true;
    }
    options
        .compiled_variable_protocol()
        .is_some_and(|compiler| {
            compiler.compiled_local_name_is_byte_unique(name)
                && state
                    .source_variables
                    .original_formal_topology
                    .as_ref()
                    .and_then(|topology| topology.first_compiled_formal_name(name, compiler, false))
                    .is_none_or(|primary| primary == name)
        })
}

fn original_alias_template_input_is_current(
    row: &DeclarationLayoutObservation,
    input: &crate::signature_scan::scope::SignatureSourceNameInput,
) -> bool {
    let Some(owner) = row.snapshot.state.logical_source_name_advice_input() else {
        return input.is_current(&row.snapshot.state.source_variables);
    };
    if !owner.has_logical_source_name_context()
        || owner.lexer_config() != row.config
        || row.snapshot.state.baseline.native_entry.is_some()
        || super::super::source_analysis_entry::source_input_dialect(owner).authored_name_policy()
            != Some(input.policy())
    {
        return false;
    }
    let image = row.entry.source().origin.source_image();
    match input {
        crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord(word) => {
            word.source_image() == image && word.lexer_config() == row.config
        }
        crate::signature_scan::scope::SignatureSourceNameInput::OriginalVariableRoot(root) => {
            root.source_image() == image && root.lexer_config() == row.config
        }
        crate::signature_scan::scope::SignatureSourceNameInput::OriginalValue(_)
        | crate::signature_scan::scope::SignatureSourceNameInput::OriginalLogicalListField(_) => {
            false
        }
    }
}

fn original_alias_template_local_is_byte_unique(
    row: &DeclarationLayoutObservation,
    policy: tcl_syntax::naming::NamePolicyProtocol,
    name: &[u8],
) -> bool {
    let state = &row.snapshot.state;
    if let Some(input) = state.logical_source_name_advice_input() {
        if !input.has_logical_source_name_context()
            || input.lexer_config() != row.config
            || state.baseline.native_entry.is_some()
            || state.variable_frame != *row.entry.frame()
            || !row.entry.owns_original_context(&state.source_variables)
        {
            return false;
        }
        return match row.entry.as_ref() {
            OriginalDiagnosticFrameEntry::DeclaredProcedure(body) => {
                body.logical_alias_local_name_is_byte_unique(policy, row.config)
            }
            _ => false,
        };
    }
    original_local_primary_is_byte_unique(state, policy, name)
}

fn original_alias_template_rows_match<'a>(
    rows: impl Iterator<Item = &'a DeclarationLayoutObservation>,
    input: &crate::signature_scan::scope::SignatureSourceNameInput,
    receiver: crate::signature_scan::variable_symbol::OriginalVariableSymbolReceiver,
    alias: &crate::signature_scan::variable_symbol::OriginalVariableAliasReceipt,
    requires_qualified_target: impl Fn() -> bool,
    absolute_root: impl Fn() -> bool,
) -> Option<bool> {
    let mut found = false;
    for row in rows {
        if !original_alias_template_input_is_current(row, input)
            || !original_alias_template_local_is_byte_unique(
                row,
                input.policy(),
                alias.local().as_bytes(),
            )
        {
            return None;
        }
        if requires_qualified_target() {
            let namespace = if absolute_root() {
                Some(crate::signature_scan::scope::SignatureNamespaceScope::root(
                    Some(input.policy()),
                ))
            } else {
                row.snapshot
                    .state
                    .original_namespace_geometry(&row.namespace, input.policy())
            }?;
            let target = crate::signature_scan::variable_symbol::SignatureSourceVariableSymbol::from_original_input(
                input,
                &namespace,
                crate::signature_scan::variable_symbol::OriginalVariableSymbolPurpose::QualifiedOnly,
                receiver,
            );
            if target.as_ref() != Some(alias.target()) {
                return None;
            }
        }
        found = true;
    }
    Some(found)
}

impl SourceCommandBindings {
    pub(crate) fn original_namespace_variable_symbol_at_span(
        &self,
        span: tcl_lexer::Span,
        input: &crate::signature_scan::scope::SignatureSourceNameInput,
        receiver: crate::signature_scan::variable_symbol::OriginalVariableSymbolReceiver,
        config: tcl_lexer::LexerConfig,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<(
        crate::signature_scan::scope::SignatureNamespaceScope,
        tcl_core_types::NameBytes,
    )> {
        if !matches!(
            receiver,
            crate::signature_scan::variable_symbol::OriginalVariableSymbolReceiver::LexicalRoot
                | crate::signature_scan::variable_symbol::OriginalVariableSymbolReceiver::Operand(
                    _
                )
        ) {
            return None;
        }
        let form = receiver.input_form(input)?;
        let origin = self.root_origin.as_ref()?;
        let frame = self.original_variable_frame_at_span(span, config);
        let mut selected = None;
        for (site, rows) in &self.declaration_layouts {
            if &site.source != origin || site.offset > span.start() {
                continue;
            }
            let Some(rows) = original_declaration_layouts(rows) else {
                continue;
            };
            let first = rows.clone().next()?;
            if first.config != config
                || !first.entry.owns_source(origin, span.start())
                || frame.as_ref().is_some_and(|frame| {
                    first.entry.frame() != frame.frame() || first.entry.source() != frame.source()
                })
                || !first.words.iter().any(|word| {
                    word.source().span.start() <= span.start()
                        && span.end() <= word.source().span.end()
                })
            {
                continue;
            }
            for row in rows {
                let state = &row.snapshot.state;
                let context = &state.source_variables;
                input.is_current(context).then_some(())?;
                let place = crate::var_resolve::resolve_evaluated_variable_input(
                    form,
                    context,
                    false,
                    registry,
                    tcl_registry::TraceOperation::Read,
                );
                if place.dynamic || place.kind == crate::place::PlaceKind::Unknown {
                    return None;
                }
                let cell = place.cell.as_ref()?;
                let crate::place::CellOwner::NamespaceIdentity(owner) = &cell.owner else {
                    return None;
                };
                let value = (
                    state.original_namespace_geometry(owner, input.policy())?,
                    cell.name.clone(),
                );
                if selected.as_ref().is_some_and(|previous| previous != &value) {
                    return None;
                }
                selected = Some(value);
            }
        }
        selected
    }

    /// Original local root selected unanimously in the declaration's own
    /// point. Aliases and unknown bindings cannot borrow the displayed scope.
    pub(crate) fn original_local_variable_symbol_at_span(
        &self,
        span: tcl_lexer::Span,
        input: &crate::signature_scan::scope::SignatureSourceNameInput,
        receiver: crate::signature_scan::variable_symbol::OriginalVariableSymbolReceiver,
        config: tcl_lexer::LexerConfig,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<(SourceOriginalVariableFrame, tcl_core_types::NameBytes)> {
        use tcl_syntax::naming::{
            NativeNameContext, NativeVariableInputForm, NativeVariableRootGeometry,
        };
        let frame = self.original_variable_frame_at_span(span, config);
        #[cfg(test)]
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_SYMBOLS").is_some() {
            eprintln!(
                "ORIGINAL_VARIABLE_LOCAL span={span:?} bytes={:?} frame={} rows={}",
                input.bytes(),
                frame.is_some(),
                self.declaration_layouts.len()
            );
        }
        let frame = frame?;
        let protocol = input.policy().recipe();
        if !matches!(
            receiver,
            crate::signature_scan::variable_symbol::OriginalVariableSymbolReceiver::LexicalRoot
                | crate::signature_scan::variable_symbol::OriginalVariableSymbolReceiver::Operand(
                    _
                )
        ) {
            return None;
        }
        let form = receiver.input_form(input)?;
        let selected = match form {
            NativeVariableInputForm::Combined(bytes) => protocol.combined_variable_input(bytes),
            NativeVariableInputForm::Separate { root, element } => {
                protocol.separate_variable_input(root, element)
            }
        };
        let NativeVariableRootGeometry::Local(simple) =
            protocol.variable_root_geometry(NativeNameContext::root(), selected.root().selected())
        else {
            return None;
        };
        let origin = self.root_origin.as_ref()?;
        let mut found = false;
        for (site, rows) in self.root_layouts_spanning(span.start(), span.end()) {
            if &site.source != origin || site.offset > span.start() {
                continue;
            }
            let Some(rows) = original_declaration_layouts(rows) else {
                continue;
            };
            let first = rows.clone().next()?;
            if first.config != config
                || first.entry.frame() != frame.frame()
                || first.entry.source() != frame.source()
                || !first.words.iter().any(|word| {
                    word.source().span.start() <= span.start()
                        && span.end() <= word.source().span.end()
                })
            {
                continue;
            }
            for row in rows {
                let state = &row.snapshot.state;
                let context = &state.source_variables;
                if !input.is_current(context) {
                    return None;
                }
                // The lexical/compiler and runtime projections must select
                // this same exact primary. A raw-NUL LVT match cannot use this
                // deliberately value-free uniform naming projection.
                if !original_local_primary_is_byte_unique(state, input.policy(), simple.as_bytes())
                {
                    return None;
                }
                let place = crate::var_resolve::resolve_evaluated_variable_input(
                    form,
                    context,
                    false,
                    registry,
                    tcl_registry::TraceOperation::Read,
                );
                let cell = place.cell.as_ref()?;
                #[cfg(test)]
                if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_SYMBOLS")
                    .is_some()
                {
                    eprintln!(
                        "ORIGINAL_VARIABLE_LOCAL_POINT span={span:?} site={} kind={:?} dynamic={} owner={:?} name={:?} activation={:?} bindings_unknown={}",
                        site.offset,
                        place.kind,
                        place.dynamic,
                        cell.owner,
                        cell.name,
                        context.activation,
                        context.dynamic_bindings
                    );
                }
                if place.dynamic
                    || place.kind == crate::place::PlaceKind::Unknown
                    || !matches!(&cell.owner, crate::place::CellOwner::Activation(activation) if context.activation.as_ref() == Some(activation))
                    || cell.name != simple
                {
                    return None;
                }
                found = true;
            }
        }
        found.then_some((frame, simple))
    }

    #[cfg(test)]
    pub(crate) fn original_variable_uses_global_frame_at_span(
        &self,
        span: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
    ) -> bool {
        let Some(origin) = &self.root_origin else {
            return false;
        };
        let mut found = false;
        for (site, rows) in &self.declaration_layouts {
            if &site.source != origin || site.offset > span.start() {
                continue;
            }
            let Some(rows) = original_declaration_layouts(rows) else {
                continue;
            };
            let Some(first) = rows.clone().next() else {
                continue;
            };
            if first.config != config
                || !first.entry.owns_source(origin, span.start())
                || !first.words.iter().any(|word| {
                    word.source().span.start() <= span.start()
                        && span.end() <= word.source().span.end()
                })
            {
                continue;
            }
            if rows.clone().any(|row| {
                row.entry.frame().layout() != &crate::var_resolve::VariableExecutionFrame::Global
            }) {
                return false;
            }
            found = true;
        }
        found
    }

    pub(crate) fn original_variable_alias_frame_at_span(
        &self,
        span: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
    ) -> Option<(SourceOriginalVariableFrame, u32)> {
        let frame = self.original_variable_frame_at_span(span, config)?;
        let origin = self.root_origin.as_ref()?;
        let mut found = None;
        for (site, rows) in &self.declaration_layouts {
            if &site.source != origin || site.offset > span.start() {
                continue;
            }
            let Some(rows) = original_declaration_layouts(rows) else {
                continue;
            };
            let first = rows.clone().next()?;
            if first.config == config
                && first.entry.frame() == frame.frame()
                && first.entry.source() == frame.source()
                && first.words.iter().any(|word| word.source().span == span)
            {
                if found.is_some_and(|offset| offset != site.offset) {
                    return None;
                }
                found = Some(site.offset);
            }
        }
        Some((frame, found?))
    }

    /// Original local spelling and declaration frame for a conditional source
    /// alias template. Physical-cell matching remains a separate operation.
    pub(crate) fn original_variable_alias_template_matches_at_span(
        &self,
        span: tcl_lexer::Span,
        input: &crate::signature_scan::scope::SignatureSourceNameInput,
        receiver: crate::signature_scan::variable_symbol::OriginalVariableSymbolReceiver,
        alias: &crate::signature_scan::variable_symbol::OriginalVariableAliasReceipt,
        config: tcl_lexer::LexerConfig,
    ) -> bool {
        use tcl_syntax::naming::{
            NativeNameContext, NativeVariableInputForm, NativeVariableRootGeometry,
        };
        if input.policy() != alias.target().policy() || alias.frame().lexer_config() != config {
            return false;
        }
        let Some(form) = receiver.input_form(input) else {
            return false;
        };
        let protocol = input.policy().recipe();
        let selected = match form {
            NativeVariableInputForm::Combined(bytes) => protocol.combined_variable_input(bytes),
            NativeVariableInputForm::Separate { root, element } => {
                protocol.separate_variable_input(root, element)
            }
        };
        let declaration =
            crate::signature_scan::variable_symbol::OriginalVariableAliasAdvice::from_receipt(
                alias,
            );
        let geometry =
            protocol.variable_root_geometry(NativeNameContext::root(), selected.root().selected());
        let local =
            matches!(&geometry, NativeVariableRootGeometry::Local(name) if name == alias.local());
        if span != declaration.span()
            && !local
            && matches!(geometry, NativeVariableRootGeometry::Local(_))
        {
            return false;
        }
        let Some(frame) = self.original_variable_frame_at_span(span, config) else {
            return false;
        };
        if &frame != alias.frame() {
            return false;
        }
        let Some(origin) = &self.root_origin else {
            return false;
        };
        let mut found = false;
        for (site, observations) in &self.declaration_layouts {
            if &site.source != origin
                || site.offset < alias.invocation_offset()
                || site.offset > span.start()
            {
                continue;
            }
            let Some(rows) = original_declaration_layouts(observations) else {
                continue;
            };
            let Some(first) = rows.clone().next() else {
                continue;
            };
            if first.config != config
                || first.entry.frame() != frame.frame()
                || first.entry.source() != frame.source()
                || !first.words.iter().any(|word| {
                    word.source().span.start() <= span.start()
                        && span.end() <= word.source().span.end()
                })
            {
                continue;
            }
            let Some(site_found) = original_alias_template_rows_match(
                rows,
                input,
                receiver,
                alias,
                || span != declaration.span() && !local,
                || {
                    selected.root().qualification()
                        == tcl_syntax::naming::NativeNameQualification::Absolute
                },
            ) else {
                return false;
            };
            found |= site_found;
        }
        found
    }

    pub(crate) fn original_variable_alias_matches_at_span(
        &self,
        span: tcl_lexer::Span,
        input: &crate::signature_scan::scope::SignatureSourceNameInput,
        receiver: crate::signature_scan::variable_symbol::OriginalVariableSymbolReceiver,
        alias: &crate::signature_scan::variable_symbol::OriginalVariableAliasReceipt,
        config: tcl_lexer::LexerConfig,
        registry: &tcl_registry::CommandRegistry,
    ) -> bool {
        use tcl_syntax::naming::{
            NativeNameContext, NativeVariableInputForm, NativeVariableRootGeometry,
        };
        let protocol = input.policy().recipe();
        if input.policy() != alias.target().policy() || alias.frame().lexer_config() != config {
            return false;
        }
        if !matches!(
            receiver,
            crate::signature_scan::variable_symbol::OriginalVariableSymbolReceiver::LexicalRoot
                | crate::signature_scan::variable_symbol::OriginalVariableSymbolReceiver::Operand(
                    _
                )
        ) {
            return false;
        }
        let Some(form) = receiver.input_form(input) else {
            return false;
        };
        let projection = match form {
            NativeVariableInputForm::Combined(bytes) => protocol.combined_variable_input(bytes),
            NativeVariableInputForm::Separate { root, element } => {
                protocol.separate_variable_input(root, element)
            }
        };
        if !matches!(protocol.variable_root_geometry(NativeNameContext::root(), projection.root().selected()), NativeVariableRootGeometry::Local(ref name) if name == alias.local())
        {
            return false;
        }
        let Some(origin) = &self.root_origin else {
            return false;
        };
        let mut found = false;
        for (site, rows) in &self.declaration_layouts {
            if &site.source != origin
                || site.offset < alias.invocation_offset()
                || site.offset > span.start()
            {
                continue;
            }
            let Some(rows) = original_declaration_layouts(rows) else {
                continue;
            };
            let Some(first) = rows.clone().next() else {
                continue;
            };
            if first.config != config
                || first.entry.frame() != alias.frame().frame()
                || first.entry.source() != alias.frame().source()
                || !first.words.iter().any(|word| {
                    word.source().span.start() <= span.start()
                        && span.end() <= word.source().span.end()
                })
            {
                continue;
            }
            for row in rows {
                let context = &row.snapshot.state.source_variables;
                if !input.is_current(context) {
                    return false;
                }
                if !original_local_primary_is_byte_unique(
                    &row.snapshot.state,
                    input.policy(),
                    alias.local().as_bytes(),
                ) {
                    return false;
                }
                let place = crate::var_resolve::resolve_evaluated_variable_input(
                    form,
                    context,
                    false,
                    registry,
                    tcl_registry::TraceOperation::Read,
                );
                if place.cell.is_none() {
                    #[cfg(test)]
                    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_SYMBOLS")
                        .is_some()
                    {
                        eprintln!(
                            "ORIGINAL_VARIABLE_ALIAS_POINT span={span:?} producer={} site={} local={:?} kind={:?} dynamic={} bindings_unknown={} missing_cell",
                            alias.invocation_offset(),
                            site.offset,
                            alias.local(),
                            place.kind,
                            place.dynamic,
                            context.dynamic_bindings
                        );
                    }
                    return false;
                }
                if !original_alias_place_matches(row, input, alias, &place) {
                    return false;
                }
                found = true;
            }
        }
        found
    }

    pub(crate) fn original_variable_formal_occurrences(
        &self,
        config: tcl_lexer::LexerConfig,
    ) -> Vec<crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence> {
        use crate::signature_scan::variable_symbol::SignatureSourceVariableOccurrence;
        let Some(origin) = &self.root_origin else {
            return Vec::new();
        };
        let mut result = Vec::new();
        let mut seen = Vec::new();
        for rows in self.declaration_layouts.values() {
            let Some(rows) = original_declaration_layouts(rows) else {
                continue;
            };
            let Some(row) = rows.clone().next() else {
                continue;
            };
            if row.config != config
                || &row.entry.source().origin != origin
                || !matches!(
                    row.entry.frame().layout(),
                    crate::var_resolve::VariableExecutionFrame::Procedure { .. }
                        | crate::var_resolve::VariableExecutionFrame::ReceiverMethod { .. }
                )
            {
                continue;
            }
            if seen.contains(&row.entry) {
                continue;
            }
            seen.push(row.entry.clone());
            let Some(topology) = row.entry.original_formal_topology() else {
                continue;
            };
            let Some(fields) = topology.original_name_fields() else {
                continue;
            };
            let frame = SourceOriginalVariableFrame {
                source: row.entry.source().clone(),
                frame: row.entry.frame().clone(),
                config,
                logical_lambda: None,
            };
            for field in fields {
                let agrees = rows.clone().all(|row| {
                    original_local_primary_is_byte_unique(
                        &row.snapshot.state,
                        field.input.policy(),
                        field.name.as_bytes(),
                    ) && field.input.is_current(&row.snapshot.state.source_variables)
                });
                if !agrees {
                    continue;
                }
                result.push(
                    SignatureSourceVariableOccurrence::from_original_formal_input(
                        field.span,
                        field.input,
                        frame.clone(),
                        field.name,
                        field.recipe,
                    ),
                );
            }
        }
        result
    }

    pub(crate) fn original_variable_formal_name_is_unrepresented(
        &self,
        symbol: &crate::signature_scan::variable_symbol::SignatureSourceVariableSymbol,
    ) -> bool {
        let crate::signature_scan::variable_symbol::SignatureSourceVariableSlot::Local {
            frame,
            simple,
        } = symbol.slot()
        else {
            return false;
        };
        let represented = self.original_variable_formal_occurrences(frame.lexer_config());
        let mut found = false;
        for rows in self.declaration_layouts.values() {
            let Some(rows) = original_declaration_layouts(rows) else {
                continue;
            };
            for row in rows {
                if row.entry.frame() != frame.frame()
                    || row.entry.source() != frame.source()
                    || row.config != frame.lexer_config()
                {
                    continue;
                }
                found = true;
                let Some(topology) = row.entry.original_formal_topology() else {
                    if matches!(
                        row.entry.as_ref(),
                        OriginalDiagnosticFrameEntry::ScriptActivation { .. }
                            | OriginalDiagnosticFrameEntry::LogicalScriptActivation { .. }
                    ) {
                        continue;
                    }
                    return true;
                };
                if topology
                    .parameters()
                    .iter()
                    .any(|parameter| parameter.name.as_slice() == simple.as_bytes())
                    && !represented
                        .iter()
                        .any(|occurrence| occurrence.symbol() == symbol)
                {
                    return true;
                }
            }
        }
        !found
    }

    /// Select the innermost original local naming frame owning a source cursor.
    /// Whitespace belongs to the retained body image; command words and display
    /// scope spans do not manufacture a frame.
    pub(crate) fn original_variable_frame_at_offset(
        &self,
        offset: u32,
        config: tcl_lexer::LexerConfig,
    ) -> Option<SourceOriginalVariableFrame> {
        let origin = self.root_origin.as_ref()?;
        let mut found: Option<SourceOriginalVariableFrame> = None;
        #[cfg(debug_assertions)]
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_COMPLETION").is_some() {
            eprintln!(
                "ORIGINAL_VARIABLE_FRAME offset={offset} layouts={} deferred={}",
                self.declaration_layouts.len(),
                self.deferred.len(),
            );
        }
        for observations in self.declaration_layouts.values() {
            #[cfg(debug_assertions)]
            if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_COMPLETION")
                .is_some()
            {
                for row in observations {
                    if row.entry.owns_source_cursor(origin, offset) {
                        let layout = match row.entry.frame().layout() {
                            crate::var_resolve::VariableExecutionFrame::Global => "global",
                            crate::var_resolve::VariableExecutionFrame::Procedure { .. } => {
                                "procedure"
                            }
                            crate::var_resolve::VariableExecutionFrame::ReceiverMethod {
                                ..
                            } => "receiver",
                            crate::var_resolve::VariableExecutionFrame::NamespaceActivation {
                                ..
                            } => "namespace-activation",
                            _ => "other",
                        };
                        eprintln!(
                            "ORIGINAL_VARIABLE_FRAME offset={offset} site={} issuer={:?} layout={layout} source_base={} source_bytes={} config={} frame={} origin={} context={}",
                            row.words
                                .first()
                                .map_or(0, |word| word.source().span.start()),
                            row.issuer,
                            row.entry.source().base(),
                            row.entry.source().text.len(),
                            row.config == config,
                            row.snapshot.state.variable_frame == *row.entry.frame(),
                            row.snapshot.state.current_source_origin.as_ref()
                                == Some(&row.entry.source().origin),
                            row.entry
                                .owns_original_context(&row.snapshot.state.source_variables),
                        );
                    }
                }
            }
            let Some(original) = original_declaration_layouts(observations) else {
                continue;
            };
            let first = original.clone().next()?;
            if first.config != config
                || !first.entry.owns_source_cursor(origin, offset)
                || !matches!(
                    first.entry.frame().layout(),
                    crate::var_resolve::VariableExecutionFrame::Procedure { .. }
                        | crate::var_resolve::VariableExecutionFrame::ReceiverMethod { .. }
                        | crate::var_resolve::VariableExecutionFrame::NamespaceActivation { .. }
                )
            {
                continue;
            }
            let frame = SourceOriginalVariableFrame {
                source: first.entry.source().clone(),
                frame: first.entry.frame().clone(),
                config,
                logical_lambda: None,
            };
            match &found {
                Some(previous) if previous.source.text.len() < frame.source.text.len() => {}
                Some(previous) if previous.source.text.len() == frame.source.text.len() => {
                    if previous != &frame {
                        #[cfg(debug_assertions)]
                        if tcl_dialect::trace_env::var_os(
                            "TCL_LSP_TRACE_ORIGINAL_VARIABLE_COMPLETION",
                        )
                        .is_some()
                        {
                            eprintln!(
                                "ORIGINAL_VARIABLE_FRAME offset={offset} stage=conflicting-owners source_bytes={}",
                                frame.source.text.len(),
                            );
                        }
                        return None;
                    }
                }
                _ => found = Some(frame),
            }
        }
        #[cfg(debug_assertions)]
        if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_COMPLETION").is_some() {
            eprintln!(
                "ORIGINAL_VARIABLE_FRAME offset={offset} stage=selected found={}",
                found.is_some(),
            );
        }
        found
    }

    /// Select a local naming frame solely from authentic original body rows.
    /// Entered caller alternatives, unknown frames and disagreeing recipes
    /// cannot donate a declaration identity from their displayed scope names.
    pub(crate) fn original_variable_frame_at_span(
        &self,
        span: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
    ) -> Option<SourceOriginalVariableFrame> {
        let origin = self.root_origin.as_ref()?;
        let mut found: Option<SourceOriginalVariableFrame> = None;
        for (site, observations) in self.root_layouts_spanning(span.start(), span.end()) {
            if &site.source != origin || site.offset > span.start() {
                continue;
            }
            let Some(original) = original_declaration_layouts(observations) else {
                continue;
            };
            let first = original.clone().next()?;
            if first.config != config
                || !first.entry.owns_source(origin, span.start())
                || !first.words.iter().any(|word| {
                    word.source().span.start() <= span.start()
                        && span.end() <= word.source().span.end()
                })
                || !matches!(
                    first.entry.frame().layout(),
                    crate::var_resolve::VariableExecutionFrame::Procedure { .. }
                        | crate::var_resolve::VariableExecutionFrame::ReceiverMethod { .. }
                        | crate::var_resolve::VariableExecutionFrame::NamespaceActivation { .. }
                )
            {
                continue;
            }
            let frame = SourceOriginalVariableFrame {
                source: first.entry.source().clone(),
                frame: first.entry.frame().clone(),
                config,
                logical_lambda: None,
            };
            match &found {
                Some(previous) if previous.source.text.len() < frame.source.text.len() => {}
                Some(previous) if previous.source.text.len() == frame.source.text.len() => {
                    if previous != &frame {
                        return None;
                    }
                }
                _ => found = Some(frame),
            }
        }
        found
    }
}

pub(in super::super) fn root_diagnostic_namespace(
    state: &ModuleCommandBindings,
    current: &super::super::SourceNamespaceKey,
) -> Option<super::super::SourceNamespaceKey> {
    let root = state.source_root_namespace_key()?;
    if state.variable_frame.layout() != &crate::var_resolve::VariableExecutionFrame::Global
        || current != &root
        || state
            .variable_frame
            .namespace_identity()
            .is_some_and(|key| key != &root)
        || (state.baseline.native_entry.is_some()
            && state.variable_frame.namespace_identity() != Some(&root))
    {
        return None;
    }
    Some(root)
}

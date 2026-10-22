// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Readonly original formal topology, independently of entering an activation.

use super::original_name::SignatureSourceNameKey;
use super::scope::SignatureSourceNameInput;
use tcl_syntax::formal_params::{ByteFormalParameter, FormalArityError, FormalByteArgumentBinding};

/// Original `ParamList` value and its independently selected binding grammar.
/// This describes source argument topology; it supplies no installed command,
/// frame, current cell, contents, native object or completion guarantee.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SignatureSourceFormalParameters {
    input: SignatureSourceNameKey,
    parameters: Vec<ByteFormalParameter>,
    grammar: tcl_dialect::ParameterGrammar,
}

/// One readonly original parameter field and its independently mapped source
/// extent. A decoded child never becomes a fabricated complete lexical word.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureSourceFormalField {
    input: SignatureSourceNameInput,
    span: Option<tcl_lexer::Span>,
}

/// Readonly name bytes and optional original name extent. Quote markers are
/// source conventions; these rows supply no frame, cell or edit permission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceFormalName {
    name: Vec<u8>,
    span: Option<tcl_lexer::Span>,
    quoted_marker: bool,
}

/// Conditional Unicode source formals from a genuine lambda-list field.
/// The complete roster stays retained; these are not Native formal objects,
/// installed locals, argument bindings or editable declaration inputs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct LogicalSourceFormalParameters {
    input: super::name_value::LogicalSourceListNameInput,
    parameters: Vec<tcl_syntax::formal_params::FormalParameter>,
    names: Vec<OriginalSourceFormalName>,
}

/// Typed source topology for a selected authored Logical procedure `ParamList`.
/// It retains the original word and complete source parameter objects, without
/// treating them as Native formal bytes, an installed local roster or a frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicalProcedureFormalTopology {
    original: tcl_lexer::NativeWord,
    parameters: Vec<tcl_syntax::formal_params::FormalParameter>,
    names: Vec<OriginalSourceFormalName>,
}

impl std::hash::Hash for LogicalProcedureFormalTopology {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hash::hash(&self.original, state);
        for parameter in &self.parameters {
            std::hash::Hash::hash(&parameter.name, state);
            std::hash::Hash::hash(&parameter.default, state);
        }
        for name in &self.names {
            std::hash::Hash::hash(&name.name, state);
            std::hash::Hash::hash(&name.span, state);
            std::hash::Hash::hash(&name.quoted_marker, state);
        }
    }
}

impl LogicalProcedureFormalTopology {
    pub(crate) fn from_selected_declaration(
        original: &tcl_lexer::NativeWord,
        parameters: &[tcl_syntax::formal_params::FormalParameter],
        names: Vec<OriginalSourceFormalName>,
        config: tcl_lexer::LexerConfig,
    ) -> Option<Self> {
        if original.config() != config
            || names.len() != parameters.len()
            || !names
                .iter()
                .zip(parameters)
                .all(|(name, parameter)| name.name() == parameter.name.as_bytes())
        {
            return None;
        }
        Some(Self {
            original: original.clone(),
            parameters: parameters.to_vec(),
            names,
        })
    }

    /// Selected source Logical formal objects, preserving order/defaults.
    #[must_use]
    pub fn parameters(&self) -> &[tcl_syntax::formal_params::FormalParameter] {
        &self.parameters
    }

    /// Original name fields mapped by the selected source-list owner.
    #[must_use]
    pub fn name_sources(&self) -> &[OriginalSourceFormalName] {
        &self.names
    }

    /// Argument-count topology under the source Logical procedure grammar.
    #[must_use]
    pub fn argument_count_shape(&self) -> tcl_syntax::formal_params::FormalArgumentCountShape {
        tcl_syntax::formal_params::formal_argument_count_shape(
            self.parameters
                .iter()
                .map(|parameter| (parameter.name == "args", parameter.default.is_some())),
            tcl_dialect::ParameterGrammar::Tcl,
        )
    }
}

impl LogicalSourceFormalParameters {
    pub(crate) fn from_lambda_fields(
        fields: &[super::name_value::LogicalSourceListNameInput],
        dialect: tcl_registry::InvocationDialect,
    ) -> Option<Self> {
        super::name_value::LogicalSourceListNameInput::complete_unchanged_roster(fields)?;
        if !matches!(fields.len(), 2 | 3) {
            return None;
        }
        let input = fields.first()?;
        if dialect.authored_name_policy() != Some(input.policy())
            || dialect.parameter_grammar() == Some(tcl_dialect::ParameterGrammar::Jim)
            || !matches!(
                input.policy().recipe(),
                tcl_syntax::naming::NativeNameProtocol::C(_)
            )
        {
            return None;
        }
        let parameter_field = input.original_field();
        let parameters = tcl_syntax::formal_params::parse_formal_parameters_in(
            parameter_field.value(),
            tcl_dialect::ParameterGrammar::Tcl,
        )
        .ok()?;
        if dialect.parameter_grammar().is_none()
            && tcl_syntax::formal_params::parse_formal_parameters_in(
                parameter_field.value(),
                tcl_dialect::ParameterGrammar::Jim,
            )
            .ok()?
                != parameters
        {
            return None;
        }
        let specifiers = parameter_field.elements()?;
        if specifiers.len() != parameters.len() {
            return None;
        }
        let mut names = Vec::with_capacity(parameters.len());
        for (specifier, parameter) in specifiers.iter().zip(&parameters) {
            if specifier
                .original_word()
                .image()
                .bytes()
                .get(specifier.source_span()?.as_range())?
                != specifier.value().as_bytes()
            {
                return None;
            }
            let fields = specifier.elements()?;
            if fields.iter().any(|field| {
                field.source_span().is_none_or(|span| {
                    field.original_word().image().bytes().get(span.as_range())
                        != Some(field.value().as_bytes())
                })
            }) {
                return None;
            }
            let name = fields.first()?;
            if fields.len() != 1 + usize::from(parameter.default.is_some())
                || name.value() != parameter.name
                || fields.get(1).map(|field| field.value()) != parameter.default.as_deref()
            {
                return None;
            }
            // The full configured source-list owner supplies geometry. The
            // shared semantic parse must agree; cooked names cannot donate
            // an interior source range or an original formal-name input.
            let span = name.source_span()?;
            if name.original_word().image().bytes().get(span.as_range())?
                != parameter.name.as_bytes()
            {
                return None;
            }
            names.push(OriginalSourceFormalName::new(
                parameter.name.as_bytes().to_vec(),
                Some(span),
                false,
            ));
        }
        Some(Self {
            input: input.clone(),
            parameters,
            names,
        })
    }

    pub(crate) fn parameters(&self) -> &[tcl_syntax::formal_params::FormalParameter] {
        &self.parameters
    }

    pub(crate) fn name_sources(&self) -> &[OriginalSourceFormalName] {
        &self.names
    }

    pub(crate) fn parameter_field(&self) -> &tcl_syntax::word_rules::OriginalSourceListElement {
        self.input.original_field()
    }
}

impl OriginalSourceFormalName {
    pub(crate) fn new(name: Vec<u8>, span: Option<tcl_lexer::Span>, quoted_marker: bool) -> Self {
        Self {
            name,
            span,
            quoted_marker,
        }
    }

    /// Decoded declaration name bytes, independent of source spelling.
    #[must_use]
    pub fn name(&self) -> &[u8] {
        &self.name
    }

    /// Tight original name extent when source geometry is available.
    #[must_use]
    pub const fn source_span(&self) -> Option<tcl_lexer::Span> {
        self.span
    }

    /// Whether the original name has the quoted positional-marker convention.
    #[must_use]
    pub const fn quoted_marker(&self) -> bool {
        self.quoted_marker
    }

    pub(crate) fn source_is_quoted(image: &tcl_lexer::SourceImage, span: tcl_lexer::Span) -> bool {
        let bytes = image.bytes();
        span.start()
            .checked_sub(1)
            .and_then(|start| bytes.get(start as usize))
            == Some(&b'"')
            && bytes.get(span.end() as usize) == Some(&b'"')
    }
}

impl SignatureSourceFormalField {
    /// Exact original list child, without a displayed-name conversion.
    #[must_use]
    pub const fn original_input(&self) -> &SignatureSourceNameInput {
        &self.input
    }

    /// Original source extent, when the complete parent and field literal
    /// mapping is independently available. Escape-produced parent values can
    /// retain a child without supplying an editable source substring.
    #[must_use]
    pub const fn source_span(&self) -> Option<tcl_lexer::Span> {
        self.span
    }
}

impl SignatureSourceFormalParameters {
    /// Parse this authentic original operand as parameter-list syntax under
    /// an independently selected engine grammar. The caller separately owns
    /// its Registry `ParamList` role; this issues no definition or entered frame.
    #[must_use]
    pub fn from_original_input(
        input: &SignatureSourceNameKey,
        dialect: tcl_registry::InvocationDialect,
    ) -> Option<Self> {
        let topology =
            crate::command_binding::formal_topology::OriginalFormalTopology::from_original_key(
                input.clone(),
                dialect,
            )?;
        Some(Self::from_original_topology(&topology))
    }

    pub(crate) fn from_original_topology(
        topology: &crate::command_binding::formal_topology::OriginalFormalTopology,
    ) -> Self {
        Self {
            input: topology.original_input().clone(),
            parameters: topology.parameters().to_vec(),
            grammar: topology.parameter_grammar(),
        }
    }

    /// Complete original parameter-list lexical producer and counted value.
    #[must_use]
    pub fn original_input(&self) -> &SignatureSourceNameKey {
        &self.input
    }

    /// Actual formal binding grammar selected independently of declaration names.
    #[must_use]
    pub const fn parameter_grammar(&self) -> tcl_dialect::ParameterGrammar {
        self.grammar
    }

    /// Exact ordered formal bytes, including defaults and duplicate names.
    #[must_use]
    pub fn parameters(&self) -> &[ByteFormalParameter] {
        &self.parameters
    }

    /// Exact name field from the actual selected formal specifier. Caller-link
    /// and rest naming remain source syntax rather than installed cell names.
    #[must_use]
    pub fn name_field(&self, parameter: usize) -> Option<SignatureSourceFormalField> {
        self.original_field(parameter, 0)
    }

    /// Name bytes and readonly original source geometry from this retained
    /// Native topology, independently of declaration or activation liveness.
    #[must_use]
    pub fn name_sources(&self) -> Vec<OriginalSourceFormalName> {
        self.parameters
            .iter()
            .enumerate()
            .map(|(index, formal)| {
                let span = self.name_field(index).and_then(|field| field.source_span());
                let quoted = span.is_some_and(|span| {
                    OriginalSourceFormalName::source_is_quoted(self.input.source_image(), span)
                });
                OriginalSourceFormalName::new(formal.name.clone(), span, quoted)
            })
            .collect()
    }

    /// Exact optional second specifier field. Jim rest-name semantics remain
    /// independently described by `bindings()`, rather than a cell projection.
    #[must_use]
    pub fn default_field(&self, parameter: usize) -> Option<SignatureSourceFormalField> {
        self.parameters.get(parameter)?.default.as_ref()?;
        self.original_field(parameter, 1)
    }

    fn original_field(&self, parameter: usize, field: usize) -> Option<SignatureSourceFormalField> {
        original_formal_field(
            &SignatureSourceNameInput::OriginalWord(self.input.clone()),
            &self.parameters,
            self.grammar,
            parameter,
            field,
        )
    }

    /// Bind an argument count using the selected required/default/rest/link
    /// grammar. Returned rows describe syntax and ordinals, without installing
    /// variables or resolving a caller frame.
    pub fn bindings(
        &self,
        count: usize,
    ) -> Result<Vec<FormalByteArgumentBinding>, FormalArityError> {
        tcl_syntax::formal_params::bind_formal_argument_bytes(&self.parameters, count, self.grammar)
    }

    /// Count bounds from the original ordered formal storage and its selected
    /// grammar. This supplies no successful call, binding or activation.
    #[must_use]
    pub fn argument_count_shape(&self) -> tcl_syntax::formal_params::FormalArgumentCountShape {
        tcl_syntax::formal_params::formal_argument_count_shape(
            self.parameters
                .iter()
                .map(|parameter| (parameter.name == b"args", parameter.default.is_some())),
            self.grammar,
        )
    }

    /// Exact immutable producer image, channel and full parser correspondence.
    #[must_use]
    pub fn matches_source(
        &self,
        image: &tcl_lexer::SourceImage,
        config: tcl_lexer::LexerConfig,
    ) -> bool {
        self.input.source_image() == image && self.input.lexer_config() == config
    }
}

/// Original source formal topology selected for one retained procedure body.
/// Native byte protocol and authored Logical source fields remain distinct.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SourceProcedureFormalTopology {
    /// Native procedure `ParamList` selected by its Native declaration recipe.
    Native(SignatureSourceFormalParameters),
    /// Authored Logical `ParamList` selected by its retained Logical declaration.
    Logical(LogicalProcedureFormalTopology),
}

impl SourceProcedureFormalTopology {
    /// Number of source formals, without installing locals or an activation.
    #[must_use]
    pub fn len(&self) -> usize {
        match self {
            Self::Native(formals) => formals.parameters().len(),
            Self::Logical(formals) => formals.parameters().len(),
        }
    }

    /// Whether the selected topology contains no parameters.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Source-level argument count under the independently selected purpose.
    #[must_use]
    pub fn argument_count_shape(&self) -> tcl_syntax::formal_params::FormalArgumentCountShape {
        match self {
            Self::Native(formals) => formals.argument_count_shape(),
            Self::Logical(formals) => formals.argument_count_shape(),
        }
    }
}

fn original_formal_field(
    parent: &SignatureSourceNameInput,
    parameters: &[ByteFormalParameter],
    grammar: tcl_dialect::ParameterGrammar,
    parameter: usize,
    field: usize,
) -> Option<SignatureSourceFormalField> {
    let formal = parameters.get(parameter)?;
    let protocol = parent.policy().recipe();
    let selected = protocol.formal_parameter_list_input(parent.bytes());
    let outer = tcl_syntax::list::split_native_list_elements(
        selected.selected(),
        protocol.string_protocol(),
    )
    .ok()?;
    let children = parent.original_list_elements()?;
    if outer.len() != parameters.len() || children.len() != outer.len() {
        return None;
    }
    let child = children.get(parameter)?;
    let element = outer.get(parameter)?;
    let jim_single = grammar == tcl_dialect::ParameterGrammar::Jim && formal.default.is_none();
    let (input, native_range) = if jim_single {
        if field != 0 {
            return None;
        }
        (child.clone(), Some(element.source.value.clone()))
    } else {
        let nested_input = protocol.formal_parameter_list_input(child.bytes());
        let nested = tcl_syntax::list::split_native_list_elements(
            nested_input.selected(),
            protocol.string_protocol(),
        )
        .ok()?;
        let inner = nested.get(field)?;
        let input = child.original_list_element(field)?;
        // A field that is the whole specifier (`n\uD800`, no default) owns
        // the specifier's own source extent, escapes included: its bytes are
        // exactly that element's decoded value. A field inside an escaped
        // specifier has no literal offset there and abstains.
        let whole = (!element.source.literal && input.bytes() == child.bytes())
            .then(|| element.source.value.clone());
        let range = whole.or_else(|| {
            element.source.literal.then(|| {
                Some(
                    element
                        .source
                        .value
                        .start
                        .checked_add(inner.source.value.start)?
                        ..element
                            .source
                            .value
                            .start
                            .checked_add(inner.source.value.end)?,
                )
            })
            .flatten()
        });
        (input, range)
    };
    let expected = match field {
        0 => formal.name.as_slice(),
        1 => formal.default.as_deref()?,
        _ => return None,
    };
    if input.bytes() != expected {
        return None;
    }
    let span = native_range.and_then(|range| parent.original_static_value_source_extent(range));
    Some(SignatureSourceFormalField { input, span })
}

/// Readonly parameter-list syntax from an original value child. Complete-word
/// provenance stays with its actual ancestor; no lexical Key or entered formal
/// frame is manufactured for this value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureSourceFormalParameterValue {
    input: SignatureSourceNameInput,
    parameters: Vec<ByteFormalParameter>,
    grammar: tcl_dialect::ParameterGrammar,
}

impl SignatureSourceFormalParameterValue {
    /// Parse the retained value using an independently selected engine grammar.
    /// Registry LambdaParam/ParamList selection remains a separate obligation.
    #[must_use]
    pub fn from_original_input(
        input: &SignatureSourceNameInput,
        dialect: tcl_registry::InvocationDialect,
    ) -> Option<Self> {
        match input {
            SignatureSourceNameInput::OriginalWord(_)
            | SignatureSourceNameInput::OriginalValue(_) => {}
            SignatureSourceNameInput::OriginalVariableRoot(_)
            | SignatureSourceNameInput::OriginalLogicalListField(_) => return None,
        }
        let protocol = input.policy().recipe();
        if dialect.native_name_protocol().or_else(|| {
            dialect
                .authored_name_policy()
                .map(tcl_syntax::naming::NamePolicyProtocol::recipe)
        }) != Some(protocol)
        {
            return None;
        }
        Some(Self {
            input: input.clone(),
            parameters: crate::command_binding::formal_topology::native_formal_parameters(
                input.bytes(),
                protocol,
            )?,
            grammar: dialect.parameter_grammar()?,
        })
    }

    /// Retained original value lineage, without borrowing a complete Word key.
    #[must_use]
    pub const fn original_input(&self) -> &SignatureSourceNameInput {
        &self.input
    }

    /// Exact ordered source formal bytes, defaults and duplicates.
    #[must_use]
    pub fn parameters(&self) -> &[ByteFormalParameter] {
        &self.parameters
    }

    /// Pure argument-count topology; no caller frame or variables are installed.
    pub fn bindings(
        &self,
        count: usize,
    ) -> Result<Vec<FormalByteArgumentBinding>, FormalArityError> {
        tcl_syntax::formal_params::bind_formal_argument_bytes(&self.parameters, count, self.grammar)
    }

    /// Pure count bounds from the same actual native formal binding owner.
    /// The source receipt supplies no invocation or successful activation.
    #[must_use]
    pub fn argument_count_shape(&self) -> tcl_syntax::formal_params::FormalArgumentCountShape {
        tcl_syntax::formal_params::formal_argument_count_shape(
            self.parameters
                .iter()
                .map(|parameter| (parameter.name == b"args", parameter.default.is_some())),
            self.grammar,
        )
    }

    /// Actual selected formal name field and independently available source extent.
    #[must_use]
    pub fn name_field(&self, parameter: usize) -> Option<SignatureSourceFormalField> {
        original_formal_field(&self.input, &self.parameters, self.grammar, parameter, 0)
    }

    /// Actual optional second field; Jim rest binding remains independently selected.
    #[must_use]
    pub fn default_field(&self, parameter: usize) -> Option<SignatureSourceFormalField> {
        self.parameters.get(parameter)?.default.as_ref()?;
        original_formal_field(&self.input, &self.parameters, self.grammar, parameter, 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // Implementation contract: naming.variable.original-readonly-formal-topology
    // docs/design/analysis/name-resolution-proofs/original-readonly-formal-topology.md
    fn original_readonly_lambda_formal_value_preserves_ancestry_and_source_fields() {
        let dialect = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
        let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
        for source in [
            r"apply {{n\uD800 {optional DEFAULT}} {return}}",
            r#"apply "{n\uD800 {optional DEFAULT}} {return}""#,
        ] {
            let image = tcl_lexer::SourceImage::document(source);
            let words = tcl_lexer::native_script_words_in(
                image.clone(),
                tcl_lexer::Span::new(0, u32::try_from(image.len()).unwrap()),
                config,
            )
            .unwrap();
            let key = SignatureSourceNameKey::from_original_native_word(
                &words.commands[0].words[1],
                tcl_syntax::word_rules::WordValueRules::from_config(&config),
                dialect.authored_name_policy().unwrap(),
            )
            .unwrap();
            let lambda = SignatureSourceNameInput::OriginalWord(key);
            let params = lambda.original_list_element(0).unwrap();
            assert!(params.original_word_key().is_none());
            let formals =
                SignatureSourceFormalParameterValue::from_original_input(&params, dialect).unwrap();
            assert_eq!(formals.original_input(), &params);
            assert_eq!(formals.bindings(1).unwrap().len(), 2);
            let name = formals.name_field(0).unwrap();
            let default = formals.default_field(1).unwrap();
            assert_eq!(name.original_input().bytes(), b"n\xed\xa0\x80");
            assert_eq!(default.original_input().bytes(), b"DEFAULT");
            assert!(name.original_input().original_word_key().is_none());
            if source.as_bytes()[6] == b'{' {
                assert_eq!(&source[name.source_span().unwrap().as_range()], r"n\uD800");
                assert_eq!(
                    &source[default.source_span().unwrap().as_range()],
                    "DEFAULT"
                );
            } else {
                assert!(name.source_span().is_none());
                assert!(default.source_span().is_none());
            }
        }
    }

    #[test]
    // Implementation contract: naming.variable.original-readonly-formal-topology
    // docs/design/analysis/name-resolution-proofs/original-readonly-formal-topology.md
    fn original_readonly_worker_formal_fields_keep_exact_child_inputs_and_extents() {
        for name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
            let point = tcl_dialect::model::DialectPoint::of_dialect_name(Some(name)).unwrap();
            let dialect = tcl_registry::InvocationDialect::of_point(point);
            let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
            for source in [
                r"method pick {n\uD800 {optional DEFAULT}} {}",
                r#"method pick "n\uD800 {optional DEFAULT}" {}"#,
            ] {
                let image = tcl_lexer::SourceImage::document(source);
                let words = tcl_lexer::native_script_words_in(
                    image.clone(),
                    tcl_lexer::Span::new(0, u32::try_from(image.len()).unwrap()),
                    config,
                )
                .unwrap();
                let input = SignatureSourceNameKey::from_original_native_word(
                    &words.commands[0].words[2],
                    tcl_syntax::word_rules::WordValueRules::from_config(&config),
                    dialect.authored_name_policy().unwrap(),
                )
                .unwrap();
                let formals =
                    SignatureSourceFormalParameters::from_original_input(&input, dialect).unwrap();
                let first = formals.name_field(0).unwrap();
                let second = formals.name_field(1).unwrap();
                let default = formals.default_field(1).unwrap();
                assert_eq!(first.original_input().bytes(), b"n\xed\xa0\x80");
                assert_eq!(second.original_input().bytes(), b"optional");
                assert_eq!(default.original_input().bytes(), b"DEFAULT");
                assert!(first.original_input().original_word_key().is_none());
                assert!(formals.default_field(0).is_none());
                if source.as_bytes()[12] == b'{' {
                    assert_eq!(&source[first.source_span().unwrap().as_range()], r"n\uD800");
                    assert_eq!(
                        &source[second.source_span().unwrap().as_range()],
                        "optional"
                    );
                    assert_eq!(
                        &source[default.source_span().unwrap().as_range()],
                        "DEFAULT"
                    );
                } else {
                    assert!(first.source_span().is_none());
                    assert!(second.source_span().is_none());
                    assert!(default.source_span().is_none());
                }
                let counterfactual =
                    tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_1);
                if dialect != counterfactual {
                    assert!(
                        SignatureSourceFormalParameters::from_original_input(
                            &input,
                            counterfactual
                        )
                        .is_none(),
                        "{name}: {source}"
                    );
                }
            }
        }
    }

    #[test]
    // Implementation contract: naming.variable.original-readonly-formal-topology
    // docs/design/analysis/name-resolution-proofs/original-readonly-formal-topology.md
    fn original_readonly_formals_keep_opaque_default_rest_and_source_currency() {
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
            let point = tcl_dialect::model::DialectPoint::of_dialect_name(Some(profile)).unwrap();
            let dialect = tcl_registry::InvocationDialect::of_point(point);
            let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
            let source = r"proc p {n\uD800 {optional DEFAULT} args} {return}";
            let image = tcl_lexer::SourceImage::document(source);
            let plan = tcl_lexer::native_script_words_in(
                image.clone(),
                tcl_lexer::Span::new(0, u32::try_from(image.len()).unwrap()),
                config,
            )
            .unwrap();
            let input = SignatureSourceNameKey::from_original_native_word(
                &plan.commands[0].words[2],
                tcl_syntax::word_rules::WordValueRules::from_config(&config),
                dialect.authored_name_policy().unwrap(),
            )
            .unwrap();
            let topology =
                crate::command_binding::formal_topology::OriginalFormalTopology::from_original_key(
                    input.clone(),
                    dialect,
                )
                .unwrap();
            let formals = SignatureSourceFormalParameters::from_original_topology(&topology);
            assert_eq!(formals.original_input(), &input);
            assert_eq!(formals.parameters()[0].name, b"n\xed\xa0\x80");
            assert_eq!(
                formals.parameters()[1].default.as_deref(),
                Some(b"DEFAULT".as_slice())
            );
            assert!(formals.bindings(0).is_err());
            assert!(matches!(
                formals.bindings(1).unwrap().as_slice(),
                [
                    FormalByteArgumentBinding::Value {
                        parameter: 0,
                        argument: 0
                    },
                    FormalByteArgumentBinding::Default { parameter: 1 },
                    FormalByteArgumentBinding::Rest {
                        parameter: 2,
                        start: 1,
                        len: 0,
                        ..
                    }
                ]
            ));
            assert!(matches!(
                formals.bindings(3).unwrap().last(),
                Some(FormalByteArgumentBinding::Rest {
                    start: 2,
                    len: 1,
                    ..
                })
            ));
            assert!(formals.matches_source(&image, config));
            assert!(
                !formals.matches_source(&tcl_lexer::SourceImage::native(source.as_bytes()), config)
            );
            assert!(!formals.matches_source(
                &tcl_lexer::SourceImage::document(&format!("{source}; list changed")),
                config
            ));
            let mut other = config;
            other.strict_quoting = !other.strict_quoting;
            assert!(!formals.matches_source(&image, other));
        }
    }
}

// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original native formal bytes and their independently selected activation plan.

use crate::signature_scan::scope::SignatureSourceNameKey;
use tcl_registry::InvocationDialect;
use tcl_syntax::formal_params::{ByteFormalParameter, FormalByteArgumentBinding};

pub(crate) fn native_formal_parameters(
    bytes: &[u8],
    protocol: tcl_syntax::naming::NativeNameProtocol,
) -> Option<Vec<ByteFormalParameter>> {
    let split = |bytes: &[u8]| {
        tcl_syntax::list::split_native_list_bytes(
            protocol.formal_parameter_list_input(bytes).selected(),
            protocol.string_protocol(),
        )
        .map(|values| {
            values
                .into_iter()
                .map(std::borrow::Cow::into_owned)
                .collect()
        })
    };
    let specifiers: Vec<Vec<u8>> = split(bytes).ok()?;
    tcl_syntax::formal_params::parse_formal_parameter_values(
        &specifiers,
        protocol,
        |value, _| split(value),
        |value| Ok(value.clone()),
    )
    .ok()
}

/// Validate original counted `ParamList` value bytes under an independently
/// selected formal-list recipe. This grants syntax only, with no original
/// source, installed formal, activation, current read or compiler capability.
pub(crate) fn native_formal_parameter_bytes_valid(
    bytes: &[u8],
    protocol: tcl_syntax::naming::NativeNameProtocol,
) -> bool {
    native_formal_parameters(bytes, protocol).is_some()
}

/// The immutable original parameter-list producer. Display metadata never
/// supplies its storage keys, list extents, defaults, or argument topology.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OriginalFormalTopology {
    producer: SignatureSourceNameKey,
    parameters: Vec<ByteFormalParameter>,
    grammar: tcl_dialect::ParameterGrammar,
}

impl std::hash::Hash for OriginalFormalTopology {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hash::hash(&self.producer, state);
        std::hash::Hash::hash(&self.grammar, state);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OriginalFormalNameRecipe {
    topology: std::sync::Arc<OriginalFormalTopology>,
    parameter: usize,
    field: usize,
}

pub(crate) struct OriginalFormalNameField {
    pub(crate) span: tcl_lexer::Span,
    pub(crate) input: crate::signature_scan::scope::SignatureSourceNameInput,
    pub(crate) name: tcl_core_types::NameBytes,
    pub(crate) recipe: OriginalFormalNameRecipe,
}

impl OriginalFormalNameRecipe {
    pub(crate) fn renamed_input(&self, new_name: &[u8]) -> Option<Vec<u8>> {
        tcl_syntax::formal_params::parse_formal_parameter_values(
            &[new_name.to_vec()],
            self.topology.producer.policy().recipe(),
            |value, _| Ok::<_, ()>(vec![value.clone()]),
            |value| Ok::<_, ()>(value.clone()),
        )
        .ok()?;
        let mut proposed = self.topology.parameters.clone();
        let original = proposed.get(self.parameter)?;
        if self.field == 1 {
            proposed[self.parameter].default = Some(new_name.to_vec());
        } else {
            let old_name = original.name.clone();
            for parameter in &mut proposed {
                if parameter.name == old_name {
                    parameter.name = new_name.to_vec();
                }
            }
        }
        for count in 0..=proposed.len().checked_add(1)? {
            let old = self.topology.bindings(count);
            let new = tcl_syntax::formal_params::bind_formal_argument_bytes(
                &proposed,
                count,
                self.topology.grammar,
            );
            match (old, new) {
                (Err(_), Err(_)) => {}
                (Ok(old), Ok(new)) => {
                    if old.len() != new.len()
                        || !old
                            .iter()
                            .zip(&new)
                            .all(|(old, new)| same_binding_topology(old, new))
                    {
                        return None;
                    }
                }
                _ => return None,
            }
        }
        Some(new_name.to_vec())
    }
}

fn same_binding_topology(old: &FormalByteArgumentBinding, new: &FormalByteArgumentBinding) -> bool {
    use FormalByteArgumentBinding as Binding;
    match (old, new) {
        (
            Binding::Value {
                parameter: old_parameter,
                argument: old_argument,
            },
            Binding::Value {
                parameter,
                argument,
            },
        )
        | (
            Binding::CallerLink {
                parameter: old_parameter,
                argument: old_argument,
                ..
            },
            Binding::CallerLink {
                parameter,
                argument,
                ..
            },
        ) => old_parameter == parameter && old_argument == argument,
        (
            Binding::Default {
                parameter: old_parameter,
            },
            Binding::Default { parameter },
        ) => old_parameter == parameter,
        (
            Binding::Rest {
                parameter: old_parameter,
                start: old_start,
                len: old_len,
                ..
            },
            Binding::Rest {
                parameter,
                start,
                len,
                ..
            },
        ) => old_parameter == parameter && old_start == start && old_len == len,
        _ => false,
    }
}

impl OriginalFormalTopology {
    pub(crate) fn from_original_key(
        producer: SignatureSourceNameKey,
        dialect: InvocationDialect,
    ) -> Option<Self> {
        let protocol = producer.policy().recipe();
        if dialect.native_name_protocol().or_else(|| {
            dialect
                .authored_name_policy()
                .map(tcl_syntax::naming::NamePolicyProtocol::recipe)
        }) != Some(protocol)
        {
            return None;
        }
        let parameters = native_formal_parameters(producer.bytes(), protocol)?;
        Some(Self {
            producer,
            parameters,
            grammar: dialect.parameter_grammar()?,
        })
    }

    /// Exact readonly native list children and independently mapped original
    /// name extents. A child never becomes a fabricated complete source word.
    pub(crate) fn original_name_fields(&self) -> Option<Vec<OriginalFormalNameField>> {
        use crate::signature_scan::scope::SignatureSourceNameInput;
        use tcl_syntax::naming::{NativeNameContext, NativeVariableRootGeometry};
        let protocol = self.producer.policy().recipe();
        let word = self.producer.original_word();
        let content = word.content_span().ok()?;
        let raw = word.image().bytes().get(content.as_range())?;
        let literal = tcl_syntax::backslash::native_source_literal_bytes(
            raw,
            word.image().channel(),
            protocol.string_protocol(),
        )
        .ok()?;
        if literal.as_ref() != self.producer.bytes() {
            return None;
        }
        let selected = protocol.formal_parameter_list_input(self.producer.bytes());
        let outer = tcl_syntax::list::split_native_list_elements(
            selected.selected(),
            protocol.string_protocol(),
        )
        .ok()?;
        let parent = SignatureSourceNameInput::OriginalWord(self.producer.clone());
        let children = parent.original_list_elements()?;
        if outer.len() != self.parameters.len() || children.len() != outer.len() {
            return None;
        }
        let bindings = self.bindings(self.parameters.len()).ok()?;
        let mut result = Vec::with_capacity(outer.len());
        for (parameter, (outer, child)) in outer.iter().zip(children).enumerate() {
            let formal = &self.parameters[parameter];
            #[cfg(test)]
            if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_VARIABLE_SYMBOLS").is_some() {
                eprintln!(
                    "ORIGINAL_FORMAL_FIELD policy={:?} parameter={parameter} name={:?} child={:?} default={} outer_literal={}",
                    self.producer.policy(),
                    formal.name,
                    child.bytes(),
                    formal.default.is_some(),
                    outer.source.literal
                );
            }
            if bindings.iter().any(|binding| matches!(binding, FormalByteArgumentBinding::CallerLink { parameter: ordinal, .. } if *ordinal == parameter)) { return None; }
            let renamed_rest = bindings.iter().find_map(|binding| match binding {
                FormalByteArgumentBinding::Rest {
                    parameter: ordinal,
                    name,
                    ..
                } if *ordinal == parameter && name != &formal.name => Some(name.as_slice()),
                _ => None,
            });
            let field = usize::from(renamed_rest.is_some());
            let name = renamed_rest.unwrap_or(&formal.name);
            let parsed = protocol.combined_variable_input(name);
            if parsed.element().is_some()
                || !matches!(protocol.variable_root_geometry(NativeNameContext::root(), parsed.root().selected()), NativeVariableRootGeometry::Local(ref simple) if simple.as_bytes() == name)
            {
                return None;
            }
            let child_selected = protocol.formal_parameter_list_input(child.bytes());
            let nested = tcl_syntax::list::split_native_list_elements(
                child_selected.selected(),
                protocol.string_protocol(),
            )
            .ok()?;
            let input = if protocol.is_jim084() && formal.default.is_none() {
                child.clone()
            } else {
                child.original_list_element(field)?
            };
            if protocol.formal_storage_name_input(input.bytes()).selected() != name {
                return None;
            }
            let native_range = if formal.default.is_none() {
                outer.source.value.clone()
            } else {
                if !outer.source.literal {
                    return None;
                }
                let inner = &nested.get(field)?.source.value;
                outer.source.value.start.checked_add(inner.start)?
                    ..outer.source.value.start.checked_add(inner.end)?
            };
            let span = Self::original_field_span(word, raw, protocol, native_range)?;
            result.push(OriginalFormalNameField {
                span,
                input,
                name: name.into(),
                recipe: OriginalFormalNameRecipe {
                    topology: std::sync::Arc::new(self.clone()),
                    parameter,
                    field,
                },
            });
        }
        Some(result)
    }

    fn original_field_span(
        word: &tcl_lexer::NativeWord,
        raw: &[u8],
        protocol: tcl_syntax::naming::NativeNameProtocol,
        native_range: std::ops::Range<usize>,
    ) -> Option<tcl_lexer::Span> {
        let content = word.content_span().ok()?;
        let mapped = tcl_syntax::backslash::native_source_literal_extent(
            raw,
            word.image().channel(),
            protocol.string_protocol(),
            native_range,
        )?;
        Some(tcl_lexer::Span::new(
            content
                .start()
                .checked_add(u32::try_from(mapped.start).ok()?)?,
            content
                .start()
                .checked_add(u32::try_from(mapped.end).ok()?)?,
        ))
    }

    /// Ordered declaration-storage prefix, before any body local is allocated.
    /// A matching formal is the native compiler's first candidate; absence
    /// here does not close the remaining compiled-local inventory.
    pub(crate) fn first_compiled_formal_name(
        &self,
        requested: &[u8],
        protocol: tcl_syntax::naming::NativeCompiledVariableProtocol,
        dynamic: bool,
    ) -> Option<&[u8]> {
        self.parameters.iter().find_map(|formal| {
            let matches = if dynamic {
                protocol.dynamic_local_names_equal(&formal.name, requested)
            } else {
                protocol.compiled_local_names_equal(&formal.name, requested)
            };
            matches.then_some(formal.name.as_slice())
        })
    }

    pub(crate) fn original_input(&self) -> &SignatureSourceNameKey {
        &self.producer
    }

    pub(crate) fn parameter_grammar(&self) -> tcl_dialect::ParameterGrammar {
        self.grammar
    }

    pub(crate) fn parameters(&self) -> &[ByteFormalParameter] {
        &self.parameters
    }

    /// Original scalar destinations for a fixed-arity source reassignment.
    /// The binding and variable-name purposes remain separate from rendering,
    /// normal completion, contents, compiler admission and physical frames.
    pub(crate) fn fixed_scalar_binding_names(
        &self,
        count: usize,
    ) -> Option<Vec<tcl_core_types::NameBytes>> {
        use tcl_syntax::naming::{NativeNameContext, NativeVariableRootGeometry};
        let protocol = self.producer.policy().recipe();
        self.bindings(count)
            .ok()?
            .into_iter()
            .map(|binding| {
                let FormalByteArgumentBinding::Value { parameter, .. } = binding else {
                    return None;
                };
                let name = &self.parameters.get(parameter)?.name;
                let selected = protocol.combined_variable_input(name);
                if selected.element().is_some() {
                    return None;
                }
                match protocol
                    .variable_root_geometry(NativeNameContext::root(), selected.root().selected())
                {
                    NativeVariableRootGeometry::Local(simple) if simple.as_bytes() == name => {
                        Some(simple)
                    }
                    _ => None,
                }
            })
            .collect()
    }

    /// Independently retained producer's string recipe for source rendering.
    pub(crate) fn source_string_protocol(&self) -> tcl_syntax::native_string::NativeStringProtocol {
        self.producer.policy().string_protocol()
    }

    /// Identify only an original formal installed as this fresh activation's
    /// direct scalar slot. The caller separately proves the entered frame,
    /// uninterrupted source prefix and observer/alias effects. This supplies
    /// neither contents nor normal completion nor a native local-table token.
    pub(crate) fn fresh_scalar_formal_name(
        &self,
        receiver: &crate::place::Place,
        context: &crate::var_resolve::ResolveContext,
    ) -> bool {
        use crate::place::{CellGeneration, CellOwner, PlaceKind};
        use tcl_syntax::naming::{NativeNameContext, NativeVariableRootGeometry};
        let protocol = self.producer.policy().recipe();
        let Some(cell) = receiver.cell.as_ref() else {
            return false;
        };
        if context
            .invocation_dialect
            .and_then(InvocationDialect::parameter_grammar)
            != Some(self.grammar)
            || context
                .execution_name_policy
                .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
                .map(tcl_syntax::naming::NamePolicyProtocol::recipe)
                != Some(protocol)
            || receiver.kind != PlaceKind::Scalar
            || receiver.index.is_some()
            || receiver.dynamic
            || cell.generation == CellGeneration::Unknown
            || cell.generation
                != context
                    .generations
                    .get(&crate::var_resolve::cell_key(receiver))
                    .copied()
                    .unwrap_or_default()
            || !matches!(&cell.owner, CellOwner::Activation(identity)
                if context.activation.as_ref() == Some(identity))
        {
            return false;
        }
        let Ok(bindings) = self.bindings(self.parameters.len()) else {
            return false;
        };
        bindings.iter().any(|binding| {
            let name = match binding {
                FormalByteArgumentBinding::Value { parameter, .. }
                | FormalByteArgumentBinding::Default { parameter } => {
                    self.parameters[*parameter].name.as_slice()
                }
                FormalByteArgumentBinding::Rest { name, .. } => name.as_slice(),
                FormalByteArgumentBinding::CallerLink { .. } => return false,
            };
            let selected = protocol.combined_variable_input(name);
            selected.element().is_none()
                && matches!(protocol.variable_root_geometry(
                    NativeNameContext::new(&tcl_core_types::ByteNamespacePath::root()),
                    selected.root().selected()),
                    NativeVariableRootGeometry::Local(ref simple)
                    if simple.as_bytes() == name && cell.name.as_bytes() == name)
        })
    }

    #[cfg(test)]
    fn producer(&self) -> &SignatureSourceNameKey {
        &self.producer
    }

    pub(crate) fn bindings(
        &self,
        count: usize,
    ) -> Result<Vec<FormalByteArgumentBinding>, tcl_syntax::formal_params::FormalArityError> {
        tcl_syntax::formal_params::bind_formal_argument_bytes(&self.parameters, count, self.grammar)
    }

    pub(super) fn bind(
        &self,
        context: &mut crate::var_resolve::ResolveContext,
        actual: &[Option<&str>],
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<bool> {
        if context.invocation_dialect?.parameter_grammar()? != self.grammar {
            return None;
        }
        let Ok(bindings) = self.bindings(actual.len()) else {
            return Some(false);
        };
        context.original_formal_topology = Some(std::sync::Arc::new(self.clone()));
        for binding in bindings {
            let (name, value) = match binding {
                FormalByteArgumentBinding::Value {
                    parameter,
                    argument,
                } => (
                    self.parameters[parameter].name.as_slice(),
                    actual[argument].map(str::to_owned),
                ),
                FormalByteArgumentBinding::Default { parameter } => (
                    self.parameters[parameter].name.as_slice(),
                    self.parameters[parameter]
                        .default
                        .as_ref()
                        .and_then(|value| std::str::from_utf8(value).ok())
                        .map(str::to_owned),
                ),
                FormalByteArgumentBinding::Rest {
                    ref name,
                    start,
                    len,
                    ..
                } => {
                    let value = actual
                        .get(start..start + len)?
                        .iter()
                        .copied()
                        .collect::<Option<Vec<_>>>()
                        .map(tcl_syntax::list::join_list);
                    if context.bind_original_formal_incoming(
                        name,
                        value.as_deref(),
                        self.producer.policy().recipe(),
                        registry,
                    )? {
                        continue;
                    }
                    return Some(false);
                }
                FormalByteArgumentBinding::CallerLink {
                    ref name, argument, ..
                } => {
                    let name = std::str::from_utf8(name).ok()?;
                    match crate::variable_bindings::bind_caller_reference(
                        context,
                        name,
                        actual[argument],
                        registry,
                    ) {
                        crate::variable_bindings::CallerReferenceBinding::Bound => continue,
                        crate::variable_bindings::CallerReferenceBinding::Missing => {
                            return Some(false);
                        }
                        crate::variable_bindings::CallerReferenceBinding::Unknown => return None,
                    }
                }
            };
            if !context.bind_original_formal_incoming(
                name,
                value.as_deref(),
                self.producer.policy().recipe(),
                registry,
            )? {
                return Some(false);
            }
        }
        Some(true)
    }

    pub(super) fn seed_unknown(
        &self,
        context: &mut crate::var_resolve::ResolveContext,
        registry: &tcl_registry::CommandRegistry,
    ) {
        let actual = vec![None; self.parameters.len()];
        if self.bind(context, &actual, registry).is_none() {
            context.activation_contents_world = Some(crate::var_resolve::ContentsWorld::Unknown);
        }
    }

    /// Checked legacy metadata. Opaque names use a diagnostic label; all cell
    /// installation and argument binding use `parameters` and `bindings`.
    pub(crate) fn advisory_parameters(&self) -> Vec<tcl_syntax::formal_params::FormalParameter> {
        self.parameters
            .iter()
            .map(|formal| tcl_syntax::formal_params::FormalParameter {
                name: std::str::from_utf8(&formal.name).map_or_else(
                    |_| {
                        format!(
                            "{:?}",
                            tcl_core_types::NameBytes::from(formal.name.as_slice())
                        )
                    },
                    str::to_owned,
                ),
                default: formal
                    .default
                    .as_ref()
                    .and_then(|value| std::str::from_utf8(value).ok())
                    .map(str::to_owned),
            })
            .collect()
    }
}

pub(super) fn capture(
    image: &tcl_lexer::SourceImage,
    words: &[crate::ir::WordExpr],
    invocation: u32,
    parameter_word: usize,
    config: tcl_lexer::LexerConfig,
    dialect: InvocationDialect,
    policy: tcl_syntax::naming::NamePolicyProtocol,
) -> Option<OriginalFormalTopology> {
    let native = crate::registry_invocation::original_native_compiler_words(
        image, words, invocation, config,
    )?;
    OriginalFormalTopology::from_original_key(
        SignatureSourceNameKey::from_original_native_word(
            native.get(parameter_word)?,
            tcl_syntax::word_rules::WordValueRules::from_config(&config),
            policy,
        )?,
        dialect,
    )
}

pub(super) fn from_invocation(
    native: super::SourceNativeInvocation<'_>,
    facts: &tcl_registry::InvocationFacts,
    state: &super::ModuleCommandBindings,
    context: super::SourceExecutionContext<'_>,
) -> Option<OriginalFormalTopology> {
    let argument = facts.arg_roles.iter().find_map(|(index, role)| {
        (*role == tcl_registry::ArgRole::ParamList)
            .then_some(facts.argument_offset + usize::from(*index))
    })?;
    let word = native.script_operands().written_word(argument)?;
    let written = native
        .words
        .iter()
        .position(|owned| std::ptr::eq(owned, word))?;
    capture(
        state.current_source_origin.as_ref()?.source_image(),
        native.words,
        native.segment.span.start(),
        written,
        context.config,
        state.source_variables.invocation_dialect?,
        state
            .source_variables
            .execution_name_policy?
            .native_recipe()?,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_dialect(name: &str) -> InvocationDialect {
        let point = tcl_dialect::model::DialectPoint::of_dialect_name(Some(name))
            .expect("fixture selects an explicit stock engine point");
        InvocationDialect::of_point(point)
    }

    fn fixture(profile: &str, source: &[u8]) -> Option<OriginalFormalTopology> {
        let dialect = fixture_dialect(profile);
        let policy = dialect.authored_name_policy()?;
        let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
        let words = tcl_lexer::native_script_words_in(
            tcl_lexer::SourceImage::native(source),
            tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
            config,
        )
        .ok()?;
        OriginalFormalTopology::from_original_key(
            SignatureSourceNameKey::from_original_native_word(
                &words.commands[0].words[0],
                tcl_syntax::word_rules::WordValueRules::from_config(&config),
                policy,
            )?,
            dialect,
        )
    }

    #[test]
    // Implementation contract: naming.formal.original-topology-and-storage
    // docs/design/analysis/name-resolution-proofs/formal.original-topology-and-storage.md
    fn original_formal_topology_keeps_counted_storage_and_native_list_extent() {
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let topology = fixture(profile, b"{k\0tail second}").unwrap();
            let old = matches!(profile, "tcl8.4" | "tcl8.5");
            assert_eq!(
                topology.parameters()[0].name,
                if old { b"k".as_slice() } else { b"k\0tail" }
            );
            assert_eq!(topology.parameters().len(), if old { 1 } else { 2 });
            assert_eq!(topology.producer().bytes(), b"k\0tail second");
            let opaque = fixture(profile, br"{k\uD800}").unwrap();
            // Braced ParamList retains its escape until native list parsing.
            assert_eq!(opaque.parameters()[0].name, b"k\xed\xa0\x80");
        }
    }

    // Implementation contract: naming.formal.original-topology-and-storage
    // docs/design/analysis/name-resolution-proofs/formal.original-topology-and-storage.md
    #[test]
    fn original_formals_bind_duplicate_default_rest_and_jim_setter_roles() {
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let topology = fixture(profile, b"{a a {d DEFAULT} args}").unwrap();
            let bindings = topology.bindings(2).unwrap();
            assert_eq!(
                bindings
                    .iter()
                    .filter(|binding| matches!(binding, FormalByteArgumentBinding::Value { .. }))
                    .count(),
                if profile == "jim" { 2 } else { 1 }
            );
            assert!(bindings.iter().any(|binding| matches!(
                binding,
                FormalByteArgumentBinding::Default { parameter: 2 }
            )));
            assert!(
                bindings.iter().any(|binding| matches!(
                    binding,
                    FormalByteArgumentBinding::Rest { len: 0, .. }
                ))
            );
            assert!(topology.bindings(1).is_err());
            assert_eq!(
                fixture(profile, b"{::N::p a(k)}").is_some(),
                profile == "jim"
            );
        }
        assert!(fixture("tcl8.6", b"$parameters").is_none());
        assert!(fixture("tcl8.6", b"{*}{a}").is_none());
    }

    // Implementation contract: naming.formal.original-topology-and-storage
    // docs/design/analysis/name-resolution-proofs/formal.original-topology-and-storage.md
    #[test]
    fn decoded_formal_installation_uses_storage_keys_and_fresh_frame_only() {
        use crate::place::{CellGeneration, CellIdentity, CellOwner, PlaceKind};
        use crate::var_resolve::{ContentsOrigin, ContentsPresence, ResolveContext};
        use tcl_syntax::naming::ExecutionNamePolicy;
        let registry = tcl_registry::CommandRegistry::build_default();
        for profile_name in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let dialect = fixture_dialect(profile_name);
            let policy = dialect.authored_name_policy().unwrap();
            let mut context = ResolveContext::for_function("::f");
            context.invocation_dialect = Some(dialect);
            context.execution_name_policy = Some(ExecutionNamePolicy::NativeRecipe(policy));
            context.retain_namespace_world(
                super::super::SourceNamespaceKey::authored("::"),
                [super::super::SourceNamespaceKey::authored("::")],
                Some(policy.recipe()),
            );
            let topology = fixture(profile_name, br"{p\uD800}").unwrap();
            assert_eq!(
                topology.bind(&mut context, &[Some("VALUE")], &registry),
                Some(true),
                "{profile_name}"
            );
            let name = &topology.parameters()[0].name;
            let mut exact = crate::place::scalar("REPORT", crate::place::LOCAL_NS, false);
            exact.cell = Some(CellIdentity {
                owner: CellOwner::Activation(context.activation.clone().unwrap()),
                name: tcl_core_types::NameBytes::from(name.as_slice()),
                generation: CellGeneration::Incoming,
                interpreter: context.interpreter.clone(),
                storage_domain: None,
                execution: context.execution,
            });
            assert_eq!(context.contents_presence(&exact), ContentsPresence::Defined);
            assert_eq!(context.contents_origin(&exact), ContentsOrigin::Incoming);
            assert_eq!(
                context.literal_contents_at(&exact, &registry),
                Some("VALUE")
            );
            let mut other = exact.clone();
            other.cell.as_mut().unwrap().name = b"p\xed\xa0\x81".into();
            assert_ne!(
                crate::var_resolve::cell_key(&exact),
                crate::var_resolve::cell_key(&other)
            );
            assert_ne!(context.contents_presence(&other), ContentsPresence::Defined);
            context.execution_name_policy = None;
            assert_eq!(
                topology.bind(&mut context, &[Some("VALUE")], &registry),
                None
            );
            assert_eq!(exact.kind, PlaceKind::Scalar);
        }
    }
}

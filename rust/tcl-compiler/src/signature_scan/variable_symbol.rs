// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original source variable symbol geometry, independently of live cells.

use super::scope::{SignatureNamespaceScope, SignatureSourceNameInput};
use tcl_core_types::{ByteNamespacePath, NameBytes};
use tcl_syntax::naming::{NamePolicyProtocol, NativeNameProtocol, NativeVariableRootGeometry};

/// Namespace variable identity for source navigation. This is naming geometry;
/// it supplies no namespace incarnation, alias/lifetime, current read, object
/// representation, source completion or native compilation capability.
#[derive(Debug, Clone)]
pub struct SignatureSourceVariableSymbol {
    slot: SignatureSourceVariableSlot,
    policy: NamePolicyProtocol,
    rename_prefix: Option<NameBytes>,
}

impl PartialEq for SignatureSourceVariableSymbol {
    fn eq(&self, other: &Self) -> bool {
        self.slot == other.slot && self.policy == other.policy
    }
}
impl Eq for SignatureSourceVariableSymbol {}
impl std::hash::Hash for SignatureSourceVariableSymbol {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hash::hash(&self.slot, state);
        std::hash::Hash::hash(&self.policy, state);
    }
}

/// Purpose-selected native table geometry. Jim keys are root-flat variables,
/// independently of command qualification and rendered rooted names.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SignatureSourceVariableSlot {
    /// C variable namespace components and the selected simple root key.
    C {
        /// Constructed native namespace components, never split from display.
        namespace: ByteNamespacePath,
        /// Exact selected variable root key, independently of an array index.
        simple: NameBytes,
    },
    /// Jim's exact global variable table key, without a fabricated root marker.
    Jim(NameBytes),
    /// Original procedure/method declaration frame and its exact local key.
    Local {
        /// Sealed original body identity, never a rendered procedure name.
        frame: crate::command_binding::SourceOriginalVariableFrame,
        /// Selected original local root key, independently of element indices.
        simple: NameBytes,
    },
}

/// The source scope owns whether an unqualified operand names namespace
/// storage. Qualified references in procedure frames cannot donate that fact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OriginalVariableSymbolPurpose {
    NamespaceStorage,
    QualifiedOnly,
}

/// Independently selected original variable receiver purpose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OriginalVariableSymbolReceiver {
    /// Authentic lexical root; its separate-index flag remains independent.
    LexicalRoot,
    /// Actual Registry-selected argv receiver form.
    Operand(tcl_registry::resolved_invocation::VariableReceiverOperandForm),
    /// Namespace/global alias target in its independently selected namespace.
    NamespaceAlias,
    /// Namespace variable setter without a local link at the true global frame.
    NamespaceVariableWithoutLink,
    /// Name child of an independently retained original `ParamList` topology.
    FormalDeclaration,
}

impl OriginalVariableSymbolReceiver {
    /// Original root geometry selected by this source receiver. A lexical
    /// root, argv trace subject and namespace setter keep distinct purposes.
    #[must_use]
    pub fn input_form(
        self,
        input: &SignatureSourceNameInput,
    ) -> Option<tcl_syntax::naming::NativeVariableInputForm<'_>> {
        use tcl_syntax::naming::NativeVariableInputForm;
        let protocol = input.policy().recipe();
        match self {
            Self::LexicalRoot => {
                let SignatureSourceNameInput::OriginalVariableRoot(root) = input else {
                    return None;
                };
                Some(if root.is_separate_array_root() {
                    NativeVariableInputForm::Separate {
                        root: input.bytes(),
                        element: None,
                    }
                } else {
                    NativeVariableInputForm::Combined(input.bytes())
                })
            }
            Self::Operand(form) => {
                (!matches!(input, SignatureSourceNameInput::OriginalVariableRoot(_)))
                    .then_some(())?;
                form.input_form(protocol, input.bytes())
            }
            Self::NamespaceAlias | Self::NamespaceVariableWithoutLink => {
                (!matches!(input, SignatureSourceNameInput::OriginalVariableRoot(_)))
                    .then_some(())?;
                let bytes = if self == Self::NamespaceVariableWithoutLink && protocol.is_jim084() {
                    protocol.namespace_tail_bytes(input.bytes())
                } else {
                    input.bytes()
                };
                Some(NativeVariableInputForm::Combined(bytes))
            }
            Self::FormalDeclaration => None,
        }
    }
    fn written_input_form(
        self,
        input: &SignatureSourceNameInput,
    ) -> Option<tcl_syntax::naming::NativeVariableInputForm<'_>> {
        if self == Self::NamespaceVariableWithoutLink {
            // The namespace setter selects a tail for its target, while the
            // independent written operand retains its qualifier for an edit.
            Some(tcl_syntax::naming::NativeVariableInputForm::Combined(
                input.bytes(),
            ))
        } else {
            self.input_form(input)
        }
    }
}

/// Conditional Registry-selected source write-name card. The retained input
/// and receiver form describe a possible original handler; they do not prove
/// a store, a selected cell, alias relationship or editable reference set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalVariableWriteAdvice {
    span: tcl_lexer::Span,
    input: SignatureSourceNameInput,
    form: tcl_registry::resolved_invocation::VariableReceiverOperandForm,
    frame: Option<crate::command_binding::SourceOriginalVariableFrame>,
    namespace: Option<SignatureNamespaceScope>,
    conditional_metadata: Option<crate::registry_invocation::OriginalConditionalRegistryMetadata>,
}

impl OriginalVariableWriteAdvice {
    pub(crate) fn new(
        span: tcl_lexer::Span,
        input: SignatureSourceNameInput,
        form: tcl_registry::resolved_invocation::VariableReceiverOperandForm,
        frame: Option<crate::command_binding::SourceOriginalVariableFrame>,
        namespace: Option<SignatureNamespaceScope>,
        conditional_metadata: Option<
            crate::registry_invocation::OriginalConditionalRegistryMetadata,
        >,
    ) -> Self {
        Self {
            span,
            input,
            form,
            frame,
            namespace,
            conditional_metadata,
        }
    }

    /// Original written receiver extent in the consumer source.
    #[must_use]
    pub const fn span(&self) -> tcl_lexer::Span {
        self.span
    }
    /// Exact independently retained name producer; no display-name bridge.
    #[must_use]
    pub const fn original_name_input(&self) -> &SignatureSourceNameInput {
        &self.input
    }
    /// Naming form of the conditional Registry handler, without guest Normal.
    #[must_use]
    pub const fn receiver_form(
        &self,
    ) -> tcl_registry::resolved_invocation::VariableReceiverOperandForm {
        self.form
    }
    /// Authentic lexical frame when its body recipe is available.
    #[must_use]
    pub fn original_frame(&self) -> Option<&crate::command_binding::SourceOriginalVariableFrame> {
        self.frame.as_ref()
    }
    /// Full conditional Registry source schema and unresolved applicability.
    /// This retains the actual analysis context without selecting a handler,
    /// cell, successful store or editable reference set.
    #[must_use]
    pub const fn conditional_metadata(
        &self,
    ) -> Option<&crate::registry_invocation::OriginalConditionalRegistryMetadata> {
        self.conditional_metadata.as_ref()
    }
    /// Independently retained lexical namespace; not the selected variable cell.
    #[must_use]
    pub fn original_namespace(&self) -> Option<&SignatureNamespaceScope> {
        self.namespace.as_ref()
    }
}

impl SignatureSourceVariableSymbol {
    pub(crate) fn from_original_input(
        input: &SignatureSourceNameInput,
        namespace: &SignatureNamespaceScope,
        purpose: OriginalVariableSymbolPurpose,
        receiver: OriginalVariableSymbolReceiver,
    ) -> Option<Self> {
        let policy = input.policy();
        if !matches!(
            (namespace, policy.recipe()),
            (SignatureNamespaceScope::C(_), NativeNameProtocol::C(_))
                | (SignatureNamespaceScope::Jim(_), NativeNameProtocol::Jim084)
        ) {
            return None;
        }
        let protocol = policy.recipe();
        let selected = match receiver.input_form(input)? {
            tcl_syntax::naming::NativeVariableInputForm::Combined(bytes) => {
                protocol.combined_variable_input(bytes)
            }
            tcl_syntax::naming::NativeVariableInputForm::Separate { root, element } => {
                protocol.separate_variable_input(root, element)
            }
        };
        let slot = match protocol
            .variable_root_geometry(namespace.context()?, selected.root().selected())
        {
            NativeVariableRootGeometry::CNamespace { namespace, simple } => {
                SignatureSourceVariableSlot::C { namespace, simple }
            }
            NativeVariableRootGeometry::JimAbsolute(key) => SignatureSourceVariableSlot::Jim(key),
            NativeVariableRootGeometry::Local(simple) => {
                if purpose != OriginalVariableSymbolPurpose::NamespaceStorage {
                    return None;
                }
                match namespace {
                    SignatureNamespaceScope::C(namespace) => SignatureSourceVariableSlot::C {
                        namespace: namespace.clone(),
                        simple,
                    },
                    SignatureNamespaceScope::Jim(namespace) => SignatureSourceVariableSlot::Jim(
                        tcl_syntax::naming::jim_global_variable_key_bytes(
                            namespace.as_bytes(),
                            simple.as_bytes(),
                        )
                        .into(),
                    ),
                    SignatureNamespaceScope::Symbolic(_) => return None,
                }
            }
        };
        let rename_prefix = match (
            &slot,
            protocol.variable_root_geometry(namespace.context()?, selected.root().selected()),
        ) {
            (SignatureSourceVariableSlot::Jim(key), NativeVariableRootGeometry::JimAbsolute(_)) => {
                // This is a partition of the original written root, not a
                // namespace projection of Jim's intrinsic flat table key.
                let tail = tcl_syntax::naming::written_command_tail(selected.root().selected());
                Some(
                    key.as_bytes()
                        .get(..key.len().checked_sub(tail.len())?)?
                        .into(),
                )
            }
            (SignatureSourceVariableSlot::Jim(_), NativeVariableRootGeometry::Local(_)) => {
                let SignatureNamespaceScope::Jim(home) = namespace else {
                    return None;
                };
                Some(tcl_syntax::naming::jim_global_variable_key_bytes(home.as_bytes(), b"").into())
            }
            _ => None,
        };
        Some(Self {
            slot,
            policy,
            rename_prefix,
        })
    }

    pub(crate) fn from_original_local_input(
        input: &SignatureSourceNameInput,
        frame: crate::command_binding::SourceOriginalVariableFrame,
        simple: NameBytes,
    ) -> Self {
        Self {
            slot: SignatureSourceVariableSlot::Local { frame, simple },
            policy: input.policy(),
            rename_prefix: None,
        }
    }

    /// Whether this symbol denotes namespace storage rather than a local frame.
    #[must_use]
    pub const fn is_namespace(&self) -> bool {
        !matches!(self.slot, SignatureSourceVariableSlot::Local { .. })
    }

    /// Selected rename tail, independently of a display or a Jim namespace
    /// interpretation. Jim requires the retained original root partition.
    #[must_use]
    pub fn rename_tail(&self) -> Option<&[u8]> {
        match &self.slot {
            SignatureSourceVariableSlot::C { simple, .. }
            | SignatureSourceVariableSlot::Local { simple, .. } => Some(simple.as_bytes()),
            SignatureSourceVariableSlot::Jim(key) => key
                .as_bytes()
                .strip_prefix(self.rename_prefix.as_ref()?.as_bytes()),
        }
    }

    /// Change only the independently selected root key. A proposed tail must
    /// remain one scalar, unqualified native key under the retained policy.
    /// No current table, lifetime, alias or completion authority is supplied.
    #[must_use]
    pub fn renamed(&self, new_native_tail: &[u8]) -> Option<Self> {
        let protocol = self.policy.recipe();
        let selected = protocol.combined_variable_input(new_native_tail);
        if selected.element().is_some()
            || selected.root().selected() != new_native_tail
            || !matches!(protocol.variable_root_geometry(tcl_syntax::naming::NativeNameContext::root(), new_native_tail), NativeVariableRootGeometry::Local(ref simple) if simple.as_bytes() == new_native_tail)
            || tcl_syntax::naming::is_qualified(new_native_tail)
        {
            return None;
        }
        let slot = match &self.slot {
            SignatureSourceVariableSlot::C { namespace, .. } => SignatureSourceVariableSlot::C {
                namespace: namespace.clone(),
                simple: new_native_tail.into(),
            },
            SignatureSourceVariableSlot::Local { frame, .. } => {
                SignatureSourceVariableSlot::Local {
                    frame: frame.clone(),
                    simple: new_native_tail.into(),
                }
            }
            SignatureSourceVariableSlot::Jim(_) => {
                let mut key = self.rename_prefix.as_ref()?.as_bytes().to_vec();
                key.extend_from_slice(new_native_tail);
                SignatureSourceVariableSlot::Jim(key.into())
            }
        };
        Some(Self {
            slot,
            policy: self.policy,
            rename_prefix: self.rename_prefix.clone(),
        })
    }

    /// Exact selected byte geometry, without a display reconstruction.
    #[must_use]
    pub const fn slot(&self) -> &SignatureSourceVariableSlot {
        &self.slot
    }

    /// Retained naming recipe and its independent authority.
    #[must_use]
    pub const fn policy(&self) -> NamePolicyProtocol {
        self.policy
    }
}

/// Source-only local alias candidate from its authentic declaration receipt.
/// A complete read is unnecessary; this supplies no entered link, selected
/// access, current cell, value or editable reference set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OriginalVariableAliasAdvice<'a> {
    receipt: &'a OriginalVariableAliasReceipt,
}

impl<'a> OriginalVariableAliasAdvice<'a> {
    pub(crate) const fn from_receipt(receipt: &'a OriginalVariableAliasReceipt) -> Self {
        Self { receipt }
    }

    /// Original declaration frame owning this local spelling.
    #[must_use]
    pub fn original_frame(&self) -> &'a crate::command_binding::SourceOriginalVariableFrame {
        self.receipt.frame()
    }

    /// Purpose-selected local name, independently of the target cell spelling.
    #[must_use]
    pub fn local_name(&self) -> &'a NameBytes {
        self.receipt.local()
    }

    /// Genuine complete operand which introduced the local alias spelling.
    #[must_use]
    pub const fn original_name_input(&self) -> &'a SignatureSourceNameInput {
        &self.receipt.local_input
    }

    /// Actual local-name declaration extent in its original source owner.
    #[must_use]
    pub const fn span(&self) -> tcl_lexer::Span {
        self.receipt.local_span
    }
}

/// Unresolved premises of an original alias-name template. These cannot be
/// consumed as an entered link, selected element or refactoring certificate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OriginalVariableAliasTemplateObligation {
    /// The written alias operation may fail or never execute.
    WrittenAliasApplicability,
    /// Source target-root geometry does not establish an actual target cell.
    UnavailableTargetCell,
    /// Callbacks and unknown earlier effects may change the binding.
    UnprovedBindingContinuity,
}

/// Conditional source alias relationship for readonly navigation. It retains
/// the real declaration, name operands and frame without selecting a target
/// element, successful link, entered activation or editable reference set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalVariableAliasTemplate {
    span: tcl_lexer::Span,
    input: SignatureSourceNameInput,
    alias: OriginalVariableAliasReceipt,
}

impl OriginalVariableAliasTemplate {
    pub(crate) fn new(
        span: tcl_lexer::Span,
        input: SignatureSourceNameInput,
        alias: OriginalVariableAliasReceipt,
    ) -> Self {
        Self { span, input, alias }
    }
    /// Explicit unresolved source applicability, independent of target naming.
    #[must_use]
    pub const fn obligations(&self) -> &'static [OriginalVariableAliasTemplateObligation] {
        &[
            OriginalVariableAliasTemplateObligation::WrittenAliasApplicability,
            OriginalVariableAliasTemplateObligation::UnavailableTargetCell,
            OriginalVariableAliasTemplateObligation::UnprovedBindingContinuity,
        ]
    }
    /// Actual original local-name occurrence, never reconstructed from a label.
    #[must_use]
    pub const fn span(&self) -> tcl_lexer::Span {
        self.span
    }
    /// Original local source input, independently of the target name operand.
    #[must_use]
    pub const fn original_name_input(&self) -> &SignatureSourceNameInput {
        &self.input
    }
    /// Original body declaration frame; this is not an entered frame.
    #[must_use]
    pub fn original_frame(&self) -> &crate::command_binding::SourceOriginalVariableFrame {
        self.alias.frame()
    }
    /// Target's original variable-root naming geometry, without a cell identity.
    #[must_use]
    pub fn target_symbol(&self) -> &SignatureSourceVariableSymbol {
        self.alias.target()
    }
    /// Genuine target producer. A compound array root retains its unknown
    /// evaluated index independently and cannot become a whole argv value.
    #[must_use]
    pub const fn target_name_input(&self) -> &SignatureSourceNameInput {
        &self.alias.target_input
    }
    /// Purpose-selected local key supplied by the real alias declaration.
    #[must_use]
    pub fn local_name(&self) -> &NameBytes {
        self.alias.local()
    }
    /// Whether this occurrence is the local alias declaration itself.
    #[must_use]
    pub fn is_declaration(&self) -> bool {
        self.span == self.alias.local_span
    }
}

/// The alias declaration owner supplies local-name projection to both source
/// receipts and source-order blockers. It creates no executed alias or cell.
pub(crate) fn original_variable_alias_local_name(
    input: &SignatureSourceNameInput,
    purpose: crate::registry_invocation::DeclarationVariableAliasPurpose,
) -> Option<NameBytes> {
    use crate::registry_invocation::DeclarationVariableAliasPurpose as Purpose;
    use tcl_syntax::naming::NativeNameContext;
    let protocol = input.policy().recipe();
    let bytes = match purpose {
        Purpose::Global => tcl_syntax::naming::global_local_name_bytes(protocol, input.bytes())?,
        Purpose::NamespaceVariable => {
            tcl_syntax::naming::variable_local_name_bytes(protocol, input.bytes())
        }
        Purpose::Upvar => input.bytes().to_vec(),
        Purpose::NamespaceUpvar => protocol
            .namespace_upvar_local_input(input.bytes())
            .ok()?
            .selected()
            .to_vec(),
    };
    let selected = protocol.variable_alias_local_input(&bytes);
    let NativeVariableRootGeometry::Local(local) =
        protocol.variable_root_geometry(NativeNameContext::root(), selected.selected())
    else {
        return None;
    };
    let parsed = protocol.combined_variable_input(local.as_bytes());
    (parsed.element().is_none() && parsed.root().selected() == local.as_bytes()).then_some(local)
}

/// Immutable original alias declaration relationship. It does not prove an
/// executed link; reads must independently match their exact retained point.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OriginalVariableAliasReceipt {
    frame: crate::command_binding::SourceOriginalVariableFrame,
    local: NameBytes,
    target: SignatureSourceVariableSymbol,
    target_input: SignatureSourceNameInput,
    local_input: SignatureSourceNameInput,
    derived: bool,
    invocation_offset: u32,
    local_span: tcl_lexer::Span,
}
impl OriginalVariableAliasReceipt {
    pub(crate) fn new(
        frame: crate::command_binding::SourceOriginalVariableFrame,
        local: NameBytes,
        target: SignatureSourceVariableSymbol,
        target_input: SignatureSourceNameInput,
        local_input: SignatureSourceNameInput,
        derived: bool,
        source_site: (u32, tcl_lexer::Span),
    ) -> Self {
        Self {
            frame,
            local,
            target,
            target_input,
            local_input,
            derived,
            invocation_offset: source_site.0,
            local_span: source_site.1,
        }
    }
    pub(crate) fn frame(&self) -> &crate::command_binding::SourceOriginalVariableFrame {
        &self.frame
    }
    pub(crate) fn local(&self) -> &NameBytes {
        &self.local
    }
    pub(crate) fn target(&self) -> &SignatureSourceVariableSymbol {
        &self.target
    }
    pub(crate) const fn invocation_offset(&self) -> u32 {
        self.invocation_offset
    }
}

/// One original source naming occurrence and its independent geometry. A
/// lexical variable root never becomes a complete word or an editable value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureSourceVariableOccurrence {
    span: tcl_lexer::Span,
    input: SignatureSourceNameInput,
    namespace: Option<SignatureNamespaceScope>,
    receiver: OriginalVariableSymbolReceiver,
    symbol: SignatureSourceVariableSymbol,
    declaration: bool,
    alias: Option<OriginalVariableAliasReceipt>,
    formal: Option<crate::command_binding::formal_topology::OriginalFormalNameRecipe>,
}

impl SignatureSourceVariableOccurrence {
    pub(crate) fn from_original_namespace_cell(
        span: tcl_lexer::Span,
        input: SignatureSourceNameInput,
        namespace: SignatureNamespaceScope,
        selected_namespace: &SignatureNamespaceScope,
        simple: NameBytes,
        receiver: OriginalVariableSymbolReceiver,
        declaration: bool,
    ) -> Option<Self> {
        let policy = input.policy();
        let slot = match (selected_namespace, policy.recipe()) {
            (SignatureNamespaceScope::C(namespace), NativeNameProtocol::C(_)) => {
                SignatureSourceVariableSlot::C {
                    namespace: namespace.clone(),
                    simple,
                }
            }
            (SignatureNamespaceScope::Jim(_), NativeNameProtocol::Jim084) => {
                SignatureSourceVariableSlot::Jim(simple)
            }
            _ => return None,
        };
        let mut symbol = SignatureSourceVariableSymbol::from_original_input(
            &input,
            &namespace,
            OriginalVariableSymbolPurpose::NamespaceStorage,
            receiver,
        )?;
        // The exact observed owner supplies storage selection, including C8's
        // global fallback. Written qualification remains an independent edit.
        symbol.slot = slot;
        Some(Self {
            span,
            input,
            namespace: Some(namespace),
            receiver,
            symbol,
            declaration,
            alias: None,
            formal: None,
        })
    }

    pub(crate) fn from_original_input(
        span: tcl_lexer::Span,
        input: SignatureSourceNameInput,
        namespace: SignatureNamespaceScope,
        purpose: OriginalVariableSymbolPurpose,
        receiver: OriginalVariableSymbolReceiver,
        declaration: bool,
    ) -> Option<Self> {
        let symbol = SignatureSourceVariableSymbol::from_original_input(
            &input, &namespace, purpose, receiver,
        )?;
        Some(Self {
            span,
            input,
            namespace: Some(namespace),
            receiver,
            symbol,
            declaration,
            alias: None,
            formal: None,
        })
    }

    pub(crate) fn from_original_local_input(
        span: tcl_lexer::Span,
        input: SignatureSourceNameInput,
        namespace: Option<SignatureNamespaceScope>,
        frame: crate::command_binding::SourceOriginalVariableFrame,
        simple: NameBytes,
        receiver: OriginalVariableSymbolReceiver,
        declaration: bool,
    ) -> Self {
        let symbol =
            SignatureSourceVariableSymbol::from_original_local_input(&input, frame, simple);
        Self {
            span,
            input,
            namespace,
            receiver,
            symbol,
            declaration,
            alias: None,
            formal: None,
        }
    }

    pub(crate) fn from_original_alias_input(
        span: tcl_lexer::Span,
        input: SignatureSourceNameInput,
        namespace: Option<SignatureNamespaceScope>,
        receiver: OriginalVariableSymbolReceiver,
        alias: OriginalVariableAliasReceipt,
    ) -> Self {
        Self {
            span,
            input,
            namespace,
            receiver,
            symbol: alias.target.clone(),
            declaration: false,
            alias: Some(alias),
            formal: None,
        }
    }
    pub(crate) fn from_original_formal_input(
        span: tcl_lexer::Span,
        input: SignatureSourceNameInput,
        frame: crate::command_binding::SourceOriginalVariableFrame,
        simple: NameBytes,
        recipe: crate::command_binding::formal_topology::OriginalFormalNameRecipe,
    ) -> Self {
        let symbol =
            SignatureSourceVariableSymbol::from_original_local_input(&input, frame, simple);
        Self {
            span,
            input,
            namespace: None,
            receiver: OriginalVariableSymbolReceiver::FormalDeclaration,
            symbol,
            declaration: true,
            alias: None,
            formal: Some(recipe),
        }
    }

    /// Whether this source spelling follows the target's renamed root. An
    /// independently named upvar local remains unchanged in a target rename.
    #[must_use]
    pub fn rename_changes_spelling(&self) -> bool {
        self.alias.as_ref().is_none_or(|alias| alias.derived)
    }

    /// Exact original rename-tail extent for editor preparation. Independently
    /// named alias locals and readonly computed values cannot supply a tail
    /// span. Formal fields retain their authentic list-child extent.
    #[must_use]
    pub fn rename_span(&self) -> Option<tcl_lexer::Span> {
        self.rename_changes_spelling().then_some(())?;
        (self.renamed_input(self.symbol.rename_tail()?)?.as_slice() == self.input.bytes())
            .then_some(())?;
        if self.formal.is_some() {
            return Some(self.span);
        }
        let form = self.receiver.written_input_form(&self.input)?;
        let extent = self
            .input
            .policy()
            .recipe()
            .variable_root_tail_extent(form)?;
        match &self.input {
            SignatureSourceNameInput::OriginalVariableRoot(root) => {
                let name = root.name_span()?;
                let raw = root.source_image().bytes().get(name.as_range())?;
                let selected = tcl_syntax::backslash::native_source_literal_extent(
                    raw,
                    root.source_image().channel(),
                    root.policy().string_protocol(),
                    extent,
                )?;
                Some(tcl_lexer::Span::new(
                    name.start()
                        .checked_add(u32::try_from(selected.start).ok()?)?,
                    name.start()
                        .checked_add(u32::try_from(selected.end).ok()?)?,
                ))
            }
            SignatureSourceNameInput::OriginalWord(key) => {
                tcl_registry::native_compiler_words::NativeCompilerWords::capture(
                    std::slice::from_ref(key.original_word()),
                    key.policy().string_protocol(),
                )
                .ok()?
                .original_literal_extent(0, extent)
            }
            SignatureSourceNameInput::OriginalValue(_) => None,
        }
    }

    pub(crate) fn merge_declaration(&mut self, other: &Self) -> bool {
        if self.span != other.span
            || self.input != other.input
            || self.namespace != other.namespace
            || self.alias != other.alias
            || self.formal != other.formal
            || self.receiver != other.receiver
            || self.symbol != other.symbol
        {
            return false;
        }
        self.declaration |= other.declaration;
        true
    }

    pub(crate) fn retain_declaration(&mut self) {
        if self.receiver
            != OriginalVariableSymbolReceiver::Operand(
                tcl_registry::resolved_invocation::VariableReceiverOperandForm::TraceSubject,
            )
        {
            self.declaration = true;
        }
    }

    /// Exact native replacement input for this selected occurrence. Source
    /// quoting and lexical-root editing stay with their independent edit owner.
    /// A differently named alias cannot be manufactured as this occurrence.
    #[must_use]
    pub fn renamed_input(&self, new_native_tail: &[u8]) -> Option<Vec<u8>> {
        use tcl_syntax::naming::NativeVariableInputForm;
        let target = self.symbol.renamed(new_native_tail)?;
        if let Some(formal) = &self.formal {
            return formal.renamed_input(new_native_tail);
        }
        if self.alias.as_ref().is_some_and(|alias| !alias.derived) {
            return Some(self.input.bytes().to_vec());
        }
        let form = self.receiver.written_input_form(&self.input)?;
        let protocol = self.input.policy().recipe();
        let mut proposed = protocol.replace_variable_root_tail(form, new_native_tail)?;
        if self.receiver
            == OriginalVariableSymbolReceiver::Operand(
                tcl_registry::resolved_invocation::VariableReceiverOperandForm::TraceSubject,
            )
        {
            let NativeVariableInputForm::Combined(selected) = form else {
                return None;
            };
            proposed.extend_from_slice(self.input.bytes().get(selected.len()..)?);
        }
        // Keep qualification and the independently retained local frame. A
        // proposed root cannot silently redirect the selected namespace.
        let selected = match self.receiver {
            OriginalVariableSymbolReceiver::LexicalRoot
                if matches!(form, NativeVariableInputForm::Separate { .. }) =>
            {
                self.input
                    .policy()
                    .recipe()
                    .separate_variable_input(&proposed, None)
            }
            OriginalVariableSymbolReceiver::Operand(form) => {
                let NativeVariableInputForm::Combined(selected) =
                    form.input_form(protocol, &proposed)?
                else {
                    return None;
                };
                protocol.combined_variable_input(selected)
            }
            _ => protocol.combined_variable_input(&proposed),
        };
        if self.alias.is_some() {
            return (selected.root().selected() == new_native_tail).then_some(proposed);
        }
        match target.slot() {
            SignatureSourceVariableSlot::Local { simple, .. } => {
                (selected.root().selected() == simple.as_bytes()).then_some(proposed)
            }
            SignatureSourceVariableSlot::C { namespace, simple } => {
                let context = self.namespace.as_ref()?.context()?;
                match self
                    .input
                    .policy()
                    .recipe()
                    .variable_root_geometry(context, selected.root().selected())
                {
                    NativeVariableRootGeometry::CNamespace {
                        namespace: new_namespace,
                        simple: new_simple,
                    } if &new_namespace == namespace && &new_simple == simple => Some(proposed),
                    NativeVariableRootGeometry::Local(new_simple)
                        if self.namespace
                            == Some(SignatureNamespaceScope::C(namespace.clone()))
                            && &new_simple == simple =>
                    {
                        Some(proposed)
                    }
                    _ => None,
                }
            }
            SignatureSourceVariableSlot::Jim(key) => {
                let SignatureNamespaceScope::Jim(home) = self.namespace.as_ref()? else {
                    return None;
                };
                let root = if self.receiver
                    == OriginalVariableSymbolReceiver::NamespaceVariableWithoutLink
                {
                    self.input
                        .policy()
                        .recipe()
                        .namespace_tail_bytes(selected.root().selected())
                } else {
                    selected.root().selected()
                };
                (tcl_syntax::naming::jim_global_variable_key_bytes(home.as_bytes(), root)
                    == key.as_bytes())
                .then_some(proposed)
            }
        }
    }

    /// The actual original occurrence extent, in its retained source image.
    #[must_use]
    pub const fn span(&self) -> tcl_lexer::Span {
        self.span
    }
    /// Immutable original producer, preserving lexical versus operand kind.
    #[must_use]
    pub const fn original_name_input(&self) -> &SignatureSourceNameInput {
        &self.input
    }
    /// Independently retained original caller namespace geometry.
    #[must_use]
    pub fn original_namespace(&self) -> Option<&SignatureNamespaceScope> {
        self.namespace.as_ref()
    }
    /// Actual selected naming form; a nominal array role does not supply it.
    #[must_use]
    pub const fn receiver(&self) -> OriginalVariableSymbolReceiver {
        self.receiver
    }

    /// Purpose-selected readonly variable symbol, without live cell authority.
    #[must_use]
    pub const fn symbol(&self) -> &SignatureSourceVariableSymbol {
        &self.symbol
    }

    /// Original local alias spelling and declaration frame for lexical
    /// completion. This retains no entered link, live cell or value authority.
    #[must_use]
    pub fn original_local_alias(
        &self,
    ) -> Option<(
        &crate::command_binding::SourceOriginalVariableFrame,
        &NameBytes,
    )> {
        self.alias
            .as_ref()
            .map(|alias| (alias.frame(), alias.local()))
    }

    /// Original local-name operand introducing this selected alias. Its
    /// spelling can differ from the target cell; navigation to this scoping
    /// declaration does not rename or merge that independent local name.
    #[must_use]
    pub fn original_alias_declaration_span(&self) -> Option<tcl_lexer::Span> {
        self.alias.as_ref().map(|alias| alias.local_span)
    }
    /// An authentic source name-role definition occurs at this same span.
    #[must_use]
    pub const fn is_declaration(&self) -> bool {
        self.declaration
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_lexer::{LexerConfig, SourceImage, Span};
    use tcl_syntax::word_rules::WordValueRules;

    fn original_word(profile: &str, source: &[u8]) -> SignatureSourceNameInput {
        let dialect = tcl_registry::InvocationDialect::of_point(
            tcl_dialect::model::DialectPoint::of_dialect_name(Some(profile)).unwrap(),
        );
        let config = LexerConfig::from_grammar(dialect.lexer_grammar);
        let image = SourceImage::native(source);
        let plan = tcl_lexer::native_script_words_in(
            image.clone(),
            Span::new(0, u32::try_from(image.len()).unwrap()),
            config,
        )
        .unwrap();
        SignatureSourceNameInput::OriginalWord(
            super::super::scope::SignatureSourceNameKey::from_original_native_word(
                &plan.commands[0].words[0],
                WordValueRules::from_config(&config),
                dialect.authored_name_policy().unwrap(),
            )
            .unwrap(),
        )
    }

    #[test]
    fn original_variable_symbols_keep_native_namespace_and_flat_jim_keys() {
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let raw = original_word(profile, b"{::N::v\0tail}");
            let encoded = original_word(profile, b"::N::v\\u0000tail");
            let context = match raw.policy().recipe() {
                NativeNameProtocol::C(_) => SignatureNamespaceScope::C(ByteNamespacePath::root()),
                NativeNameProtocol::Jim084 => SignatureNamespaceScope::Jim(b"".as_slice().into()),
            };
            let symbol = SignatureSourceVariableSymbol::from_original_input(
                &raw,
                &context,
                OriginalVariableSymbolPurpose::QualifiedOnly,
                OriginalVariableSymbolReceiver::Operand(
                    tcl_registry::resolved_invocation::VariableReceiverOperandForm::Combined,
                ),
            )
            .unwrap();
            let numeric = SignatureSourceVariableSymbol::from_original_input(
                &encoded,
                &context,
                OriginalVariableSymbolPurpose::QualifiedOnly,
                OriginalVariableSymbolReceiver::Operand(
                    tcl_registry::resolved_invocation::VariableReceiverOperandForm::Combined,
                ),
            )
            .unwrap();
            match symbol.slot() {
                SignatureSourceVariableSlot::C { namespace, simple } => {
                    assert_eq!(
                        *namespace,
                        ByteNamespacePath::from_segments([b"N".as_slice()])
                    );
                    assert_eq!(simple.as_bytes(), b"v");
                    let SignatureSourceVariableSlot::C { simple, .. } = numeric.slot() else {
                        panic!("C recipe");
                    };
                    assert_eq!(simple.as_bytes(), b"v\xc0\x80tail");
                    assert_ne!(symbol, numeric);
                }
                SignatureSourceVariableSlot::Local { .. } => {
                    panic!("qualified namespace input selected a local frame")
                }
                SignatureSourceVariableSlot::Jim(key) => {
                    assert_eq!(key.as_bytes(), b"N::v\0tail");
                    assert_eq!(symbol, numeric);
                }
            }
            assert_eq!(symbol.policy(), raw.policy());
            let local = original_word(profile, b"{relative::v}");
            let local_symbol = SignatureSourceVariableSymbol::from_original_input(
                &local,
                &context,
                OriginalVariableSymbolPurpose::QualifiedOnly,
                OriginalVariableSymbolReceiver::Operand(
                    tcl_registry::resolved_invocation::VariableReceiverOperandForm::Combined,
                ),
            );
            assert_eq!(local_symbol.is_none(), profile == "jim");
            assert!(
                SignatureSourceVariableSymbol::from_original_input(
                    &raw,
                    &SignatureNamespaceScope::Symbolic("::N".into()),
                    OriginalVariableSymbolPurpose::NamespaceStorage,
                    OriginalVariableSymbolReceiver::Operand(
                        tcl_registry::resolved_invocation::VariableReceiverOperandForm::Combined
                    )
                )
                .is_none()
            );
        }
    }

    #[test]
    fn unqualified_variable_symbols_require_independent_namespace_storage_purpose() {
        for profile in ["tcl8.6", "jim"] {
            let input = original_word(profile, b"v\0tail");
            let context = if profile == "jim" {
                SignatureNamespaceScope::Jim(b"N".as_slice().into())
            } else {
                SignatureNamespaceScope::C(ByteNamespacePath::from_segments([b"N".as_slice()]))
            };
            assert!(
                SignatureSourceVariableSymbol::from_original_input(
                    &input,
                    &context,
                    OriginalVariableSymbolPurpose::QualifiedOnly,
                    OriginalVariableSymbolReceiver::Operand(
                        tcl_registry::resolved_invocation::VariableReceiverOperandForm::Combined
                    )
                )
                .is_none()
            );
            let symbol = SignatureSourceVariableSymbol::from_original_input(
                &input,
                &context,
                OriginalVariableSymbolPurpose::NamespaceStorage,
                OriginalVariableSymbolReceiver::Operand(
                    tcl_registry::resolved_invocation::VariableReceiverOperandForm::Combined,
                ),
            )
            .unwrap();
            match symbol.slot() {
                SignatureSourceVariableSlot::C { namespace, simple } => {
                    assert_eq!(
                        *namespace,
                        ByteNamespacePath::from_segments([b"N".as_slice()])
                    );
                    assert_eq!(simple.as_bytes(), b"v\0tail");
                }
                SignatureSourceVariableSlot::Jim(key) => assert_eq!(key.as_bytes(), b"N::v\0tail"),
                SignatureSourceVariableSlot::Local { .. } => {
                    panic!("namespace purpose selected a local")
                }
            }
            let conflict = if profile == "jim" {
                SignatureNamespaceScope::C(ByteNamespacePath::root())
            } else {
                SignatureNamespaceScope::Jim(b"".as_slice().into())
            };
            assert!(
                SignatureSourceVariableSymbol::from_original_input(
                    &input,
                    &conflict,
                    OriginalVariableSymbolPurpose::NamespaceStorage,
                    OriginalVariableSymbolReceiver::Operand(
                        tcl_registry::resolved_invocation::VariableReceiverOperandForm::Combined
                    )
                )
                .is_none()
            );
        }
    }

    #[test]
    fn original_rename_spans_select_only_the_native_root_tail() {
        for source in [
            b"::N::v\\uD800(k)".as_slice(),
            b"\"::N::v\\uD800(k)\"",
            b"{::N::v\xed\xa0\x80(k)}",
        ] {
            let input = original_word("tcl9.0", source);
            let SignatureSourceNameInput::OriginalWord(key) = &input else {
                unreachable!()
            };
            let span = key.original_word().span();
            let site = SignatureSourceVariableOccurrence::from_original_input(
                span,
                input,
                SignatureNamespaceScope::C(ByteNamespacePath::root()),
                OriginalVariableSymbolPurpose::QualifiedOnly,
                OriginalVariableSymbolReceiver::Operand(
                    tcl_registry::resolved_invocation::VariableReceiverOperandForm::Combined,
                ),
                true,
            )
            .unwrap();
            assert_eq!(
                site.symbol().rename_tail(),
                Some(b"v\xed\xa0\x80".as_slice())
            );
            let extent = site.rename_span().unwrap();
            assert_eq!(
                source.get(extent.as_range()),
                Some(if source.contains(&b'\\') {
                    b"v\\uD800".as_slice()
                } else {
                    b"v\xed\xa0\x80".as_slice()
                })
            );
            assert_eq!(site.renamed_input(b"next"), Some(b"::N::next(k)".to_vec()));
        }
        let input = original_word("tcl8.6", b"local");
        let site = SignatureSourceVariableOccurrence::from_original_namespace_cell(
            Span::new(0, 5),
            input,
            SignatureNamespaceScope::C(ByteNamespacePath::from_segments([b"N".as_slice()])),
            &SignatureNamespaceScope::C(ByteNamespacePath::root()),
            b"local".as_slice().into(),
            OriginalVariableSymbolReceiver::Operand(
                tcl_registry::resolved_invocation::VariableReceiverOperandForm::Combined,
            ),
            true,
        )
        .unwrap();
        assert!(
            site.rename_span().is_none(),
            "a global fallback cannot supply a current-namespace tail edit"
        );
    }

    #[test]
    // Implementation contract: naming.variable.trace-source-receiver-purpose
    // docs/design/analysis/name-resolution-proofs/trace-source-receiver-purpose.md
    fn original_trace_source_geometry_and_rename_preserve_ignored_original_suffix() {
        use tcl_registry::resolved_invocation::VariableReceiverOperandForm::TraceSubject;
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let input = original_word(profile, b"{::N::v\0tail(k)}");
            let SignatureSourceNameInput::OriginalWord(key) = &input else {
                panic!("authentic word");
            };
            let occurrence = SignatureSourceVariableOccurrence::from_original_input(
                key.original_word().span(),
                input,
                SignatureNamespaceScope::C(ByteNamespacePath::root()),
                OriginalVariableSymbolPurpose::QualifiedOnly,
                OriginalVariableSymbolReceiver::Operand(TraceSubject),
                false,
            )
            .unwrap();
            let SignatureSourceVariableSlot::C { namespace, simple } = occurrence.symbol().slot()
            else {
                panic!("selected C namespace");
            };
            assert_eq!(
                namespace,
                &ByteNamespacePath::from_segments([b"N".as_slice()])
            );
            assert_eq!(simple.as_bytes(), b"v");
            assert!(!occurrence.is_declaration());
            assert_eq!(
                occurrence.renamed_input(b"new"),
                Some(b"::N::new\0tail(k)".to_vec())
            );
            let span = occurrence.rename_span().unwrap();
            assert_eq!(
                occurrence
                    .original_name_input()
                    .original_word_key()
                    .unwrap()
                    .source_image()
                    .bytes()
                    .get(span.as_range()),
                Some(b"v".as_slice())
            );
            let mut same = occurrence.clone();
            same.retain_declaration();
            assert!(
                !same.is_declaration(),
                "trace installation cannot donate a declaration"
            );
        }
        let input = original_word("jim", b"::v");
        assert!(
            OriginalVariableSymbolReceiver::Operand(TraceSubject)
                .input_form(&input)
                .is_none()
        );
    }
}

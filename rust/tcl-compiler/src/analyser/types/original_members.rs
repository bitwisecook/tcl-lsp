// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original member-name metadata, independent of reporting-name tables.
//!
//! The original word producer owns each counted name. This ordered ledger
//! records declarations and mutations observed by the definition walker; it
//! proves neither execution of that walker nor a live native method table.

use super::{MemberSide, MethodDef};
use crate::{
    command_binding::CommandAllocationSite,
    signature_scan::{
        original_name::SourceOriginalNameOccurrence, scope::SignatureSourceNameInput,
    },
};
use tcl_core_types::NameBytes;
use tcl_syntax::naming::NamePolicyProtocol;

/// Original member declaration syntax and its independently produced name.
/// A list child or frozen readonly value never becomes a static naming word,
/// editable name, installed method or entered definition-worker receipt.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceMemberDeclaration {
    site: CommandAllocationSite,
    input: SignatureSourceNameInput,
    word: tcl_lexer::NativeWord,
    static_occurrence: Option<SourceOriginalNameOccurrence>,
    source_installer:
        Option<std::sync::Arc<crate::command_binding::OriginalCatalogueSourceCandidate>>,
}

impl OriginalSourceMemberDeclaration {
    fn from_static(occurrence: SourceOriginalNameOccurrence) -> Self {
        Self {
            site: occurrence.site().clone(),
            input: SignatureSourceNameInput::OriginalWord(occurrence.name_input().clone()),
            word: occurrence.name_input().original_word().clone(),
            static_occurrence: Some(occurrence),
            source_installer: None,
        }
    }

    pub(crate) fn from_original_input(
        site: &CommandAllocationSite,
        word: &tcl_lexer::NativeWord,
        input: SignatureSourceNameInput,
    ) -> Option<Self> {
        if word.image() != site.source.source_image()
            || site.offset > word.span().start()
            || word.group().expand
            || word.config().escapes != input.policy().string_protocol().escape_syntax()
        {
            return None;
        }
        let static_occurrence = match &input {
            SignatureSourceNameInput::OriginalWord(key) => {
                (key.original_word() == word).then_some(())?;
                Some(SourceOriginalNameOccurrence::new(site, key.clone())?)
            }
            SignatureSourceNameInput::OriginalValue(_) => None,
            SignatureSourceNameInput::OriginalVariableRoot(_) => return None,
        };
        Some(Self {
            site: site.clone(),
            input,
            word: word.clone(),
            static_occurrence,
            source_installer: None,
        })
    }

    pub(crate) fn with_source_installer(
        mut self,
        installer: Option<std::sync::Arc<crate::command_binding::OriginalCatalogueSourceCandidate>>,
    ) -> Option<Self> {
        if installer.as_ref().is_some_and(|installer| {
            !installer.matches_source(self.word.image(), self.word.config())
                || installer.original_head().name_input().policy() != self.input.policy()
                || installer.site().source != self.site.source
                || installer.site().offset >= self.site.offset
        }) {
            return None;
        }
        self.source_installer = installer;
        Some(self)
    }

    /// Conditional Registry schema of a source installer, when this readonly
    /// declaration comes from its literal list. Applicability obligations remain
    /// on the schema; neither its body nor its installation is proved executed.
    #[must_use]
    pub fn source_installer(
        &self,
    ) -> Option<&crate::command_binding::OriginalCatalogueSourceCandidate> {
        self.source_installer.as_deref()
    }

    /// Actual whole worker command site, distinct from the value's producer.
    #[must_use]
    pub const fn site(&self) -> &CommandAllocationSite {
        &self.site
    }
    /// Canonical declaration name, distinct from its later moved route.
    #[must_use]
    pub const fn original_name_input(&self) -> &SignatureSourceNameInput {
        &self.input
    }
    /// Authentic whole worker argument. Its syntax can contain substitution;
    /// retaining its span does not turn the produced value into an edit recipe.
    #[must_use]
    pub const fn original_word(&self) -> &tcl_lexer::NativeWord {
        &self.word
    }
    /// Static complete-word naming provenance, when independently available.
    #[must_use]
    pub const fn static_occurrence(&self) -> Option<&SourceOriginalNameOccurrence> {
        self.static_occurrence.as_ref()
    }
}

/// One original method declaration and its current source-metadata route.
/// A moved route retains its canonical declaration and body independently.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceMethodMetadata {
    declaration: OriginalSourceMemberDeclaration,
    input: SignatureSourceNameInput,
    purpose: tcl_registry::definer::DefinitionMemberNamePurpose,
    side: MemberSide,
    method: MethodDef,
    exported: bool,
    native_class_delegate: bool,
    forward_prefix: Option<Vec<SignatureSourceNameInput>>,
    parameters: Option<tcl_lexer::NativeWord>,
    body: Option<tcl_lexer::NativeWord>,
    source_dialect: Option<tcl_registry::InvocationDialect>,
}

impl OriginalSourceMethodMetadata {
    pub(crate) fn new(
        declaration: SourceOriginalNameOccurrence,
        side: MemberSide,
        method: MethodDef,
        exported: bool,
        native_class_delegate: bool,
        forward_prefix: Option<Vec<SignatureSourceNameInput>>,
        purpose: tcl_registry::definer::DefinitionMemberNamePurpose,
    ) -> Option<Self> {
        let declaration = OriginalSourceMemberDeclaration::from_static(declaration);
        let input = declaration.original_name_input().clone();
        purpose
            .admits(input.policy().recipe(), input.bytes())
            .then_some(())?;
        Some(Self {
            declaration,
            input,
            purpose,
            side,
            method,
            exported,
            native_class_delegate,
            forward_prefix,
            parameters: None,
            body: None,
            source_dialect: None,
        })
    }

    pub(crate) fn from_declaration(
        declaration: OriginalSourceMemberDeclaration,
        side: MemberSide,
        method: MethodDef,
        exported: bool,
        native_class_delegate: bool,
        forward_prefix: Option<Vec<SignatureSourceNameInput>>,
        purpose: tcl_registry::definer::DefinitionMemberNamePurpose,
    ) -> Option<Self> {
        if let Some(original) = declaration.static_occurrence() {
            return Self::new(
                original.clone(),
                side,
                method,
                exported,
                native_class_delegate,
                forward_prefix,
                purpose,
            );
        }
        let input = declaration.original_name_input().clone();
        purpose
            .admits(input.policy().recipe(), input.bytes())
            .then_some(())?;
        Some(Self {
            declaration,
            input,
            purpose,
            side,
            method,
            exported,
            native_class_delegate,
            forward_prefix,
            parameters: None,
            body: None,
            source_dialect: None,
        })
    }

    pub(crate) fn with_body_role_words(
        mut self,
        parameters: Option<tcl_lexer::NativeWord>,
        body: Option<tcl_lexer::NativeWord>,
        source_dialect: Option<tcl_registry::InvocationDialect>,
    ) -> Option<Self> {
        let original = self.declaration.original_word();
        if parameters.iter().chain(body.iter()).any(|word| {
            word.image() != original.image()
                || word.config() != original.config()
                || word.group().expand
        }) {
            return None;
        }
        self.source_dialect = source_dialect.filter(|dialect| {
            original.config().grammar_over(dialect.lexer_grammar) == dialect.lexer_grammar
        });
        self.parameters = parameters;
        self.body = body;
        Some(self)
    }

    /// Whole original formal-list argument selected by the declaration grammar.
    /// It supplies source roles, without parameter binding or an activation.
    #[must_use]
    pub const fn parameters_word(&self) -> Option<&tcl_lexer::NativeWord> {
        self.parameters.as_ref()
    }
    /// Whole original script argument selected by the declaration grammar.
    /// It supplies source roles, without entered or complete body execution.
    #[must_use]
    pub const fn body_word(&self) -> Option<&tcl_lexer::NativeWord> {
        self.body.as_ref()
    }

    /// Independently selected source dialect for the retained argument roles.
    /// An unknown entry supplies no parameter grammar or execution authority.
    #[must_use]
    pub const fn source_dialect(&self) -> Option<tcl_registry::InvocationDialect> {
        self.source_dialect
    }

    /// Original declaration syntax and independent readonly/static name value.
    #[must_use]
    pub const fn declaration(&self) -> &OriginalSourceMemberDeclaration {
        &self.declaration
    }
    /// Current original route operand, distinct from the canonical body name.
    #[must_use]
    pub const fn original_name_input(&self) -> &SignatureSourceNameInput {
        &self.input
    }
    /// Registry-selected native method or source-value-only naming axis.
    #[must_use]
    pub const fn name_purpose(&self) -> tcl_registry::definer::DefinitionMemberNamePurpose {
        self.purpose
    }
    /// Definition receiver's own member table.
    #[must_use]
    pub const fn side(&self) -> MemberSide {
        self.side
    }
    /// Body/signature/report metadata at the canonical declaration.
    #[must_use]
    pub const fn metadata(&self) -> &MethodDef {
        &self.method
    }
    /// Declaration or subsequent explicit source visibility state.
    #[must_use]
    pub const fn exported(&self) -> bool {
        self.exported
    }
    /// Registry-selected classmethod delegate role, separate from allocation.
    #[must_use]
    pub const fn native_class_delegate(&self) -> bool {
        self.native_class_delegate
    }
    /// Original target and captured values of a forward, when independently
    /// retained. Reporting strings cannot fill a missing prefix.
    #[must_use]
    pub fn forward_prefix(&self) -> Option<&[SignatureSourceNameInput]> {
        self.forward_prefix.as_deref()
    }
}

/// Registry-selected effect on original member names.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OriginalSourceMemberEffectKind {
    /// Delete each independently retained operand name.
    Delete,
    /// Move the first operand's route to the second operand.
    Move,
    /// Apply explicit visibility to each independently retained name.
    Visibility(bool),
}

/// One exact original mutation, including its own operation site. Operand
/// producers and the canonical declaration never donate the mutation time.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalSourceMemberEffect {
    site: CommandAllocationSite,
    side: MemberSide,
    kind: OriginalSourceMemberEffectKind,
    inputs: Vec<SignatureSourceNameInput>,
    purpose: tcl_registry::definer::DefinitionMemberNamePurpose,
}

impl OriginalSourceMemberEffect {
    pub(crate) fn new(
        allocation_site: CommandAllocationSite,
        side: MemberSide,
        kind: OriginalSourceMemberEffectKind,
        inputs: Vec<SignatureSourceNameInput>,
        purpose: tcl_registry::definer::DefinitionMemberNamePurpose,
    ) -> Option<Self> {
        let first = inputs.first()?;
        if inputs.iter().any(|input| {
            input.policy() != first.policy()
                || !purpose.admits(input.policy().recipe(), input.bytes())
        }) {
            return None;
        }
        if kind == OriginalSourceMemberEffectKind::Move && inputs.len() != 2 {
            return None;
        }
        Some(Self {
            site: allocation_site,
            side,
            kind,
            inputs,
            purpose,
        })
    }
    /// Actual original mutation command site.
    #[must_use]
    pub const fn site(&self) -> &CommandAllocationSite {
        &self.site
    }
    /// Own receiver table affected by this operation.
    #[must_use]
    pub const fn side(&self) -> MemberSide {
        self.side
    }
    /// Registry-selected operation, independent of keyword display.
    #[must_use]
    pub const fn kind(&self) -> OriginalSourceMemberEffectKind {
        self.kind
    }
    /// Independently selected naming axis of this original effect.
    #[must_use]
    pub const fn name_purpose(&self) -> tcl_registry::definer::DefinitionMemberNamePurpose {
        self.purpose
    }
    /// Exact operand producers in Registry-selected operation order.
    #[must_use]
    pub fn inputs(&self) -> &[SignatureSourceNameInput] {
        &self.inputs
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Operation {
    Declaration(Box<OriginalSourceMethodMetadata>),
    Effect(OriginalSourceMemberEffect),
}

/// Ordered original member metadata. Unknown/computed input remains an
/// explicit coverage gap; the compatibility String table supplies no names.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OriginalSourceMemberLedger {
    operations: Vec<Operation>,
    unavailable: bool,
}

impl OriginalSourceMemberLedger {
    pub(crate) fn withdraw(&mut self) {
        self.unavailable = true;
    }
    pub(crate) fn declare(&mut self, declaration: OriginalSourceMethodMetadata) {
        self.operations
            .push(Operation::Declaration(Box::new(declaration)));
    }
    pub(crate) fn effect(&mut self, effect: OriginalSourceMemberEffect) {
        self.operations.push(Operation::Effect(effect));
    }
    pub(crate) fn absorb(&mut self, other: &Self, other_ran_second: bool) {
        self.unavailable |= other.unavailable;
        let mut operations = if other_ran_second {
            std::mem::take(&mut self.operations)
        } else {
            other.operations.clone()
        };
        let tail = if other_ran_second {
            &other.operations
        } else {
            &self.operations
        };
        for operation in tail {
            if !operations.contains(operation) {
                operations.push(operation.clone());
            }
        }
        self.operations = operations;
    }

    /// Original declarations, including those later moved or removed. These
    /// are source candidates; this inventory is not a live dispatch table.
    pub fn declarations(&self) -> impl Iterator<Item = &OriginalSourceMethodMetadata> {
        self.operations
            .iter()
            .filter_map(|operation| match operation {
                Operation::Declaration(declaration) => Some(declaration.as_ref()),
                Operation::Effect(_) => None,
            })
    }
    /// Original effects in the definition walk's actual retained order.
    pub fn effects(&self) -> impl Iterator<Item = &OriginalSourceMemberEffect> {
        self.operations
            .iter()
            .filter_map(|operation| match operation {
                Operation::Effect(effect) => Some(effect),
                Operation::Declaration(_) => None,
            })
    }
    /// Fold exact byte routes through this record's own observed operations.
    /// Missing names or conflicting moves supply no effective metadata view.
    #[must_use]
    pub fn methods(&self, side: MemberSide) -> Option<Vec<OriginalSourceMethodMetadata>> {
        self.methods_in_source_horizon(side, None)
    }
    /// Original source routes strictly before this call in the same immutable
    /// source origin. This supplies source order, never runtime currency.
    #[must_use]
    pub fn methods_before_source_call(
        &self,
        side: MemberSide,
        call: &CommandAllocationSite,
    ) -> Option<Vec<OriginalSourceMethodMetadata>> {
        self.methods_in_source_horizon(side, Some(call))
    }
    fn methods_in_source_horizon(
        &self,
        side: MemberSide,
        before: Option<&CommandAllocationSite>,
    ) -> Option<Vec<OriginalSourceMethodMetadata>> {
        if self.unavailable {
            return None;
        }
        let mut methods = Vec::<OriginalSourceMethodMetadata>::new();
        for operation in &self.operations {
            if !operation_in_source_horizon(operation, before)? {
                continue;
            }
            match operation {
                Operation::Declaration(declaration) if declaration.side == side => {
                    methods.retain(|method| !same_name(&method.input, &declaration.input));
                    methods.push(declaration.as_ref().clone());
                }
                Operation::Effect(effect) if effect.side == side => {
                    apply_effect(&mut methods, effect)?;
                }
                _ => {}
            }
        }
        Some(methods)
    }
    /// Select an exact own-table source metadata route. The caller still
    /// owns class/receiver identity, method precedence and lookup currency.
    #[must_use]
    pub fn method_for_input(
        &self,
        side: MemberSide,
        input: &SignatureSourceNameInput,
    ) -> Option<OriginalSourceMethodMetadata> {
        self.methods(side)?.into_iter().find(|method| {
            method
                .purpose
                .admits(input.policy().recipe(), input.bytes())
                && same_name(&method.input, input)
        })
    }
    /// Visibility advice for a family-owned inherited method. Exact own
    /// overrides, missing coverage, deletion and route moves decline; original
    /// explicit export/unexport effects amend the authored default visibility.
    #[must_use]
    pub fn inherited_visibility_for_input(
        &self,
        side: MemberSide,
        input: &SignatureSourceNameInput,
        default_exported: bool,
    ) -> Option<bool> {
        self.inherited_visibility_in_source_horizon(side, input, default_exported, None)
    }
    /// Same inherited visibility question under exact original before-call
    /// source order. Later exports, removals and overrides cannot donate facts.
    #[must_use]
    pub fn inherited_visibility_before_source_call(
        &self,
        side: MemberSide,
        input: &SignatureSourceNameInput,
        default_exported: bool,
        call: &CommandAllocationSite,
    ) -> Option<bool> {
        self.inherited_visibility_in_source_horizon(side, input, default_exported, Some(call))
    }
    fn inherited_visibility_in_source_horizon(
        &self,
        side: MemberSide,
        input: &SignatureSourceNameInput,
        default_exported: bool,
        before: Option<&CommandAllocationSite>,
    ) -> Option<bool> {
        if self
            .methods_in_source_horizon(side, before)?
            .iter()
            .any(|method| same_name(&method.input, input))
        {
            return None;
        }
        let mut exported = default_exported;
        for effect in self.effects().filter(|effect| effect.side == side) {
            if !site_in_source_horizon(effect.site(), before)? {
                continue;
            }
            if !effect.inputs.iter().any(|owned| same_name(owned, input)) {
                continue;
            }
            match effect.kind {
                OriginalSourceMemberEffectKind::Visibility(value) => exported = value,
                OriginalSourceMemberEffectKind::Delete | OriginalSourceMemberEffectKind::Move => {
                    return None;
                }
            }
        }
        Some(exported)
    }

    /// Effective exact native name keys from this record's observed routes.
    #[must_use]
    pub fn names(&self, side: MemberSide) -> Option<Vec<(NameBytes, NamePolicyProtocol)>> {
        Some(
            self.methods(side)?
                .into_iter()
                .map(|method| (NameBytes::from(method.input.bytes()), method.input.policy()))
                .collect(),
        )
    }
}

fn operation_in_source_horizon(
    operation: &Operation,
    before: Option<&CommandAllocationSite>,
) -> Option<bool> {
    let site = match operation {
        Operation::Declaration(declaration) => declaration.declaration().site(),
        Operation::Effect(effect) => effect.site(),
    };
    site_in_source_horizon(site, before)
}
fn site_in_source_horizon(
    site: &CommandAllocationSite,
    before: Option<&CommandAllocationSite>,
) -> Option<bool> {
    before.map_or(Some(true), |call| {
        (site.source == call.source).then_some(site.offset < call.offset)
    })
}

fn same_name(left: &SignatureSourceNameInput, right: &SignatureSourceNameInput) -> bool {
    left.policy() == right.policy() && left.bytes() == right.bytes()
}

fn apply_effect(
    methods: &mut Vec<OriginalSourceMethodMetadata>,
    effect: &OriginalSourceMemberEffect,
) -> Option<()> {
    if methods
        .iter()
        .any(|method| method.purpose != effect.purpose)
    {
        return None;
    }
    match effect.kind {
        OriginalSourceMemberEffectKind::Delete => {
            for input in &effect.inputs {
                methods.retain(|method| !same_name(&method.input, input));
            }
        }
        OriginalSourceMemberEffectKind::Move => {
            let [from, to] = effect.inputs.as_slice() else {
                return None;
            };
            if same_name(from, to) || methods.iter().any(|method| same_name(&method.input, to)) {
                return None;
            }
            let Some(index) = methods
                .iter()
                .position(|method| same_name(&method.input, from))
            else {
                // An augmentation may move a declaration owned by another
                // record. Its effect survives for the independently joined fold.
                return Some(());
            };
            let mut moved = methods.remove(index);
            moved.input = to.clone();
            methods.push(moved);
        }
        OriginalSourceMemberEffectKind::Visibility(exported) => {
            for input in &effect.inputs {
                if let Some(method) = methods
                    .iter_mut()
                    .find(|method| same_name(&method.input, input))
                {
                    method.exported = exported;
                }
            }
        }
    }
    Some(())
}

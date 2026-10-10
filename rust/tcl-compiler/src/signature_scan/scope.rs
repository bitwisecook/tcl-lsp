// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Authored namespace geometry for signature assistance, without runtime tokens.

pub use super::name_value::{
    SignatureSourceNameInput, SignatureSourceNameValue, SignatureSourceStaticListContainer,
};
pub use super::original_name::SignatureSourceNameKey;

use tcl_core_types::{ByteCommandSlot, ByteNamespacePath, NameBytes};
use tcl_syntax::naming::{NamePolicyProtocol, NativeNameContext, NativeNameProtocol};

/// Source namespace geometry. These values supply no namespace existence,
/// entered frame, command identity or native compilation permission.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SignatureNamespaceScope {
    /// Explicit symbolic compatibility context without an audited name policy.
    Symbolic(String),
    /// C namespace components retained before any display rendering.
    C(ByteNamespacePath),
    /// Jim's counted flat namespace value, independent of C components.
    Jim(NameBytes),
}

impl SignatureNamespaceScope {
    /// Whether the retained source geometry is the selected provider's root.
    #[must_use]
    pub fn is_root(&self) -> bool {
        match self {
            Self::C(path) => path.is_root(),
            Self::Jim(value) => value.as_bytes().is_empty(),
            Self::Symbolic(key) => key == "::",
        }
    }

    /// Readonly label from retained geometry, including opaque native bytes.
    /// The result grants no component reconstruction, lookup or source spelling.
    #[must_use]
    pub fn reporting_label(&self) -> String {
        match self {
            Self::Symbolic(label) => label.clone(),
            Self::C(path) => tcl_syntax::native_string::resident_name_label(
                &tcl_syntax::naming::native_namespace_full_name_bytes(path),
            ),
            Self::Jim(value) => format!(
                "::{}",
                tcl_syntax::native_string::resident_name_label(value.as_bytes()),
            ),
        }
    }

    pub(crate) fn root(policy: Option<NamePolicyProtocol>) -> Self {
        match policy.map(NamePolicyProtocol::recipe) {
            Some(NativeNameProtocol::C(_)) => Self::C(ByteNamespacePath::root()),
            Some(NativeNameProtocol::Jim084) => Self::Jim(NameBytes::from(b"".as_slice())),
            None => Self::Symbolic("::".to_owned()),
        }
    }

    /// Presentation only; its colon boundaries cannot recover this scope.
    #[must_use]
    pub fn display(&self) -> Option<String> {
        match self {
            Self::Symbolic(key) => Some(key.clone()),
            Self::C(path) => Some(format!(
                "::{}",
                tcl_syntax::naming::checked_namespace_path_utf8(path)
                    .ok()?
                    .join("::")
            )),
            Self::Jim(value) => Some(crate::naming::root_unrooted_key(value.try_utf8().ok()?)),
        }
    }

    /// Retained projection context only; it grants no namespace existence or token.
    #[must_use]
    pub fn context(&self) -> Option<NativeNameContext<'_>> {
        match self {
            Self::C(path) => Some(NativeNameContext::new(path)),
            Self::Jim(value) => Some(NativeNameContext::with_jim_namespace(
                &ROOT,
                value.as_bytes(),
            )),
            Self::Symbolic(_) => None,
        }
    }

    pub(crate) fn context_for_policy(
        &self,
        policy: NamePolicyProtocol,
    ) -> Option<NativeNameContext<'_>> {
        match (self, policy.recipe()) {
            (Self::C(_), NativeNameProtocol::C(_)) | (Self::Jim(_), NativeNameProtocol::Jim084) => {
                self.context()
            }
            _ => None,
        }
    }

    /// Select a child namespace from independently retained original native units.
    /// This is naming geometry only, without namespace existence or entry proof.
    #[must_use]
    pub fn child_from_key(&self, key: &SignatureSourceNameKey) -> Option<Self> {
        self.child_from_input(&SignatureSourceNameInput::OriginalWord(key.clone()))
    }

    /// Namespace geometry from a complete word or independently produced value.
    /// A lexical variable root does not donate an evaluated namespace operand.
    #[must_use]
    pub fn child_from_input(&self, input: &SignatureSourceNameInput) -> Option<Self> {
        if matches!(input, SignatureSourceNameInput::OriginalVariableRoot(_)) {
            return None;
        }
        let policy = input.policy();
        let context = self.context_for_policy(policy)?;
        match policy.recipe() {
            recipe @ NativeNameProtocol::C(_) => Some(Self::C(
                recipe.namespace_address_path(context, input.bytes()).ok()?,
            )),
            recipe @ NativeNameProtocol::Jim084 => Some(Self::Jim(
                recipe
                    .jim_namespace_canonical_input(context, input.bytes())
                    .ok()?
                    .selected()
                    .into(),
            )),
        }
    }

    /// Strict namespace ancestry from retained components. Unsupported Jim
    /// parents remain unknown; rendered labels never supply boundaries.
    #[must_use]
    pub fn is_strict_ancestor_of(&self, child: &Self, policy: NamePolicyProtocol) -> Option<bool> {
        match (self, child, policy.recipe()) {
            (Self::C(parent), Self::C(child), NativeNameProtocol::C(_)) => Some(
                parent.as_segments().len() < child.as_segments().len()
                    && child.as_segments().starts_with(parent.as_segments()),
            ),
            (Self::Jim(parent), Self::Jim(child), recipe @ NativeNameProtocol::Jim084) => {
                let mut current = child.as_bytes();
                loop {
                    if current == parent.as_bytes() {
                        return Some(current != child.as_bytes());
                    }
                    if current.is_empty() {
                        return Some(false);
                    }
                    if current.contains(&0) {
                        return None;
                    }
                    let next = recipe.namespace_qualifier_bytes(current);
                    if next == current {
                        return None;
                    }
                    current = next;
                }
            }
            _ => None,
        }
    }

    /// Optional written namespace spelling checked against the retained geometry.
    #[must_use]
    pub fn source_spelling(&self, policy: Option<NamePolicyProtocol>) -> Option<String> {
        match (self, policy.map(NamePolicyProtocol::recipe)) {
            (Self::Symbolic(key), None) => Some(key.clone()),
            (Self::C(path), Some(recipe @ NativeNameProtocol::C(_))) => {
                tcl_syntax::naming::native_namespace_source_spelling(recipe, path)
            }
            (Self::Jim(value), Some(NativeNameProtocol::Jim084)) => {
                tcl_syntax::naming::native_jim_namespace_source_spelling(value.as_bytes())
            }
            _ => None,
        }
    }

    pub(crate) fn child(&self, written: &str, policy: Option<NamePolicyProtocol>) -> Option<Self> {
        let Some(policy) = policy else {
            let Self::Symbolic(namespace) = self else {
                return None;
            };
            return Some(Self::Symbolic(crate::naming::qualify_namespace(
                namespace, written,
            )));
        };
        let recipe = policy.recipe();
        match self {
            Self::C(_) => Some(Self::C(
                recipe
                    .namespace_address_path(self.context()?, written.as_bytes())
                    .ok()?,
            )),
            Self::Jim(_) => Some(Self::Jim(
                recipe
                    .jim_namespace_canonical_input(self.context()?, written.as_bytes())
                    .ok()?
                    .selected()
                    .into(),
            )),
            Self::Symbolic(_) => None,
        }
    }
}

static ROOT: ByteNamespacePath = ByteNamespacePath::root();

/// A selected authored declaration slot. This retains naming policy and
/// component geometry, never an actual command token or entered lookup proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SourceCommandPublication {
    /// Ordinary named-command publication from original source.
    NamedCommand,
    /// `TclOO` object publication using its independent naming purpose.
    TclOoObject,
    /// Registry provider grammar's authored assistance, without native allocation.
    ProviderAdvice,
}

/// An original source publication and its retained naming coordinates.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SignatureSourceCommand {
    publication: SourceCommandPublication,
    policy: NamePolicyProtocol,
    slot: ByteCommandSlot,
}

impl SignatureSourceCommand {
    /// Original static procedure publication from the sealed byte producer.
    /// Name creation remains independent of lookup, body entry and native tokens.
    #[must_use]
    pub fn procedure_from_key(
        namespace: &SignatureNamespaceScope,
        key: &SignatureSourceNameKey,
    ) -> Option<Self> {
        let policy = key.policy();
        let recipe = policy.recipe();
        let context = namespace.context_for_policy(policy)?;
        if let NativeNameProtocol::C(version) = recipe {
            let selected = recipe.command_lookup_slot(context, key.bytes()).ok()?;
            tcl_registry::native_procedure::procedure_name_creation_error(
                tcl_registry::InvocationDialect::for_version(version),
                selected.namespace.is_root(),
                selected.simple.as_bytes(),
            )?
            .ok()?;
        }
        Some(Self::new(
            policy,
            recipe.command_publication_slot(context, key.bytes()).ok()?,
        ))
    }

    /// Original `TclOO` object name's independently selected publication purpose.
    #[must_use]
    pub fn object_from_key(
        namespace: &SignatureNamespaceScope,
        key: &SignatureSourceNameKey,
    ) -> Option<Self> {
        let policy = key.policy();
        Some(Self {
            publication: SourceCommandPublication::TclOoObject,
            policy,
            slot: policy
                .recipe()
                .oo_object_publication_slot(namespace.context_for_policy(policy)?, key.bytes())
                .ok()?,
        })
    }

    /// Original source naming units used only for Registry declaration advice.
    #[must_use]
    pub fn provider_advice_from_key(
        namespace: &SignatureNamespaceScope,
        key: &SignatureSourceNameKey,
    ) -> Option<Self> {
        let policy = key.policy();
        Some(Self {
            publication: SourceCommandPublication::ProviderAdvice,
            policy,
            slot: policy
                .recipe()
                .command_publication_slot(namespace.context_for_policy(policy)?, key.bytes())
                .ok()?,
        })
    }

    /// Alias allocation at the audited interpreter-root naming purpose.
    #[must_use]
    pub fn alias_from_key_at_root(key: &SignatureSourceNameKey) -> Option<Self> {
        Some(Self::new(
            key.policy(),
            key.policy()
                .recipe()
                .alias_publication_slot(NativeNameContext::root(), key.bytes())
                .ok()?,
        ))
    }

    /// Authored procedure publication in retained source namespace geometry.
    /// Name creation validation and publication remain distinct from lookup.
    #[must_use]
    pub(crate) fn procedure_in_context(
        policy: NamePolicyProtocol,
        namespace: &SignatureNamespaceScope,
        written: &str,
    ) -> Option<Self> {
        let recipe = policy.recipe();
        let context = namespace.context_for_policy(policy)?;
        if let NativeNameProtocol::C(version) = recipe {
            let selected = recipe
                .command_lookup_slot(context, written.as_bytes())
                .ok()?;
            tcl_registry::native_procedure::procedure_name_creation_error(
                tcl_registry::InvocationDialect::for_version(version),
                selected.namespace.is_root(),
                selected.simple.as_bytes(),
            )?
            .ok()?;
        }
        Some(Self::new(
            policy,
            recipe
                .command_publication_slot(context, written.as_bytes())
                .ok()?,
        ))
    }

    /// Authored `TclOO` object publication, without provider or object-token authority.
    #[must_use]
    pub(crate) fn object_in_context(
        policy: NamePolicyProtocol,
        namespace: &SignatureNamespaceScope,
        written: &str,
    ) -> Option<Self> {
        Some(Self {
            publication: SourceCommandPublication::TclOoObject,
            policy,
            slot: policy
                .recipe()
                .oo_object_publication_slot(
                    namespace.context_for_policy(policy)?,
                    written.as_bytes(),
                )
                .ok()?,
        })
    }

    /// Conditional original procedure source publication, with retained
    /// declaration policy and current authored slot. No installed command follows.
    #[must_use]
    pub fn procedure_from_original_source_publication(
        publication: &crate::command_binding::OriginalSourceProcedurePublication,
    ) -> Self {
        Self {
            publication: SourceCommandPublication::NamedCommand,
            policy: publication.policy(),
            slot: publication.source_slot().clone(),
        }
    }

    /// Current conditional class source slot with its canonical original
    /// publication purpose. Factory correspondence does not grant allocation.
    pub(crate) fn class_from_original_source_publication(
        publication: &crate::command_binding::OriginalSourceClassPublication,
        declaration: &crate::signature_scan::original_name::SourceDeclarationMetadata<
            crate::analyser::ClassDef,
        >,
    ) -> Option<Self> {
        publication.matches_declaration(declaration).then_some(())?;
        (declaration.name().policy() == publication.policy()).then_some(())?;
        Some(Self {
            publication: declaration.name().publication(),
            policy: publication.policy(),
            slot: publication.source_slot().clone(),
        })
    }

    /// The publication purpose, independent of actual provider allocation.
    #[must_use]
    pub const fn publication(&self) -> SourceCommandPublication {
        self.publication
    }

    /// Counted publication presentation, carrying no written-lookup authority.
    #[must_use]
    pub(crate) fn reported_full_name(&self) -> Option<String> {
        String::from_utf8(tcl_syntax::naming::native_command_full_name_bytes(
            &self.slot,
        ))
        .ok()
    }
    /// Select an authored alias publication from its original global-root
    /// operand. This supplies no actual publication or command-token authority.
    #[must_use]
    pub fn alias_at_root(policy: NamePolicyProtocol, written: &str) -> Option<Self> {
        let slot = policy
            .recipe()
            .alias_publication_slot(NativeNameContext::root(), written.as_bytes())
            .ok()?;
        Some(Self::new(policy, slot))
    }

    pub(super) fn new(policy: NamePolicyProtocol, slot: ByteCommandSlot) -> Self {
        Self {
            publication: SourceCommandPublication::NamedCommand,
            policy,
            slot,
        }
    }

    /// Selected authored slot, independent of the command's reported full name.
    #[must_use]
    pub fn slot(&self) -> &ByteCommandSlot {
        &self.slot
    }

    /// Explicit authored naming recipe; it authenticates no physical engine.
    #[must_use]
    pub fn policy(&self) -> NamePolicyProtocol {
        self.policy
    }

    /// Optional globally written name selecting this exact slot.
    #[must_use]
    pub fn source_spelling(&self) -> Option<String> {
        tcl_syntax::naming::native_command_source_spelling(self.policy.recipe(), &self.slot)
    }

    pub(crate) fn body_scope(&self) -> Option<SignatureNamespaceScope> {
        match self.policy.recipe() {
            NativeNameProtocol::C(_) => {
                Some(SignatureNamespaceScope::C(self.slot.namespace.clone()))
            }
            NativeNameProtocol::Jim084 => Some(SignatureNamespaceScope::Jim(
                self.policy
                    .recipe()
                    .jim_procedure_namespace(self.slot.simple.as_bytes())
                    .ok()?
                    .into(),
            )),
        }
    }

    pub(crate) fn simple_name(&self) -> Option<String> {
        let bytes = self.slot.simple.as_bytes();
        let simple = match self.body_scope()? {
            SignatureNamespaceScope::Jim(home) if !home.as_bytes().is_empty() => {
                bytes.get(home.as_bytes().len().checked_add(2)?..)?
            }
            SignatureNamespaceScope::C(_) | SignatureNamespaceScope::Jim(_) => bytes,
            SignatureNamespaceScope::Symbolic(_) => return None,
        };
        Some(std::str::from_utf8(simple).ok()?.to_owned())
    }

    /// One source word preserving the exact slot under a caller's retained
    /// channel and full lexer configuration. This is spelling assistance only.
    #[must_use]
    pub fn source_word_in(
        &self,
        channel: tcl_lexer::SourceChannel,
        config: tcl_lexer::LexerConfig,
    ) -> Option<String> {
        tcl_syntax::naming::native_command_source_word(
            self.policy.recipe(),
            &self.slot,
            channel,
            config,
        )
    }

    /// Whether original native units select this slot in the retained namespace.
    /// Neither matching bytes nor this lookup proves the slot is installed.
    #[must_use]
    pub fn matches_key(
        &self,
        namespace: &SignatureNamespaceScope,
        key: &SignatureSourceNameKey,
    ) -> bool {
        if self.policy != key.policy() {
            return false;
        }
        SignatureSourceLookup::from_key(namespace.clone(), key)
            .and_then(|lookup| lookup.candidates())
            .and_then(|candidates| candidates.into_iter().next())
            .is_some_and(|slot| slot == self.slot)
    }

    /// Whether an original written name's local lookup selects this authored
    /// slot. This supplies navigation assistance, not runtime dispatch proof.
    #[must_use]
    pub fn matches_written(&self, namespace: &SignatureNamespaceScope, written: &str) -> bool {
        let Some(context) = namespace.context() else {
            return false;
        };
        match (self.policy.recipe(), namespace) {
            (NativeNameProtocol::C(_), SignatureNamespaceScope::C(_)) => self
                .policy
                .recipe()
                .command_lookup_slot(context, written.as_bytes())
                .is_ok_and(|slot| slot == self.slot),
            (NativeNameProtocol::Jim084, SignatureNamespaceScope::Jim(_)) => self
                .policy
                .recipe()
                .jim_command_lookup_keys(context, written.as_bytes())
                .ok()
                .and_then(|keys| keys.into_iter().next())
                .is_some_and(|key| self.slot.namespace.is_root() && self.slot.simple == key),
            _ => false,
        }
    }
}

/// Original source command lookup and exact caller geometry. Candidate naming
/// supplies no command existence, class kind, token or provider survival.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SignatureSourceLookup {
    policy: NamePolicyProtocol,
    namespace: SignatureNamespaceScope,
    written: NameBytes,
    original: Option<SignatureSourceNameInput>,
}

impl SignatureSourceLookup {
    /// Retain original lookup bytes in a context supported by this naming policy.
    /// Returns `None` when the original namespace context is unavailable.
    #[must_use]
    pub fn new(
        policy: NamePolicyProtocol,
        namespace: SignatureNamespaceScope,
        written: String,
    ) -> Option<Self> {
        namespace.context_for_policy(policy)?;
        Some(Self {
            policy,
            namespace,
            written: written.into(),
            original: None,
        })
    }

    /// Original byte lookup in an independently retained caller scope.
    /// Opaque source units need no Unicode rendering to select candidates.
    #[must_use]
    pub fn from_key(
        namespace: SignatureNamespaceScope,
        key: &SignatureSourceNameKey,
    ) -> Option<Self> {
        namespace.context_for_policy(key.policy())?;
        Some(Self {
            policy: key.policy(),
            namespace,
            written: NameBytes::from(key.bytes()),
            original: Some(SignatureSourceNameInput::OriginalWord(key.clone())),
        })
    }

    /// Original complete-word or readonly produced units in a retained scope.
    /// The provenance distinction survives lookup and supplies no edit grant.
    #[must_use]
    pub fn from_input(
        namespace: SignatureNamespaceScope,
        input: &SignatureSourceNameInput,
    ) -> Option<Self> {
        if matches!(input, SignatureSourceNameInput::OriginalVariableRoot(_)) {
            return None;
        }
        namespace.context_for_policy(input.policy())?;
        Some(Self {
            policy: input.policy(),
            namespace,
            written: NameBytes::from(input.bytes()),
            original: Some(input.clone()),
        })
    }

    /// Original naming producer, with its complete-word/value distinction.
    #[must_use]
    pub fn original_name_input(&self) -> Option<&SignatureSourceNameInput> {
        self.original.as_ref()
    }

    /// Exact written native units, without a source or execution grant.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        self.written.as_bytes()
    }

    /// Optional exact UTF-8 presentation; never a naming input.
    #[must_use]
    pub fn written(&self) -> Option<&str> {
        self.written.try_utf8().ok()
    }

    /// Original complete lexical producer; authored compatibility has none.
    #[must_use]
    pub fn original_name_key(&self) -> Option<&SignatureSourceNameKey> {
        self.original.as_ref()?.original_word_key()
    }

    /// The independently selected naming policy for this source lookup.
    #[must_use]
    pub fn policy(&self) -> NamePolicyProtocol {
        self.policy
    }

    /// Keep every declaration at the first matching published byte slot.
    /// The input order of declarations cannot override lookup order. This
    /// metadata match supplies no existence, installation or lifetime grant.
    #[must_use]
    pub fn first_matching_publications<'a, T>(
        &self,
        declarations: impl IntoIterator<Item = (&'a SignatureSourceCommand, T)>,
    ) -> Vec<T> {
        let Some(candidates) = self.candidates() else {
            return Vec::new();
        };
        first_matching_byte_publications(self.policy, &candidates, declarations)
    }

    /// Ordered current/global geometry, without namespace-path or import authority.
    #[must_use]
    pub fn candidates(&self) -> Option<Vec<ByteCommandSlot>> {
        let recipe = self.policy.recipe();
        let context = self.namespace.context_for_policy(self.policy)?;
        match recipe {
            NativeNameProtocol::C(_) => {
                let mut slots = vec![
                    recipe
                        .command_lookup_slot(context, self.written.as_bytes())
                        .ok()?,
                ];
                if !self.written.as_bytes().starts_with(b"::") {
                    let global = recipe
                        .command_lookup_slot(NativeNameContext::root(), self.written.as_bytes())
                        .ok()?;
                    if !slots.contains(&global) {
                        slots.push(global);
                    }
                }
                Some(slots)
            }
            NativeNameProtocol::Jim084 => Some(
                recipe
                    .jim_command_lookup_keys(context, self.written.as_bytes())
                    .ok()?
                    .into_iter()
                    .map(|simple| ByteCommandSlot {
                        namespace: ByteNamespacePath::root(),
                        simple,
                    })
                    .collect(),
            ),
        }
    }
}

/// Match publication metadata against one independently retained ordered
/// byte lookup path. Every record at the first matching slot survives; this
/// pure projection supplies no command existence or temporal installation.
#[must_use]
pub fn first_matching_byte_publications<'a, T>(
    policy: NamePolicyProtocol,
    candidates: &[ByteCommandSlot],
    declarations: impl IntoIterator<Item = (&'a SignatureSourceCommand, T)>,
) -> Vec<T> {
    first_matching_byte_slots(
        policy,
        candidates,
        declarations
            .into_iter()
            .map(|(name, payload)| (name.slot(), name.policy(), payload)),
    )
}

/// Match exact slot metadata at the first candidate. The selected policy and
/// byte geometry remain independent of existence, installation and execution.
#[must_use]
pub fn first_matching_byte_slots<'a, T>(
    policy: NamePolicyProtocol,
    candidates: &[ByteCommandSlot],
    declarations: impl IntoIterator<Item = (&'a ByteCommandSlot, NamePolicyProtocol, T)>,
) -> Vec<T> {
    let mut first = None;
    let mut matches = Vec::new();
    for (slot, selected_policy, payload) in declarations {
        if selected_policy != policy {
            continue;
        }
        let Some(rank) = candidates.iter().position(|candidate| candidate == slot) else {
            continue;
        };
        if first.is_none_or(|selected| rank < selected) {
            first = Some(rank);
            matches.clear();
        }
        if first == Some(rank) {
            matches.push(payload);
        }
    }
    matches
}

#[cfg(test)]
mod original_key_tests {
    use super::*;

    fn key(source: &[u8], policy: NamePolicyProtocol) -> SignatureSourceNameKey {
        let grammar = match policy.recipe() {
            NativeNameProtocol::C(version) => tcl_dialect::grammar_of_dialect_name(Some(&format!(
                "tcl{}",
                version.version_string()
            ))),
            NativeNameProtocol::Jim084 => tcl_dialect::grammar_of_dialect_name(Some("jimtcl")),
        };
        let config = tcl_lexer::LexerConfig::from_grammar(grammar);
        let parsed = tcl_lexer::native_script_words_in(
            tcl_lexer::SourceImage::native(source),
            tcl_lexer::Span::new(0, u32::try_from(source.len()).unwrap()),
            config,
        )
        .unwrap();
        SignatureSourceNameKey::from_original_native_word(
            &parsed.commands[0].words[0],
            tcl_syntax::word_rules::WordValueRules::from_config(&config),
            policy,
        )
        .unwrap()
    }

    #[test]
    fn command_cstring_purpose_distinguishes_original_raw_and_escaped_zero() {
        let policy = NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_6);
        let scope = SignatureNamespaceScope::root(Some(policy));
        let raw = key(b"p\0tail", policy);
        let escaped = key(br"p\u0000tail", policy);
        let raw = SignatureSourceCommand::procedure_from_key(&scope, &raw).unwrap();
        let escaped = SignatureSourceCommand::procedure_from_key(&scope, &escaped).unwrap();
        assert_eq!(raw.slot().simple.as_bytes(), b"p");
        assert_eq!(escaped.slot().simple.as_bytes(), b"p\xc0\x80tail");
        assert_ne!(raw.slot(), escaped.slot());
    }

    #[test]
    fn first_matching_publications_preserve_lookup_priority_and_all_records() {
        let policy = NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_6);
        let root = SignatureNamespaceScope::root(Some(policy));
        let local = root.child_from_key(&key(b"N", policy)).unwrap();
        let input = key(br"p\uD800", policy);
        let other_input = key(br"p\uD801", policy);
        let root_name = SignatureSourceCommand::procedure_from_key(&root, &input).unwrap();
        let local_name = SignatureSourceCommand::procedure_from_key(&local, &input).unwrap();
        let other_name = SignatureSourceCommand::procedure_from_key(&local, &other_input).unwrap();
        let different_policy = NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V9_0);
        let different_name = SignatureSourceCommand::procedure_from_key(
            &SignatureNamespaceScope::root(Some(different_policy)),
            &key(br"::N::p\uD800", different_policy),
        )
        .unwrap();
        let lookup = SignatureSourceLookup::from_key(local, &input).unwrap();
        assert!(lookup.written().is_none());
        assert_eq!(
            lookup.first_matching_publications([
                (&root_name, "root-first"),
                (&different_name, "wrong-policy"),
                (&local_name, "local-first"),
                (&other_name, "other-byte-name"),
                (&root_name, "root-last"),
                (&local_name, "local-second"),
            ]),
            vec!["local-first", "local-second"]
        );
        assert_eq!(
            lookup.first_matching_publications([(&root_name, "root")]),
            vec!["root"]
        );
        assert!(
            lookup
                .first_matching_publications([(&other_name, "other")])
                .is_empty()
        );
    }

    #[test]
    fn lexical_variable_root_cannot_supply_command_operand_value() {
        let policy = NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_6);
        let config = tcl_lexer::LexerConfig::from_grammar(tcl_dialect::grammar_of_dialect_name(
            Some("tcl8.6"),
        ));
        let plan = tcl_lexer::native_script_words_in(
            tcl_lexer::SourceImage::native(b"$p".as_slice()),
            tcl_lexer::Span::new(0, 2),
            config,
        )
        .unwrap();
        let word = &plan.commands[0].words[0];
        let part = word.executable_parts().all_parts().next().unwrap().span;
        let root = super::super::variable_name::SignatureSourceVariableRoot::from_original_word(
            word,
            part,
            tcl_syntax::word_rules::WordValueRules::from_config(&config),
            policy,
        )
        .unwrap();
        let input = SignatureSourceNameInput::OriginalVariableRoot(root);
        let scope = SignatureNamespaceScope::root(Some(policy));
        assert!(SignatureSourceLookup::from_input(scope.clone(), &input).is_none());
        assert!(SignatureSourceLookup::from_key(scope, &key(b"p", policy)).is_some());
    }

    #[test]
    fn opaque_source_lookup_and_publication_do_not_require_unicode_display() {
        let policy = NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_6);
        let scope = SignatureNamespaceScope::root(Some(policy));
        let input = key(br"p\uD800", policy);
        let lookup = SignatureSourceLookup::from_key(scope.clone(), &input).unwrap();
        let declaration = SignatureSourceCommand::procedure_from_key(&scope, &input).unwrap();
        assert!(lookup.written().is_none());
        assert_eq!(
            lookup.candidates().unwrap(),
            vec![declaration.slot().clone()]
        );
        assert!(declaration.matches_key(&scope, &input));
        assert_eq!(lookup.original_name_key(), Some(&input));
    }
}

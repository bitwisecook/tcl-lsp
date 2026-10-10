// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Sealed static original naming units, independent of display and execution.

use tcl_core_types::NameBytes;
use tcl_lexer::{LexerConfig, NativeWord, SourceImage, Span};
use tcl_syntax::{naming::NamePolicyProtocol, word_rules::WordValueRules};

/// One complete original static naming word and its native value units.
///
/// Its private producer preserves the source channel, complete lexical
/// configuration and independently selected string protocol. This receipt
/// grants no command publication, native activation or writable edit recipe.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SignatureSourceNameKey {
    original: NativeWord,
    bytes: NameBytes,
    rules: WordValueRules,
    policy: NamePolicyProtocol,
}

impl SignatureSourceNameKey {
    /// Capture a complete static lexical word through the canonical byte owner.
    /// Substitution, expansion or conflicting word/string rules abstain.
    #[must_use]
    pub fn from_original_native_word(
        original: &NativeWord,
        rules: WordValueRules,
        policy: NamePolicyProtocol,
    ) -> Option<Self> {
        if original.group().expand
            || rules != WordValueRules::from_config(&original.config())
            || original.config().escapes != policy.string_protocol().escape_syntax()
        {
            return None;
        }
        let words = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
            std::slice::from_ref(original),
            policy.string_protocol(),
        )
        .ok()?;
        Some(Self {
            original: original.clone(),
            bytes: NameBytes::from(words.literal(0)?),
            rules,
            policy,
        })
    }

    /// Actual produced units; Unicode reporting cannot reconstruct this value.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        self.bytes.as_bytes()
    }

    /// The independently selected name provider, including its authority.
    #[must_use]
    pub const fn policy(&self) -> NamePolicyProtocol {
        self.policy
    }

    /// Original producer image and input channel.
    #[must_use]
    pub fn source_image(&self) -> &SourceImage {
        self.original.image()
    }

    /// Complete grouped original word extent, without an edit grant.
    #[must_use]
    pub fn span(&self) -> Span {
        self.original.span()
    }

    /// Retained complete lexical configuration.
    #[must_use]
    pub fn lexer_config(&self) -> LexerConfig {
        self.original.config()
    }

    /// Retained value/list rules, independently of command semantics.
    #[must_use]
    pub const fn word_value_rules(&self) -> WordValueRules {
        self.rules
    }

    /// Sealed original lexical producer.
    #[must_use]
    pub fn original_word(&self) -> &NativeWord {
        &self.original
    }

    /// Exact UTF-8 advice only; unavailable display does not withdraw identity.
    #[must_use]
    pub fn display(&self) -> Option<&str> {
        self.bytes.try_utf8().ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(source: &[u8], policy: NamePolicyProtocol) -> Option<SignatureSourceNameKey> {
        let config = match policy.recipe() {
            tcl_syntax::naming::NativeNameProtocol::C(version) => {
                LexerConfig::from_grammar(tcl_dialect::grammar_of_dialect_name(Some(&format!(
                    "tcl{}",
                    version.version_string()
                ))))
            }
            tcl_syntax::naming::NativeNameProtocol::Jim084 => {
                LexerConfig::from_grammar(tcl_dialect::grammar_of_dialect_name(Some("jimtcl")))
            }
        };
        let plan = tcl_lexer::native_script_words_in(
            SourceImage::native(source),
            Span::new(0, u32::try_from(source.len()).unwrap()),
            config,
        )
        .unwrap();
        SignatureSourceNameKey::from_original_native_word(
            &plan.commands[0].words[0],
            WordValueRules::from_config(&config),
            policy,
        )
    }

    #[test]
    fn static_name_key_keeps_raw_zero_distinct_from_escaped_native_zero() {
        for version in tcl_dialect::TclVersion::ALL {
            let policy = NamePolicyProtocol::authored_tcl(version);
            let raw = key(b"p\0tail", policy).unwrap();
            let escaped = key(br"p\u0000tail", policy).unwrap();
            assert_eq!(raw.bytes(), b"p\0tail");
            assert_eq!(escaped.bytes(), b"p\xc0\x80tail");
            assert_ne!(raw, escaped);
        }
    }

    #[test]
    fn opaque_static_units_retain_identity_without_unicode_advice() {
        let policy = NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_6);
        let key = key(br"p\uD800", policy).unwrap();
        assert_eq!(key.bytes(), b"p\xed\xa0\x80");
        assert!(key.display().is_none());
    }

    #[test]
    fn name_key_declines_dynamic_expanded_or_conflicting_protocol() {
        let policy = NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_6);
        assert!(key(b"$name", policy).is_none());
        assert!(key(b"{*}{p}", policy).is_none());
        let original = key(b"p", policy).unwrap();
        assert!(
            SignatureSourceNameKey::from_original_native_word(
                original.original_word(),
                WordValueRules::JIM,
                policy,
            )
            .is_none()
        );
        assert!(
            SignatureSourceNameKey::from_original_native_word(
                original.original_word(),
                WordValueRules::TCL,
                NamePolicyProtocol::authored_jim084(),
            )
            .is_none()
        );
    }
}

/// A static naming word issued from an independently retained original command
/// layout. The site owns the original source instance; it is not an execution,
/// allocation incarnation or editable source recipe.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceOriginalNameOccurrence {
    site: crate::command_binding::CommandAllocationSite,
    input: SignatureSourceNameKey,
}

impl SourceOriginalNameOccurrence {
    pub(crate) fn new(
        site: &crate::command_binding::CommandAllocationSite,
        input: SignatureSourceNameKey,
    ) -> Option<Self> {
        if site.source.source_image() != input.source_image() || site.offset > input.span().start()
        {
            return None;
        }
        Some(Self {
            site: site.clone(),
            input,
        })
    }

    /// Genuine original definer and its effective naming operand. A captured
    /// alias operand retains its own earlier anchor, never a call-site span.
    pub(crate) fn from_original_class_factory(
        declaration: &crate::command_binding::OriginalSourceClassDeclaration,
    ) -> Option<Self> {
        let input = declaration
            .name_input()
            .native_input()?
            .original_word_key()?
            .clone();
        let site = declaration.factory().site();
        (site.source.source_image() == input.source_image()).then(|| Self {
            site: site.clone(),
            input,
        })
    }

    /// Independently retained original command layout site.
    #[must_use]
    pub fn site(&self) -> &crate::command_binding::CommandAllocationSite {
        &self.site
    }

    /// Complete original static naming word, without a dispatch or edit grant.
    #[must_use]
    pub fn name_input(&self) -> &SignatureSourceNameKey {
        &self.input
    }
}

/// Original declaration metadata retained independently of UI name maps.
/// Publication geometry and the source layout remain separate from an actual
/// installed command incarnation or positioned implementation reference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceDeclarationMetadata<T> {
    original: SourceOriginalNameOccurrence,
    name: super::scope::SignatureSourceCommand,
    metadata: T,
}

impl<T> SourceDeclarationMetadata<T> {
    pub(crate) fn new(
        original: &SourceOriginalNameOccurrence,
        name: super::scope::SignatureSourceCommand,
        metadata: T,
    ) -> Option<Self> {
        if original.name_input().policy() != name.policy() {
            return None;
        }
        Some(Self {
            original: original.clone(),
            name,
            metadata,
        })
    }

    /// Complete original declaration layout and naming producer, independent
    /// of current installation, command liveness or native body admission.
    #[must_use]
    pub fn original_occurrence(&self) -> &SourceOriginalNameOccurrence {
        &self.original
    }

    /// Exact declaration source layout, independent of an installation lifetime.
    #[must_use]
    pub fn declaration_site(&self) -> &crate::command_binding::CommandAllocationSite {
        self.original.site()
    }

    /// Complete original declaration naming producer.
    #[must_use]
    pub fn name_input(&self) -> &SignatureSourceNameKey {
        self.original.name_input()
    }

    /// Purpose-selected native publication slot, independent of display.
    #[must_use]
    pub fn name(&self) -> &super::scope::SignatureSourceCommand {
        &self.name
    }

    /// Immutable declaration metadata at this original source site.
    #[must_use]
    pub fn metadata(&self) -> &T {
        &self.metadata
    }
}

/// Original native package naming units and the separately selected package
/// `CString` purpose. Optional display and diagnostic prose supply neither.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourcePackageName {
    input: super::scope::SignatureSourceNameInput,
    key: tcl_registry::native_package::NativePackageNameKey,
}

impl SourcePackageName {
    pub(crate) fn from_input(input: super::scope::SignatureSourceNameInput) -> Self {
        let key = tcl_registry::native_package::NativePackageNameKey::from_native_units(
            input.bytes(),
            input.policy(),
        );
        Self { input, key }
    }

    /// Original word or readonly value producer, independent of package state.
    #[must_use]
    pub fn input(&self) -> &super::scope::SignatureSourceNameInput {
        &self.input
    }

    /// Canonical package-purpose key with independently retained policy.
    #[must_use]
    pub fn key(&self) -> &tcl_registry::native_package::NativePackageNameKey {
        &self.key
    }
}

#[cfg(test)]
mod original_package_name_tests {
    use super::super::scope::SignatureSourceNameInput;
    use super::*;

    #[test]
    fn package_key_uses_native_units_and_its_own_cstring_purpose() {
        let policy = NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_6);
        let config = LexerConfig::from_grammar(
            tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6)
                .lexer_grammar,
        );
        let mut captured = Vec::new();
        for bytes in [b"p\0tail".as_slice(), br"p\u0000tail".as_slice()] {
            let image = SourceImage::native(bytes);
            let plan = tcl_lexer::native_script_words_in(
                image.clone(),
                Span::new(0, u32::try_from(image.len()).unwrap()),
                config,
            )
            .unwrap();
            let key = SignatureSourceNameKey::from_original_native_word(
                &plan.commands[0].words[0],
                WordValueRules::from_config(&config),
                policy,
            )
            .unwrap();
            captured.push(SourcePackageName::from_input(
                SignatureSourceNameInput::OriginalWord(key),
            ));
        }
        assert_eq!(captured[0].input().bytes(), b"p\0tail");
        assert_eq!(captured[0].key().bytes(), b"p");
        assert_eq!(captured[1].key().bytes(), b"p\xc0\x80tail");
        assert_ne!(captured[0].key(), captured[1].key());
    }

    #[test]
    fn analyser_and_background_packages_retain_original_opaque_operands() {
        let source = r"package provide p\uD800 1
package ifneeded p\uD801 1 {return}
package require p\uD800";
        let analysis = crate::analyser::Analyser::new().analyse(source, "tcl8.6");
        let registry = tcl_registry::CommandRegistry::build_default()
            .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.6").unwrap());
        let scan = crate::signature_scan::extract_signatures(source, &registry);
        assert_eq!(analysis.package_provides.len(), 1);
        assert_eq!(analysis.package_ifneededs.len(), 1);
        assert_eq!(scan.package_provides.len(), 1);
        assert_eq!(scan.package_ifneededs.len(), 1);
        for (provides, ifneededs, requires) in [
            (
                &analysis.package_provides,
                &analysis.package_ifneededs,
                &analysis.package_requires,
            ),
            (
                &scan.package_provides,
                &scan.package_ifneededs,
                &scan.package_requires,
            ),
        ] {
            let provided = provides[0].original_name.as_ref().unwrap();
            let registered = ifneededs[0].original_name.as_ref().unwrap();
            let required = requires[0].original_name.as_ref().unwrap();
            assert_eq!(provided.key().bytes(), b"p\xed\xa0\x80");
            assert_eq!(registered.key().bytes(), b"p\xed\xa0\x81");
            assert_eq!(provided.key(), required.key());
            assert_ne!(provided.key(), registered.key());
            assert!(
                provided
                    .input()
                    .original_word_key()
                    .unwrap()
                    .display()
                    .is_none()
            );
        }
    }
}

/// An original written operand with independently retained caller namespace
/// geometry from the same complete invocation. It supplies no argument role,
/// command selection, namespace lifetime or entered execution authority.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceOriginalNamespaceOperand {
    input: super::scope::SignatureSourceNameInput,
    context: super::scope::SignatureNamespaceScope,
    site: crate::command_binding::CommandAllocationSite,
    conditional_declaration: Option<SourceOriginalNameOccurrence>,
}

impl SourceOriginalNamespaceOperand {
    pub(crate) fn new(
        input: super::scope::SignatureSourceNameInput,
        context: super::scope::SignatureNamespaceScope,
        site: crate::command_binding::CommandAllocationSite,
    ) -> Self {
        Self {
            input,
            context,
            site,
            conditional_declaration: None,
        }
    }
    // Implementation contract: naming.namespace.original-export-source-advice
    // docs/design/analysis/name-resolution-proofs/namespace-original-export-source-advice.md
    pub(crate) fn from_conditional_declaration(
        original: &SourceOriginalNameOccurrence,
        context: super::scope::SignatureNamespaceScope,
    ) -> Option<Self> {
        use super::scope::{SignatureNamespaceScope, SignatureSourceNameInput};
        use tcl_syntax::naming::NativeNameProtocol;
        if !matches!(
            (&context, original.name_input().policy().recipe()),
            (SignatureNamespaceScope::C(_), NativeNameProtocol::C(_))
                | (SignatureNamespaceScope::Jim(_), NativeNameProtocol::Jim084)
        ) {
            return None;
        }
        Some(Self {
            input: SignatureSourceNameInput::OriginalWord(original.name_input().clone()),
            context,
            site: original.site().clone(),
            conditional_declaration: Some(original.clone()),
        })
    }
    /// Original whole-word or readonly-value producer, without an argument-role grant.
    #[must_use]
    pub fn name_input(&self) -> &super::scope::SignatureSourceNameInput {
        &self.input
    }
    /// Independently retained caller naming geometry, without namespace existence.
    #[must_use]
    pub fn context(&self) -> &super::scope::SignatureNamespaceScope {
        &self.context
    }
    /// Actual source invocation site that owns the complete operand vector.
    #[must_use]
    pub fn site(&self) -> &crate::command_binding::CommandAllocationSite {
        &self.site
    }
    /// Conditional source layout used when no current caller scope is available.
    /// Its declaration scope grants no namespace existence, execution or completion.
    #[must_use]
    pub fn conditional_declaration(&self) -> Option<&SourceOriginalNameOccurrence> {
        self.conditional_declaration.as_ref()
    }
}

/// One original namespace export pattern or leading clear event. Immutable
/// source naming advice does not establish export completion or a live import.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceNamespaceExport {
    operand: SourceOriginalNamespaceOperand,
    selected: Option<tcl_core_types::NameBytes>,
    span: Span,
}

impl SourceNamespaceExport {
    pub(crate) fn from_original(
        operand: SourceOriginalNamespaceOperand,
        span: Span,
        clears: bool,
    ) -> Option<Self> {
        use tcl_syntax::naming::{NativeNameProtocol, NativeNamePurpose};
        if !matches!(
            operand.context(),
            super::scope::SignatureNamespaceScope::C(_)
        ) || !matches!(
            operand.name_input().policy().recipe(),
            NativeNameProtocol::C(_)
        ) {
            return None;
        }
        let projection = operand
            .name_input()
            .policy()
            .recipe()
            .namespace_pattern_input(
                operand.name_input().bytes(),
                NativeNamePurpose::NamespaceExportPattern,
            )
            .ok()?;
        if clears && projection.selected() != b"-clear" {
            return None;
        }
        if !clears && tcl_syntax::naming::is_qualified(projection.selected()) {
            return None;
        }
        let selected = (!clears).then(|| tcl_core_types::NameBytes::from(projection.selected()));
        Some(Self {
            operand,
            selected,
            span,
        })
    }
    /// Original export operand producer, retained independently of its selected pattern.
    #[must_use]
    pub fn name_input(&self) -> &super::scope::SignatureSourceNameInput {
        self.operand.name_input()
    }
    /// Same-invocation exporting namespace geometry, without a live export table.
    #[must_use]
    pub fn context(&self) -> &super::scope::SignatureNamespaceScope {
        self.operand.context()
    }
    /// Actual source invocation site, independent of event ordering advice.
    #[must_use]
    pub fn site(&self) -> &crate::command_binding::CommandAllocationSite {
        self.operand.site()
    }
    /// Independent static source layout for a conditional declaration event.
    /// A current invocation operand retains no conditional declaration here.
    #[must_use]
    pub fn conditional_declaration(&self) -> Option<&SourceOriginalNameOccurrence> {
        self.operand.conditional_declaration()
    }
    /// Exact selected native name policy retained by the operand.
    #[must_use]
    pub fn policy(&self) -> tcl_syntax::naming::NamePolicyProtocol {
        self.name_input().policy()
    }
    /// Original operand extent used only for source positioning.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }
    /// Whether the selected leading control clears earlier pattern events.
    #[must_use]
    pub const fn clears(&self) -> bool {
        self.selected.is_none()
    }
    /// Purpose-selected counted pattern bytes; absent for a clear event.
    #[must_use]
    pub fn pattern(&self) -> Option<&[u8]> {
        self.selected
            .as_ref()
            .map(tcl_core_types::NameBytes::as_bytes)
    }
    /// Exact namespace and counted-name query under the same selected policy.
    /// The result is source candidate advice, without table or import authority.
    #[must_use]
    pub fn matches_command(
        &self,
        slot: &tcl_core_types::ByteCommandSlot,
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> Option<bool> {
        let super::scope::SignatureNamespaceScope::C(path) = self.context() else {
            return None;
        };
        if policy != self.policy() || *path != slot.namespace {
            return None;
        }
        tcl_syntax::native_glob::NativeGlobProtocol::from_name_policy(policy)
            .match_name_pattern(
                tcl_syntax::native_glob::NativeNameGlobPurpose::ExportFilter,
                self.pattern()?,
                slot.simple.as_bytes(),
            )
            .ok()
    }
}

/// An authentic namespace import/forget pattern and its selected byte geometry.
/// This records source naming advice, not a completed import or live namespace.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceNamespacePattern {
    input: super::scope::SignatureSourceNameInput,
    context: super::scope::SignatureNamespaceScope,
    purpose: tcl_syntax::naming::NativeNamePurpose,
    parts: tcl_syntax::naming::NativeNamespacePatternParts,
    span: Span,
    site: crate::command_binding::CommandAllocationSite,
    forced: bool,
    conditional_declaration: Option<SourceOriginalNameOccurrence>,
}

impl SourceNamespacePattern {
    pub(crate) fn from_original(
        operand: SourceOriginalNamespaceOperand,
        purpose: tcl_syntax::naming::NativeNamePurpose,
        span: Span,
        forced: bool,
    ) -> Option<Self> {
        use super::scope::{SignatureNamespaceScope, SignatureSourceNameInput};
        use tcl_syntax::naming::{NativeNameProtocol, NativeNamePurpose};
        let SourceOriginalNamespaceOperand {
            input,
            context,
            site,
            conditional_declaration,
        } = operand;
        if matches!(input, SignatureSourceNameInput::OriginalVariableRoot(_))
            || !matches!(
                purpose,
                NativeNamePurpose::NamespaceImportPattern
                    | NativeNamePurpose::NamespaceForgetPattern
            )
            || !matches!(
                (&context, input.policy().recipe()),
                (SignatureNamespaceScope::C(_), NativeNameProtocol::C(_))
                    | (SignatureNamespaceScope::Jim(_), NativeNameProtocol::Jim084)
            )
        {
            return None;
        }
        let parts = input
            .policy()
            .recipe()
            .namespace_pattern_parts(context.context()?, input.bytes(), purpose)
            .ok()?;
        Some(Self {
            input,
            context,
            purpose,
            parts,
            span,
            site,
            forced,
            conditional_declaration,
        })
    }

    /// Actual original import or forget invocation site.
    #[must_use]
    pub fn site(&self) -> &crate::command_binding::CommandAllocationSite {
        &self.site
    }
    /// Independent static source layout for a conditional import or forget.
    /// It supplies naming geometry without a completed handler or live alias.
    #[must_use]
    pub fn conditional_declaration(&self) -> Option<&SourceOriginalNameOccurrence> {
        self.conditional_declaration.as_ref()
    }
    /// Independently selected leading import control; forget never carries it.
    #[must_use]
    pub const fn forced(&self) -> bool {
        self.forced
    }
    /// Project a genuine call input's selected command tail into the retained
    /// import source namespace. This is query geometry without command
    /// existence, import or dispatch authority.
    #[must_use]
    pub fn source_slot_for_call_input(
        &self,
        input: &super::scope::SignatureSourceNameInput,
    ) -> Option<tcl_core_types::ByteCommandSlot> {
        use tcl_syntax::naming::NativeNameContext;
        if input.policy() != self.input.policy()
            || matches!(
                input,
                super::scope::SignatureSourceNameInput::OriginalVariableRoot(_)
            )
        {
            return None;
        }
        let slot = input
            .policy()
            .recipe()
            .command_lookup_slot(NativeNameContext::root(), input.bytes())
            .ok()?;
        self.source_slot_for_command_slot(&slot, input.policy())
    }

    /// Project an independently retained command slot's counted tail into the
    /// import source geometry. Intermediate aliases need no source declaration
    /// in that namespace; this projection supplies no alias installation.
    #[must_use]
    pub fn source_slot_for_command_slot(
        &self,
        slot: &tcl_core_types::ByteCommandSlot,
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> Option<tcl_core_types::ByteCommandSlot> {
        use tcl_syntax::naming::{NativeNamePurpose, NativeNamespacePatternSource};
        if self.purpose != NativeNamePurpose::NamespaceImportPattern
            || policy != self.input.policy()
        {
            return None;
        }
        let Some(NativeNamespacePatternSource::C(path)) = self.parts.source.as_ref() else {
            return None;
        };
        Some(tcl_core_types::ByteCommandSlot {
            namespace: path.clone(),
            simple: slot.simple.clone(),
        })
    }

    /// Match an independently retained source command against the exact import
    /// source namespace and counted pattern tail. No imported token is issued.
    #[must_use]
    pub fn matches_imported_command(
        &self,
        slot: &tcl_core_types::ByteCommandSlot,
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> Option<bool> {
        if self.purpose != tcl_syntax::naming::NativeNamePurpose::NamespaceImportPattern
            || self.input.policy() != policy
        {
            return None;
        }
        let Some(tcl_syntax::naming::NativeNamespacePatternSource::C(path)) =
            self.parts.source.as_ref()
        else {
            return None;
        };
        if *path != slot.namespace {
            return Some(false);
        }
        tcl_syntax::native_glob::NativeGlobProtocol::from_name_policy(policy)
            .match_name_pattern(
                tcl_syntax::native_glob::NativeNameGlobPurpose::ImportSearch,
                self.parts.tail.as_bytes(),
                slot.simple.as_bytes(),
            )
            .ok()
    }

    /// Original whole operand or readonly value; neither grants an edit.
    #[must_use]
    pub fn name_input(&self) -> &super::scope::SignatureSourceNameInput {
        &self.input
    }
    /// Independently retained source naming context at this operation.
    #[must_use]
    pub fn context(&self) -> &super::scope::SignatureNamespaceScope {
        &self.context
    }
    /// Import and forget use independent native pattern purposes.
    #[must_use]
    pub const fn purpose(&self) -> tcl_syntax::naming::NativeNamePurpose {
        self.purpose
    }
    /// Exact selected source namespace and counted tail pattern.
    #[must_use]
    pub fn parts(&self) -> &tcl_syntax::naming::NativeNamespacePatternParts {
        &self.parts
    }
    /// Original operand position; readonly computed values remain noneditable.
    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }
}

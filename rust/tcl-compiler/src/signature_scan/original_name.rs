// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Sealed static original naming units, independent of display and execution.

use tcl_core_types::NameBytes;
use tcl_lexer::{LexerConfig, NativeWord, SourceImage, Span};
use tcl_syntax::{naming::NamePolicyProtocol, word_rules::WordValueRules};

/// Positively selected Logical source naming, independent of authored Native
/// recipes and actual compilation entries. The descriptor grants only a word
/// projection; complete invocation and availability joins remain with callers.
#[derive(Debug, Clone, Copy)]
pub(crate) struct LogicalSourceNamePurpose {
    _private: (),
}

impl LogicalSourceNamePurpose {
    pub(crate) fn for_input(
        input: &crate::analyser::ResolvedAnalysisInput,
        entry: Option<&crate::command_binding::SourceAnalysisEntry>,
    ) -> Option<Self> {
        (input.has_logical_source_name_context()
            && entry.is_none_or(|entry| {
                entry.native_entry.is_none()
                    && !matches!(
                        entry.execution_name_policy,
                        Some(tcl_syntax::naming::ExecutionNamePolicy::NativeRecipe(_))
                    )
            }))
        .then_some(Self { _private: () })
    }
}

/// The source input selects one original word owner. Symbolic standalone
/// metadata has no name policy, Native key, command cell or execution receipt.
#[derive(Debug, Clone, Copy)]
pub(crate) enum OriginalSourceNamePurpose {
    Native(NamePolicyProtocol),
    Logical(NamePolicyProtocol, LogicalSourceNamePurpose),
    Symbolic,
}

impl OriginalSourceNamePurpose {
    pub(crate) fn for_analysis_input(
        input: &crate::analyser::ResolvedAnalysisInput,
        entry: Option<&crate::command_binding::SourceAnalysisEntry>,
        policy: NamePolicyProtocol,
    ) -> Self {
        LogicalSourceNamePurpose::for_input(input, entry).map_or(Self::Native(policy), |logical| {
            Self::Logical(policy, logical)
        })
    }

    pub(crate) const fn policy(self) -> Option<NamePolicyProtocol> {
        match self {
            Self::Native(policy) | Self::Logical(policy, _) => Some(policy),
            Self::Symbolic => None,
        }
    }

    pub(crate) fn key(
        self,
        original: &NativeWord,
        rules: WordValueRules,
    ) -> Option<SignatureSourceNameKey> {
        match self {
            Self::Native(policy) => {
                SignatureSourceNameKey::from_original_native_word(original, rules, policy)
            }
            Self::Logical(policy, _) => {
                SignatureSourceNameKey::from_original_logical_source_word(original, rules, policy)
            }
            Self::Symbolic => None,
        }
    }

    /// Complete Logical source list fields retain their own whole-word grammar
    /// and Unicode naming units. Native list/value inputs remain separate.
    pub(crate) fn logical_list_inputs(
        self,
        original: &NativeWord,
        rules: WordValueRules,
    ) -> Option<Vec<super::name_value::LogicalSourceListNameInput>> {
        let Self::Logical(policy, logical) = self else {
            return None;
        };
        super::name_value::LogicalSourceListNameInput::from_original_word(
            original, rules, policy, logical,
        )
    }

    pub(crate) fn symbolic_word(
        self,
        original: &NativeWord,
        rules: WordValueRules,
    ) -> Option<String> {
        if !matches!(self, Self::Symbolic)
            || rules != WordValueRules::from_config(&original.config())
        {
            return None;
        }
        let value = tcl_syntax::word_rules::original_static_word_unicode_value(original)?;
        if value.contains(&0) {
            return None;
        }
        String::from_utf8(value).ok()
    }
}

/// One complete original static naming word and its independently formed units.
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

    /// Capture readonly Unicode source units under the actual Document grammar.
    /// The authored naming abstraction is independent of that grammar and does
    /// not supply a Native string, literal pool, cell or execution capability.
    pub(crate) fn from_original_logical_source_word(
        original: &NativeWord,
        rules: WordValueRules,
        policy: NamePolicyProtocol,
    ) -> Option<Self> {
        if policy.authority() != tcl_syntax::naming::NamePolicyAuthority::AuthoredSimulation
            || rules != WordValueRules::from_config(&original.config())
        {
            return None;
        }
        let bytes = tcl_syntax::word_rules::original_static_word_unicode_value(original)?;
        Some(Self {
            original: original.clone(),
            bytes: bytes.into(),
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
    fn original_logical_name_units_keep_actual_grammar_without_native_word_admission() {
        // naming.variable.original-alias-source-template
        // docs/design/analysis/name-resolution-proofs/original-alias-source-template.md
        let config =
            LexerConfig::for_file_grammar(tcl_dialect::DialectProfile::plain_tcl().grammar);
        let policy = NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_6);
        let image = SourceImage::document(r"name\U0001F600");
        let plan = tcl_lexer::native_script_words_in(
            image.clone(),
            Span::new(0, u32::try_from(image.len()).unwrap()),
            config,
        )
        .unwrap();
        let original = &plan.commands[0].words[0];
        let key = SignatureSourceNameKey::from_original_logical_source_word(
            original,
            WordValueRules::from_config(&config),
            policy,
        )
        .expect("actual Logical document value");
        assert_eq!(key.bytes(), "name😀".as_bytes());
        assert_eq!(key.source_image(), &image);
        assert_eq!(key.lexer_config(), config);
        assert_eq!(key.policy(), policy);
        assert!(
            SignatureSourceNameKey::from_original_native_word(
                original,
                WordValueRules::from_config(&config),
                policy,
            )
            .is_none(),
            "source advice cannot supply the mismatched Native recipe"
        );
        for source in ["$name", "{*}{name}", r"name\uD800"] {
            let image = SourceImage::document(source);
            let plan = tcl_lexer::native_script_words_in(
                image.clone(),
                Span::new(0, u32::try_from(image.len()).unwrap()),
                config,
            )
            .unwrap();
            assert!(
                SignatureSourceNameKey::from_original_logical_source_word(
                    &plan.commands[0].words[0],
                    WordValueRules::from_config(&config),
                    policy,
                )
                .is_none(),
                "{source}"
            );
        }
        let native = NamePolicyProtocol::for_native_point(
            tcl_dialect::model::DialectPoint::of_dialect_name(Some("tcl9.0")).unwrap(),
        )
        .unwrap();
        assert!(
            SignatureSourceNameKey::from_original_logical_source_word(
                original,
                WordValueRules::from_config(&config),
                native,
            )
            .is_none()
        );
        assert!(
            SignatureSourceNameKey::from_original_logical_source_word(
                original,
                WordValueRules::JIM,
                policy,
            )
            .is_none()
        );
        let native_image = SourceImage::native(b"name".as_slice());
        let native_plan =
            tcl_lexer::native_script_words_in(native_image.clone(), Span::new(0, 4), config)
                .unwrap();
        assert!(
            SignatureSourceNameKey::from_original_logical_source_word(
                &native_plan.commands[0].words[0],
                WordValueRules::from_config(&config),
                policy,
            )
            .is_none()
        );
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
        match input {
            super::scope::SignatureSourceNameInput::OriginalWord(_)
            | super::scope::SignatureSourceNameInput::OriginalValue(_) => {}
            super::scope::SignatureSourceNameInput::OriginalVariableRoot(_)
            | super::scope::SignatureSourceNameInput::OriginalLogicalListField(_) => return None,
        }
        if input.policy() != self.input.policy() {
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

    /// Match an independently retained source command through the selected import
    /// search. C matches a namespace and tail; Jim searches its actual flat key.
    /// No imported token is issued.
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
        if let (
            tcl_syntax::naming::NativeNamespacePatternSource::Jim(_),
            super::scope::SignatureNamespaceScope::Jim(current),
        ) = (self.parts.source.as_ref()?, &self.context)
        {
            let recipe = policy.recipe();
            if !recipe.is_jim084() || !slot.namespace.is_root() {
                return None;
            }
            // The script refuses importing a name from its own namespace.
            if recipe.namespace_qualifier_bytes(slot.simple.as_bytes()) == current.as_bytes() {
                return Some(false);
            }
            let canonical = recipe
                .jim_namespace_canonical_input(
                    self.context.context()?,
                    self.parts.selected.as_bytes(),
                )
                .ok()?;
            return recipe
                .jim_core_command_names(
                    Some(canonical.selected()),
                    &[slot.simple.as_bytes().to_vec()],
                    false,
                )
                .ok()
                .map(|names| !names.is_empty());
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

    /// Conditional source relation between an actual forget pattern and an
    /// independently retained import/publication. This supplies no executed
    /// removal or live-slot absence. Unsupported policy/context stays unknown.
    #[must_use]
    pub fn forgets_source_import(
        &self,
        import: &Self,
        command: &super::scope::SignatureSourceCommand,
    ) -> Option<bool> {
        use tcl_syntax::{
            naming::{NativeNamePurpose, NativeNamespacePatternSource},
            native_glob::{NativeGlobProtocol, NativeNameGlobPurpose},
        };
        if self.purpose != NativeNamePurpose::NamespaceForgetPattern
            || import.purpose != NativeNamePurpose::NamespaceImportPattern
            || self.input.policy() != import.input.policy()
            || self.input.policy() != command.policy()
            || self.context != import.context
            || !matches!(self.context, super::scope::SignatureNamespaceScope::C(_))
        {
            return None;
        }
        if !import.matches_imported_command(command.slot(), command.policy())? {
            return Some(false);
        }
        let purpose = match self.parts.source.as_ref() {
            Some(NativeNamespacePatternSource::C(namespace)) => {
                if namespace != &command.slot().namespace {
                    return Some(false);
                }
                NativeNameGlobPurpose::ForgetOriginFilter
            }
            None => NativeNameGlobPurpose::ForgetOwnSearch,
            _ => return None,
        };
        NativeGlobProtocol::from_name_policy(command.policy())
            .match_name_pattern(
                purpose,
                self.parts.tail.as_bytes(),
                command.slot().simple.as_bytes(),
            )
            .ok()
    }

    /// Borrow the actual imported tail from a matching retained publication.
    /// Jim's source slot contains one flat key, not C namespace components.
    /// This projection supplies no live alias, completed handler or Native proof.
    #[must_use]
    pub fn matched_imported_command_tail<'a>(
        &self,
        command: &'a super::scope::SignatureSourceCommand,
    ) -> Option<&'a [u8]> {
        if self.matches_imported_command(command.slot(), command.policy()) != Some(true) {
            return None;
        }
        Some(if command.policy().recipe().is_jim084() {
            command
                .policy()
                .recipe()
                .namespace_tail_bytes(command.slot().simple.as_bytes())
        } else {
            command.slot().simple.as_bytes()
        })
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

#[cfg(test)]
mod original_import_publication_tests {
    use super::SourceNamespacePattern;
    use crate::analyser::Analyser;

    #[test]
    fn actual_import_publications_keep_selected_search_and_original_tail() {
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
            for (name, pattern) in [
                ("p*", r"p\*"),
                ("p?", r"p\?"),
                ("p[", r"p\["),
                (r"p\x", r"p\\x"),
            ] {
                let source = format!(
                    "namespace eval ::Source {{proc {{{name}}} {{}} {{}}}}; namespace eval ::Target {{namespace import {{::Source::{pattern}}}}}"
                );
                let analysis = Analyser::new().analyse(&source, dialect);
                let import = analysis.original_namespace_patterns().next().unwrap();
                let publication = analysis.original_procedure_declarations().next().unwrap();
                assert_eq!(
                    SourceNamespacePattern::matched_imported_command_tail(
                        import,
                        publication.name()
                    ),
                    Some(name.as_bytes()),
                    "{dialect}/{pattern}"
                );
                let foreign = if dialect == "jimtcl" {
                    tcl_syntax::naming::NamePolicyProtocol::authored_tcl(
                        tcl_dialect::TclVersion::V8_6,
                    )
                } else {
                    tcl_syntax::naming::NamePolicyProtocol::authored_jim084()
                };
                assert_eq!(
                    import.matches_imported_command(publication.name().slot(), foreign),
                    None
                );
            }
        }
        let analysis = Analyser::new().analyse(
            "namespace eval ::Self {proc p {} {}; namespace import ::Self::*}",
            "jimtcl",
        );
        let import = analysis.original_namespace_patterns().next().unwrap();
        let publication = analysis.original_procedure_declarations().next().unwrap();
        assert_eq!(
            import.matches_imported_command(publication.name().slot(), publication.name().policy()),
            Some(false)
        );
        assert_eq!(
            SourceNamespacePattern::matched_imported_command_tail(import, publication.name()),
            None
        );
    }
}

/// Genuine Logical namespace source role and its independently retained caller
/// context. This supplies neither namespace existence nor entered execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogicalSourceNamespaceOccurrence {
    image: SourceImage,
    input: crate::analyser::ResolvedAnalysisInput,
    operand: SourceOriginalNamespaceOperand,
    row: crate::analyser::types::NamespaceRef,
    words: crate::registry_invocation::source_structure::OriginalRegistryWords,
}

impl LogicalSourceNamespaceOccurrence {
    /// Join the exact analyser row to its whole original argv, Registry role,
    /// independent namespace context and actual source allocation instruction.
    #[must_use]
    pub fn capture(
        analysis: &crate::analyser::AnalysisResult,
        row: &crate::analyser::types::NamespaceRef,
    ) -> Option<Self> {
        let input = analysis.resolved_input.as_ref()?;
        input.has_logical_source_name_context().then_some(())?;
        analysis
            .namespace_refs
            .iter()
            .any(|current| std::ptr::eq(current, row))
            .then_some(())?;
        let realm = analysis.retained_command_realm()?;
        realm.matches_resolved_analysis_input(input).then_some(())?;
        let original = row.original_name_input.as_ref()?;
        let word = original.original_word_key()?.original_word();
        let image = word.image();
        analysis
            .matches_original_source_image(image, input.lexer_config())
            .then_some(())?;
        let operand = realm.original_namespace_operand_at_span_in_source(
            image,
            row.span,
            input.lexer_config(),
        )?;
        (operand.name_input() == original
            && row.source_context.as_ref() == Some(operand.context())
            && row.source_namespace == operand.context().child_from_input(original)
            && row.source_namespace.is_some()
            && row.name_policy == Some(original.policy()))
        .then_some(())?;
        let words = crate::registry_invocation::source_structure::source_registry_words_at(
            image.try_text().ok()?,
            analysis,
            operand.site().offset,
        )?;
        let mut ordinals = words.roles()?.iter().filter_map(|(ordinal, role)| {
            (*role == tcl_registry::ArgRole::NamespaceName
                && words.original_argument_word(*ordinal) == Some(word))
            .then_some(*ordinal)
        });
        let ordinal = ordinals.next()?;
        ordinals.next().is_none().then_some(())?;
        let context = input.borrowed_context_registry();
        let declaration_agrees = words.with_source_schema(context, |schema| {
            let declares = schema.semantics.traits.contains(tcl_registry::Traits::DECLARES_NAMESPACE);
            let ensure = schema.authored_logical_source_state_transitions().is_some_and(|transitions| transitions.facts().iter().any(|fact| {
                matches!(&fact.transition, tcl_registry::StateTransition::Namespace(tcl_registry::NamespaceTransition::Ensure { namespace: tcl_registry::NamespaceTransitionTarget::Named(subject) }) if subject.argument_index() == Some(ordinal))
            }));
            row.declares == declares && (!declares || ensure)
        })?;
        declaration_agrees.then_some(Self {
            image: image.clone(),
            input: input.clone(),
            operand,
            row: row.clone(),
            words,
        })
    }

    /// Exact original row; report labels are not its source identity.
    #[must_use]
    pub fn row(&self) -> &crate::analyser::types::NamespaceRef {
        &self.row
    }

    /// Full immutable input for independent cross-document correspondence.
    #[must_use]
    pub fn input(&self) -> &crate::analyser::ResolvedAnalysisInput {
        &self.input
    }

    /// Immutable whole original document image.
    #[must_use]
    pub fn image(&self) -> &SourceImage {
        &self.image
    }

    /// Actual original source allocation, independent of equal source reanalysis.
    #[must_use]
    pub fn shares_original_allocation(&self, other: &Self) -> bool {
        std::sync::Arc::ptr_eq(&self.operand.site().source, &other.operand.site().source)
            && self.operand.site().offset == other.operand.site().offset
            && same_namespace_source_row(&self.row, &other.row)
            && self.input == other.input
    }

    /// Revalidate the actual complete input and retained role/context producer.
    /// Equal reports cannot stand in for the original operand or Ensure facet.
    #[must_use]
    pub fn is_current(&self, analysis: &crate::analyser::AnalysisResult) -> bool {
        analysis
            .namespace_refs
            .iter()
            .filter(|row| same_namespace_source_row(row, &self.row))
            .any(|row| {
                Self::capture(analysis, row).is_some_and(|current| {
                    current.input == self.input
                        && current.image == self.image
                        && current.operand == self.operand
                        && current.words == self.words
                })
            })
    }
}

fn same_namespace_source_row(
    left: &crate::analyser::types::NamespaceRef,
    right: &crate::analyser::types::NamespaceRef,
) -> bool {
    left.original_name_input == right.original_name_input
        && left.source_context == right.source_context
        && left.source_namespace == right.source_namespace
        && left.name_policy == right.name_policy
        && left.span == right.span
        && left.declares == right.declares
}

#[cfg(test)]
mod logical_namespace_occurrence_tests {
    use super::*;
    use crate::analyser::Analyser;

    #[test]
    fn namespace_source_occurrence_requires_the_actual_role_scope_and_ensure() {
        let source = "namespace eval ::A {}; namespace exists ::A";
        let analysis = Analyser::new().analyse(source, "tcl");
        let declaration = analysis
            .namespace_refs
            .iter()
            .find(|row| row.declares)
            .unwrap();
        let captured = LogicalSourceNamespaceOccurrence::capture(&analysis, declaration).unwrap();
        assert!(captured.is_current(&analysis));
        assert_eq!(captured.image(), &SourceImage::document(source));
        let mut reporting = analysis.clone();
        for row in &mut reporting.namespace_refs {
            row.original_name = "OTHER".to_owned();
            row.qualified_name = "OTHER".to_owned();
            row.source_span = None;
        }
        assert!(captured.is_current(&reporting));
        let mut wrong_declaration = analysis.clone();
        let reference_span = {
            let reference = wrong_declaration
                .namespace_refs
                .iter_mut()
                .find(|row| !row.declares)
                .unwrap();
            reference.declares = true;
            reference.span
        };
        let reference = wrong_declaration
            .namespace_refs
            .iter()
            .find(|row| row.span == reference_span)
            .unwrap();
        assert!(LogicalSourceNamespaceOccurrence::capture(&wrong_declaration, reference).is_none());
        let mut no_scope = analysis.clone();
        no_scope.namespace_refs[0].source_context = None;
        assert!(
            LogicalSourceNamespaceOccurrence::capture(&no_scope, &no_scope.namespace_refs[0])
                .is_none()
        );
        let changed = Analyser::new().analyse("namespace eval ::B {}", "tcl");
        assert!(!captured.is_current(&changed));
    }
}

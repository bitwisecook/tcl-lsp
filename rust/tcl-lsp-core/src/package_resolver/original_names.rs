// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Package and autoload command queries using retained original byte producers.

mod file_candidates;
pub use file_candidates::{OriginalPackageFileCandidate, OriginalPackageFileCandidateKind};

use super::PackageResolver;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};
use tcl_compiler::{
    command_binding::SourceInvocationBinding,
    signature_scan::scope::{
        SignatureSourceCommand, SignatureSourceNameKey, SignatureSourceNameValue,
    },
};
use tcl_core_types::NameBytes;
use tcl_lexer::{LexerConfig, SourceImage, Span};
use tcl_syntax::{naming::NamePolicyProtocol, word_rules::WordValueRules};

/// Package advice identity with an explicit source/metadata boundary.
/// Unowned source text cannot manufacture a native database key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PackageRequirementAdviceKey {
    /// Independently retained source operand projected by the package purpose.
    Original(tcl_registry::native_package::NativePackageNameKey),
    /// Explicit ASCII catalogue/dependency metadata, without source authority.
    AuthoredMetadata(String),
    /// Computed or missing source identity, including non-ASCII report labels.
    Unknown,
}

impl PackageRequirementAdviceKey {
    /// Retain the authentic package operand; a missing receipt remains unknown
    /// even when the report happens to contain a readable package name.
    #[must_use]
    pub fn from_source_requirement(
        requirement: &tcl_compiler::signature_scan::types::SignaturePackageRequire,
    ) -> Self {
        Self::from_original_name(requirement.original_name.as_ref())
    }

    /// Transport the shared source package receipt across analyser and
    /// signature metadata. Missing original input remains unknown.
    #[must_use]
    pub fn from_original_name(
        name: Option<&tcl_compiler::signature_scan::original_name::SourcePackageName>,
    ) -> Self {
        name.map_or(Self::Unknown, |name| Self::Original(name.key().clone()))
    }

    /// Explicit unowned catalogue metadata. This supplies no source-name
    /// producer and never converts a Unicode report into native units.
    #[must_use]
    pub fn authored_metadata(name: &str) -> Self {
        if name.is_ascii() && !name.as_bytes().contains(&0) {
            Self::AuthoredMetadata(name.to_owned())
        } else {
            Self::Unknown
        }
    }

    /// Compare a fixed ASCII Registry package fact using its metadata purpose,
    /// without claiming that this fact was a source-produced package operand.
    #[must_use]
    pub fn matches_ascii_metadata(&self, name: &str) -> bool {
        match self {
            Self::Original(key) => key.matches_ascii(name),
            Self::AuthoredMetadata(metadata) => name.is_ascii() && metadata == name,
            Self::Unknown => false,
        }
    }

    /// Retained native package purpose, independently of its reporting label.
    #[must_use]
    pub const fn original(&self) -> Option<&tcl_registry::native_package::NativePackageNameKey> {
        match self {
            Self::Original(key) => Some(key),
            _ => None,
        }
    }
}

/// One package requirement's identity and independent version-selection facts.
/// The caller supplies the preference selected at this requirement's source
/// point; a dependency keeps its own constraints and preference when inherited.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageRequirementAdvice {
    key: PackageRequirementAdviceKey,
    requirements: Vec<String>,
    exact: bool,
    prefer: super::PackagePrefer,
}

impl std::hash::Hash for PackageRequirementAdvice {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::hash::Hash::hash(&self.key, state);
        std::hash::Hash::hash(&self.requirements, state);
        std::hash::Hash::hash(&self.exact, state);
        std::hash::Hash::hash(&matches!(self.prefer, super::PackagePrefer::Latest), state);
    }
}

impl PackageRequirementAdvice {
    /// Retain a source requirement's authentic name and all alternative version
    /// words. `prefer` is an independently selected caller-point fact.
    #[must_use]
    pub fn from_source_requirement(
        requirement: &tcl_compiler::signature_scan::types::SignaturePackageRequire,
        prefer: super::PackagePrefer,
    ) -> Self {
        Self::new(
            PackageRequirementAdviceKey::from_source_requirement(requirement),
            requirement.requirements.clone(),
            requirement.exact,
            prefer,
        )
    }

    /// Couple existing name advice with version metadata. This constructor
    /// cannot mint a source name, source point or package execution receipt.
    #[must_use]
    pub fn new(
        key: PackageRequirementAdviceKey,
        requirements: Vec<String>,
        exact: bool,
        prefer: super::PackagePrefer,
    ) -> Self {
        Self {
            key,
            requirements,
            exact,
            prefer,
        }
    }

    /// Explicit unconstrained catalogue advice, with no source operand grant.
    #[must_use]
    pub fn authored_metadata(name: &str, prefer: super::PackagePrefer) -> Self {
        Self::new(
            PackageRequirementAdviceKey::authored_metadata(name),
            Vec::new(),
            false,
            prefer,
        )
    }

    /// Independently retained package name identity.
    #[must_use]
    pub const fn key(&self) -> &PackageRequirementAdviceKey {
        &self.key
    }
    /// All version alternatives, in the order supplied by this requirement.
    #[must_use]
    pub fn requirements(&self) -> &[String] {
        &self.requirements
    }
    /// Whether the source selected exact version matching.
    #[must_use]
    pub const fn exact(&self) -> bool {
        self.exact
    }
    /// Preference selected at this requirement, without inheriting another
    /// requirement's source timeline or package-name policy.
    #[must_use]
    pub const fn prefer(&self) -> super::PackagePrefer {
        self.prefer
    }

    fn unknown(mut self) -> Self {
        self.key = PackageRequirementAdviceKey::Unknown;
        self
    }
}

impl PackageResolver {
    /// Retain exact original package operands under each C loader release.
    /// Registration guards remain symbolic; this inventory supplies source
    /// candidates and does not prove an index or loader executed.
    pub fn add_original_pkg_index(
        &mut self,
        content: &str,
        directory: &Path,
        index: &Path,
        exists: &dyn Fn(&Path) -> bool,
        list_files: &dyn Fn(&Path) -> Vec<PathBuf>,
    ) {
        self.invalidate();
        for version in tcl_dialect::TclVersion::ALL {
            for info in
                original_package_entries(content, directory, index, exists, list_files, version)
            {
                self.original_packages
                    .entry(info.name.clone())
                    .or_default()
                    .push(info);
            }
        }
    }

    /// Exact package-registration candidates using the selected native key.
    /// The same shared version/guard owner serves authored and original data.
    #[must_use]
    pub fn candidate_original_providers_for_require(
        &self,
        name: &tcl_registry::native_package::NativePackageNameKey,
        requirements: &[&str],
        exact: bool,
        prefer: super::PackagePrefer,
    ) -> Vec<&OriginalPackageInfo> {
        let Some(infos) = self.original_packages.get(name) else {
            return Vec::new();
        };
        let tcl_syntax::naming::NativeNameProtocol::C(version) = name.policy().recipe() else {
            return Vec::new();
        };
        let candidates = super::candidate_package_infos(
            &infos.iter().map(|info| &info.info).collect::<Vec<_>>(),
            requirements,
            exact,
            prefer,
            Some(version),
        );
        infos
            .iter()
            .filter(|info| candidates.contains(&&info.info))
            .collect()
    }

    /// Candidate implementation files for a genuine original package key.
    #[must_use]
    pub fn resolve_original_require(
        &self,
        name: &tcl_registry::native_package::NativePackageNameKey,
        requirements: &[&str],
        exact: bool,
        prefer: super::PackagePrefer,
    ) -> Vec<PathBuf> {
        let mut files = Vec::new();
        for info in self.candidate_original_providers_for_require(name, requirements, exact, prefer)
        {
            for file in &info.info.source_files {
                if !files.contains(file) {
                    files.push(file.clone());
                }
            }
        }
        files
    }

    /// Whether the original package database contains this exact key.
    #[must_use]
    pub fn provides_original_package(
        &self,
        name: &tcl_registry::native_package::NativePackageNameKey,
    ) -> bool {
        self.original_packages
            .get(name)
            .is_some_and(|infos| !infos.is_empty())
    }

    /// Command publications from files contributed by exact original packages.
    #[must_use]
    pub fn original_packages_defined_commands(
        &self,
        available: &[tcl_registry::native_package::NativePackageNameKey],
        extract: &dyn Fn(&Path) -> Vec<SignatureSourceCommand>,
    ) -> HashSet<SignatureSourceCommand> {
        available
            .iter()
            .flat_map(|name| {
                self.resolve_original_require(name, &[], false, super::PackagePrefer::default())
            })
            .flat_map(|file| extract(&file))
            .collect()
    }

    /// Source package dependencies through exact original database keys.
    /// The caller owns declaration/read scope and supplies admitted source
    /// dependencies; this closure proves no loader or package operation ran.
    #[must_use]
    pub fn transitive_original_available_packages(
        &self,
        roots: &[tcl_registry::native_package::NativePackageNameKey],
        dependencies: &dyn Fn(&Path) -> Vec<tcl_registry::native_package::NativePackageNameKey>,
    ) -> HashSet<tcl_registry::native_package::NativePackageNameKey> {
        let mut available = HashSet::new();
        let mut pending = roots.to_vec();
        while let Some(name) = pending.pop() {
            if !available.insert(name.clone()) {
                continue;
            }
            for file in
                self.resolve_original_require(&name, &[], false, super::PackagePrefer::default())
            {
                pending.extend(dependencies(&file).into_iter().filter(|dependency| {
                    dependency.policy() == name.policy() && !available.contains(dependency)
                }));
            }
        }
        available
    }

    /// Candidate implementation files for explicit package advice. Original
    /// requirements use only original providers and their own release/policy;
    /// ASCII catalogue metadata stays on the existing metadata database.
    #[must_use]
    pub fn resolve_package_advice(
        &self,
        name: &PackageRequirementAdviceKey,
        requirements: &[&str],
        exact: bool,
        prefer: super::PackagePrefer,
        target: Option<tcl_dialect::TclVersion>,
    ) -> Vec<PathBuf> {
        match name {
            PackageRequirementAdviceKey::Original(key) => {
                self.resolve_original_require(key, requirements, exact, prefer)
            }
            PackageRequirementAdviceKey::AuthoredMetadata(name) => {
                let mut files = Vec::new();
                for provider in
                    self.candidate_providers_for_require(name, requirements, exact, prefer, target)
                {
                    for file in &provider.source_files {
                        if !files.contains(file) {
                            files.push(file.clone());
                        }
                    }
                }
                files
            }
            PackageRequirementAdviceKey::Unknown => Vec::new(),
        }
    }

    /// Resolve a complete requirement through the shared version/guard owner.
    /// Source keys select their own native policy; unowned metadata uses the
    /// explicit target. An unknown identity yields no implementation files.
    #[must_use]
    pub fn resolve_requirement_advice(
        &self,
        requirement: &PackageRequirementAdvice,
        target: Option<tcl_dialect::TclVersion>,
    ) -> Vec<PathBuf> {
        let alternatives = requirement
            .requirements
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>();
        self.resolve_package_advice(
            requirement.key(),
            &alternatives,
            requirement.exact(),
            requirement.prefer(),
            target,
        )
    }

    /// Dependency closure preserving each edge's exact constraints and source
    /// preference. Unknown identities and incompatible source policies remain
    /// explicit residuals and never donate dependency files or source grants.
    #[must_use]
    pub fn transitive_requirement_advice(
        &self,
        roots: &[PackageRequirementAdvice],
        target: Option<tcl_dialect::TclVersion>,
        dependencies: &dyn Fn(&Path) -> Vec<PackageRequirementAdvice>,
    ) -> HashSet<PackageRequirementAdvice> {
        self.transitive_requirement_advice_with_context(roots, target, &|path, _| {
            dependencies(path)
        })
    }

    /// Context-preserving dependency closure. The scanner receives the actual
    /// requirement which selected this file, including its preference; a
    /// locally raised preference can therefore reach subsequent dependencies.
    #[must_use]
    pub fn transitive_requirement_advice_with_context(
        &self,
        roots: &[PackageRequirementAdvice],
        target: Option<tcl_dialect::TclVersion>,
        dependencies: &dyn Fn(&Path, &PackageRequirementAdvice) -> Vec<PackageRequirementAdvice>,
    ) -> HashSet<PackageRequirementAdvice> {
        let mut available = HashSet::new();
        let mut pending = roots.to_vec();
        while let Some(requirement) = pending.pop() {
            if !available.insert(requirement.clone())
                || requirement.key() == &PackageRequirementAdviceKey::Unknown
            {
                continue;
            }
            for file in self.resolve_requirement_advice(&requirement, target) {
                pending.extend(
                    dependencies(&file, &requirement)
                        .into_iter()
                        .map(|dependency| {
                            match (requirement.key().original(), dependency.key().original()) {
                                (Some(parent), Some(child))
                                    if parent.policy() != child.policy() =>
                                {
                                    dependency.unknown()
                                }
                                _ => dependency,
                            }
                        })
                        .filter(|dependency| !available.contains(dependency)),
                );
            }
        }
        available
    }

    /// Source publications contributed by precisely selected requirement
    /// candidates. Registration and dependency advice prove no loader ran.
    #[must_use]
    pub fn requirement_advice_defined_commands(
        &self,
        available: &[PackageRequirementAdvice],
        target: Option<tcl_dialect::TclVersion>,
        extract: &dyn Fn(&Path) -> Vec<SignatureSourceCommand>,
    ) -> HashSet<SignatureSourceCommand> {
        available
            .iter()
            .flat_map(|requirement| self.resolve_requirement_advice(requirement, target))
            .flat_map(|file| extract(&file))
            .collect()
    }

    /// Shared dependency closure over independently supplied source operands
    /// and explicit catalogue metadata. It proves package availability advice,
    /// never loader execution; unknown source identities contribute no key.
    #[must_use]
    pub fn transitive_package_advice(
        &self,
        roots: &[PackageRequirementAdviceKey],
        target: Option<tcl_dialect::TclVersion>,
        dependencies: &dyn Fn(&Path) -> Vec<PackageRequirementAdviceKey>,
    ) -> HashSet<PackageRequirementAdviceKey> {
        let unconstrained = |key| {
            PackageRequirementAdvice::new(key, Vec::new(), false, super::PackagePrefer::default())
        };
        let roots = roots.iter().cloned().map(unconstrained).collect::<Vec<_>>();
        self.transitive_requirement_advice(&roots, target, &|path| {
            dependencies(path).into_iter().map(unconstrained).collect()
        })
        .into_iter()
        .map(|requirement| requirement.key)
        .collect()
    }

    /// Exact source command publications contributed by the shared typed
    /// package closure. Metadata package names do not become source keys.
    #[must_use]
    pub fn package_advice_defined_commands(
        &self,
        available: &[PackageRequirementAdviceKey],
        target: Option<tcl_dialect::TclVersion>,
        extract: &dyn Fn(&Path) -> Vec<SignatureSourceCommand>,
    ) -> HashSet<SignatureSourceCommand> {
        available
            .iter()
            .flat_map(|name| {
                self.resolve_package_advice(
                    name,
                    &[],
                    false,
                    super::PackagePrefer::default(),
                    target,
                )
            })
            .flat_map(|file| extract(&file))
            .collect()
    }

    /// Retain original static tclIndex registrations for each C loader release.
    /// The source parser and value producer retain their own input/configuration;
    /// authored compatibility AutoIndexEntry strings supply no original keys.
    pub fn add_original_tcl_index(
        &mut self,
        content: &str,
        index_dir: &Path,
        exists: &dyn Fn(&Path) -> bool,
    ) {
        self.invalidate();
        for version in tcl_dialect::TclVersion::ALL {
            let profile = tcl_dialect::DialectProfile::find(version.dialect_name())
                .expect("C release profile");
            let config = LexerConfig::from_grammar(
                tcl_registry::InvocationDialect::of_profile(profile).lexer_grammar,
            );
            let policy = NamePolicyProtocol::authored_tcl(version);
            for entry in original_index_entries(content, index_dir, exists, config, policy) {
                self.original_auto_index
                    .entry((
                        policy.string_protocol(),
                        NameBytes::from(entry.input.bytes()),
                    ))
                    .or_default()
                    .push(entry);
            }
        }
    }

    /// Original auto_load candidates at the command's retained caller point.
    /// Native byte identity is independent of its optional diagnostic rendering.
    #[must_use]
    pub fn resolve_original_auto_command(
        &self,
        key: &SignatureSourceNameKey,
        invocation: &SourceInvocationBinding,
    ) -> Vec<PathBuf> {
        let Some(candidates) = invocation.original_autoload_command_candidates(key) else {
            return Vec::new();
        };
        for candidate in candidates {
            if let Some(files) = self
                .original_auto_index
                .get(&(key.policy().string_protocol(), candidate))
            {
                return files.iter().map(|entry| entry.file.clone()).collect();
            }
        }
        Vec::new()
    }

    /// Whether a retained original command can be loaded from a genuine index.
    /// Missing caller scope or source ownership supplies no global fallback.
    #[must_use]
    pub fn auto_loads_original_command(
        &self,
        key: &SignatureSourceNameKey,
        invocation: &SourceInvocationBinding,
    ) -> bool {
        !self
            .resolve_original_auto_command(key, invocation)
            .is_empty()
    }

    /// Exact command-publication slots contributed by reachable package sources.
    /// The caller owns the original declaration extractor; presentation maps
    /// cannot produce this inventory.
    #[must_use]
    pub fn package_defined_original_commands(
        &self,
        available: &[String],
        target: Option<tcl_dialect::TclVersion>,
        defined_commands: &dyn Fn(&Path) -> Vec<SignatureSourceCommand>,
    ) -> HashSet<SignatureSourceCommand> {
        available
            .iter()
            .flat_map(|package| self.reachable_files(package, target))
            .flat_map(|file| defined_commands(&file))
            .collect()
    }

    /// Join original caller candidates to independently published package slots.
    /// This supplies package availability advice, never runtime token identity.
    #[must_use]
    pub fn package_defines_original_command(
        &self,
        key: &SignatureSourceNameKey,
        invocation: &SourceInvocationBinding,
        commands: &HashSet<SignatureSourceCommand>,
    ) -> bool {
        invocation
            .original_static_command_lookup(key)
            .and_then(|lookup| {
                lookup.matching_publications(commands.iter().map(|command| (command, command)))
            })
            .is_some_and(|selected| !selected.is_empty())
    }
}

/// Original package-name producer paired with its selected database purpose.
/// File/version metadata is advisory independently of native package identity.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalPackageInfo {
    input: SignatureSourceNameKey,
    name: tcl_registry::native_package::NativePackageNameKey,
    info: super::PackageInfo,
    files: Vec<OriginalPackageFileCandidate>,
}

impl OriginalPackageInfo {
    /// Complete original operand and its full source/grammar lineage.
    #[must_use]
    pub const fn name_input(&self) -> &SignatureSourceNameKey {
        &self.input
    }
    /// Independently purpose-selected native package database key.
    #[must_use]
    pub const fn name(&self) -> &tcl_registry::native_package::NativePackageNameKey {
        &self.name
    }
    /// Version, source candidates and symbolic registration guards.
    #[must_use]
    pub const fn metadata(&self) -> &super::PackageInfo {
        &self.info
    }
    /// File candidates retain source-operand, deferred-text or directory
    /// provenance independently of the native package-name key. None proves
    /// that a deferred loader ran, a source command succeeded or a file loaded.
    #[must_use]
    pub fn file_candidates(&self) -> &[OriginalPackageFileCandidate] {
        &self.files
    }
}

fn original_package_entries(
    content: &str,
    directory: &Path,
    index: &Path,
    exists: &dyn Fn(&Path) -> bool,
    list_files: &dyn Fn(&Path) -> Vec<PathBuf>,
    version: tcl_dialect::TclVersion,
) -> Vec<OriginalPackageInfo> {
    use tcl_registry::model::binding::PackageTransition;
    use tcl_registry::{InvocationWord, InvocationWords, StateTransition};
    let dialect = tcl_registry::InvocationDialect::for_version(version);
    let config = LexerConfig::for_file_grammar(dialect.lexer_grammar);
    let policy = dialect
        .authored_name_policy()
        .expect("C loader naming recipe");
    let registry =
        tcl_registry::model::ingress::static_context_for(version.dialect_name()).commands();
    let profile =
        tcl_dialect::DialectProfile::find(version.dialect_name()).expect("C loader release");
    let analysis = crate::source_structure::analyse_document(content, profile, registry, config);
    let Some(bindings) = analysis.retained_command_realm() else {
        return Vec::new();
    };
    let filenames = file_candidates::FileCandidateScanner::new(content, &analysis);
    let mut entries = Vec::new();
    if super::reachability::scan_with_analysis(
        content,
        &analysis,
        registry,
        config,
        &mut |reached| {
            let Some(words) = reached.original_words else {
                return;
            };
            let keys: Vec<_> = words
                .iter()
                .map(|word| {
                    SignatureSourceNameKey::from_original_native_word(
                        word,
                        WordValueRules::from_config(&config),
                        policy,
                    )
                })
                .collect();
            let arguments: Vec<_> = keys
                .iter()
                .skip(1)
                .map(|key| {
                    key.as_ref()
                        .and_then(SignatureSourceNameKey::display)
                        .map_or(InvocationWord::Dynamic, InvocationWord::Literal)
                })
                .collect();
            let Some(head_key) = keys.first().and_then(Option::as_ref) else {
                return;
            };
            let invocation =
                bindings.invocation_at_source("", head_key.original_word().group().span.start());
            let Some(target) = invocation.original_registry_target_for_name(head_key) else {
                return;
            };
            let Some(head) = target.registry_identity() else {
                return;
            };
            let Some(resolved) = registry
                .resolve_structured_invocation(
                    InvocationWords::structured(InvocationWord::Literal(head), &arguments)
                        .with_dialect(dialect),
                    dialect.authoring_query(),
                )
                .resolved()
            else {
                return;
            };
            let transitions = resolved.state_transitions();
            for fact in transitions.facts() {
                let StateTransition::Package(PackageTransition::Ifneeded {
                    package,
                    version: version_subject,
                    script: Some(script),
                    script_provided: true,
                }) = &fact.transition
                else {
                    continue;
                };
                let Some(input) = package
                    .argument_index()
                    .and_then(|ordinal| keys.get(ordinal + 1))
                    .and_then(Option::as_ref)
                    .cloned()
                else {
                    continue;
                };
                let Some(advertised) = version_subject
                    .argument_index()
                    .and_then(|ordinal| keys.get(ordinal + 1))
                    .and_then(Option::as_ref)
                    .and_then(SignatureSourceNameKey::display)
                else {
                    continue;
                };
                if !tcl_dialect::validate_version_for(advertised, version) {
                    continue;
                }
                let Some(body_index) = script.argument_index() else {
                    continue;
                };
                let Some(loader) = words.get(body_index + 1) else {
                    continue;
                };
                let files = filenames.as_ref().map_or_else(Vec::new, |scanner| {
                    scanner.collect(&words, loader, directory, exists, list_files)
                });
                let source_files = files.iter().map(|file| file.path().to_owned()).collect();
                entries.push(OriginalPackageInfo {
                    name: tcl_registry::native_package::NativePackageNameKey::from_native_units(
                        input.bytes(),
                        policy,
                    ),
                    info: super::PackageInfo {
                        name: input.display().unwrap_or_default().to_owned(),
                        version: advertised.to_owned(),
                        source_files,
                        pkg_index_path: index.to_owned(),
                        conditions: reached.conditions.clone(),
                    },
                    input,
                    files,
                });
            }
        },
    )
    .is_none()
    {
        return Vec::new();
    }
    entries
}

#[derive(Debug)]
pub(super) struct OriginalAutoIndexRegistration {
    input: SignatureSourceNameValue,
    file: PathBuf,
}

fn original_index_entries(
    content: &str,
    index_dir: &Path,
    exists: &dyn Fn(&Path) -> bool,
    config: LexerConfig,
    policy: NamePolicyProtocol,
) -> Vec<OriginalAutoIndexRegistration> {
    let image = SourceImage::document(content);
    let Some(lines) = SignatureSourceNameValue::original_source_lines(&image, config, policy)
    else {
        return Vec::new();
    };
    let mut entries = Vec::new();
    if lines.first().map(SignatureSourceNameValue::bytes)
        == Some(b"# Tcl autoload index file: each line identifies a Tcl".as_slice())
    {
        for value in lines.into_iter().skip(1) {
            if value.bytes().starts_with(b"#") || value.list_length() != Some(2) {
                continue;
            }
            let Some(input) = value.list_element(0) else {
                continue;
            };
            let Some(filename) = value.list_element(1) else {
                continue;
            };
            let Some(filename) = filename.display() else {
                continue;
            };
            let file = index_dir.join(filename);
            if exists(&file) {
                entries.push(OriginalAutoIndexRegistration { input, file });
            }
        }
        return entries;
    }
    let Some(length) = u32::try_from(content.len()).ok() else {
        return Vec::new();
    };
    let Ok(plan) = tcl_lexer::native_script_words_in(image, Span::new(0, length), config) else {
        return Vec::new();
    };
    let version = match policy.recipe() {
        tcl_syntax::naming::NativeNameProtocol::C(version) => version,
        _ => return entries,
    };
    let profile =
        tcl_dialect::DialectProfile::find(version.dialect_name()).expect("C loader release");
    let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
    let analysis = crate::source_structure::analyse_document(content, profile, registry, config);
    let Some(context) = analysis
        .resolved_input
        .as_ref()
        .map(|input| input.context_registry())
    else {
        return entries;
    };
    let commands =
        tcl_compiler::segmenter::segment_commands_with_offset_and_config(content, 0, config);
    for command in plan.commands {
        let words = &command.words;
        if words.len() != 3 {
            continue;
        }
        let Some(head) = SignatureSourceNameKey::from_original_native_word(
            &words[0],
            WordValueRules::from_config(&config),
            policy,
        ) else {
            continue;
        };
        let Some(command) = commands.iter().find(|command| {
            command
                .argv
                .first()
                .is_some_and(|word| word.span.start() == head.span().start())
        }) else {
            continue;
        };
        let Some(schema) =
            tcl_compiler::registry_invocation::source_structure::source_registry_words(
                content, &analysis, command,
            )
        else {
            continue;
        };
        if schema.with_source_schema(&context, |schema| {
            schema.semantics.lowering_hook == Some(tcl_registry::hooks::LoweringHookId::Set)
        }) != Some(true)
        {
            continue;
        }
        let Some(input) = SignatureSourceNameValue::from_original_variable_index(
            &words[1],
            WordValueRules::from_config(&config),
            policy,
        ) else {
            continue;
        };
        if input.variable_root() != Some(b"auto_index".as_slice()) {
            continue;
        }
        let Some(inner) = super::word_unwrap(content, words[2].tokens()) else {
            continue;
        };
        let scripts = super::walk_command_words_with_config(&inner, config);
        if scripts.len() != 1 {
            continue;
        }
        let Some(arg) = super::list_source_file_arg(&inner, &scripts[0]) else {
            continue;
        };
        let Some(filename) = super::source_filename(&inner, arg) else {
            continue;
        };
        let file = index_dir.join(filename.trim().trim_matches('"'));
        if exists(&file) {
            entries.push(OriginalAutoIndexRegistration { input, file });
        }
    }
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_package_advice_keeps_unowned_reports_and_metadata_distinct() {
        let unknown = PackageRequirementAdviceKey::from_original_name(None);
        assert_eq!(unknown, PackageRequirementAdviceKey::Unknown);
        assert!(!unknown.matches_ascii_metadata("Tk"));
        let metadata = PackageRequirementAdviceKey::authored_metadata("Tk");
        assert!(metadata.matches_ascii_metadata("Tk"));
        assert!(metadata.original().is_none());
        assert_eq!(
            PackageRequirementAdviceKey::authored_metadata("p\u{d7ff}"),
            PackageRequirementAdviceKey::Unknown
        );
        assert_eq!(
            PackageRequirementAdviceKey::authored_metadata("p\0tail"),
            PackageRequirementAdviceKey::Unknown
        );
        let resolver = PackageResolver::new();
        let available = resolver.transitive_package_advice(
            &[unknown, metadata.clone()],
            Some(tcl_dialect::TclVersion::V9_1),
            &|_| panic!("missing providers cannot supply dependency source"),
        );
        assert_eq!(
            available,
            HashSet::from([metadata, PackageRequirementAdviceKey::Unknown])
        );
    }

    #[test]
    fn original_package_requirement_closure_keeps_versions_and_preference() {
        // The shared candidate owner also serves exact_require_selects_that_release_or_nothing
        // and prefer_latest_selects_the_prerelease in package_resolver::tests;
        // those tests retain their concrete C Tcl version-selection controls.
        let index = "package ifneeded w 1.2 {source stable.tcl}\npackage ifneeded w 1.3b1 {source beta.tcl}\npackage ifneeded w 2.0 {source major.tcl}\npackage ifneeded dep 1.0 {source dep-old.tcl}\npackage ifneeded dep 2.0 {source dep-new.tcl}";
        for version in [
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
            tcl_dialect::TclVersion::V9_1,
        ] {
            let mut resolver = PackageResolver::new();
            resolver.add_original_pkg_index(
                index,
                Path::new("/library"),
                Path::new("/library/pkgIndex.tcl"),
                &|_| true,
                &|_| Vec::new(),
            );
            let entries = original_package_entries(
                index,
                Path::new("/library"),
                Path::new("/library/pkgIndex.tcl"),
                &|_| true,
                &|_| Vec::new(),
                version,
            );
            let key = |name: &str| {
                PackageRequirementAdviceKey::Original(
                    entries
                        .iter()
                        .find(|entry| entry.name().matches_ascii(name))
                        .expect("authentic original registration")
                        .name()
                        .clone(),
                )
            };
            let require = |name: &str, alternatives: &[&str], exact, prefer| {
                PackageRequirementAdvice::new(
                    key(name),
                    alternatives
                        .iter()
                        .map(|version| (*version).to_owned())
                        .collect(),
                    exact,
                    prefer,
                )
            };
            let exact = require("w", &["1.2"], true, super::super::PackagePrefer::Latest);
            let stable = require("w", &["1.0"], false, super::super::PackagePrefer::Stable);
            let latest = require("w", &["1.0"], false, super::super::PackagePrefer::Latest);
            let alternatives = require(
                "w",
                &["0.9", "2.0"],
                false,
                super::super::PackagePrefer::Stable,
            );
            assert_eq!(
                resolver.resolve_requirement_advice(&exact, Some(version)),
                vec![PathBuf::from("/library/stable.tcl")]
            );
            assert_eq!(
                resolver.resolve_requirement_advice(&stable, Some(version)),
                vec![PathBuf::from("/library/stable.tcl")]
            );
            assert_eq!(
                resolver.resolve_requirement_advice(&latest, Some(version)),
                vec![PathBuf::from("/library/beta.tcl")]
            );
            assert_eq!(
                resolver.resolve_requirement_advice(&alternatives, Some(version)),
                vec![PathBuf::from("/library/major.tcl")]
            );
            let dependency = require("dep", &["1.0"], true, super::super::PackagePrefer::Stable);
            let available =
                resolver.transitive_requirement_advice(&[latest.clone()], Some(version), &|file| {
                    if file == Path::new("/library/beta.tcl") {
                        vec![dependency.clone()]
                    } else if file == Path::new("/library/dep-old.tcl") {
                        vec![exact.clone()]
                    } else if file == Path::new("/library/stable.tcl") {
                        vec![latest.clone()]
                    } else {
                        panic!("constraints must not visit {file:?}")
                    }
                });
            assert_eq!(available, HashSet::from([latest, dependency, exact]));
            let foreign_key = tcl_registry::native_package::NativePackageNameKey::from_native_units(
                b"dep",
                NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V8_4),
            );
            let foreign = PackageRequirementAdvice::new(
                PackageRequirementAdviceKey::Original(foreign_key),
                vec!["1.0".to_owned()],
                true,
                super::super::PackagePrefer::Latest,
            );
            let unknown = foreign.clone().unknown();
            let available =
                resolver.transitive_requirement_advice(&[stable.clone()], Some(version), &|file| {
                    assert_eq!(file, Path::new("/library/stable.tcl"));
                    vec![foreign.clone()]
                });
            assert_eq!(available, HashSet::from([stable, unknown]));

            let root = require("w", &["1.0"], false, super::super::PackagePrefer::Stable);
            let raised = require("dep", &["1.0"], true, super::super::PackagePrefer::Latest);
            let after_raise = require("w", &["1.0"], false, super::super::PackagePrefer::Latest);
            let available = resolver.transitive_requirement_advice_with_context(
                &[root.clone()],
                Some(version),
                &|file, parent| {
                    if file == Path::new("/library/stable.tcl") {
                        assert_eq!(parent.prefer(), super::super::PackagePrefer::Stable);
                        vec![raised.clone()]
                    } else if file == Path::new("/library/dep-old.tcl") {
                        assert_eq!(parent.prefer(), super::super::PackagePrefer::Latest);
                        vec![after_raise.clone()]
                    } else {
                        assert_eq!(file, Path::new("/library/beta.tcl"));
                        assert_eq!(parent.prefer(), super::super::PackagePrefer::Latest);
                        Vec::new()
                    }
                },
            );
            assert_eq!(available, HashSet::from([root, raised, after_raise]));
        }
    }

    #[test]
    fn original_package_keys_keep_opaque_names_and_nested_source_owners() {
        let source = r"if {1} {package ifneeded p\uD800 1.0 {source a.tcl}}\npackage ifneeded p\uD801 2.0 {source b.tcl}".replace("\\n", "\n");
        for version in tcl_dialect::TclVersion::ALL {
            let entries = original_package_entries(
                &source,
                Path::new("/library"),
                Path::new("/library/pkgIndex.tcl"),
                &|_| true,
                &|_| Vec::new(),
                version,
            );
            assert_eq!(entries.len(), 2, "{version:?}");
            assert_eq!(entries[0].name.bytes(), b"p\xed\xa0\x80");
            assert_eq!(entries[1].name.bytes(), b"p\xed\xa0\x81");
            assert!(entries[0].name_input().display().is_none());
            assert_eq!(
                entries[0].name_input().source_image(),
                &SourceImage::document(&source)
            );
            assert!(entries[0].name_input().span().start() > 10);
            let mut resolver = PackageResolver::new();
            resolver.add_original_pkg_index(
                &source,
                Path::new("/library"),
                Path::new("/library/pkgIndex.tcl"),
                &|_| true,
                &|_| Vec::new(),
            );
            assert_eq!(
                resolver.resolve_original_require(
                    entries[0].name(),
                    &[],
                    false,
                    super::super::PackagePrefer::Stable
                ),
                vec![PathBuf::from("/library/a.tcl")]
            );
            assert_eq!(
                resolver.resolve_original_require(
                    entries[1].name(),
                    &[],
                    false,
                    super::super::PackagePrefer::Stable
                ),
                vec![PathBuf::from("/library/b.tcl")]
            );
        }
    }

    #[test]
    fn original_index_grammar_requires_the_current_registry_implementation() {
        let version = tcl_dialect::TclVersion::V9_1;
        let config = LexerConfig::from_grammar(
            tcl_registry::InvocationDialect::for_version(version).lexer_grammar,
        );
        let policy = NamePolicyProtocol::authored_tcl(version);
        let source = "proc set args {return ignored}\nset auto_index(p) [list source [file join $dir p.tcl]]";
        assert!(
            original_index_entries(source, Path::new("/library"), &|_| true, config, policy)
                .is_empty()
        );
        let source = "proc package args {return ignored}\npackage ifneeded p 1.0 {source p.tcl}";
        assert!(
            original_package_entries(
                source,
                Path::new("/library"),
                Path::new("/library/pkgIndex.tcl"),
                &|_| true,
                &|_| Vec::new(),
                version
            )
            .is_empty()
        );
    }

    #[test]
    fn original_old_index_uses_list_grammar_and_translated_line_boundaries() {
        for version in tcl_dialect::TclVersion::ALL {
            let config = LexerConfig::from_grammar(
                tcl_registry::InvocationDialect::for_version(version).lexer_grammar,
            );
            for newline in ["\n", "\r\n", "\r"] {
                let source = format!(
                    "# Tcl autoload index file: each line identifies a Tcl{newline}{{$literal}} {{file name.tcl}}{newline}"
                );
                let entries = original_index_entries(
                    &source,
                    Path::new("/library"),
                    &|_| true,
                    config,
                    NamePolicyProtocol::authored_tcl(version),
                );
                assert_eq!(entries.len(), 1, "{version:?}: {newline:?}");
                assert_eq!(entries[0].input.bytes(), b"$literal");
                assert_eq!(entries[0].file, Path::new("/library/file name.tcl"));
            }
        }
    }

    #[test]
    fn original_index_keeps_counted_literal_names_and_does_not_trim_indices() {
        let config = LexerConfig::from_grammar(
            tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_1)
                .lexer_grammar,
        );
        let source = "set {auto_index(p\0tail)} [list source [file join $dir p.tcl]]\nset {auto_index( spaced )} [list source [file join $dir q.tcl]]";
        let entries = original_index_entries(
            &source,
            Path::new("/library"),
            &|_| true,
            config,
            NamePolicyProtocol::authored_tcl(tcl_dialect::TclVersion::V9_1),
        );
        assert_eq!(
            entries
                .iter()
                .map(|entry| (entry.input.bytes(), entry.file.as_path()))
                .collect::<Vec<_>>(),
            vec![
                (b"p\xc0\x80tail".as_slice(), Path::new("/library/p.tcl")),
                (b" spaced ".as_slice(), Path::new("/library/q.tcl"))
            ]
        );
    }
}

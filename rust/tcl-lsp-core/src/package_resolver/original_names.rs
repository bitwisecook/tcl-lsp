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

//! Package and autoload command queries using retained original byte producers.

mod advertisement_eligibility;
mod file_candidates;
pub(super) mod index_loader;
pub use file_candidates::{OriginalPackageFileCandidate, OriginalPackageFileCandidateKind};

use super::PackageResolver;
use std::{
    collections::{BTreeMap, HashSet},
    path::{Path, PathBuf},
    sync::Arc,
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
        self.add_original_pkg_index_for_releases(
            content,
            directory,
            index,
            exists,
            list_files,
            &tcl_dialect::TclVersion::ALL,
        );
    }

    pub(super) fn add_original_pkg_index_for_releases(
        &mut self,
        content: &str,
        directory: &Path,
        index: &Path,
        exists: &dyn Fn(&Path) -> bool,
        list_files: &dyn Fn(&Path) -> Vec<PathBuf>,
        releases: &[tcl_dialect::TclVersion],
    ) {
        self.invalidate();
        // The same text read with core `package`/`if` semantics, as
        // `tclPkgUnknown` sources it, decides version guards the original
        // survey keeps undecidable. An entry it excludes for a release is not
        // offered for that release.
        let decided = super::parse_pkg_index(content, directory, index, exists, list_files);
        for &version in releases {
            for info in
                original_package_entries(content, directory, index, exists, list_files, version)
            {
                if guard_excludes(&decided, &info.info, version) {
                    continue;
                }
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
        if let tcl_syntax::naming::NativeNameProtocol::C(release) = name.policy().recipe()
            && let Some(module) =
                self.modules
                    .provider(name.bytes(), requirements, exact, prefer, release)
        {
            return module.source_files.clone();
        }
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
        let module = match name.policy().recipe() {
            tcl_syntax::naming::NativeNameProtocol::C(release) => {
                self.modules.provides(name.bytes(), release)
            }
            tcl_syntax::naming::NativeNameProtocol::Jim084 => false,
        };
        module
            || self
                .original_packages
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
                let original = self.resolve_original_require(key, requirements, exact, prefer);
                if !original.is_empty() || self.original_packages.contains_key(key) {
                    return original;
                }
                // A registration the original survey could not see at all
                // (nested in `apply`, a loop or both arms of a branch) falls
                // back to the textual index under the key's own release.
                let tcl_syntax::naming::NativeNameProtocol::C(version) = key.policy().recipe()
                else {
                    return Vec::new();
                };
                let Ok(name) = std::str::from_utf8(key.bytes()) else {
                    return Vec::new();
                };
                let mut files = Vec::new();
                for provider in self.candidate_providers_for_require(
                    name,
                    requirements,
                    exact,
                    prefer,
                    Some(version),
                ) {
                    for file in &provider.source_files {
                        if !files.contains(file) {
                            files.push(file.clone());
                        }
                    }
                }
                files
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
    /// authored compatibility `AutoIndexEntry` strings supply no original keys.
    pub fn add_original_tcl_index(
        &mut self,
        content: &str,
        index_dir: &Path,
        exists: &dyn Fn(&Path) -> bool,
    ) {
        self.add_original_tcl_index_for_releases(
            content,
            index_dir,
            exists,
            &tcl_dialect::TclVersion::ALL,
        );
    }

    pub(super) fn add_original_tcl_index_for_releases(
        &mut self,
        content: &str,
        index_dir: &Path,
        exists: &dyn Fn(&Path) -> bool,
        releases: &[tcl_dialect::TclVersion],
    ) {
        self.invalidate();
        let requests: Vec<_> = releases
            .iter()
            .map(|&version| {
                let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name())
                    .expect("C release profile");
                let config = LexerConfig::from_grammar(
                    tcl_native_model::InvocationDialect::of_profile(profile).lexer_grammar,
                );
                let policy = NamePolicyProtocol::authored_tcl(version);
                (config, policy)
            })
            .collect();
        for ((_, policy), entries) in requests.iter().copied().zip(original_index_entries_batch(
            content, index_dir, exists, &requests,
        )) {
            for entry in entries {
                self.original_auto_index
                    .entry((policy.recipe(), NameBytes::from(entry.input.bytes())))
                    .or_default()
                    .push(entry);
            }
        }
    }

    /// Original `auto_load` candidates at the command's retained caller point.
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
                .get(&(key.policy().recipe(), candidate))
            {
                return files.iter().map(|entry| entry.file.clone()).collect();
            }
        }
        Vec::new()
    }

    /// Genuine index files for separately typed conditional deferred-body SOURCE
    /// advice. These are source header candidates, never a resolved binding or
    /// proof that `auto_load` executes at the future body invocation.
    #[must_use]
    pub fn resolve_conditional_native_autoload_source(
        &self,
        advice: &tcl_compiler::registry_invocation::source_structure::OriginalNativeDeferredAutoloadAdvice,
    ) -> Vec<PathBuf> {
        for candidate in advice.candidates() {
            if let Some(entries) = self
                .original_auto_index
                .get(&(advice.name().policy().recipe(), candidate.clone()))
            {
                return entries.iter().map(|entry| entry.file.clone()).collect();
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

/// Survey independent release inputs in pairs while preserving release and
/// directory order. The caller owns the rescan memo and all existence checks;
/// workers receive only immutable source and return their exact release rows.
fn original_index_entries_batch(
    content: &str,
    index_dir: &Path,
    exists: &dyn Fn(&Path) -> bool,
    requests: &[(LexerConfig, NamePolicyProtocol)],
) -> Vec<Vec<OriginalAutoIndexRegistration>> {
    let mut results = Vec::with_capacity(requests.len());
    for pair in requests.chunks(2) {
        let mut rows: Vec<Option<Vec<OriginalAutoIndexRegistration>>> = pair
            .iter()
            .map(|&(config, policy)| cached_index_survey(content, index_dir, config, policy))
            .collect();
        #[cfg(not(target_family = "wasm"))]
        if pair.len() == 2
            && rows.iter().all(Option::is_none)
            && std::thread::available_parallelism().is_ok_and(|count| count.get() > 1)
        {
            let (left, right) = std::thread::scope(|scope| {
                match std::thread::Builder::new().spawn_scoped(scope, || {
                    survey_index_entries(content, index_dir, pair[0].0, pair[0].1)
                }) {
                    Ok(left) => {
                        let right = survey_index_entries(content, index_dir, pair[1].0, pair[1].1);
                        (
                            left.join()
                                .unwrap_or_else(|error| std::panic::resume_unwind(error)),
                            right,
                        )
                    }
                    Err(_) => (
                        survey_index_entries(content, index_dir, pair[0].0, pair[0].1),
                        survey_index_entries(content, index_dir, pair[1].0, pair[1].1),
                    ),
                }
            });
            rows[0] = Some(left);
            rows[1] = Some(right);
        }
        for (&(config, policy), row) in pair.iter().zip(rows.drain(..)) {
            let entries =
                row.unwrap_or_else(|| survey_index_entries(content, index_dir, config, policy));
            if cached_index_survey(content, index_dir, config, policy).is_none() {
                remember_index_survey(content, index_dir, config, policy, &entries);
            }
            results.push(
                entries
                    .into_iter()
                    .filter(|entry| exists(&entry.file))
                    .collect(),
            );
        }
    }
    results
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

/// Which loader evaluates an index, and how much of it the analysis keeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum IndexAnalysis {
    /// A `pkgIndex.tcl`, whole-document analysis.
    Package,
    /// A `tclIndex` inside `auto_load_index`; `structure_only` keeps only
    /// the source-owned registration structure.
    Autoload { structure_only: bool },
}

/// The frame `auto_load_index` evaluates a `tclIndex` in (`eval [read $f]`
/// inside the procedure): `global auto_index auto_path`, and the local `dir`
/// naming the index directory. tclsh 8.4.20 to 9.1.0: a bare `set` in the
/// index stays local and `$dir` is the directory (`init.tcl`).
fn tcl_index_entry_frame(index_dir: &Path) -> Arc<tcl_compiler::analyser::EntryProcedureFrame> {
    Arc::new(
        tcl_compiler::analyser::EntryProcedureFrame::new()
            .with_global("auto_index")
            .with_global("auto_path")
            .with_local("dir", index_dir.to_str().map(str::to_owned)),
    )
}

fn analyse_package_index(
    content: &str,
    profile: &'static tcl_dialect::DialectProfile,
    registry: &tcl_registry::CommandRegistry,
    config: LexerConfig,
    path: &Path,
    native_commands: Option<usize>,
) -> tcl_compiler::analyser::AnalysisResult {
    analyse_index_with(
        content,
        profile,
        registry,
        config,
        path,
        native_commands,
        IndexAnalysis::Package,
    )
}

fn analyse_index_with(
    content: &str,
    profile: &'static tcl_dialect::DialectProfile,
    registry: &tcl_registry::CommandRegistry,
    config: LexerConfig,
    path: &Path,
    native_commands: Option<usize>,
    analyse: IndexAnalysis,
) -> tcl_compiler::analyser::AnalysisResult {
    let trace = std::env::var_os("TCL_LSP_TRACE_WORKSPACE_SCAN").is_some();
    let started = trace.then(std::time::Instant::now);
    if trace {
        eprintln!(
            "[workspace_scan] package.analysis.begin path={} bytes={} profile={} native_commands={native_commands:?}",
            path.display(),
            content.len(),
            profile.name
        );
    }
    let analysis = crate::source_structure::analyse_loader_index(
        content,
        profile,
        registry,
        config,
        match analyse {
            IndexAnalysis::Package => None,
            IndexAnalysis::Autoload { .. } => Some(tcl_index_entry_frame(path)),
        },
        analyse
            == IndexAnalysis::Autoload {
                structure_only: true,
            },
    );
    if let Some(started) = started {
        eprintln!(
            "[workspace_scan] package.analysis.end path={} elapsed_ms={} commands={} procedures={} classes={}",
            path.display(),
            started.elapsed().as_millis(),
            analysis.command_invocations.len(),
            analysis.all_procs.len(),
            analysis.all_classes.len()
        );
    }
    analysis
}

fn guard_excludes(
    decided: &[super::PackageInfo],
    entry: &super::PackageInfo,
    version: tcl_dialect::TclVersion,
) -> bool {
    let mut same = decided
        .iter()
        .filter(|info| info.name == entry.name && info.version == entry.version)
        .peekable();
    same.peek().is_some()
        && same.all(|info| {
            info.availability(Some(version)) == super::reachability::Availability::Unavailable
        })
}

fn original_package_entries(
    content: &str,
    directory: &Path,
    index: &Path,
    exists: &dyn Fn(&Path) -> bool,
    list_files: &dyn Fn(&Path) -> Vec<PathBuf>,
    version: tcl_dialect::TclVersion,
) -> Vec<OriginalPackageInfo> {
    if !advertisement_eligibility::may_advertise(content, version) {
        return Vec::new();
    }
    original_package_entries_full(content, directory, index, exists, list_files, version)
}

fn original_package_entries_full(
    content: &str,
    directory: &Path,
    index: &Path,
    exists: &dyn Fn(&Path) -> bool,
    list_files: &dyn Fn(&Path) -> Vec<PathBuf>,
    version: tcl_dialect::TclVersion,
) -> Vec<OriginalPackageInfo> {
    use tcl_registry::model::binding::PackageTransition;
    use tcl_registry::{InvocationWord, InvocationWords, StateTransition};
    let dialect = tcl_native_model::InvocationDialect::for_version(version);
    let config = LexerConfig::for_file_grammar(dialect.lexer_grammar);
    let policy = dialect
        .authored_name_policy()
        .expect("C loader naming recipe");
    let registry =
        tcl_registry::model::ingress::static_context_for(version.dialect_profile_name()).commands();
    let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name())
        .expect("C loader release");
    let analysis = analyse_package_index(content, profile, registry, config, index, None);
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
            let keys = package_entry_word_keys(&words, config, policy);
            let arguments = package_entry_invocation_arguments(&keys);
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
                let Some(input) = package_entry_name_input(&keys, package) else {
                    continue;
                };
                let Some(advertised) =
                    package_entry_advertised_version(&keys, version_subject, version)
                else {
                    continue;
                };
                let Some(loader) = package_entry_loader_word(&words, script) else {
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

fn package_entry_name_input(
    keys: &[Option<SignatureSourceNameKey>],
    package: &tcl_registry::state_transition::TransitionSubject,
) -> Option<SignatureSourceNameKey> {
    package
        .argument_index()
        .and_then(|ordinal| keys.get(ordinal + 1))
        .and_then(Option::as_ref)
        .cloned()
}

fn package_entry_word_keys(
    words: &[tcl_lexer::NativeWord],
    config: LexerConfig,
    policy: NamePolicyProtocol,
) -> Vec<Option<SignatureSourceNameKey>> {
    words
        .iter()
        .map(|word| {
            SignatureSourceNameKey::from_original_native_word(
                word,
                WordValueRules::from_config(&config),
                policy,
            )
        })
        .collect()
}

fn package_entry_invocation_arguments(
    keys: &[Option<SignatureSourceNameKey>],
) -> Vec<tcl_registry::InvocationWord<'_>> {
    use tcl_registry::InvocationWord;
    keys.iter()
        .skip(1)
        .map(|key| {
            key.as_ref()
                .and_then(SignatureSourceNameKey::display)
                .map_or(InvocationWord::Dynamic, InvocationWord::Literal)
        })
        .collect()
}

fn package_entry_advertised_version<'a>(
    keys: &'a [Option<SignatureSourceNameKey>],
    version_subject: &tcl_registry::state_transition::TransitionSubject,
    version: tcl_dialect::TclVersion,
) -> Option<&'a str> {
    let advertised = version_subject
        .argument_index()
        .and_then(|ordinal| keys.get(ordinal + 1))
        .and_then(Option::as_ref)
        .and_then(SignatureSourceNameKey::display)?;
    if !tcl_dialect::validate_version_for(advertised, version) {
        return None;
    }
    Some(advertised)
}

fn package_entry_loader_word<'a>(
    words: &'a [tcl_lexer::NativeWord],
    script: &tcl_registry::state_transition::TransitionSubject,
) -> Option<&'a tcl_lexer::NativeWord> {
    let body_index = script.argument_index()?;
    words.get(body_index + 1)
}

#[derive(Debug, Clone)]
pub(super) struct OriginalAutoIndexRegistration {
    input: SignatureSourceNameValue,
    file: PathBuf,
}

/// The `tclIndex` registrations whose target file exists.
///
/// The source survey owns a fresh complete source realm and exact per-release
/// Registry input. Authenticated missing schemas or invalid authored filename
/// layouts omit only that registration. Other missing proofs retain the full
/// structural analyser; no partial analysis or runtime result is produced.
/// It is a pure function of index bytes, directory, grammar and naming policy, so
/// it is memoised across rescans (user decision 2026-10-19T11:00Z
/// `product_fixes`: "host Tcl install rescan cache"): a folder add or
/// workspace rescan re-surveys the same installed indexes. Only the extracted
/// registrations are retained, never the analysis; existence is checked on
/// every call. The memo is per thread (scans run on blocking workers), bounded
/// by entry count and retained index bytes.
#[cfg(test)]
fn original_index_entries(
    content: &str,
    index_dir: &Path,
    exists: &dyn Fn(&Path) -> bool,
    config: LexerConfig,
    policy: NamePolicyProtocol,
) -> Vec<OriginalAutoIndexRegistration> {
    let surveyed = cached_index_survey(content, index_dir, config, policy).unwrap_or_else(|| {
        let surveyed = survey_index_entries(content, index_dir, config, policy);
        remember_index_survey(content, index_dir, config, policy, &surveyed);
        surveyed
    });
    surveyed
        .into_iter()
        .filter(|entry| exists(&entry.file))
        .collect()
}

type IndexSurveyCacheRow = (
    Box<str>,
    PathBuf,
    LexerConfig,
    NamePolicyProtocol,
    Vec<OriginalAutoIndexRegistration>,
);

fn with_index_survey_cache<R>(
    f: impl FnOnce(&mut std::collections::VecDeque<IndexSurveyCacheRow>) -> R,
) -> R {
    thread_local! {
        static MEMO: std::cell::RefCell<std::collections::VecDeque<IndexSurveyCacheRow>> =
            const { std::cell::RefCell::new(std::collections::VecDeque::new()) };
    }
    MEMO.with_borrow_mut(f)
}

fn cached_index_survey(
    content: &str,
    dir: &Path,
    config: LexerConfig,
    policy: NamePolicyProtocol,
) -> Option<Vec<OriginalAutoIndexRegistration>> {
    with_index_survey_cache(|memo| {
        memo.iter()
            .find(|(bytes, cached_dir, grammar, naming, _)| {
                bytes.as_ref() == content
                    && cached_dir.as_path() == dir
                    && *grammar == config
                    && *naming == policy
            })
            .map(|row| row.4.clone())
    })
}

fn remember_index_survey(
    content: &str,
    dir: &Path,
    config: LexerConfig,
    policy: NamePolicyProtocol,
    entries: &[OriginalAutoIndexRegistration],
) {
    const MAX_ENTRIES: usize = 64;
    const MAX_INDEX_BYTES: usize = 4 * 1024 * 1024;
    with_index_survey_cache(|memo| {
        memo.push_back((
            content.into(),
            dir.to_path_buf(),
            config,
            policy,
            entries.to_vec(),
        ));
        while memo.len() > MAX_ENTRIES
            || memo.iter().map(|row| row.0.len()).sum::<usize>() > MAX_INDEX_BYTES
        {
            memo.pop_front();
        }
    });
}

enum IndexSurveyContext {
    Analysed(Box<tcl_compiler::analyser::AnalysisResult>),
    Source(Box<tcl_compiler::registry_invocation::source_structure::OriginalSourceRegistryContext>),
}

impl IndexSurveyContext {
    fn registry_context(&self) -> Option<std::sync::Arc<tcl_registry::model::ContextRegistry>> {
        match self {
            Self::Analysed(analysis) => Some(analysis.resolved_input.as_ref()?.context_registry()),
            Self::Source(source) => Some(source.editing_input().context_registry()),
        }
    }

    fn loader<'a>(&'a self, content: &'a str) -> Option<index_loader::IndexLoader<'a>> {
        match self {
            Self::Analysed(analysis) => index_loader::IndexLoader::new(content, analysis),
            Self::Source(source) => index_loader::IndexLoader::from_source_context(content, source),
        }
    }

    fn words(
        &self,
        content: &str,
        command: &tcl_source::segment::SegmentedCommand,
    ) -> Option<tcl_compiler::registry_invocation::source_structure::OriginalRegistryWords> {
        match self {
            Self::Analysed(analysis) => {
                tcl_compiler::registry_invocation::source_structure::source_registry_words(
                    content, analysis, command,
                )
            }
            Self::Source(source) => source.registration_words(command),
        }
    }
}

fn survey_index_entries(
    content: &str,
    index_dir: &Path,
    config: LexerConfig,
    policy: NamePolicyProtocol,
) -> Vec<OriginalAutoIndexRegistration> {
    survey_index_entries_from_source(content, index_dir, config, policy).unwrap_or_else(|| {
        survey_index_entries_with(
            content,
            index_dir,
            config,
            policy,
            IndexAnalysis::Autoload {
                structure_only: true,
            },
        )
    })
}

fn survey_index_entries_from_source(
    content: &str,
    index_dir: &Path,
    config: LexerConfig,
    policy: NamePolicyProtocol,
) -> Option<Vec<OriginalAutoIndexRegistration>> {
    survey_index_entries_using(
        content,
        index_dir,
        config,
        policy,
        |profile, registry, _| {
            let (commands, expressions) = tcl_compiler::analyser::utils::document_stub_declarations(
                content,
                None,
                profile.name,
            );
            if !commands.is_empty()
                || !expressions.is_empty()
                || !tcl_compiler::analyser::utils::parse_supports_directives(content).is_empty()
                || !tcl_compiler::analyser::utils::parse_provides_directives(content).is_empty()
            {
                return None;
            }
            let generation = crate::context_for_dialect_profile(profile);
            let loader_context = std::sync::Arc::new(
                generation.with_command_store(registry.snapshot().shared_registry()),
            );
            let input = tcl_compiler::analyser::ResolvedAnalysisInput::new(
                profile,
                profile,
                loader_context,
                config,
            )
            .with_entry_procedure_frame(tcl_index_entry_frame(index_dir));
            let source = tcl_compiler::registry_invocation::source_structure::OriginalSourceRegistryContext::capture_standalone_structure(content, input);
            Some(IndexSurveyContext::Source(Box::new(source)))
        },
    )
}

fn survey_index_entries_with(
    content: &str,
    index_dir: &Path,
    config: LexerConfig,
    policy: NamePolicyProtocol,
    analyse: IndexAnalysis,
) -> Vec<OriginalAutoIndexRegistration> {
    survey_index_entries_using(
        content,
        index_dir,
        config,
        policy,
        |profile, registry, count| {
            Some(IndexSurveyContext::Analysed(Box::new(analyse_index_with(
                content,
                profile,
                registry,
                config,
                index_dir,
                Some(count),
                analyse,
            ))))
        },
    )
    .unwrap_or_default()
}

fn survey_index_entries_using(
    content: &str,
    index_dir: &Path,
    config: LexerConfig,
    policy: NamePolicyProtocol,
    capture: impl FnOnce(
        &'static tcl_dialect::DialectProfile,
        &tcl_registry::CommandRegistry,
        usize,
    ) -> Option<IndexSurveyContext>,
) -> Option<Vec<OriginalAutoIndexRegistration>> {
    let image = SourceImage::document(content);
    let Some(lines) = SignatureSourceNameValue::original_source_lines(&image, config, policy)
    else {
        return Some(Vec::new());
    };
    let mut entries = Vec::new();
    if lines.first().map(SignatureSourceNameValue::bytes)
        == Some(b"# Tcl autoload index file: each line identifies a Tcl".as_slice())
    {
        append_legacy_index_registrations(lines, index_dir, &mut entries);
        return Some(entries);
    }
    let Some(length) = u32::try_from(content.len()).ok() else {
        return Some(Vec::new());
    };
    let Ok(plan) = tcl_lexer::native_script_words_in(image, Span::new(0, length), config) else {
        return Some(Vec::new());
    };
    let tcl_syntax::naming::NativeNameProtocol::C(version) = policy.recipe() else {
        return Some(entries);
    };
    let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name())
        .expect("C loader release");
    let registry = tcl_registry::model::ingress::static_context_for_profile(profile).commands();
    let captured = capture(profile, registry, plan.commands.len())?;
    let fresh = matches!(captured, IndexSurveyContext::Source(_));
    if fresh && plan.fatal_tail.is_some() {
        return None;
    }
    let Some(registry_context) = captured.registry_context() else {
        return Some(entries);
    };
    let Some(filenames) = captured.loader(content) else {
        return Some(entries);
    };
    let commands = tcl_source::segment::segment_commands_with_offset_and_config(content, 0, config);
    let commands_by_offset = index_commands_by_offset(&commands);
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
        let Some(command) = commands_by_offset.get(&head.span().start()).copied() else {
            continue;
        };
        let Some(schema) = captured.words(content, command) else {
            if fresh && filenames.registration_abstention(words).is_none() {
                return None;
            }
            continue;
        };
        let selected = schema.with_source_schema(&registry_context, |schema| {
            schema.semantics.lowering_hook == Some(tcl_registry::hooks::LoweringHookId::Set)
        });
        if fresh && selected.is_none() {
            return None;
        }
        if selected != Some(true) {
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
        let std::ops::ControlFlow::Continue(file) =
            select_index_filename(&filenames, &words[2], index_dir, fresh)?
        else {
            continue;
        };
        entries.push(OriginalAutoIndexRegistration { input, file });
    }
    Some(entries)
}

fn index_commands_by_offset(
    commands: &[tcl_source::segment::SegmentedCommand],
) -> BTreeMap<u32, &tcl_source::segment::SegmentedCommand> {
    let mut index = BTreeMap::new();
    for command in commands {
        if let Some(head) = command.argv.first() {
            // Preserve the original scan's first match, including duplicates.
            index.entry(head.span.start()).or_insert(command);
        }
    }
    index
}

fn select_index_filename(
    filenames: &index_loader::IndexLoader<'_>,
    loader: &tcl_lexer::NativeWord,
    index_dir: &Path,
    fresh: bool,
) -> Option<std::ops::ControlFlow<(), PathBuf>> {
    if fresh {
        match filenames.survey_filename_with_layout_proof(loader, index_dir) {
            index_loader::OriginalIndexFilenameSurvey::Filename(file) => {
                Some(std::ops::ControlFlow::Continue(file))
            }
            index_loader::OriginalIndexFilenameSurvey::NoContribution(certificate)
                if certificate.matches_current_query(loader, index_dir) =>
            {
                Some(std::ops::ControlFlow::Break(()))
            }
            index_loader::OriginalIndexFilenameSurvey::InvalidLayout(certificate)
                if certificate.matches_current_query(loader, index_dir) =>
            {
                Some(std::ops::ControlFlow::Break(()))
            }
            index_loader::OriginalIndexFilenameSurvey::NoContribution(_)
            | index_loader::OriginalIndexFilenameSurvey::InvalidLayout(_)
            | index_loader::OriginalIndexFilenameSurvey::Unavailable => None,
        }
    } else {
        Some(filenames.filename(loader, index_dir).map_or(
            std::ops::ControlFlow::Break(()),
            std::ops::ControlFlow::Continue,
        ))
    }
}

fn append_legacy_index_registrations(
    lines: Vec<SignatureSourceNameValue>,
    index_dir: &Path,
    entries: &mut Vec<OriginalAutoIndexRegistration>,
) {
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
        entries.push(OriginalAutoIndexRegistration { input, file });
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_index_body_purpose_tests;

#[cfg(test)]
mod original_filename_parser_purpose_tests;

#[cfg(test)]
mod original_index_filename_data_tests;

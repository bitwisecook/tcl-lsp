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

//! The identity of one source instance: its interned kind, constructors and activation names.

use super::command_identity::CommandAllocationSite;
use super::source_point_entry::OriginalSourcePointOrigin;
use super::{
    AllocationMemoTable, DeferredSourceBody, SourceCommandTarget, original_source_projection,
};
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, Mutex, OnceLock, Weak};

/// Source instance in which an executable command was allocated.
#[derive(Debug, Clone)]
pub struct SourceOriginId {
    pub(super) kind: Arc<SourceOriginKind>,
    pub(super) fingerprint: u64,
    activation_key: u64,
}

#[derive(Default)]
pub(super) struct SourceOriginInterner {
    pub(super) buckets: HashMap<u64, Vec<(Weak<SourceOriginKind>, u64)>>,
    pub(super) next_key: u64,
}

impl SourceOriginInterner {
    pub(super) fn intern(&mut self, source: &Arc<SourceOriginKind>, fingerprint: u64) -> u64 {
        if self.next_key.is_multiple_of(64) {
            self.buckets.retain(|_, sources| {
                sources.retain(|(source, _)| source.strong_count() != 0);
                !sources.is_empty()
            });
        }
        let bucket = self.buckets.entry(fingerprint).or_default();
        // Equal independent sources keep separate allocations (each owns its
        // memos) under one key; dead ones are dropped so a bucket holds only
        // live allocations.
        bucket.retain(|(retained, _)| retained.strong_count() != 0);
        let existing = bucket.iter().find_map(|(retained, key)| {
            retained
                .upgrade()
                .filter(|retained| retained == source)
                .map(|_| *key)
        });
        let key = existing.unwrap_or_else(|| {
            self.next_key = self
                .next_key
                .checked_add(1)
                .expect("source identity exhausted");
            self.next_key
        });
        bucket.push((Arc::downgrade(source), key));
        key
    }
}

/// Semantic source identity retained behind a cached fingerprint.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum SourceOriginKind {
    /// Source-only child view from an authentic parent point. This is neither
    /// actual file loading nor an entered Native/runtime source instance.
    InheritedPoint {
        /// Privately issued identity; the full entry independently owns state.
        parent: OriginalSourcePointOrigin,
        /// Exact child image and source channel supplied for this view.
        source: tcl_lexer::SourceImage,
    },
    /// Authored source bytes supplied by the compilation driver.
    Authored(tcl_lexer::SourceImage),
    /// Driver-attested file bytes, retaining distinct file/provider identity.
    Loaded {
        /// Exact selected path; no filename catalogue proves this source.
        path: Arc<str>,
        /// Stable selected implementation identity supplied by the driver.
        implementation_id: Arc<str>,
        /// Exact decoded script bytes consumed by native source evaluation.
        source: tcl_lexer::SourceImage,
    },
    /// Conditional source syntax decoded from an original configured list field.
    /// This is not native script materialisation, a document root or body entry.
    ConditionalField {
        /// Actual original parent invocation in its own source instance.
        parent: CommandAllocationSite,
        /// Actual effective post-head lambda argument ordinal.
        argument: usize,
        /// Actual list field ordinal within the complete lambda word.
        field: usize,
        /// Decoded source-only script bytes in their distinct source instance.
        source: tcl_lexer::SourceImage,
    },
    /// Script materialised from the frozen operands of another invocation.
    Derived {
        /// Parent invocation, including its source instance.
        parent: CommandAllocationSite,
        /// Effective argument positions contributing to the script.
        arguments: Vec<usize>,
        /// Exact concatenated source, retaining semantic equality beyond hashes.
        source: tcl_lexer::SourceImage,
        /// Parsing protocol of the materialised value.
        kind: MaterialisedSourceKind,
    },
}

impl SourceOriginId {
    /// Retain one authored source instance without hashing its bytes at each
    /// command allocation or immutable interpreter snapshot.
    #[must_use]
    pub fn authored(source: &Arc<str>) -> Self {
        Self::authored_image(tcl_lexer::SourceImage::document(source))
    }

    /// Retain the exact original source and its selected input channel.
    #[must_use]
    pub fn authored_image(source: tcl_lexer::SourceImage) -> Self {
        Self::from_kind(SourceOriginKind::Authored(source))
    }

    /// Retain an actual selected file independently of equal bytes in another file.
    #[must_use]
    pub fn loaded(path: Arc<str>, implementation_id: Arc<str>, source: &Arc<str>) -> Self {
        Self::loaded_image(
            path,
            implementation_id,
            tcl_lexer::SourceImage::native(Arc::<[u8]>::from(source.as_bytes())),
        )
    }

    /// Retain selected file identity and the driver's exact source receipt.
    #[must_use]
    pub fn loaded_image(
        path: Arc<str>,
        implementation_id: Arc<str>,
        source: tcl_lexer::SourceImage,
    ) -> Self {
        Self::from_kind(SourceOriginKind::Loaded {
            path,
            implementation_id,
            source,
        })
    }

    /// Full semantic identity; fingerprints never substitute for this data.
    #[must_use]
    pub fn kind(&self) -> &SourceOriginKind {
        self.kind.as_ref()
    }

    /// Original source bytes and channel, without a display conversion.
    #[must_use]
    pub fn source_image(&self) -> &tcl_lexer::SourceImage {
        match self.kind() {
            SourceOriginKind::InheritedPoint { source, .. }
            | SourceOriginKind::Authored(source)
            | SourceOriginKind::Loaded { source, .. }
            | SourceOriginKind::Derived { source, .. }
            | SourceOriginKind::ConditionalField { source, .. } => source,
        }
    }

    /// Whole-source Unicode view for consumers whose contract requires text.
    ///
    /// # Errors
    /// Returns the original UTF-8 failure; opaque bytes retain their identity.
    pub fn try_text(&self) -> Result<&str, std::str::Utf8Error> {
        self.source_image().try_text()
    }

    pub(super) fn derived(
        parent: CommandAllocationSite,
        arguments: Vec<usize>,
        source: &Arc<str>,
        kind: MaterialisedSourceKind,
    ) -> Self {
        Self::derived_image(
            parent,
            arguments,
            tcl_lexer::SourceImage::native(Arc::<[u8]>::from(source.as_bytes())),
            kind,
        )
    }

    pub(super) fn derived_image(
        parent: CommandAllocationSite,
        arguments: Vec<usize>,
        source: tcl_lexer::SourceImage,
        kind: MaterialisedSourceKind,
    ) -> Self {
        Self::from_kind(SourceOriginKind::Derived {
            parent,
            arguments,
            source,
            kind,
        })
    }

    pub(super) fn from_kind(kind: SourceOriginKind) -> Self {
        static INTERNER: OnceLock<Mutex<SourceOriginInterner>> = OnceLock::new();
        let kind = Arc::new(kind);
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        kind.hash(&mut hasher);
        let fingerprint = std::hash::Hasher::finish(&hasher);
        let activation_key = INTERNER
            .get_or_init(|| Mutex::new(SourceOriginInterner::default()))
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .intern(&kind, fingerprint);
        Self {
            kind,
            fingerprint,
            activation_key,
        }
    }

    /// Reuse only derived lexical geometry of this immutable source instance.
    /// Complete config, offset and original vector are matched independently
    /// of every caller's current lookup, input, metadata and Native admission.
    #[doc(hidden)]
    #[must_use]
    pub fn original_lexical_command(
        &self,
        words: &[tcl_source::words::WordExpr],
        offset: u32,
        config: tcl_lexer::LexerConfig,
    ) -> Option<Arc<crate::registry_invocation::OriginalSourceCommandProjection>> {
        self.original_lexical_projection()
            .capture(self.source_image(), words, offset, config)
    }

    /// Checked original words of `region` in this source's own image. Clean
    /// nested command-body verdicts are shared with every other walk of this
    /// immutable source instance; the plan equals the unshared capture.
    pub(crate) fn original_script_words_in(
        &self,
        region: tcl_lexer::Span,
        config: tcl_lexer::LexerConfig,
        purpose: tcl_lexer::SourceSegmentationPurpose,
    ) -> Result<tcl_lexer::NativeScriptWordsPlan, tcl_lexer::NativeScriptWordsUnavailable> {
        let projection = self.original_lexical_projection();
        tcl_lexer::native_script_words_in_with_purpose_and_memo(
            self.source_image().clone(),
            region,
            config,
            purpose,
            projection.nested_cuts(),
        )
    }

    pub(super) fn original_lexical_projection(
        &self,
    ) -> Arc<original_source_projection::OriginalSourceProjectionMemo> {
        static MEMOS: OnceLock<
            Mutex<
                AllocationMemoTable<
                    SourceOriginKind,
                    original_source_projection::OriginalSourceProjectionMemo,
                >,
            >,
        > = OnceLock::new();
        MEMOS
            .get_or_init(|| Mutex::new(AllocationMemoTable::default()))
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get_or_init(&self.kind)
    }

    pub(super) const fn fingerprint(&self) -> u64 {
        self.fingerprint
    }

    pub(super) fn activation_key(source: &Arc<Self>) -> u64 {
        source.activation_key
    }
}

pub(super) fn source_activation_name(
    origin: Option<&Arc<SourceOriginId>>,
    label: &str,
    offset: u32,
) -> String {
    origin.map_or_else(
        || format!("{label}/site:{offset}"),
        |origin| {
            format!(
                "{label}/source:{}/site:{offset}",
                SourceOriginId::activation_key(origin)
            )
        },
    )
}

fn source_procedure_activation_name(
    origin: Option<&Arc<SourceOriginId>>,
    target: &SourceCommandTarget,
    site: u32,
) -> String {
    let mut label = format!("{}@{}", target.command, target.implementation_generation);
    if let Some(allocation) = &target.implementation_allocation {
        use std::fmt::Write;
        write!(
            label,
            "/implementation:{}:{}:{:?}",
            SourceOriginId::activation_key(&allocation.site.source),
            allocation.site.offset,
            allocation.incarnation,
        )
        .expect("formatting an activation identity into a String");
    }
    source_activation_name(origin, &label, site)
}

pub(super) fn source_called_body_activation_name(
    origin: Option<&Arc<SourceOriginId>>,
    target: &SourceCommandTarget,
    body: &DeferredSourceBody,
    site: u32,
) -> String {
    let parent = source_procedure_activation_name(origin, target, site);
    if body.receiver_method
        && let Some((declaration, _, _)) = &body.executed_script
    {
        return format!(
            "{parent}/body:{}:{}",
            SourceOriginId::activation_key(&declaration.source),
            declaration.offset,
        );
    }
    parent
}

impl PartialEq for SourceOriginId {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.kind, &other.kind)
            || (self.fingerprint() == other.fingerprint() && self.kind == other.kind)
    }
}

impl Eq for SourceOriginId {}

impl Hash for SourceOriginId {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.fingerprint().hash(state);
    }
}

impl PartialOrd for SourceOriginId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SourceOriginId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if Arc::ptr_eq(&self.kind, &other.kind) {
            return std::cmp::Ordering::Equal;
        }
        self.fingerprint()
            .cmp(&other.fingerprint())
            .then_with(|| self.kind.cmp(&other.kind))
    }
}

/// Native protocol interpreting a materialised argv value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum MaterialisedSourceKind {
    /// Tcl script parser.
    Script,
    /// Tcl expression parser.
    Expression,
}

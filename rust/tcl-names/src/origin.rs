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

//! The identity of one source instance, the source-point origin token and the
//! executed script bytes with their truthful source relationship.

use crate::command::CommandAllocationSite;
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{Arc, Mutex, OnceLock, Weak};
use tcl_lexer::LexerConfig;
use tcl_syntax::word_rules::WordValueRules;

/// Source instance in which an executable command was allocated.
#[derive(Debug, Clone)]
pub struct SourceOriginId {
    kind: Arc<SourceOriginKind>,
    fingerprint: u64,
    activation_key: u64,
}

#[derive(Default)]
struct SourceOriginInterner {
    buckets: HashMap<u64, Vec<(Weak<SourceOriginKind>, u64)>>,
    next_key: u64,
}

impl SourceOriginInterner {
    fn intern(&mut self, source: &Arc<SourceOriginKind>, fingerprint: u64) -> u64 {
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

    /// The shared kind allocation, for memos keyed by this exact source instance.
    #[doc(hidden)]
    #[must_use]
    pub fn shared_kind(&self) -> &Arc<SourceOriginKind> {
        &self.kind
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

    #[doc(hidden)]
    #[must_use]
    pub fn derived(
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

    #[doc(hidden)]
    #[must_use]
    pub fn derived_image(
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

    #[doc(hidden)]
    #[must_use]
    pub fn from_kind(kind: SourceOriginKind) -> Self {
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

    const fn fingerprint(&self) -> u64 {
        self.fingerprint
    }

    /// Process-local activation key shared by equal live source instances.
    #[doc(hidden)]
    #[must_use]
    pub fn activation_key(source: &Arc<Self>) -> u64 {
        source.activation_key
    }
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

/// Immutable origin identity issued only after complete parent-point comparison.
/// The matching full entry owns state; this token alone grants no state access.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OriginalSourcePointOrigin(Arc<SourcePointOriginToken>);

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct SourcePointOriginToken {
    issued_key: u64,
}

impl OriginalSourcePointOrigin {
    /// The token of one issued source-point entry. Only the entry's issuer
    /// supplies the key; the token grants no state access.
    #[doc(hidden)]
    #[must_use]
    pub fn from_issued_key(issued_key: u64) -> Self {
        Self(Arc::new(SourcePointOriginToken { issued_key }))
    }

    /// Whether this token was issued for the entry holding `issued_key`.
    #[doc(hidden)]
    #[must_use]
    pub fn matches_issued_key(&self, issued_key: u64) -> bool {
        self.0.issued_key == issued_key
    }
}

/// Mapping from executed script bytes into their owning source instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExecutedScriptMapping {
    /// Every byte occurs unchanged at this base in the retained source origin.
    Contiguous {
        /// First executable byte, independent of surrounding word delimiters.
        base: u32,
    },
    /// Substitution, escape decoding or concatenation created a new source.
    Materialised,
}

/// One evaluated body, shared by word, list and concatenated-script consumers.
///
/// ```
/// use std::sync::Arc;
/// use tcl_names::command::CommandAllocationSite;
/// use tcl_names::origin::{ExecutedScriptSource, SourceOriginId};
/// let parent = CommandAllocationSite {
///     source: Arc::new(SourceOriginId::authored(&Arc::from("eval $body"))),
///     offset: 0,
/// };
/// let body = ExecutedScriptSource::materialised(parent, vec![0], "set x 1");
/// assert_eq!(body.text.try_text().unwrap(), "set x 1");
/// assert_eq!(body.base(), 0);
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ExecutedScriptSource {
    /// Exact bytes received by the native script parser after argv evaluation.
    pub text: tcl_lexer::SourceImage,
    /// Semantic source instance; offsets never masquerade as another origin.
    pub origin: Arc<SourceOriginId>,
    /// Proved correspondence to the retained source instance.
    pub mapping: ExecutedScriptMapping,
}

impl ExecutedScriptSource {
    /// Retain an unchanged slice only after checking its owning source bytes.
    #[must_use]
    pub fn contiguous(origin: Arc<SourceOriginId>, text: &str, base: u32) -> Option<Self> {
        let start = usize::try_from(base).ok()?;
        let end = start.checked_add(text.len())?;
        (origin.source_image().bytes().get(start..end) == Some(text.as_bytes())).then(|| Self {
            text: native_image(text),
            origin,
            mapping: ExecutedScriptMapping::Contiguous { base },
        })
    }

    /// Retain the actual counted script value. Original static-word decoding
    /// can establish a source extent, but only byte equality grants a
    /// contiguous map. Channel/unit conversion remains a materialised source.
    #[doc(hidden)]
    #[must_use]
    pub fn from_original_word_value(
        parent: CommandAllocationSite,
        argument: usize,
        word: &tcl_lexer::NativeWord,
        value: &[u8],
        protocol: tcl_syntax::native_string::NativeStringProtocol,
    ) -> Self {
        // Implementation contract: naming.source.original-native-script-body-value
        // docs/design/analysis/name-resolution-proofs/original-native-script-body-value.md
        let text = tcl_lexer::SourceImage::native(value);
        if word.image() == parent.source.source_image()
            && word.span().start() >= parent.offset
            && let Ok(captured) = tcl_registry::native_compiler_words::NativeCompilerWords::capture(
                std::slice::from_ref(word),
                protocol,
            )
            && captured.literal(0) == Some(value)
            && let Some(span) = captured.original_literal_extent(0, 0..value.len())
            && span.as_range().len() == value.len()
            && let Some(script) =
                Self::contiguous_image(Arc::clone(&parent.source), text.clone(), span.start())
        {
            return script;
        }
        if word.image() == parent.source.source_image()
            && word.group().kind == tcl_lexer::WordKind::Braced
            && let Ok(content) = word.content_span()
            && let Some(interior) = word
                .image()
                .bytes()
                .get(content.as_range())
                .and_then(|bytes| std::str::from_utf8(bytes).ok())
            && let Ok(value) = std::str::from_utf8(value)
            && folds_to(interior, value, word.config())
            && let Some(script) =
                Self::contiguous(Arc::clone(&parent.source), interior, content.start())
        {
            return script;
        }
        Self::materialised_image(parent, vec![argument], text)
    }

    /// Retain a computed or concatenated value without inventing authored spans.
    #[must_use]
    pub fn materialised(parent: CommandAllocationSite, arguments: Vec<usize>, value: &str) -> Self {
        Self::materialised_image(parent, arguments, native_image(value))
    }

    /// Retain an original byte script without a Unicode projection.
    #[must_use]
    pub fn materialised_image(
        parent: CommandAllocationSite,
        arguments: Vec<usize>,
        text: tcl_lexer::SourceImage,
    ) -> Self {
        Self {
            origin: Arc::new(SourceOriginId::derived_image(
                parent,
                arguments,
                text.clone(),
                MaterialisedSourceKind::Script,
            )),
            text,
            mapping: ExecutedScriptMapping::Materialised,
        }
    }

    /// Prove an unchanged byte slice while preserving its parser input channel.
    #[must_use]
    pub fn contiguous_image(
        origin: Arc<SourceOriginId>,
        text: tcl_lexer::SourceImage,
        base: u32,
    ) -> Option<Self> {
        let start = usize::try_from(base).ok()?;
        let end = start.checked_add(text.len())?;
        (origin.source_image().bytes().get(start..end) == Some(text.bytes())).then_some(Self {
            text,
            origin,
            mapping: ExecutedScriptMapping::Contiguous { base },
        })
    }

    /// Checked advisory text; the original byte script remains authoritative.
    ///
    /// # Errors
    /// Returns the retained bytes' UTF-8 failure.
    pub fn try_text(&self) -> Result<&str, std::str::Utf8Error> {
        self.text.try_text()
    }

    /// Select a script from an evaluated list through the actual list grammar.
    /// Literal element bytes retain a contiguous source map; decoded elements
    /// receive a distinct materialised origin instead of an estimated offset.
    #[must_use]
    pub fn list_element(
        &self,
        parent: CommandAllocationSite,
        argument: usize,
        element: usize,
        rules: WordValueRules,
    ) -> Option<Self> {
        let text = self.try_text().ok()?;
        let values = rules.split_list(text).ok()?;
        let value = values.get(element)?.as_ref();
        if let ExecutedScriptMapping::Contiguous { base } = self.mapping
            && let Some(offset) = literal_list_element_offset(text, element, value, rules.list)
            && let Some(base) = base.checked_add(offset)
        {
            return Some(Self {
                text: native_image(value),
                origin: Arc::clone(&self.origin),
                mapping: ExecutedScriptMapping::Contiguous { base },
            });
        }
        Some(Self::materialised(parent, vec![argument, element], value))
    }

    /// Offset used by segmentation in this actual source instance.
    #[must_use]
    pub const fn base(&self) -> u32 {
        match self.mapping {
            ExecutedScriptMapping::Contiguous { base } => base,
            ExecutedScriptMapping::Materialised => 0,
        }
    }
}

fn native_image(source: &str) -> tcl_lexer::SourceImage {
    tcl_lexer::SourceImage::native(Arc::<[u8]>::from(source.as_bytes()))
}

/// A braced body whose value differs from its bytes only by the brace
/// backslash-newline fold. Parsing the authored bytes substitutes each
/// continuation exactly as the fold does, so they yield the same commands and
/// words and keep their document coordinates. The exception is a `${…}` name,
/// which the parser takes verbatim; a body holding one stays materialised.
#[doc(hidden)]
#[must_use]
pub fn folds_to(interior: &str, value: &str, config: LexerConfig) -> bool {
    interior != value
        && config.brace_backslash_newline.folds()
        && !interior.contains("${")
        && WordValueRules::from_config(&config).collapse_braced_word(interior) == value
}

fn literal_list_element_offset(
    text: &str,
    element: usize,
    value: &str,
    syntax: tcl_dialect::ListParse,
) -> Option<u32> {
    let mut next = 0;
    for index in 0..=element {
        let item = tcl_syntax::list::find_element_with_syntax(text, next, syntax)
            .ok()
            .flatten()?;
        next = item.next;
        if index == element && text.get(item.value.clone()) == Some(value) {
            return u32::try_from(item.value.start).ok();
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cached_source_origin_fingerprints_do_not_replace_semantic_identity() {
        let mut left = SourceOriginId::authored(&Arc::from("proc p {} {return LEFT}"));
        let mut right = SourceOriginId::authored(&Arc::from("proc p {} {return RIGHT}"));
        left.fingerprint = 7;
        right.fingerprint = 7;
        assert_ne!(left, right);
        assert_ne!(left.cmp(&right), std::cmp::Ordering::Equal);
        let mut identical = SourceOriginId::authored(&Arc::from("proc p {} {return LEFT}"));
        identical.fingerprint = 7;
        assert_eq!(left, identical);
    }

    #[test]
    fn full_source_identity_breaks_fingerprint_collisions() {
        let mut left = SourceOriginId::authored(&Arc::from("left"));
        let mut equal = SourceOriginId::authored(&Arc::from("left"));
        let mut foreign = SourceOriginId::authored(&Arc::from("right"));
        left.fingerprint = 0;
        equal.fingerprint = 0;
        foreign.fingerprint = 0;
        let roots = [left, equal, foreign];
        for left in &roots {
            for right in &roots {
                assert_eq!(left == right, left.cmp(right).is_eq());
            }
        }
        assert_eq!(roots[0], roots[1]);
        assert_ne!(roots[0], roots[2]);
    }

    #[test]
    fn activation_source_ids_check_full_equality_and_keep_equal_live_clones() {
        let left = Arc::new(SourceOriginId::authored(&Arc::from("left")));
        let right = Arc::new(SourceOriginId::authored(&Arc::from("right")));
        let mut interner = SourceOriginInterner::default();
        let collision_key = interner.intern(&left.kind, 7);
        assert_ne!(collision_key, interner.intern(&right.kind, 7));
        let retained_payload = Arc::clone(&left.kind);
        assert_eq!(collision_key, interner.intern(&retained_payload, 7));
        let first = SourceOriginId::activation_key(&left);
        assert_ne!(first, SourceOriginId::activation_key(&right));
        let clone = Arc::new(left.as_ref().clone());
        assert_eq!(first, SourceOriginId::activation_key(&clone));
        drop(left);
        let equal = Arc::new(clone.as_ref().clone());
        assert_eq!(first, SourceOriginId::activation_key(&equal));
        let derived = Arc::new(SourceOriginId::derived(
            CommandAllocationSite {
                source: Arc::clone(&equal),
                offset: 0,
            },
            vec![1],
            &Arc::from("left"),
            MaterialisedSourceKind::Script,
        ));
        assert_ne!(first, SourceOriginId::activation_key(&derived));
    }
}

// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Target-neutral identities for registry-declared intrinsic operations.
//!
//! An intrinsic identifies semantics that more than one target may implement.
//! It is not a promise that specialisation is legal: live command binding,
//! trace, interpreter, effect, and representation proofs remain separate.

use tcl_dialect::{StringCharacterModel, TclVersion};
use tcl_runtime_api::guard::{GuardDomain, GuardDomains, GuardIdentity};

use crate::hooks::{CodegenHookId, InlineCodegenHookId, LoweringHookId};
use crate::semantic_operation::SemanticOperationId;

/// The release variant of a member whose guarded implementation contract
/// reads the same on every release.
const RUNTIME_INVARIANT_SEMANTICS: u32 = 0;
const TCL8_UTF16_STRING_SEMANTICS: u32 = 1;
const TCL9_SCALAR_STRING_SEMANTICS: u32 = 2;
/// Tcl 8.4/8.5, whose three-byte `TCL_UTF_MAX` makes a supplementary code
/// point count as its four UTF-8 bytes rather than as a surrogate pair. A
/// distinct key, not a reuse of the 8.6 one: the whole point of this value is
/// that an implementation attested under one string model must not be shared
/// with a release that counts differently, and 8.4 answers 4 where 8.6
/// answers 2. Appended rather than renumbered, because these variants are
/// stable.
const TCL84_BMP_STRING_SEMANTICS: u32 = 3;
const JIM084_UTF8_STRING_SEMANTICS: u32 = 4;

/// Width of the release-variant field, the low bits of a guarded semantics
/// key.
const VARIANT_BITS: u32 = 3;
/// Width of the revision field, above the release variant.
const REVISION_BITS: u32 = 14;
/// Where a member's own [`IntrinsicId::stable_id`] sits in its keys: the
/// rest of the word, above the revision.
const MEMBER_SHIFT: u32 = VARIANT_BITS + REVISION_BITS;
/// The largest revision a key can carry.
const MAX_REVISION: u32 = (1 << REVISION_BITS) - 1;
/// The largest stable identity a key can carry.
const MAX_MEMBER_ID: u32 = u32::MAX >> MEMBER_SHIFT;

/// Each member's guarded-contract revision.
///
/// A member's row rises when what a compiled fast path may assume of its
/// implementation changes, and then only that member's guard-semantics keys
/// move: compiled code attested against the old contract stops matching the
/// live implementation, and every other member's guards stay valid. Every
/// row starts at `0`.
///
/// One explicit row per member, matched by [`IntrinsicId::stable_id`] and
/// never by declaration order; the length is the catalogue's, so a member
/// added without a row fails to compile.
const SEMANTICS_REVISION: [(IntrinsicId, u32); 34] = [
    (IntrinsicId::ListAssign, 0),
    (IntrinsicId::ListLength, 0),
    (IntrinsicId::ListIndex, 0),
    (IntrinsicId::ListRange, 0),
    (IntrinsicId::ListReplace, 0),
    (IntrinsicId::ListInsert, 0),
    (IntrinsicId::ListSet, 0),
    (IntrinsicId::ListConstruct, 0),
    (IntrinsicId::DictGet, 0),
    (IntrinsicId::DictSet, 0),
    (IntrinsicId::DictUnset, 0),
    (IntrinsicId::DictIncr, 0),
    (IntrinsicId::DictAppend, 0),
    (IntrinsicId::DictListAppend, 0),
    (IntrinsicId::StringIndex, 0),
    (IntrinsicId::StringRange, 0),
    (IntrinsicId::StringEqual, 0),
    (IntrinsicId::StringCompare, 0),
    (IntrinsicId::StringReplace, 0),
    (IntrinsicId::StringLength, 0),
    (IntrinsicId::StringIs, 0),
    (IntrinsicId::Regexp, 0),
    (IntrinsicId::InfoExists, 0),
    (IntrinsicId::ArrayExists, 0),
    (IntrinsicId::ArrayNames, 0),
    (IntrinsicId::ArraySize, 0),
    (IntrinsicId::Concat, 0),
    (IntrinsicId::ChannelWrite, 0),
    (IntrinsicId::InfoCommandsResolve, 0),
    (IntrinsicId::InfoLevel, 0),
    (IntrinsicId::ProcedureNoOp, 0),
    (IntrinsicId::NamespaceCurrent, 0),
    (IntrinsicId::NamespaceOrigin, 0),
    (IntrinsicId::NamespaceCode, 0),
];

/// One `'static` list of a member's keys, one per release variant it may
/// attest, computed from [`SEMANTICS_REVISION`] at compile time.
macro_rules! member_keys {
    ($member:expr; $($variant:expr),+ $(,)?) => {
        const { &[$($member.semantics_key($variant, &SEMANTICS_REVISION)),+] }
    };
}

/// Target-neutral identity of a registry-described intrinsic operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum IntrinsicId {
    /// Assign list elements to variables.
    ListAssign,
    /// Return a list's length.
    ListLength,
    /// Select one or more list elements by index.
    ListIndex,
    /// Select an inclusive list range.
    ListRange,
    /// Replace an inclusive list range.
    ListReplace,
    /// Insert elements into a list.
    ListInsert,
    /// Update a list held in a variable.
    ListSet,
    /// Construct a Tcl list from arguments.
    ListConstruct,
    /// Read a nested dictionary key path.
    DictGet,
    /// Set a nested dictionary key path in a variable.
    DictSet,
    /// Remove a nested dictionary key path from a variable.
    DictUnset,
    /// Increment a dictionary entry held in a variable.
    DictIncr,
    /// Append text to a dictionary entry held in a variable.
    DictAppend,
    /// Append a list element to a dictionary entry held in a variable.
    DictListAppend,
    /// Select one character from a string.
    StringIndex,
    /// Select an inclusive string range.
    StringRange,
    /// Compare strings for equality.
    StringEqual,
    /// Lexicographically compare strings.
    StringCompare,
    /// Replace an inclusive string range.
    StringReplace,
    /// Return a string's character length.
    StringLength,
    /// Test whether a string belongs to a declared character class.
    StringIs,
    /// Match a regular expression.
    Regexp,
    /// Test whether a variable exists.
    InfoExists,
    /// Resolve an absolute literal command name and return its singleton list, or an empty list.
    InfoCommandsResolve,
    /// Native stack-level introspection.
    InfoLevel,
    /// Test whether an array variable exists.
    ArrayExists,
    /// Return array element names.
    ArrayNames,
    /// Return an array's element count.
    ArraySize,
    /// Concatenate lists.
    Concat,
    /// Write a value to a Tcl channel.
    ///
    /// The operation covers the complete `puts` semantic surface. Backends must
    /// still select only invocation forms whose option and channel shape they
    /// implement directly.
    ChannelWrite,
    /// Native compiler result for an empty non-precompiled variadic procedure.
    ProcedureNoOp,
    /// Return the namespace of the currently executing activation.
    NamespaceCurrent,
    /// Resolve a command's original imported identity in the current namespace.
    NamespaceOrigin,
    /// Build a prefix that captures the runtime namespace around a literal script.
    NamespaceCode,
}

/// The contract family of an intrinsic: what state a fast path for it may
/// reach beyond the values it is given.
///
/// The family is the widest reach of the member under any invocation form, so
/// `string is`, whose `-failindex` stores, and `regexp`, whose match variables
/// store, are Family B although a call without those words touches nothing.
/// The lattice has no channel domain, so channel operations take the family's
/// variable-trace domain as the store operations do.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IntrinsicFamily {
    /// A function of its argument values alone, computed by a shared core in
    /// `tcl-cmd-core`: it reaches no variable, array, or channel state.
    Value,
    /// An operation over a runtime-owned state adapter. Variable and channel
    /// operations can re-enter script through traces; namespace and activation
    /// reads retain their own independent entry proofs.
    FamilyB {
        /// The domains a guard for the member must cover beyond the dispatch
        /// dependencies of its command.
        domains: GuardDomains,
        /// Whether the operation runs a variable's traces while only
        /// observing it: `info exists` a read trace, the array queries an
        /// array trace. An operation that stores runs its write traces as
        /// part of the store and is not marked.
        fires_traces: bool,
    },
}

impl IntrinsicFamily {
    /// A Family-B member that stores through an adapter.
    const STORES: Self = Self::FamilyB {
        domains: GuardDomains::one(GuardDomain::VariableTrace),
        fires_traces: false,
    };

    /// A Family-B member that observes a variable and runs its traces.
    const OBSERVES_AND_FIRES: Self = Self::FamilyB {
        domains: GuardDomains::one(GuardDomain::VariableTrace),
        fires_traces: true,
    };

    /// The guard domains a guard for a member of this family must cover.
    #[must_use]
    pub const fn guard_domains(self) -> GuardDomains {
        match self {
            Self::Value => GuardDomains::EMPTY,
            Self::FamilyB { domains, .. } => domains,
        }
    }

    /// Whether the family runs variable traces while only observing.
    #[must_use]
    pub const fn fires_traces(self) -> bool {
        matches!(
            self,
            Self::FamilyB {
                fires_traces: true,
                ..
            }
        )
    }
}

impl IntrinsicId {
    /// Every intrinsic in stable-ID order.
    pub const ALL: &'static [Self] = &[
        Self::ListAssign,
        Self::ListLength,
        Self::ListIndex,
        Self::ListRange,
        Self::ListReplace,
        Self::ListInsert,
        Self::ListSet,
        Self::ListConstruct,
        Self::DictGet,
        Self::DictSet,
        Self::DictUnset,
        Self::DictIncr,
        Self::DictAppend,
        Self::DictListAppend,
        Self::StringIndex,
        Self::StringRange,
        Self::StringEqual,
        Self::StringCompare,
        Self::StringReplace,
        Self::StringLength,
        Self::StringIs,
        Self::Regexp,
        Self::InfoExists,
        Self::InfoCommandsResolve,
        Self::InfoLevel,
        Self::ArrayExists,
        Self::ArrayNames,
        Self::ArraySize,
        Self::Concat,
        Self::ChannelWrite,
        Self::ProcedureNoOp,
        Self::NamespaceCurrent,
        Self::NamespaceOrigin,
        Self::NamespaceCode,
    ];

    /// Explicit stable scalar identity for offline artefacts and runtime guards.
    ///
    /// These values are intentionally grouped and matched, never derived from
    /// enum declaration order or an integer cast.
    #[must_use]
    pub const fn stable_id(self) -> u32 {
        match self {
            Self::ListAssign => 0x0101,
            Self::ListLength => 0x0102,
            Self::ListIndex => 0x0103,
            Self::ListRange => 0x0104,
            Self::ListReplace => 0x0105,
            Self::ListInsert => 0x0106,
            Self::ListSet => 0x0107,
            Self::ListConstruct => 0x0108,
            Self::DictGet => 0x0201,
            Self::DictSet => 0x0202,
            Self::DictUnset => 0x0203,
            Self::DictIncr => 0x0204,
            Self::DictAppend => 0x0205,
            Self::DictListAppend => 0x0206,
            Self::StringIndex => 0x0301,
            Self::StringRange => 0x0302,
            Self::StringEqual => 0x0303,
            Self::StringCompare => 0x0304,
            Self::StringReplace => 0x0305,
            Self::StringLength => 0x0306,
            Self::StringIs => 0x0307,
            Self::Regexp => 0x0401,
            Self::InfoExists => 0x0501,
            Self::InfoCommandsResolve => 0x0502,
            Self::InfoLevel => 0x0503,
            Self::ArrayExists => 0x0601,
            Self::ArrayNames => 0x0602,
            Self::ArraySize => 0x0603,
            Self::Concat => 0x0701,
            Self::ChannelWrite => 0x0801,
            Self::ProcedureNoOp => 0x0901,
            Self::NamespaceCurrent => 0x0A01,
            Self::NamespaceOrigin => 0x0A02,
            Self::NamespaceCode => 0x0A03,
        }
    }

    /// The contract family of this member.
    ///
    /// Matched per member with no wildcard arm, like [`Self::stable_id`], so
    /// a member added to the catalogue must be classified. Changing a
    /// member's family changes what a fast path may assume of it, and the
    /// manifest's intrinsic-table hash covers the family, so a runtime built
    /// against another classification is refused.
    #[must_use]
    pub const fn family(self) -> IntrinsicFamily {
        match self {
            Self::ListLength
            | Self::ListIndex
            | Self::ListRange
            | Self::ListReplace
            | Self::ListInsert
            | Self::ListConstruct
            | Self::DictGet
            | Self::StringIndex
            | Self::StringRange
            | Self::StringEqual
            | Self::StringCompare
            | Self::StringReplace
            | Self::StringLength
            | Self::Concat => IntrinsicFamily::Value,
            Self::ListAssign
            | Self::ListSet
            | Self::DictSet
            | Self::DictUnset
            | Self::DictIncr
            | Self::DictAppend
            | Self::DictListAppend
            | Self::StringIs
            | Self::Regexp
            | Self::ChannelWrite => IntrinsicFamily::STORES,
            Self::ProcedureNoOp => IntrinsicFamily::Value,
            Self::InfoCommandsResolve | Self::NamespaceOrigin | Self::NamespaceCode => {
                IntrinsicFamily::FamilyB {
                    domains: GuardDomains::one(GuardDomain::CommandEnvironment)
                        .with(GuardDomain::Namespace),
                    fires_traces: false,
                }
            }
            Self::NamespaceCurrent => IntrinsicFamily::FamilyB {
                domains: GuardDomains::one(GuardDomain::Namespace),
                fires_traces: false,
            },
            Self::InfoLevel => IntrinsicFamily::FamilyB {
                domains: GuardDomains::one(GuardDomain::Interpreter),
                fires_traces: false,
            },
            Self::InfoExists | Self::ArrayExists | Self::ArrayNames | Self::ArraySize => {
                IntrinsicFamily::OBSERVES_AND_FIRES
            }
        }
    }

    /// The intrinsic a registry guard identity names, if it names one.
    #[must_use]
    pub fn from_guard_identity(identity: GuardIdentity) -> Option<Self> {
        identity.registry_stable_id().and_then(Self::from_stable_id)
    }

    /// The domains a guard request for `identity` must cover: its family's,
    /// for an identity of this vocabulary, and none for any other.
    #[must_use]
    pub fn required_guard_domains(identity: GuardIdentity) -> GuardDomains {
        Self::from_guard_identity(identity).map_or(GuardDomains::EMPTY, |member| {
            member.family().guard_domains()
        })
    }

    /// This member's row in `revisions`, an explicit [`SEMANTICS_REVISION`]-shaped
    /// table: a parameter, not always the live table, so a test can bump one
    /// row and prove what moves.
    const fn revision_in(self, revisions: &[(Self, u32)]) -> u32 {
        let mut index = 0;
        while index < revisions.len() {
            let (member, revision) = revisions[index];
            if member.stable_id() == self.stable_id() {
                return revision;
            }
            index += 1;
        }
        panic!("SEMANTICS_REVISION has no row for this intrinsic")
    }

    /// One guarded semantics key: this member's [`Self::stable_id`], its row
    /// of `revisions`, and a release variant, in disjoint bit fields. No two
    /// members' keys can collide, a bumped revision moves only its own
    /// member's keys, and a versioned member's release variants stay
    /// distinct from each other.
    const fn semantics_key(self, release_variant: u32, revisions: &[(Self, u32)]) -> u32 {
        let revision = self.revision_in(revisions);
        assert!(
            release_variant < 1 << VARIANT_BITS,
            "a release variant must fit its field"
        );
        assert!(
            revision <= MAX_REVISION,
            "a semantics revision must fit its field"
        );
        assert!(
            self.stable_id() <= MAX_MEMBER_ID,
            "a stable identity must fit its field"
        );
        (self.stable_id() << MEMBER_SHIFT) | (revision << VARIANT_BITS) | release_variant
    }

    /// Select the character-model facet; activation proof is independent.
    const fn character_variant(self, characters: StringCharacterModel) -> u32 {
        match self {
            Self::StringLength => match characters {
                StringCharacterModel::BmpCharsElseUtf8Bytes => TCL84_BMP_STRING_SEMANTICS,
                StringCharacterModel::Utf16CodeUnits => TCL8_UTF16_STRING_SEMANTICS,
                StringCharacterModel::UnicodeScalars => TCL9_SCALAR_STRING_SEMANTICS,
                StringCharacterModel::Jim084Utf8 => JIM084_UTF8_STRING_SEMANTICS,
            },
            _ => RUNTIME_INVARIANT_SEMANTICS,
        }
    }

    /// Stable runtime-semantics key included in guarded implementation
    /// identity: one key per member, so a member whose guarded contract moves
    /// (its row in `SEMANTICS_REVISION`) invalidates its own guards and no
    /// other member's.
    ///
    /// Most intrinsics currently have one release-invariant implementation
    /// contract. `StringLength` is additionally versioned because the
    /// releases count differently: 8.4/8.5 count BMP characters and spell a
    /// supplementary code point as its four UTF-8 bytes, 8.6 counts
    /// UTF-16-style `Tcl_UniChar` units, and 9.x counts Unicode scalar
    /// values.
    #[must_use]
    pub const fn guard_semantics_key(self, runtime: TclVersion) -> u32 {
        self.guard_semantics_key_for_characters(runtime.string_character_model())
    }

    /// Key for an independently selected native character model, including Jim.
    #[must_use]
    pub const fn guard_semantics_key_for_characters(self, characters: StringCharacterModel) -> u32 {
        self.semantics_key(self.character_variant(characters), &SEMANTICS_REVISION)
    }

    /// Every guarded runtime-semantics key this intrinsic may attest, across
    /// every selected model: one for an invariant member, four for
    /// `StringLength`.
    ///
    /// Matched per member with no wildcard arm, like [`Self::stable_id`], so
    /// a member added to the catalogue must say which releases it is
    /// versioned across rather than inherit the invariant contract.
    #[must_use]
    pub const fn guard_semantics_variants(self) -> &'static [u32] {
        match self {
            Self::ListAssign => member_keys!(Self::ListAssign; RUNTIME_INVARIANT_SEMANTICS),
            Self::ListLength => member_keys!(Self::ListLength; RUNTIME_INVARIANT_SEMANTICS),
            Self::ListIndex => member_keys!(Self::ListIndex; RUNTIME_INVARIANT_SEMANTICS),
            Self::ListRange => member_keys!(Self::ListRange; RUNTIME_INVARIANT_SEMANTICS),
            Self::ListReplace => member_keys!(Self::ListReplace; RUNTIME_INVARIANT_SEMANTICS),
            Self::ListInsert => member_keys!(Self::ListInsert; RUNTIME_INVARIANT_SEMANTICS),
            Self::ListSet => member_keys!(Self::ListSet; RUNTIME_INVARIANT_SEMANTICS),
            Self::ListConstruct => {
                member_keys!(Self::ListConstruct; RUNTIME_INVARIANT_SEMANTICS)
            }
            Self::DictGet => member_keys!(Self::DictGet; RUNTIME_INVARIANT_SEMANTICS),
            Self::DictSet => member_keys!(Self::DictSet; RUNTIME_INVARIANT_SEMANTICS),
            Self::DictUnset => member_keys!(Self::DictUnset; RUNTIME_INVARIANT_SEMANTICS),
            Self::DictIncr => member_keys!(Self::DictIncr; RUNTIME_INVARIANT_SEMANTICS),
            Self::DictAppend => member_keys!(Self::DictAppend; RUNTIME_INVARIANT_SEMANTICS),
            Self::DictListAppend => {
                member_keys!(Self::DictListAppend; RUNTIME_INVARIANT_SEMANTICS)
            }
            Self::StringIndex => member_keys!(Self::StringIndex; RUNTIME_INVARIANT_SEMANTICS),
            Self::StringRange => member_keys!(Self::StringRange; RUNTIME_INVARIANT_SEMANTICS),
            Self::StringEqual => member_keys!(Self::StringEqual; RUNTIME_INVARIANT_SEMANTICS),
            Self::StringCompare => {
                member_keys!(Self::StringCompare; RUNTIME_INVARIANT_SEMANTICS)
            }
            Self::StringReplace => {
                member_keys!(Self::StringReplace; RUNTIME_INVARIANT_SEMANTICS)
            }
            Self::StringLength => member_keys!(
                Self::StringLength;
                TCL8_UTF16_STRING_SEMANTICS,
                TCL9_SCALAR_STRING_SEMANTICS,
                TCL84_BMP_STRING_SEMANTICS,
                JIM084_UTF8_STRING_SEMANTICS,
            ),
            Self::StringIs => member_keys!(Self::StringIs; RUNTIME_INVARIANT_SEMANTICS),
            Self::Regexp => member_keys!(Self::Regexp; RUNTIME_INVARIANT_SEMANTICS),
            Self::InfoExists => member_keys!(Self::InfoExists; RUNTIME_INVARIANT_SEMANTICS),
            Self::ArrayExists => member_keys!(Self::ArrayExists; RUNTIME_INVARIANT_SEMANTICS),
            Self::ArrayNames => member_keys!(Self::ArrayNames; RUNTIME_INVARIANT_SEMANTICS),
            Self::ArraySize => member_keys!(Self::ArraySize; RUNTIME_INVARIANT_SEMANTICS),
            Self::Concat => member_keys!(Self::Concat; RUNTIME_INVARIANT_SEMANTICS),
            Self::ChannelWrite => member_keys!(Self::ChannelWrite; RUNTIME_INVARIANT_SEMANTICS),
            Self::InfoCommandsResolve => {
                member_keys!(Self::InfoCommandsResolve; RUNTIME_INVARIANT_SEMANTICS)
            }
            Self::InfoLevel => member_keys!(Self::InfoLevel; RUNTIME_INVARIANT_SEMANTICS),
            Self::ProcedureNoOp => member_keys!(Self::ProcedureNoOp; RUNTIME_INVARIANT_SEMANTICS),
            Self::NamespaceCurrent => {
                member_keys!(Self::NamespaceCurrent; RUNTIME_INVARIANT_SEMANTICS)
            }
            Self::NamespaceOrigin => {
                member_keys!(Self::NamespaceOrigin; RUNTIME_INVARIANT_SEMANTICS)
            }
            Self::NamespaceCode => member_keys!(Self::NamespaceCode; RUNTIME_INVARIANT_SEMANTICS),
        }
    }

    /// Recover an intrinsic from its explicit stable scalar identity.
    #[must_use]
    pub const fn from_stable_id(id: u32) -> Option<Self> {
        let mut index = 0;
        while index < Self::ALL.len() {
            let intrinsic = Self::ALL[index];
            if intrinsic.stable_id() == id {
                return Some(intrinsic);
            }
            index += 1;
        }
        None
    }

    pub(crate) const fn from_descriptors(
        semantic: Option<SemanticOperationId>,
        lowering: Option<LoweringHookId>,
        codegen: Option<CodegenHookId>,
        inline_codegen: Option<InlineCodegenHookId>,
    ) -> Option<Self> {
        match semantic {
            Some(SemanticOperationId::Intrinsic(id)) => Some(id),
            Some(_) => None,
            None if lowering.is_some() => None,
            None => {
                let inline = match inline_codegen {
                    Some(hook) => Self::from_legacy_inline_codegen(hook),
                    None => None,
                };
                match inline {
                    Some(id) => Some(id),
                    None => match codegen {
                        Some(hook) => Self::from_legacy_codegen(hook),
                        None => None,
                    },
                }
            }
        }
    }

    /// Stable compiler and Explorer spelling for this semantic identity.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ListAssign => "list-assign",
            Self::ListLength => "list-length",
            Self::ListIndex => "list-index",
            Self::ListRange => "list-range",
            Self::ListReplace => "list-replace",
            Self::ListInsert => "list-insert",
            Self::ListSet => "list-set",
            Self::ListConstruct => "list-construct",
            Self::DictGet => "dict-get",
            Self::DictSet => "dict-set",
            Self::DictUnset => "dict-unset",
            Self::DictIncr => "dict-incr",
            Self::DictAppend => "dict-append",
            Self::DictListAppend => "dict-list-append",
            Self::StringIndex => "string-index",
            Self::StringRange => "string-range",
            Self::StringEqual => "string-equal",
            Self::StringCompare => "string-compare",
            Self::StringReplace => "string-replace",
            Self::StringLength => "string-length",
            Self::StringIs => "string-is",
            Self::Regexp => "regexp",
            Self::InfoExists => "info-exists",
            Self::InfoCommandsResolve => "info-commands-resolve",
            Self::InfoLevel => "info-level",
            Self::ArrayExists => "array-exists",
            Self::ArrayNames => "array-names",
            Self::ArraySize => "array-size",
            Self::Concat => "concat",
            Self::ChannelWrite => "channel-write",
            Self::ProcedureNoOp => "procedure-noop",
            Self::NamespaceCurrent => "namespace-current",
            Self::NamespaceOrigin => "namespace-origin",
            Self::NamespaceCode => "namespace-code",
        }
    }

    /// Compatibility projection from a legacy statement-position `TclVM` hook.
    ///
    /// This mapping temporarily lets existing registry specs declare common
    /// intrinsic identity without duplicating every hook stamp. New specs
    /// should prefer an explicit [`crate::SemanticOperationId::Intrinsic`]
    /// declaration; the legacy hook remains only for bytecode emission.
    #[must_use]
    pub const fn from_legacy_codegen(hook: CodegenHookId) -> Option<Self> {
        match hook {
            CodegenHookId::Lassign => Some(Self::ListAssign),
            CodegenHookId::Llength => Some(Self::ListLength),
            CodegenHookId::Lrange => Some(Self::ListRange),
            CodegenHookId::Linsert => Some(Self::ListInsert),
            CodegenHookId::Lset => Some(Self::ListSet),
            CodegenHookId::Concat => Some(Self::Concat),
            CodegenHookId::Append
            | CodegenHookId::Lappend
            | CodegenHookId::Unset
            | CodegenHookId::Tailcall
            | CodegenHookId::Global
            | CodegenHookId::Upvar
            | CodegenHookId::Dict
            | CodegenHookId::Array
            | CodegenHookId::Namespace
            | CodegenHookId::Uplevel => None,
        }
    }

    /// Compatibility projection from a legacy value/catch-position `TclVM` hook.
    ///
    /// See [`Self::from_legacy_codegen`] for the migration contract.
    #[must_use]
    pub const fn from_legacy_inline_codegen(hook: InlineCodegenHookId) -> Option<Self> {
        match hook {
            InlineCodegenHookId::InfoExists => Some(Self::InfoExists),
            InlineCodegenHookId::InfoCommandsResolve => Some(Self::InfoCommandsResolve),
            InlineCodegenHookId::InfoLevel => Some(Self::InfoLevel),
            InlineCodegenHookId::NamespaceCurrent => Some(Self::NamespaceCurrent),
            InlineCodegenHookId::NamespaceOrigin => Some(Self::NamespaceOrigin),
            InlineCodegenHookId::NamespaceCode => Some(Self::NamespaceCode),
            InlineCodegenHookId::Lindex => Some(Self::ListIndex),
            InlineCodegenHookId::Lrange => Some(Self::ListRange),
            InlineCodegenHookId::Lreplace => Some(Self::ListReplace),
            InlineCodegenHookId::Linsert => Some(Self::ListInsert),
            InlineCodegenHookId::Regexp => Some(Self::Regexp),
            InlineCodegenHookId::List => Some(Self::ListConstruct),
            InlineCodegenHookId::DictGet => Some(Self::DictGet),
            InlineCodegenHookId::Expr
            | InlineCodegenHookId::Incr
            | InlineCodegenHookId::Catch
            | InlineCodegenHookId::Return
            | InlineCodegenHookId::Error
            | InlineCodegenHookId::Break
            | InlineCodegenHookId::Continue
            | InlineCodegenHookId::Yield
            | InlineCodegenHookId::YieldTo
            | InlineCodegenHookId::Try
            | InlineCodegenHookId::String
            | InlineCodegenHookId::Array => None,
        }
    }
}

/// One member of the intrinsic table as its hash reads it.
struct TableRow {
    stable_id: u32,
    family: IntrinsicFamily,
    semantics: &'static [u32],
}

/// The digest of `rows`, each read in the order given.
fn table_digest(rows: impl IntoIterator<Item = TableRow>) -> [u8; 32] {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    for row in rows {
        hasher.update(row.stable_id.to_le_bytes());
        match row.family {
            IntrinsicFamily::Value => hasher.update([0_u8]),
            IntrinsicFamily::FamilyB {
                domains,
                fires_traces,
            } => {
                hasher.update([1_u8]);
                hasher.update(domains.bits().to_le_bytes());
                hasher.update([u8::from(fires_traces)]);
            }
        }
        let count = u32::try_from(row.semantics.len()).expect("a member has few variants");
        hasher.update(count.to_le_bytes());
        for key in row.semantics {
            hasher.update(key.to_le_bytes());
        }
    }
    hasher.finalize().into()
}

/// The hash of the intrinsic table this build emits against and dispatches
/// for: every member's stable identity, its [`IntrinsicId::family`] and its
/// guarded-semantics keys ([`IntrinsicId::guard_semantics_variants`]), in
/// table order.
///
/// An artefact records the hash of the table its emitter keyed against
/// (`ArtefactIdentityManifest::intrinsic_table_hash`), and a runtime whose
/// own differs refuses it: a member added, re-classified or re-versioned on
/// one side changes what a guarded fast path may assume of the other.
#[must_use]
pub fn intrinsic_table_hash() -> [u8; 32] {
    static HASH: std::sync::OnceLock<[u8; 32]> = std::sync::OnceLock::new();
    *HASH.get_or_init(|| {
        table_digest(IntrinsicId::ALL.iter().map(|member| TableRow {
            stable_id: member.stable_id(),
            family: member.family(),
            semantics: member.guard_semantics_variants(),
        }))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows() -> Vec<TableRow> {
        IntrinsicId::ALL
            .iter()
            .map(|member| TableRow {
                stable_id: member.stable_id(),
                family: member.family(),
                semantics: member.guard_semantics_variants(),
            })
            .collect()
    }

    #[test]
    fn the_table_hash_is_the_digest_of_every_members_row() {
        assert_eq!(intrinsic_table_hash(), table_digest(rows()));
        assert_ne!(intrinsic_table_hash(), [0; 32]);
    }

    #[test]
    fn the_table_hash_moves_with_each_thing_it_covers() {
        let base = table_digest(rows());

        let mut without_one = rows();
        without_one.pop();
        assert_ne!(table_digest(without_one), base, "a member goes");

        let mut reordered = rows();
        reordered.swap(0, 1);
        assert_ne!(table_digest(reordered), base, "table order");

        let mut renumbered = rows();
        renumbered[0].stable_id += 1;
        assert_ne!(table_digest(renumbered), base, "a stable identity");

        let mut reclassified = rows();
        let value = reclassified
            .iter_mut()
            .find(|row| row.family == IntrinsicFamily::Value)
            .expect("the table has a value member");
        value.family = IntrinsicFamily::STORES;
        assert_ne!(table_digest(reclassified), base, "a family");

        let mut retraced = rows();
        let stores = retraced
            .iter_mut()
            .find(|row| row.family == IntrinsicFamily::STORES)
            .expect("the table has a storing member");
        stores.family = IntrinsicFamily::OBSERVES_AND_FIRES;
        assert_ne!(table_digest(retraced), base, "whether it fires traces");

        let mut reversioned = rows();
        reversioned[0].semantics = &[7];
        assert_ne!(table_digest(reversioned), base, "a semantics key");

        let mut extra_variant = rows();
        extra_variant[0].semantics = &[0, 0];
        assert_ne!(table_digest(extra_variant), base, "a release variant");
    }

    #[test]
    fn overlapping_legacy_hooks_share_one_intrinsic_identity() {
        assert_eq!(
            IntrinsicId::from_legacy_codegen(CodegenHookId::Lrange),
            IntrinsicId::from_legacy_inline_codegen(InlineCodegenHookId::Lrange)
        );
        assert_eq!(
            IntrinsicId::from_legacy_codegen(CodegenHookId::Linsert),
            IntrinsicId::from_legacy_inline_codegen(InlineCodegenHookId::Linsert)
        );
        assert_eq!(IntrinsicId::from_legacy_codegen(CodegenHookId::Array), None);
        assert_eq!(
            IntrinsicId::from_legacy_inline_codegen(InlineCodegenHookId::String),
            None
        );
    }

    /// The current execution-surface document names the complete catalogue.
    #[test]
    fn the_intrinsic_catalogue_is_the_size_this_plan_quotes_issue_2140() {
        assert_eq!(
            IntrinsicId::ALL.len(),
            34,
            "update the current intrinsic catalogue count in \"docs/design/compiler/wasm-native-lowering.md\""
        );
    }

    #[test]
    fn stable_ids_round_trip_without_ordinal_dependence() {
        let mut ids = std::collections::BTreeSet::new();
        for &intrinsic in IntrinsicId::ALL {
            assert!(ids.insert(intrinsic.stable_id()));
            assert_eq!(
                IntrinsicId::from_stable_id(intrinsic.stable_id()),
                Some(intrinsic)
            );
        }
        assert_eq!(IntrinsicId::StringLength.stable_id(), 0x0306);
        assert_eq!(IntrinsicId::from_stable_id(0), None);
    }

    #[test]
    fn string_length_guard_identity_is_runtime_semantics_versioned() {
        assert_ne!(
            IntrinsicId::StringLength.guard_semantics_key(tcl_dialect::TclVersion::V8_6),
            IntrinsicId::StringLength.guard_semantics_key(tcl_dialect::TclVersion::V9_0)
        );
        assert_eq!(
            IntrinsicId::ListLength.guard_semantics_key(tcl_dialect::TclVersion::V8_6),
            IntrinsicId::ListLength.guard_semantics_key(tcl_dialect::TclVersion::V9_0)
        );
    }

    /// Guard identity partitions the selected character models; it does not
    /// attest an entered command, frame or native object.
    #[test]
    fn string_length_character_model_keys_do_not_collide() {
        let models = [
            StringCharacterModel::BmpCharsElseUtf8Bytes,
            StringCharacterModel::Utf16CodeUnits,
            StringCharacterModel::UnicodeScalars,
            StringCharacterModel::Jim084Utf8,
        ];
        let keys = models
            .into_iter()
            .map(|model| IntrinsicId::StringLength.guard_semantics_key_for_characters(model))
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(keys.len(), 4);
        for key in keys {
            assert!(
                IntrinsicId::StringLength
                    .guard_semantics_variants()
                    .contains(&key)
            );
        }
    }

    /// A bumped `revisions` row for `member`, every other row as it was.
    fn bumped(member: IntrinsicId, revision: u32) -> [(IntrinsicId, u32); 34] {
        let mut table = SEMANTICS_REVISION;
        for row in &mut table {
            if row.0 == member {
                row.1 = revision;
            }
        }
        table
    }

    /// Every key `member` answers across the C Tcl and Jim character models.
    fn keys_under(
        member: IntrinsicId,
        revisions: &[(IntrinsicId, u32)],
    ) -> std::collections::BTreeSet<u32> {
        [
            StringCharacterModel::BmpCharsElseUtf8Bytes,
            StringCharacterModel::Utf16CodeUnits,
            StringCharacterModel::UnicodeScalars,
            StringCharacterModel::Jim084Utf8,
        ]
        .into_iter()
        .map(|characters| member.semantics_key(member.character_variant(characters), revisions))
        .collect()
    }

    #[test]
    fn the_revision_table_names_every_member_exactly_once() {
        assert_eq!(SEMANTICS_REVISION.len(), IntrinsicId::ALL.len());
        for &member in IntrinsicId::ALL {
            let rows = SEMANTICS_REVISION
                .iter()
                .filter(|(row, _)| *row == member)
                .count();
            assert_eq!(
                rows, 1,
                "{member:?} needs exactly one SEMANTICS_REVISION row"
            );
            assert_eq!(member.revision_in(&SEMANTICS_REVISION), 0);
        }
    }

    #[test]
    fn every_member_has_a_distinct_semantics_key() {
        let mut owners = std::collections::BTreeMap::new();
        for &member in IntrinsicId::ALL {
            for release in tcl_dialect::TclVersion::ALL {
                let key = member.guard_semantics_key(release);
                assert_ne!(key, 0, "{member:?} must not answer the no-semantics key");
                let previous = owners.insert(key, member);
                assert!(
                    previous.is_none_or(|owner| owner == member),
                    "{member:?} and {previous:?} share key {key:#x}"
                );
            }
        }
        // Distinct members, distinct variant lists: no key is on two lists.
        let mut listed = std::collections::BTreeSet::new();
        for &member in IntrinsicId::ALL {
            for &key in member.guard_semantics_variants() {
                assert!(listed.insert(key), "{member:?} lists a key another lists");
            }
        }
    }

    #[test]
    fn the_variants_are_exactly_the_keys_the_releases_answer() {
        for &member in IntrinsicId::ALL {
            let answered = keys_under(member, &SEMANTICS_REVISION);
            let listed: std::collections::BTreeSet<u32> =
                member.guard_semantics_variants().iter().copied().collect();
            assert_eq!(answered, listed, "{member:?}");
            assert_eq!(
                listed.len(),
                member.guard_semantics_variants().len(),
                "{member:?} lists a key twice"
            );
        }
        assert_eq!(
            IntrinsicId::StringLength.guard_semantics_variants().len(),
            4
        );
        assert_eq!(IntrinsicId::ListLength.guard_semantics_variants().len(), 1);
    }

    #[test]
    fn a_key_names_its_member_in_its_high_field() {
        for &member in IntrinsicId::ALL {
            for release in tcl_dialect::TclVersion::ALL {
                let key = member.guard_semantics_key(release);
                assert_eq!(key >> MEMBER_SHIFT, member.stable_id(), "{member:?}");
                assert_eq!(
                    IntrinsicId::from_stable_id(key >> MEMBER_SHIFT),
                    Some(member)
                );
            }
        }
    }

    #[test]
    fn bumping_one_members_revision_moves_no_other_key() {
        for &bumped_member in IntrinsicId::ALL {
            for revision in [1, MAX_REVISION] {
                let table = bumped(bumped_member, revision);
                for &member in IntrinsicId::ALL {
                    let before = keys_under(member, &SEMANTICS_REVISION);
                    let after = keys_under(member, &table);
                    if member == bumped_member {
                        assert!(
                            before.is_disjoint(&after),
                            "{member:?}: a bumped revision must move every one of its keys"
                        );
                        assert_eq!(before.len(), after.len(), "{member:?}");
                    } else {
                        assert_eq!(
                            before, after,
                            "bumping {bumped_member:?} moved {member:?}'s keys"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn a_bumped_member_still_collides_with_no_other() {
        let table = bumped(IntrinsicId::DictGet, MAX_REVISION);
        let mut owners = std::collections::BTreeMap::new();
        for &member in IntrinsicId::ALL {
            for key in keys_under(member, &table) {
                assert!(
                    owners
                        .insert(key, member)
                        .is_none_or(|owner| owner == member),
                    "{member:?} collides at key {key:#x}"
                );
            }
        }
    }

    #[test]
    #[should_panic(expected = "revision must fit")]
    fn a_revision_beyond_its_field_is_refused() {
        let table = bumped(IntrinsicId::Regexp, MAX_REVISION + 1);
        let _ = IntrinsicId::Regexp.semantics_key(RUNTIME_INVARIANT_SEMANTICS, &table);
    }

    #[test]
    fn string_length_answers_three_keys_across_five_releases() {
        use tcl_dialect::TclVersion;
        let key = |release| IntrinsicId::StringLength.guard_semantics_key(release);
        assert_eq!(key(TclVersion::V8_4), key(TclVersion::V8_5));
        assert_ne!(key(TclVersion::V8_5), key(TclVersion::V8_6));
        assert_ne!(key(TclVersion::V8_6), key(TclVersion::V9_0));
        assert_ne!(key(TclVersion::V8_4), key(TclVersion::V9_0));
        assert_eq!(key(TclVersion::V9_0), key(TclVersion::V9_1));
    }

    #[test]
    fn every_member_names_a_family() {
        use IntrinsicId::*;
        let (value, family_b): (Vec<_>, Vec<_>) = IntrinsicId::ALL
            .iter()
            .copied()
            .partition(|member| member.family() == IntrinsicFamily::Value);
        assert_eq!(
            value,
            [
                ListLength,
                ListIndex,
                ListRange,
                ListReplace,
                ListInsert,
                ListConstruct,
                DictGet,
                StringIndex,
                StringRange,
                StringEqual,
                StringCompare,
                StringReplace,
                StringLength,
                Concat,
                ProcedureNoOp,
            ]
        );
        assert_eq!(
            family_b,
            [
                ListAssign,
                ListSet,
                DictSet,
                DictUnset,
                DictIncr,
                DictAppend,
                DictListAppend,
                StringIs,
                Regexp,
                InfoExists,
                InfoCommandsResolve,
                InfoLevel,
                ArrayExists,
                ArrayNames,
                ArraySize,
                ChannelWrite,
                NamespaceCurrent,
                NamespaceOrigin,
                NamespaceCode,
            ]
        );
        for member in family_b {
            let IntrinsicFamily::FamilyB { domains, .. } = member.family() else {
                unreachable!("partitioned as Family B");
            };
            let expected = match member {
                InfoCommandsResolve | NamespaceOrigin | NamespaceCode => {
                    GuardDomains::one(GuardDomain::CommandEnvironment).with(GuardDomain::Namespace)
                }
                NamespaceCurrent => GuardDomains::one(GuardDomain::Namespace),
                InfoLevel => GuardDomains::one(GuardDomain::Interpreter),
                _ => GuardDomains::one(GuardDomain::VariableTrace),
            };
            assert_eq!(domains, expected, "{member:?}");
        }
    }

    #[test]
    fn trace_firing_members_are_family_b() {
        let firing: Vec<_> = IntrinsicId::ALL
            .iter()
            .copied()
            .filter(|member| member.family().fires_traces())
            .collect();
        assert_eq!(
            firing,
            [
                IntrinsicId::InfoExists,
                IntrinsicId::ArrayExists,
                IntrinsicId::ArrayNames,
                IntrinsicId::ArraySize,
            ]
        );
        for member in firing {
            assert_eq!(
                member.family(),
                IntrinsicFamily::FamilyB {
                    domains: GuardDomains::one(GuardDomain::VariableTrace),
                    fires_traces: true,
                },
                "{member:?}"
            );
        }
    }

    #[test]
    fn a_guard_request_must_cover_its_members_family_domains() {
        for &member in IntrinsicId::ALL {
            let required = member.family().guard_domains();
            let bare = GuardIdentity::registry_intrinsic(member.stable_id());
            assert_eq!(IntrinsicId::from_guard_identity(bare), Some(member));
            assert_eq!(IntrinsicId::required_guard_domains(bare), required);
            for release in tcl_dialect::TclVersion::ALL {
                let packed = GuardIdentity::registry_intrinsic_with_semantics(
                    member.stable_id(),
                    member.guard_semantics_key(release),
                );
                assert_eq!(IntrinsicId::from_guard_identity(packed), Some(member));
                assert_eq!(IntrinsicId::required_guard_domains(packed), required);
            }
            assert_eq!(
                required == GuardDomains::EMPTY,
                member.family() == IntrinsicFamily::Value,
                "{member:?}: a state-reading member requires its declared domain"
            );
        }
        let foreign = GuardIdentity::new(7, u64::from(IntrinsicId::DictSet.stable_id()));
        assert_eq!(
            IntrinsicId::required_guard_domains(foreign),
            GuardDomains::EMPTY
        );
        let unknown = GuardIdentity::registry_intrinsic(0x7fff);
        assert_eq!(
            IntrinsicId::required_guard_domains(unknown),
            GuardDomains::EMPTY
        );
    }

    #[test]
    fn command_spec_collects_intrinsics_from_its_subcommands() {
        let registry = crate::CommandRegistry::build_default();
        let string = registry.get("string").expect("string spec");
        let ids = string.intrinsic_ids();
        for expected in [
            IntrinsicId::StringCompare,
            IntrinsicId::StringEqual,
            IntrinsicId::StringIndex,
            IntrinsicId::StringIs,
            IntrinsicId::StringLength,
            IntrinsicId::StringRange,
            IntrinsicId::StringReplace,
        ] {
            assert!(ids.contains(&expected), "missing {expected:?}");
        }
    }
}

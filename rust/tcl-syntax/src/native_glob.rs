// SPDX-License-Identifier: AGPL-3.0-or-later
//! Native glob recipes with independently retained policy and object storage.
//!
//! Counted object matching, C-string scans and exact table lookup have different
//! extents. Inputs must describe actual materialised storage; bytes alone do not
//! establish an object type, native issuer, callback closure or live name.

use std::{borrow::Cow, collections::HashSet};

use tcl_core_types::c_string_extent;
use tcl_dialect::{TclVersion, model::DialectPoint};

use crate::{
    naming::{NamePolicyAuthority, NamePolicyProtocol, NativeNameProtocol},
    native_string::NativeStringInput,
    native_tcl_utf::{NativeTclUtf, TclUtfUnit},
    raw_string::{NativeStringAccessError, RawString},
};

/// Native operation selecting matching extent and exact-lookup optimisation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeNameGlobPurpose {
    /// C export filters scan; Jim exports impose no filter.
    ExportFilter,
    /// C8.4 scans even trivial import patterns; later C and Jim use exact keys.
    ImportSearch,
    /// Command enumeration uses exact lookup for trivial patterns.
    InfoCommandsSearch,
    /// Array glob enumeration uses full object keys for trivial C8.5+ patterns.
    ArrayNamesSearch,
    /// An already selected variable-enumeration scan, excluding lookup selection.
    InfoVariablesScan,
    /// Forget matches a local imported name, with trivial lookup in C8.5+.
    ForgetOwnSearch,
    /// Qualified forget always scans the original source tail in C.
    ForgetOriginFilter,
    /// Namespace child enumeration selects its release-specific trivial lookup.
    NamespaceChildrenSearch,
    /// `TclOO` instance/subclass enumeration uses its `CString` name scan.
    OoObjectNamesSearch,
    /// `TclOO` declared-variable validation uses the native `CString` matcher.
    OoDeclaredVariableValidation,
}

/// Original object storage selected before native matching converts it.
#[derive(Debug, Clone, Copy)]
pub enum NativeGlobObject<'a> {
    /// A native NULL-type object with its complete resident string.
    FreshString(&'a [u8]),
    /// Native String type without a retained Unicode cache.
    String(&'a [u8]),
    /// Actual cached native units; resident bytes, when present, remain distinct.
    CachedUnicode {
        /// Native units, including C8 surrogate units and embedded zero.
        units: &'a [u32],
        /// Original resident bytes, rather than bytes re-encoded from the cache.
        resident_bytes: Option<&'a [u8]>,
    },
    /// Native byte-array type with no resident string.
    PureByteArray(&'a [u8]),
    /// Native byte-array backing with an independently retained resident string.
    ByteArrayWithString {
        /// Existing byte-array data, not reconstructed from the string.
        bytes: &'a [u8],
        /// Original resident bytes, which native conversion leaves attached.
        resident_bytes: &'a [u8],
    },
    /// Another native type whose string has already been materialised.
    OtherString(&'a [u8]),
    /// Object residency/type is unavailable; no inference from bytes is permitted.
    Unknown,
}

/// Unsupported host capability, distinct from an ordinary false match.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeGlobUnavailable {
    /// No independently selected canonical recipe.
    ProtocolUnavailable,
    /// The native dispatch requires object storage not retained by the caller.
    ObjectRepresentationUnavailable,
    /// A claimed cached unit does not fit the selected native representation.
    InvalidUnicodeUnit,
    /// No matching recipe is audited for this operation and engine.
    PurposeUnavailable,
    /// Jim requested storage outside the retained bytes and owned terminator.
    JimAccess(NativeStringAccessError),
}

impl std::fmt::Display for NativeGlobUnavailable {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::ProtocolUnavailable => "native glob protocol is unavailable",
            Self::ObjectRepresentationUnavailable => {
                "native glob object representation is unavailable"
            }
            Self::InvalidUnicodeUnit => "cached glob unit is unavailable for this native release",
            Self::PurposeUnavailable => "native glob purpose is unavailable",
            Self::JimAccess(_) => "Jim glob access exceeds retained storage",
        })
    }
}
impl std::error::Error for NativeGlobUnavailable {}

/// Pure matching policy retaining actual versus authored issuer separately.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NativeGlobProtocol {
    policy: NamePolicyProtocol,
}

impl NativeGlobProtocol {
    /// Retain the independently selected naming provider without changing issuer.
    #[must_use]
    pub const fn from_name_policy(policy: NamePolicyProtocol) -> Self {
        Self { policy }
    }

    /// Select an actual canonical C or audited Jim engine/build point.
    #[must_use]
    pub fn for_native_point(point: DialectPoint) -> Option<Self> {
        NamePolicyProtocol::for_native_point(point).map(Self::from_name_policy)
    }

    /// Explicit logical C simulation; does not attest its physical host.
    #[must_use]
    pub const fn authored_tcl(version: TclVersion) -> Self {
        Self::from_name_policy(NamePolicyProtocol::authored_tcl(version))
    }

    /// Retained naming provider, suitable for capability/cache keys.
    #[must_use]
    pub const fn name_policy(self) -> NamePolicyProtocol {
        self.policy
    }

    /// Pure native recipe, without changing its independently retained issuer.
    #[must_use]
    pub const fn recipe(self) -> NativeNameProtocol {
        self.policy.recipe()
    }

    /// Actual versus explicitly authored issuance.
    #[must_use]
    pub const fn authority(self) -> NamePolicyAuthority {
        self.policy.authority()
    }

    /// Jim namespace export is a no-op; C applies stored export filters.
    #[must_use]
    pub const fn export_filter_applies(self) -> bool {
        !matches!(self.recipe(), NativeNameProtocol::Jim084)
    }

    /// Match an independently selected named operation on materialised operands.
    ///
    /// # Errors
    /// Jim refuses a reached access beyond retained object storage.
    pub fn match_name_pattern(
        self,
        purpose: NativeNameGlobPurpose,
        pattern: &[u8],
        candidate: &[u8],
    ) -> Result<bool, NativeGlobUnavailable> {
        match_native_name_pattern(self.recipe(), purpose, pattern, candidate)
    }

    /// Select native object dispatch using actual original type/cache residency.
    /// Matching is pure: concrete owners perform required conversions and effects.
    ///
    /// # Errors
    /// Missing storage, incompatible cached units or reached Jim accesses refuse.
    pub fn match_objects(
        self,
        pattern: NativeGlobObject<'_>,
        subject: NativeGlobObject<'_>,
        nocase: bool,
    ) -> Result<bool, NativeGlobUnavailable> {
        match_native_glob_objects(self.recipe(), pattern, subject, nocase)
    }
}

/// `Tcl_UtfToLower` over a native terminated string, using the selected library's
/// existing simple mapping table. The native in-place updater cannot expand a unit.
#[must_use]
pub fn lower_c_string_bytes(version: TclVersion, bytes: &[u8]) -> Vec<u8> {
    let bytes = c_string_extent(bytes);
    let input = GlobInput::CString(bytes, version);
    let codec = NativeTclUtf::for_version(version);
    let mut output = Vec::new();
    let mut offset = 0;
    while offset < bytes.len() {
        let unit = input.unit(offset, false);
        let folded = input.fold(unit.value, true);
        let mapped = codec.encode_character(folded);
        if let Some(mapped) = mapped.filter(|mapped| mapped.len() <= unit.width) {
            output.extend_from_slice(&mapped);
        } else {
            output.extend_from_slice(&bytes[offset..offset + unit.width]);
        }
        offset += unit.width;
    }
    output
}

/// Match the native `Tcl_StringCaseMatch` `CString` entry point. Both operands
/// stop at their first resident NUL; this does not select an object getter,
/// Unicode cache, table lookup or live interpreter issuer.
#[must_use]
pub fn match_c_string_glob(
    version: TclVersion,
    pattern: &[u8],
    subject: &[u8],
    nocase: bool,
) -> bool {
    c_match(
        GlobInput::CString(c_string_extent(pattern), version),
        GlobInput::CString(c_string_extent(subject), version),
        nocase,
    )
}

/// Compare the `CString` operands of native `strcmp` or `TclUtfCasecmp`.
/// Case-sensitive equality compares resident bytes; case-insensitive equality
/// decodes the selected release's units with the preceding decoder state and
/// native simple lowercase table. No Rust Unicode projection is required.
#[must_use]
pub fn equal_c_strings(version: TclVersion, left: &[u8], right: &[u8], nocase: bool) -> bool {
    let left = c_string_extent(left);
    let right = c_string_extent(right);
    if !nocase {
        return left == right;
    }
    let (mut left_at, mut right_at) = (0, 0);
    let (mut left_previous, mut right_previous) = (None, None);
    while left_at < left.len() && right_at < right.len() {
        let left_unit = decode_c_unit(version, &left[left_at..], left_previous, false, false);
        let right_unit = decode_c_unit(version, &right[right_at..], right_previous, false, false);
        if left_unit.value != right_unit.value
            && crate::native_tcl_case::lower(version, left_unit.value)
                != crate::native_tcl_case::lower(version, right_unit.value)
        {
            return false;
        }
        left_at += left_unit.width;
        right_at += right_unit.width;
        left_previous = Some(left_unit.value);
        right_previous = Some(right_unit.value);
    }
    left_at == left.len() && right_at == right.len()
}

/// Match native name operands using the purpose's scan or exact-table rule.
/// This recipe-only entry does not authenticate the caller's live engine.
///
/// # Errors
/// Jim refuses reached accesses beyond retained bytes and their terminator.
pub fn match_native_name_pattern(
    protocol: NativeNameProtocol,
    purpose: NativeNameGlobPurpose,
    pattern: &[u8],
    candidate: &[u8],
) -> Result<bool, NativeGlobUnavailable> {
    if matches!(
        purpose,
        NativeNameGlobPurpose::OoObjectNamesSearch
            | NativeNameGlobPurpose::OoDeclaredVariableValidation
    ) && !matches!(protocol, NativeNameProtocol::C(version) if version >= TclVersion::V8_6)
    {
        return Err(NativeGlobUnavailable::PurposeUnavailable);
    }
    if protocol == NativeNameProtocol::Jim084
        && matches!(
            purpose,
            NativeNameGlobPurpose::ForgetOwnSearch
                | NativeNameGlobPurpose::ForgetOriginFilter
                | NativeNameGlobPurpose::NamespaceChildrenSearch
        )
    {
        return Err(NativeGlobUnavailable::PurposeUnavailable);
    }
    if protocol == NativeNameProtocol::Jim084 && purpose == NativeNameGlobPurpose::ExportFilter {
        return Ok(true);
    }
    let full_object = matches!(purpose, NativeNameGlobPurpose::ArrayNamesSearch)
        || (protocol == NativeNameProtocol::Jim084
            && matches!(
                purpose,
                NativeNameGlobPurpose::InfoCommandsSearch
                    | NativeNameGlobPurpose::InfoVariablesScan
            ));
    let selected_pattern = if full_object {
        pattern
    } else {
        c_string_extent(pattern)
    };
    let selected_candidate = if full_object {
        candidate
    } else {
        c_string_extent(candidate)
    };
    if name_pattern_uses_exact_lookup(protocol, purpose, pattern) {
        return Ok(selected_pattern == selected_candidate);
    }
    match protocol {
        NativeNameProtocol::Jim084 => jim_match(selected_pattern, selected_candidate, false),
        NativeNameProtocol::C(version) => Ok(c_match(
            GlobInput::CString(c_string_extent(pattern), version),
            GlobInput::CString(c_string_extent(candidate), version),
            false,
        )),
    }
}

/// Decide only the native trivial-pattern optimisation. The caller still owns
/// namespace routing, table identity, visibility and lookup-versus-scan choice.
#[must_use]
pub fn name_pattern_uses_exact_lookup(
    protocol: NativeNameProtocol,
    purpose: NativeNameGlobPurpose,
    pattern: &[u8],
) -> bool {
    let applies = match purpose {
        NativeNameGlobPurpose::InfoCommandsSearch => true,
        NativeNameGlobPurpose::ImportSearch => protocol != NativeNameProtocol::C(TclVersion::V8_4),
        NativeNameGlobPurpose::ArrayNamesSearch
        | NativeNameGlobPurpose::ForgetOwnSearch
        | NativeNameGlobPurpose::NamespaceChildrenSearch => {
            matches!(protocol, NativeNameProtocol::C(version) if version >= TclVersion::V8_5)
        }
        NativeNameGlobPurpose::ExportFilter
        | NativeNameGlobPurpose::InfoVariablesScan
        | NativeNameGlobPurpose::ForgetOriginFilter
        | NativeNameGlobPurpose::OoObjectNamesSearch
        | NativeNameGlobPurpose::OoDeclaredVariableValidation => false,
    };
    let extent = if protocol == NativeNameProtocol::Jim084
        && purpose != NativeNameGlobPurpose::ImportSearch
    {
        pattern
    } else {
        c_string_extent(pattern)
    };
    applies && crate::glob::is_literal_bytes(extent)
}

/// The physical lookup selected for an original namespace-children pattern.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeNamespaceChildrenLookup<'a> {
    /// Walk the selected parent's original child table using native matching.
    Scan,
    /// Resolve the complete native namespace spelling, then check its parent.
    FullName(&'a [u8]),
    /// Probe the selected parent's child table with the exact physical key.
    /// A missing key means the original full-name prefix check failed.
    ChildKey(Option<&'a [u8]>),
}

/// Select namespace-children lookup without recovering a token from reporting bytes.
#[must_use]
pub fn namespace_children_lookup<'a>(
    protocol: NativeNameProtocol,
    parent_full_name: &[u8],
    pattern: &'a [u8],
) -> NativeNamespaceChildrenLookup<'a> {
    if !name_pattern_uses_exact_lookup(
        protocol,
        NativeNameGlobPurpose::NamespaceChildrenSearch,
        pattern,
    ) {
        return NativeNamespaceChildrenLookup::Scan;
    }
    let pattern = c_string_extent(pattern);
    if protocol == NativeNameProtocol::C(TclVersion::V8_5) {
        return NativeNamespaceChildrenLookup::ChildKey(
            pattern.strip_prefix(c_string_extent(parent_full_name)),
        );
    }
    NativeNamespaceChildrenLookup::FullName(pattern)
}

/// Native `string match` object dispatcher; representation is never guessed.
///
/// # Errors
/// Missing object storage or invalid cached native units refuse.
pub fn match_native_glob_objects(
    protocol: NativeNameProtocol,
    pattern: NativeGlobObject<'_>,
    subject: NativeGlobObject<'_>,
    nocase: bool,
) -> Result<bool, NativeGlobUnavailable> {
    let NativeNameProtocol::C(version) = protocol else {
        return jim_match(
            &object_bytes(protocol, pattern)?,
            &object_bytes(protocol, subject)?,
            nocase,
        );
    };
    let unicode = version == TclVersion::V8_4
        || matches!(
            subject,
            NativeGlobObject::String(_) | NativeGlobObject::CachedUnicode { .. }
        )
        || (version >= TclVersion::V8_6 && matches!(subject, NativeGlobObject::FreshString(_)));
    if unicode {
        let pattern = object_unicode(version, pattern)?;
        let subject = object_unicode(version, subject)?;
        return Ok(c_match(
            GlobInput::Units(&pattern, version),
            GlobInput::Units(&subject, version),
            nocase,
        ));
    }
    if !nocase
        && let NativeGlobObject::PureByteArray(subject) = subject
        && (version == TclVersion::V8_5 || matches!(pattern, NativeGlobObject::PureByteArray(_)))
    {
        let pattern = object_byte_array(version, pattern)?;
        let pattern: Vec<_> = pattern.iter().map(|&byte| u32::from(byte)).collect();
        let subject: Vec<_> = subject.iter().map(|&byte| u32::from(byte)).collect();
        return Ok(c_match(
            GlobInput::Units(&pattern, version),
            GlobInput::Units(&subject, version),
            false,
        ));
    }
    let pattern = object_bytes(protocol, pattern)?;
    let subject = object_bytes(protocol, subject)?;
    Ok(c_match(
        GlobInput::CString(c_string_extent(&pattern), version),
        GlobInput::CString(c_string_extent(&subject), version),
        nocase,
    ))
}

fn jim_match(pattern: &[u8], subject: &[u8], nocase: bool) -> Result<bool, NativeGlobUnavailable> {
    RawString::from_bytes(subject)
        .jim084_matches(&RawString::from_bytes(pattern), nocase)
        .map_err(NativeGlobUnavailable::JimAccess)
}

fn object_unicode(
    version: TclVersion,
    object: NativeGlobObject<'_>,
) -> Result<Vec<u32>, NativeGlobUnavailable> {
    match object {
        NativeGlobObject::CachedUnicode { units, .. } => {
            let maximum = if version < TclVersion::V9_0 {
                0xffff
            } else {
                0x10_ffff
            };
            if units.iter().any(|&unit| unit > maximum) {
                return Err(NativeGlobUnavailable::InvalidUnicodeUnit);
            }
            Ok(units.to_vec())
        }
        NativeGlobObject::PureByteArray(bytes) => {
            Ok(bytes.iter().map(|&byte| u32::from(byte)).collect())
        }
        NativeGlobObject::FreshString(bytes)
        | NativeGlobObject::String(bytes)
        | NativeGlobObject::OtherString(bytes)
        | NativeGlobObject::ByteArrayWithString {
            resident_bytes: bytes,
            ..
        } => {
            let mut output = Vec::new();
            let mut offset = 0;
            let mut previous = None;
            while offset < bytes.len() {
                let unit = decode_c_unit(version, &bytes[offset..], previous, false, false);
                output.push(unit.value);
                offset += unit.width;
                previous = Some(unit.value);
            }
            Ok(output)
        }
        NativeGlobObject::Unknown => Err(NativeGlobUnavailable::ObjectRepresentationUnavailable),
    }
}

fn object_byte_array(
    version: TclVersion,
    object: NativeGlobObject<'_>,
) -> Result<Vec<u8>, NativeGlobUnavailable> {
    if let NativeGlobObject::PureByteArray(bytes)
    | NativeGlobObject::ByteArrayWithString { bytes, .. } = object
    {
        return Ok(bytes.to_vec());
    }
    // Native SetByteArrayFromAny decodes the original resident bytes with
    // Tcl_UtfToUniChar, independently of String's cached Unicode/macro path.
    let bytes = object_bytes(NativeNameProtocol::C(version), object)?;
    let mut output = Vec::new();
    let mut at = 0;
    let mut previous = None;
    while at < bytes.len() {
        let unit = decode_c_unit(version, &bytes[at..], previous, true, false);
        output.push(u8::try_from(unit.value & 0xff).expect("masked native byte unit fits"));
        at += unit.width;
        previous = Some(unit.value);
    }
    Ok(output)
}

fn object_bytes(
    protocol: NativeNameProtocol,
    object: NativeGlobObject<'_>,
) -> Result<Cow<'_, [u8]>, NativeGlobUnavailable> {
    match object {
        NativeGlobObject::FreshString(bytes)
        | NativeGlobObject::String(bytes)
        | NativeGlobObject::OtherString(bytes)
        | NativeGlobObject::ByteArrayWithString {
            resident_bytes: bytes,
            ..
        }
        | NativeGlobObject::CachedUnicode {
            resident_bytes: Some(bytes),
            ..
        } => Ok(Cow::Borrowed(bytes)),
        NativeGlobObject::PureByteArray(bytes) => protocol
            .string_protocol()
            .materialize(NativeStringInput::PureByteArray(bytes))
            .map_err(|_| NativeGlobUnavailable::ObjectRepresentationUnavailable),
        NativeGlobObject::CachedUnicode {
            resident_bytes: None,
            ..
        }
        | NativeGlobObject::Unknown => Err(NativeGlobUnavailable::ObjectRepresentationUnavailable),
    }
}

#[derive(Clone, Copy)]
enum GlobInput<'a> {
    Units(&'a [u32], TclVersion),
    CString(&'a [u8], TclVersion),
}
impl GlobInput<'_> {
    fn len(self) -> usize {
        match self {
            Self::Units(units, _) => units.len(),
            Self::CString(bytes, _) => bytes.len(),
        }
    }
    fn raw(self, index: usize) -> u32 {
        match self {
            Self::Units(units, _) => units.get(index).copied().unwrap_or(0),
            Self::CString(bytes, _) => bytes.get(index).map_or(0, |&byte| u32::from(byte)),
        }
    }
    fn unit(self, index: usize, direct: bool) -> TclUtfUnit {
        match self {
            Self::Units(units, _) => TclUtfUnit {
                value: units.get(index).copied().unwrap_or(0),
                width: 1,
            },
            Self::CString(bytes, version) => decode_c_unit(
                version,
                bytes.get(index..).unwrap_or_default(),
                None,
                direct,
                true,
            ),
        }
    }
    fn fold(self, unit: u32, nocase: bool) -> u32 {
        if !nocase {
            return unit;
        }
        let version = match self {
            Self::Units(_, version) | Self::CString(_, version) => version,
        };
        crate::native_tcl_case::lower(version, unit)
    }
}

fn decode_c_unit(
    version: TclVersion,
    bytes: &[u8],
    previous: Option<u32>,
    direct: bool,
    c_string: bool,
) -> TclUtfUnit {
    let mut unit = NativeTclUtf::for_version(version)
        .decode_unit(bytes, previous)
        .unwrap_or(TclUtfUnit { value: 0, width: 1 });
    if !direct
        && version <= TclVersion::V8_5
        && bytes
            .first()
            .is_some_and(|byte| (0x80..0xc0).contains(byte))
    {
        unit.value |= 0xff00;
    }
    if c_string && version == TclVersion::V8_6 && (0xd800..0xdc00).contains(&unit.value) {
        let next = NativeTclUtf::for_version(version).decode_unit(
            bytes.get(unit.width..).unwrap_or_default(),
            Some(unit.value),
        );
        if let Some(next) = next
            && (0xdc00..0xe000).contains(&next.value)
        {
            unit.value = 0x10000 + ((unit.value & 0x3ff) << 10) + (next.value & 0x3ff);
            unit.width += next.width;
        }
    }
    unit
}

fn c_match(pattern: GlobInput<'_>, subject: GlobInput<'_>, nocase: bool) -> bool {
    let mut pending = vec![(0, 0)];
    let mut visited = HashSet::new();
    while let Some((mut pi, mut si)) = pending.pop() {
        loop {
            if !visited.insert((pi, si)) {
                break;
            }
            if pi == pattern.len() {
                if si == subject.len() {
                    return true;
                }
                break;
            }
            let token = pattern.raw(pi);
            if token == u32::from(b'*') {
                while pi < pattern.len() && pattern.raw(pi) == u32::from(b'*') {
                    pi += 1;
                }
                if pi == pattern.len() {
                    return true;
                }
                schedule_star(&mut pending, pattern, subject, pi, si, nocase);
                break;
            }
            if si == subject.len() {
                break;
            }
            if token == u32::from(b'?') {
                pi += 1;
                si += subject.unit(si, false).width;
                continue;
            }
            if token == u32::from(b'[') {
                let Some((next_pattern, next_subject)) =
                    match_class(pattern, subject, pi + 1, si, nocase)
                else {
                    break;
                };
                pi = next_pattern;
                si = next_subject;
                continue;
            }
            if token == u32::from(b'\\') {
                pi += 1;
                if pi == pattern.len() {
                    break;
                }
            }
            let p = pattern.unit(pi, false);
            let s = subject.unit(si, false);
            if pattern.fold(p.value, nocase) != subject.fold(s.value, nocase) {
                break;
            }
            pi += p.width;
            si += s.width;
        }
    }
    false
}

fn schedule_star(
    pending: &mut Vec<(usize, usize)>,
    pattern: GlobInput<'_>,
    subject: GlobInput<'_>,
    pi: usize,
    mut si: usize,
    nocase: bool,
) {
    let token = pattern.raw(pi);
    let special = [u32::from(b'['), u32::from(b'?'), u32::from(b'\\')].contains(&token);
    let next = pattern.fold(pattern.unit(pi, true).value, nocase);
    let mut positions = Vec::new();
    loop {
        let current = subject.unit(si, false);
        if special
            || si == subject.len()
            || next == current.value
            || next == subject.fold(current.value, nocase)
        {
            positions.push((pi, si));
        }
        if si == subject.len() {
            break;
        }
        si += current.width;
    }
    pending.extend(positions.into_iter().rev());
}

fn match_class(
    pattern: GlobInput<'_>,
    subject: GlobInput<'_>,
    mut pi: usize,
    si: usize,
    nocase: bool,
) -> Option<(usize, usize)> {
    let unit = subject.unit(si, true);
    let character = subject.fold(unit.value, nocase);
    let next_subject = si + unit.width;
    loop {
        if pi == pattern.len() || pattern.raw(pi) == u32::from(b']') {
            return None;
        }
        let first = pattern.unit(pi, true);
        let start = pattern.fold(first.value, nocase);
        pi += first.width;
        let matched = if pi < pattern.len() && pattern.raw(pi) == u32::from(b'-') {
            pi += 1;
            if pi == pattern.len() {
                return None;
            }
            let last = pattern.unit(pi, true);
            let end = pattern.fold(last.value, nocase);
            pi += last.width;
            (start <= character && character <= end) || (end <= character && character <= start)
        } else {
            start == character
        };
        if matched {
            break;
        }
    }
    while pi < pattern.len() && pattern.raw(pi) != u32::from(b']') {
        pi += 1;
    }
    let next_pattern = if pi < pattern.len() {
        pi + 1
    } else if let GlobInput::CString(bytes, version @ (TclVersion::V8_4 | TclVersion::V8_5)) =
        pattern
    {
        NativeTclUtf::for_version(version).previous_character_boundary(bytes, pi)? + 1
    } else {
        pi
    };
    Some((next_pattern, next_subject))
}

#[cfg(test)]
#[path = "native_glob_fixtures.rs"]
mod fixtures;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn c_string_entry_points_preserve_native_switch_extents_and_opaque_units() {
        for version in [
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            assert!(equal_c_strings(version, b"X\0left", b"X\0right", false));
            assert!(equal_c_strings(version, b"\xff", b"\xff", false));
            assert!(!equal_c_strings(version, b"\xff", b"\xfe", false));
            assert!(equal_c_strings(
                version,
                "É\0left".as_bytes(),
                "é\0right".as_bytes(),
                true
            ));
            assert!(match_c_string_glob(
                version,
                b"?\0ignored",
                b"\xff\0tail",
                false
            ));
            assert!(!match_c_string_glob(version, b"?", b"XX\0tail", false));
        }
    }

    const C_RECIPES: &[(TclVersion, &[u8], &[u8])] = &[
        (
            TclVersion::V8_4,
            fixtures::C84_OBJECT,
            fixtures::C84_CSTRING,
        ),
        (
            TclVersion::V8_5,
            fixtures::C85_OBJECT,
            fixtures::C85_CSTRING,
        ),
        (
            TclVersion::V8_6,
            fixtures::C86_OBJECT,
            fixtures::C86_CSTRING,
        ),
        (
            TclVersion::V9_0,
            fixtures::C90_OBJECT,
            fixtures::C90_CSTRING,
        ),
        (
            TclVersion::V9_1,
            fixtures::C91_OBJECT,
            fixtures::C91_CSTRING,
        ),
    ];

    fn expected(bits: &[u8], index: usize) -> bool {
        bits[index / 8] & (1 << (index % 8)) != 0
    }

    #[test]
    fn counted_objects_follow_fixed_native_storage_and_case_results() {
        for &(version, bits, _) in C_RECIPES {
            for storage in 0..4 {
                check_storage(version, bits, storage);
            }
        }
    }

    fn check_storage(version: TclVersion, bits: &[u8], storage: usize) {
        for (pi, &pattern) in fixtures::PATTERNS.iter().enumerate() {
            for (si, &subject) in fixtures::SUBJECTS.iter().enumerate() {
                let pu = object_unicode(version, NativeGlobObject::FreshString(pattern)).unwrap();
                let su = object_unicode(version, NativeGlobObject::FreshString(subject)).unwrap();
                let pattern_ba =
                    object_byte_array(version, NativeGlobObject::FreshString(pattern)).unwrap();
                let (p, s) = storage_inputs(storage, pattern, subject, &pu, &su);
                let index =
                    ((storage * fixtures::PATTERNS.len() + pi) * fixtures::SUBJECTS.len() + si) * 2;
                let result =
                    match_native_glob_objects(NativeNameProtocol::C(version), p, s, false).unwrap();
                assert_eq!(
                    result,
                    expected(bits, index),
                    "{version:?} storage{storage} p{pi}/s{si}"
                );
                // The native fixture invokes nocase on the same objects after
                // case-sensitive matching. C8.5 converts a mixed pattern to BA.
                let p = if version == TclVersion::V8_5 && storage == 3 {
                    NativeGlobObject::ByteArrayWithString {
                        bytes: &pattern_ba,
                        resident_bytes: pattern,
                    }
                } else {
                    p
                };
                let result =
                    match_native_glob_objects(NativeNameProtocol::C(version), p, s, true).unwrap();
                assert_eq!(
                    result,
                    expected(bits, index + 1),
                    "{version:?} nocase storage{storage} p{pi}/s{si}"
                );
            }
        }
    }

    fn storage_inputs<'a>(
        storage: usize,
        pattern: &'a [u8],
        subject: &'a [u8],
        pu: &'a [u32],
        su: &'a [u32],
    ) -> (NativeGlobObject<'a>, NativeGlobObject<'a>) {
        match storage {
            0 => (
                NativeGlobObject::FreshString(pattern),
                NativeGlobObject::FreshString(subject),
            ),
            1 => (
                NativeGlobObject::CachedUnicode {
                    units: pu,
                    resident_bytes: Some(pattern),
                },
                NativeGlobObject::CachedUnicode {
                    units: su,
                    resident_bytes: Some(subject),
                },
            ),
            2 => (
                NativeGlobObject::PureByteArray(pattern),
                NativeGlobObject::PureByteArray(subject),
            ),
            3 => (
                NativeGlobObject::FreshString(pattern),
                NativeGlobObject::PureByteArray(subject),
            ),
            _ => unreachable!(),
        }
    }

    #[test]
    fn cstring_scans_follow_fixed_native_units_including_invalid_bytes() {
        for &(version, _, bits) in C_RECIPES {
            for (pi, &pattern) in fixtures::PATTERNS.iter().enumerate() {
                for (si, &subject) in fixtures::SUBJECTS.iter().enumerate() {
                    let result = match_native_name_pattern(
                        NativeNameProtocol::C(version),
                        NativeNameGlobPurpose::ExportFilter,
                        pattern,
                        subject,
                    )
                    .unwrap();
                    assert_eq!(
                        result,
                        expected(bits, pi * fixtures::SUBJECTS.len() + si),
                        "{version:?} CString p{pi}/s{si}"
                    );
                    assert_eq!(
                        match_c_string_glob(version, pattern, subject, false),
                        expected(bits, pi * fixtures::SUBJECTS.len() + si),
                        "{version:?} CString entry p{pi}/s{si}"
                    );
                }
            }
        }
    }

    #[test]
    fn jim_objects_follow_fixed_numeric_unit_and_byte_extent_results() {
        for (pi, &pattern) in fixtures::PATTERNS.iter().enumerate() {
            for (si, &subject) in fixtures::SUBJECTS.iter().enumerate() {
                for nocase in [false, true] {
                    let result = match_native_glob_objects(
                        NativeNameProtocol::Jim084,
                        NativeGlobObject::FreshString(pattern),
                        NativeGlobObject::FreshString(subject),
                        nocase,
                    );
                    // JimGlobMatch decrements a zero slen after reading its
                    // owned terminator, then reads unrelated allocation bytes.
                    // The native result bit remains recorded, but this object
                    // receipt does not retain those bytes and must refuse.
                    if si == 0 && [2, 3, 4, 19].contains(&pi) {
                        assert_eq!(
                            result,
                            Err(NativeGlobUnavailable::JimAccess(
                                NativeStringAccessError::SeekBeyondStorage {
                                    character_index: 1,
                                    byte_offset: 1,
                                    byte_length: 0
                                }
                            ))
                        );
                        continue;
                    }
                    let result = result.unwrap();
                    let index = (pi * fixtures::SUBJECTS.len() + si) * 2 + usize::from(nocase);
                    assert_eq!(
                        result,
                        expected(fixtures::JIM_OBJECT, index),
                        "Jim p{pi}/s{si} nocase={nocase}"
                    );
                }
            }
        }
    }

    #[test]
    fn native_namespace_exact_lookup_is_separate_from_glob_equivalence() {
        for &(version, _, _) in C_RECIPES {
            let protocol = NativeNameProtocol::C(version);
            assert!(
                match_native_name_pattern(
                    protocol,
                    NativeNameGlobPurpose::ExportFilter,
                    &[0xff],
                    &[0xc3, 0xbf]
                )
                .unwrap()
            );
            assert!(
                !match_native_name_pattern(
                    protocol,
                    NativeNameGlobPurpose::InfoCommandsSearch,
                    &[0xff],
                    &[0xc3, 0xbf]
                )
                .unwrap()
            );
            assert_eq!(
                match_native_name_pattern(
                    protocol,
                    NativeNameGlobPurpose::ImportSearch,
                    &[0xff],
                    &[0xc3, 0xbf]
                )
                .unwrap(),
                version == TclVersion::V8_4
            );
            assert!(
                match_native_name_pattern(
                    protocol,
                    NativeNameGlobPurpose::InfoCommandsSearch,
                    b"[\xff]",
                    &[0xc3, 0xbf]
                )
                .unwrap()
            );
        }
        assert!(
            match_native_name_pattern(
                NativeNameProtocol::Jim084,
                NativeNameGlobPurpose::ExportFilter,
                b"A",
                &[0xff]
            )
            .unwrap()
        );
        assert!(
            match_native_name_pattern(
                NativeNameProtocol::Jim084,
                NativeNameGlobPurpose::InfoCommandsSearch,
                b"x?Z",
                b"x\0Z"
            )
            .unwrap()
        );
        assert!(
            !match_native_name_pattern(
                NativeNameProtocol::Jim084,
                NativeNameGlobPurpose::ImportSearch,
                b"x?Z",
                b"x\0Z"
            )
            .unwrap()
        );
    }

    #[test]
    fn array_trivial_cstring_test_retains_the_original_object_key() {
        for &(version, _, _) in C_RECIPES {
            let protocol = NativeNameProtocol::C(version);
            assert_eq!(
                match_native_name_pattern(
                    protocol,
                    NativeNameGlobPurpose::ArrayNamesSearch,
                    b"k\0*",
                    b"k\0z"
                )
                .unwrap(),
                version == TclVersion::V8_4
            );
            assert!(
                match_native_name_pattern(
                    protocol,
                    NativeNameGlobPurpose::ArrayNamesSearch,
                    b"k\0z",
                    b"k\0z"
                )
                .unwrap()
            );
            assert!(
                match_native_name_pattern(
                    protocol,
                    NativeNameGlobPurpose::InfoVariablesScan,
                    b"k\0*",
                    b"k\0z"
                )
                .unwrap()
            );
        }
    }

    #[test]
    fn absent_object_storage_refuses_and_cached_units_are_not_redecoded() {
        let protocol = NativeGlobProtocol::authored_tcl(TclVersion::V8_5);
        assert_eq!(
            protocol.authority(),
            NamePolicyAuthority::AuthoredSimulation
        );
        assert_eq!(
            protocol.match_objects(
                NativeGlobObject::FreshString(b"*"),
                NativeGlobObject::Unknown,
                false
            ),
            Err(NativeGlobUnavailable::ObjectRepresentationUnavailable)
        );
        let cached = NativeGlobObject::CachedUnicode {
            units: &[0],
            resident_bytes: Some(b""),
        };
        assert!(
            protocol
                .match_objects(NativeGlobObject::FreshString(b"?"), cached, false)
                .unwrap()
        );
        assert!(
            !protocol
                .match_objects(
                    NativeGlobObject::FreshString(b"?"),
                    NativeGlobObject::FreshString(&[0]),
                    false
                )
                .unwrap()
        );
        assert_eq!(
            protocol.match_objects(
                NativeGlobObject::FreshString(b"*"),
                NativeGlobObject::CachedUnicode {
                    units: &[0x10000],
                    resident_bytes: None
                },
                false
            ),
            Err(NativeGlobUnavailable::InvalidUnicodeUnit)
        );
    }
}

#[cfg(test)]
mod boundary_tests {
    use super::*;

    #[test]
    fn namespace_children_selects_original_physical_lookup_per_release() {
        let parent = b"::raw\xff";
        let child = b"::raw\xff::child\xfd";
        assert_eq!(
            namespace_children_lookup(NativeNameProtocol::C(TclVersion::V8_4), parent, child),
            NativeNamespaceChildrenLookup::Scan
        );
        assert_eq!(
            namespace_children_lookup(NativeNameProtocol::C(TclVersion::V8_5), parent, child),
            NativeNamespaceChildrenLookup::ChildKey(Some(b"::child\xfd"))
        );
        assert_eq!(
            namespace_children_lookup(
                NativeNameProtocol::C(TclVersion::V8_5),
                b"::",
                b"::child\xfd"
            ),
            NativeNamespaceChildrenLookup::ChildKey(Some(b"child\xfd"))
        );
        assert_eq!(
            namespace_children_lookup(
                NativeNameProtocol::C(TclVersion::V8_5),
                parent,
                b"::elsewhere"
            ),
            NativeNamespaceChildrenLookup::ChildKey(None)
        );
        for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
            assert_eq!(
                namespace_children_lookup(NativeNameProtocol::C(version), parent, child),
                NativeNamespaceChildrenLookup::FullName(child)
            );
        }
        for version in [
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            assert_eq!(
                namespace_children_lookup(NativeNameProtocol::C(version), parent, b"::raw\xff::*"),
                NativeNamespaceChildrenLookup::Scan
            );
        }
    }

    #[test]
    fn legacy_unclosed_multibyte_class_keeps_native_cstring_pointer_extent() {
        for version in [
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            assert_eq!(
                match_native_name_pattern(
                    NativeNameProtocol::C(version),
                    NativeNameGlobPurpose::ExportFilter,
                    b"[\xc3\xbf",
                    b"\xc3\xbf"
                )
                .unwrap(),
                version >= TclVersion::V8_6
            );
            assert!(
                match_native_glob_objects(
                    NativeNameProtocol::C(version),
                    NativeGlobObject::CachedUnicode {
                        units: &[91, 255],
                        resident_bytes: None
                    },
                    NativeGlobObject::CachedUnicode {
                        units: &[255],
                        resident_bytes: None
                    },
                    false
                )
                .unwrap()
            );
        }
    }

    #[test]
    fn cstring_path_requires_actual_resident_cache_materialisation() {
        let protocol = NativeGlobProtocol::authored_tcl(TclVersion::V8_6);
        assert_eq!(
            protocol.match_objects(
                NativeGlobObject::CachedUnicode {
                    units: &[0xd83d, 0xde00],
                    resident_bytes: None
                },
                NativeGlobObject::OtherString(b"TEXT"),
                false
            ),
            Err(NativeGlobUnavailable::ObjectRepresentationUnavailable)
        );
    }
}

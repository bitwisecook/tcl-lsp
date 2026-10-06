// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Immutable byte-preserving string storage with a checked Unicode projection.
//!
//! Native Jim strings may contain invalid UTF8 and surrogate encodings. The
//! bytes remain the source of truth; asking for Unicode never substitutes data.
//! Jim character iteration follows the measured 0.84 `utf8_tounicode` protocol,
//! and must be selected by a consumer with that actual interpreter contract.
use std::{cell::OnceCell, rc::Rc};
use tcl_dialect::StringCharacterModel;

#[path = "jim084_case_mapping.rs"]
mod jim084_case_mapping;

/// The selected simple numeric-unit case operation in pinned Jim084.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JimCaseMapping {
    /// Simple upper case, retaining supplementary units unchanged.
    Upper,
    /// Simple lower case, retaining supplementary units unchanged.
    Lower,
    /// Title case for the first unit and lower case for following units.
    Title,
}

fn jim084_map_case(codepoint: u32, mapping: &[(u16, u16)]) -> u32 {
    let Ok(code) = u16::try_from(codepoint) else {
        return codepoint;
    };
    mapping
        .binary_search_by_key(&code, |&(source, _)| source)
        .map_or(codepoint, |index| u32::from(mapping[index].1))
}

/// Apply the pinned Jim084 numeric-unit upper-case table.
#[must_use]
pub fn jim084_upper(codepoint: u32) -> u32 {
    if (u32::from(b'a')..=u32::from(b'z')).contains(&codepoint) {
        codepoint - 32
    } else {
        jim084_map_case(codepoint, jim084_case_mapping::UPPER)
    }
}

fn jim084_lower(codepoint: u32) -> u32 {
    if (u32::from(b'A')..=u32::from(b'Z')).contains(&codepoint) {
        codepoint + 32
    } else {
        jim084_map_case(codepoint, jim084_case_mapping::LOWER)
    }
}

fn jim084_title(codepoint: u32) -> u32 {
    let title = jim084_map_case(codepoint, jim084_case_mapping::TITLE);
    if title == codepoint {
        jim084_upper(codepoint)
    } else if title == 0 {
        codepoint
    } else {
        title
    }
}

/// Decode one original Jim UTF8 numeric unit and its counted byte width.
/// This selects the pinned Jim084 decoder, including its invalid-byte rules.
#[must_use]
pub fn jim084_decode(bytes: &[u8]) -> Option<(u32, usize)> {
    let first = *bytes.first()?;
    let width = match first {
        0xc0..=0xdf => 2,
        0xe0..=0xef => 3,
        0xf0..=0xf7 => 4,
        _ => 1,
    };
    if width > 1
        && bytes
            .get(..width)
            .is_some_and(|part| part[1..].iter().all(|byte| byte & 0xc0 == 0x80))
    {
        let mut codepoint = u32::from(first & (0x7f >> width));
        for byte in &bytes[1..width] {
            codepoint = (codepoint << 6) | u32::from(byte & 0x3f);
        }
        let minimum = match width {
            2 => 0x80,
            3 => 0x800,
            _ => 0x10000,
        };
        if codepoint >= minimum {
            return Some((codepoint, width));
        }
    }
    Some((u32::from(first), 1))
}

/// Exact byte cuts from the pinned Jim trim algorithm. Physical object sharing
/// and cached counts are applied separately by the concrete value owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JimStringTrimPlan {
    byte_start: usize,
    byte_end: usize,
    right_conversion: bool,
}

impl JimStringTrimPlan {
    /// Inclusive start of the retained original byte range.
    #[must_use]
    pub const fn byte_start(self) -> usize {
        self.byte_start
    }

    /// Exclusive end of the retained original byte range.
    #[must_use]
    pub const fn byte_end(self) -> usize {
        self.byte_end
    }

    /// Whether native trimright requires a string-intrep conversion.
    #[must_use]
    pub const fn right_conversion(self) -> bool {
        self.right_conversion
    }
}

/// A shared immutable string representation whose bytes need not be Unicode.
#[derive(Clone, Debug)]
pub struct RawString(Rc<RawStringData>);
#[derive(Debug)]
struct RawStringData {
    bytes: Rc<[u8]>,
    unicode: OnceCell<Result<Rc<str>, UnicodeAccessError>>,
}
/// Why a byte string cannot enter a consumer requiring valid Unicode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UnicodeAccessError {
    /// End of the valid UTF8 prefix.
    pub valid_up_to: usize,
    /// Invalid sequence length, or `None` for an incomplete final sequence.
    pub error_len: Option<usize>,
}
impl std::fmt::Display for UnicodeAccessError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "native string bytes cannot be represented as host Unicode at byte {}",
            self.valid_up_to
        )
    }
}

impl std::error::Error for UnicodeAccessError {}

/// An eager host backend cannot retain the requested native value size.
/// This capacity limit is independent of the guest interpreter's lazy values.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeMaterializationLimitError {
    requested_low: u64,
    requested_high: u64,
    limit: u64,
}

impl NativeMaterializationLimitError {
    /// Retain an exact count without increasing the alignment of shared errors.
    #[must_use]
    pub const fn new(requested: u128, limit: u64) -> Self {
        let bytes = requested.to_le_bytes();
        Self {
            requested_low: u64::from_le_bytes([
                bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
            ]),
            requested_high: u64::from_le_bytes([
                bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13], bytes[14],
                bytes[15],
            ]),
            limit,
        }
    }

    /// Element count requested by the backend's construction recipe.
    #[must_use]
    pub fn requested(self) -> u128 {
        u128::from(self.requested_high) << 64 | u128::from(self.requested_low)
    }

    /// Maximum element count supported by this backend.
    #[must_use]
    pub const fn limit(self) -> u64 {
        self.limit
    }
}

impl std::fmt::Display for NativeMaterializationLimitError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "host cannot materialize {} elements; backend limit is {}",
            self.requested(),
            self.limit()
        )
    }
}
impl std::error::Error for NativeMaterializationLimitError {}

/// A reached native operation would terminate the actual interpreter process.
/// Hosts preserve this boundary as an operational refusal outside guest catch.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeFatalCondition {
    /// A dictionary search advanced after its actual backing was mutated.
    DictionarySearchConcurrentMutation,
    /// Tcl package source inventory attempted to append to a shared List.
    SharedPackageFileListMutation,
    /// Jim's `CString` dictionary-sugar search cannot produce a valid key pointer.
    JimDictionarySubstitutionGeometry,
    /// Native Script conversion recursively re-enters a nonzero Subst cache.
    JimSubstitutionScriptReentry,
}

impl std::fmt::Display for NativeFatalCondition {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::SharedPackageFileListMutation => {
                formatter.write_str("TclListObjAppendElements called with shared object")
            }
            Self::DictionarySearchConcurrentMutation => formatter.write_str(
                "native dictionary search would abort after concurrent backing mutation",
            ),
            Self::JimDictionarySubstitutionGeometry => formatter.write_str(
                "native Jim dictionary substitution would dereference invalid name geometry",
            ),
            Self::JimSubstitutionScriptReentry => formatter.write_str(
                "native Jim Script conversion recursively re-enters substitution storage",
            ),
        }
    }
}
impl std::error::Error for NativeFatalCondition {}

/// A value access that the host cannot execute faithfully. This operational
/// channel is separate from native guest coercion errors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeValueAccessRefusal {
    /// Native fatal behavior cannot be converted to a guest completion.
    FatalCondition(NativeFatalCondition),
    /// Checked host Unicode projection of actual byte storage failed.
    Unicode(UnicodeAccessError),
    /// Native character access exceeded the retained object storage.
    StringAccess(NativeStringAccessError),
    /// Eager construction exceeds a host backend capacity, not a guest limit.
    Materialization(NativeMaterializationLimitError),
    /// This backend has no expression evaluator for a reached substitution.
    ExpressionEngineUnavailable,
    /// A native character operation lacks a selected character-unit model.
    CharacterModelUnavailable,
    /// A scalar numeric getter has no selected actual-engine input extent.
    ScalarNumericInputUnavailable,
    /// A reached native command lacks an audited selected handler protocol.
    CommandProtocolUnavailable(&'static str),
}

impl std::fmt::Display for NativeValueAccessRefusal {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::FatalCondition(error) => error.fmt(formatter),
            Self::Unicode(error) => error.fmt(formatter),
            Self::StringAccess(error) => error.fmt(formatter),
            Self::Materialization(error) => error.fmt(formatter),
            Self::ExpressionEngineUnavailable => {
                formatter.write_str("host expression evaluator is unavailable")
            }
            Self::ScalarNumericInputUnavailable => {
                formatter.write_str("native scalar numeric input policy is not selected")
            }
            Self::CharacterModelUnavailable => {
                formatter.write_str("native character model is not selected")
            }
            Self::CommandProtocolUnavailable(command) => {
                write!(formatter, "{command} native handler policy is not selected")
            }
        }
    }
}
impl std::error::Error for NativeValueAccessRefusal {}
impl From<UnicodeAccessError> for NativeValueAccessRefusal {
    fn from(error: UnicodeAccessError) -> Self {
        Self::Unicode(error)
    }
}
impl From<NativeStringAccessError> for NativeValueAccessRefusal {
    fn from(error: NativeStringAccessError) -> Self {
        Self::StringAccess(error)
    }
}
impl From<NativeMaterializationLimitError> for NativeValueAccessRefusal {
    fn from(error: NativeMaterializationLimitError) -> Self {
        Self::Materialization(error)
    }
}

/// An actual native string operation requests bytes outside the retained
/// object storage. The native terminating NUL is owned; unrelated allocation
/// contents cannot be manufactured as a guest value.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeStringAccessError {
    /// The original native object cannot supply the selected string representation.
    Unavailable(crate::native_string::NativeStringUnavailable),
    /// A leading-byte character seek has passed the retained byte extent.
    SeekBeyondStorage {
        /// Requested native character position.
        character_index: usize,
        /// Native seek position within the byte storage.
        byte_offset: usize,
        /// Retained string length, excluding its terminating NUL.
        byte_length: usize,
    },
    /// A native copy would include bytes past the retained terminating NUL.
    CopyBeyondStorage {
        /// Inclusive native byte start.
        byte_start: usize,
        /// Exclusive requested native byte end.
        byte_end: usize,
        /// Retained string length, excluding its terminating NUL.
        byte_length: usize,
    },
}

impl std::fmt::Display for NativeStringAccessError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unavailable(error) => error.fmt(formatter),
            Self::SeekBeyondStorage {
                character_index,
                byte_offset,
                byte_length,
            } => write!(
                formatter,
                "native character {character_index} seeks to byte {byte_offset} beyond retained string length {byte_length}"
            ),
            Self::CopyBeyondStorage {
                byte_start,
                byte_end,
                byte_length,
            } => write!(
                formatter,
                "native string copy {byte_start}..{byte_end} exceeds retained string length {byte_length} and its terminating NUL"
            ),
        }
    }
}

impl std::error::Error for NativeStringAccessError {}

/// One measured Jim084 character, retaining its exact original byte extent.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JimCharacter {
    /// Decoded numeric unit, which may be a surrogate or an invalid raw byte.
    pub codepoint: u32,
    /// Inclusive original byte offset.
    pub start: usize,
    /// Exclusive original byte offset.
    pub end: usize,
}

#[derive(Clone, Copy, Default)]
struct JimGlobPosition {
    pattern: usize,
    subject: usize,
    character: usize,
}

enum JimGlobFrame {
    Match(JimGlobPosition),
    AdvanceStar(JimGlobPosition),
}

enum JimGlobStep {
    Done(bool),
    Star(JimGlobPosition),
}

fn schedule_jim_glob_star(
    frames: &mut Vec<JimGlobFrame>,
    position: JimGlobPosition,
    byte_length: usize,
) {
    // Native '*' tries the remaining pattern only while its subject byte
    // extent is nonzero. In particular it does not try an empty suffix.
    if position.subject != byte_length {
        frames.push(JimGlobFrame::AdvanceStar(position));
        frames.push(JimGlobFrame::Match(position));
    }
}

impl RawString {
    /// Compute pinned Jim trim cuts without cloning or coercing the value.
    /// Backward scanning uses native start-byte rules, so unmatched trailing
    /// continuation bytes can disappear even with an empty trim set.
    #[must_use]
    pub fn jim084_trim_plan(
        &self,
        characters: &Self,
        left: bool,
        right: bool,
    ) -> JimStringTrimPlan {
        let bytes = self.bytes();
        let contains = |codepoint| {
            characters
                .jim084_characters()
                .any(|character| character.codepoint == codepoint)
        };
        let mut start = 0;
        if left {
            while start < bytes.len() {
                let (codepoint, width) = jim084_decode(&bytes[start..]).expect("owned byte");
                if !contains(codepoint) {
                    break;
                }
                start += width;
            }
        }
        let mut end = bytes.len();
        if right {
            while end > start {
                let mut candidate = end - 1;
                while candidate > start
                    && bytes[candidate] & 0x80 != 0
                    && bytes[candidate] & 0xc0 != 0xc0
                {
                    candidate -= 1;
                }
                // Native forward decode sees the full retained suffix, even
                // after the backwards scan has shortened its search extent.
                let (codepoint, width) = jim084_decode(&bytes[candidate..]).expect("owned byte");
                if !contains(codepoint) {
                    end = candidate + width;
                    break;
                }
                end = candidate;
            }
        }
        JimStringTrimPlan {
            byte_start: start,
            byte_end: end,
            right_conversion: right,
        }
    }

    /// Pinned Jim084 glob matching over actual bytes and decoded numeric units.
    /// Its star search, bracket grammar and owned terminating NUL are distinct
    /// from Unicode globbing and from Jim's fast byte equality operation.
    ///
    /// # Errors
    /// Refuses a reached decode beyond retained bytes and their terminating NUL.
    pub fn jim084_matches(
        &self,
        pattern: &Self,
        nocase: bool,
    ) -> Result<bool, NativeStringAccessError> {
        let mut frames = vec![JimGlobFrame::Match(JimGlobPosition::default())];
        while let Some(frame) = frames.pop() {
            match frame {
                JimGlobFrame::Match(position) => {
                    match self.jim084_glob_step(pattern, position, nocase)? {
                        JimGlobStep::Done(true) => return Ok(true),
                        JimGlobStep::Done(false) => {}
                        JimGlobStep::Star(position) => {
                            schedule_jim_glob_star(&mut frames, position, self.0.bytes.len());
                        }
                    }
                }
                JimGlobFrame::AdvanceStar(mut position) => {
                    let unit = self.jim084_character_at(position.subject, position.character)?;
                    position.subject = unit.end;
                    position.character += 1;
                    schedule_jim_glob_star(&mut frames, position, self.0.bytes.len());
                }
            }
        }
        Ok(false)
    }

    fn jim084_glob_step(
        &self,
        pattern: &Self,
        mut position: JimGlobPosition,
        nocase: bool,
    ) -> Result<JimGlobStep, NativeStringAccessError> {
        let pattern_bytes = &pattern.0.bytes;
        while let Some(&head) = pattern_bytes.get(position.pattern) {
            if head == b'*' {
                while pattern_bytes.get(position.pattern) == Some(&b'*') {
                    position.pattern += 1;
                }
                return Ok(if position.pattern == pattern_bytes.len() {
                    JimGlobStep::Done(true)
                } else {
                    JimGlobStep::Star(position)
                });
            }
            let unit = self.jim084_character_at(position.subject, position.character)?;
            position.subject = unit.end;
            position.character += 1;
            if head == b'[' {
                let Some(end) =
                    pattern.jim084_charset_end(position.pattern + 1, unit.codepoint, nocase)
                else {
                    return Ok(JimGlobStep::Done(false));
                };
                position.pattern = end;
                if end == pattern_bytes.len() {
                    continue;
                }
            } else if head != b'?' {
                if head == b'\\'
                    && pattern_bytes
                        .get(position.pattern + 1)
                        .is_some_and(|byte| *byte != 0)
                {
                    position.pattern += 1;
                }
                let needle = pattern.jim084_character_at(position.pattern, 0)?;
                let fold = |code| if nocase { jim084_upper(code) } else { code };
                if fold(needle.codepoint) != fold(unit.codepoint) {
                    return Ok(JimGlobStep::Done(false));
                }
            }
            position.pattern = pattern.jim084_character_at(position.pattern, 0)?.end;
            if position.subject == self.0.bytes.len() {
                while pattern_bytes.get(position.pattern) == Some(&b'*') {
                    position.pattern += 1;
                }
                break;
            }
        }
        Ok(JimGlobStep::Done(
            position.pattern == pattern_bytes.len() && position.subject == self.0.bytes.len(),
        ))
    }

    fn jim084_charset_end(&self, mut offset: usize, subject: u32, nocase: bool) -> Option<usize> {
        let fold = |code| if nocase { jim084_upper(code) } else { code };
        let subject = fold(subject);
        let mut matched = false;
        while let Some(&head) = self.0.bytes.get(offset) {
            if head == b']' {
                break;
            }
            let (first, width) = jim084_decode(&self.0.bytes[offset..])?;
            offset += width;
            let first = fold(first);
            // Jim treats a backslash within the set as a literal component.
            if head != b'\\'
                && self.0.bytes.get(offset) == Some(&b'-')
                && self.0.bytes.len() - offset > 1
            {
                let (last, width) = jim084_decode(&self.0.bytes[offset + 1..])?;
                offset += width + 1;
                let last = fold(last);
                matched |= (first.min(last)..=first.max(last)).contains(&subject);
            } else {
                matched |= first == subject;
            }
        }
        matched.then_some(offset)
    }

    /// Find the first native byte match at decoded character starts, beginning
    /// with Jim's leading-byte seek and independently retained cached counts.
    ///
    /// # Errors
    /// Refuses a native seek or comparison beyond owned bytes and their NUL.
    pub fn jim084_first(
        &self,
        count: usize,
        needle: &Self,
        needle_count: usize,
        start: usize,
    ) -> Result<Option<usize>, NativeStringAccessError> {
        if needle_count == 0 || count == 0 || needle_count > count || start > count {
            return Ok(None);
        }
        let needle_end = needle.jim084_seek(0, needle_count, needle_count)?;
        let needle_bytes = needle.copy_owned_bytes(0, needle_end)?;
        let mut offset = self.jim084_seek(0, start, start)?;
        for index in start..=count - needle_count {
            if self.owned_window_matches(offset, &needle_bytes)? {
                return Ok(Some(index));
            }
            offset = self.jim084_character_at(offset, index)?.end;
        }
        Ok(None)
    }

    /// Search backwards at every byte within Jim's exclusive native prefix.
    /// A continuation-byte match is permitted; the returned position counts
    /// decoded units whose start precedes the actual matching byte.
    ///
    /// # Errors
    /// Refuses a native seek or comparison beyond owned bytes and their NUL.
    pub fn jim084_last(
        &self,
        prefix_count: usize,
        needle: &Self,
        needle_count: usize,
    ) -> Result<Option<usize>, NativeStringAccessError> {
        let needle_end = needle.jim084_seek(0, needle_count, needle_count)?;
        let end = self.jim084_seek(0, prefix_count, prefix_count)?;
        if needle_end == 0 || end == 0 || needle_end > end {
            return Ok(None);
        }
        let needle_bytes = needle.copy_owned_bytes(0, needle_end)?;
        for offset in (0..end).rev() {
            if self.owned_byte_at(offset, prefix_count)? == needle_bytes[0]
                && self.owned_window_matches(offset, &needle_bytes)?
            {
                return Ok(Some(
                    self.jim084_characters()
                        .take_while(|unit| unit.start < offset)
                        .count(),
                ));
            }
        }
        Ok(None)
    }

    fn owned_byte_at(
        &self,
        offset: usize,
        character_index: usize,
    ) -> Result<u8, NativeStringAccessError> {
        match self.0.bytes.get(offset) {
            Some(&byte) => Ok(byte),
            None if offset == self.0.bytes.len() => Ok(0),
            None => Err(NativeStringAccessError::SeekBeyondStorage {
                character_index,
                byte_offset: offset,
                byte_length: self.0.bytes.len(),
            }),
        }
    }

    fn jim084_character_at(
        &self,
        offset: usize,
        character_index: usize,
    ) -> Result<JimCharacter, NativeStringAccessError> {
        let Some(bytes) = self.0.bytes.get(offset..) else {
            return Err(NativeStringAccessError::SeekBeyondStorage {
                character_index,
                byte_offset: offset,
                byte_length: self.0.bytes.len(),
            });
        };
        let (codepoint, width) = jim084_decode(bytes).unwrap_or((0, 1));
        Ok(JimCharacter {
            codepoint,
            start: offset,
            end: offset + width,
        })
    }

    fn owned_window_matches(
        &self,
        start: usize,
        needle: &[u8],
    ) -> Result<bool, NativeStringAccessError> {
        let len = self.0.bytes.len();
        let end = start.saturating_add(needle.len());
        if start > len || end > len + 1 {
            return Err(NativeStringAccessError::CopyBeyondStorage {
                byte_start: start,
                byte_end: end,
                byte_length: len,
            });
        }
        let stored = &self.0.bytes[start..end.min(len)];
        Ok(stored == &needle[..stored.len()] && (end <= len || needle.last() == Some(&0)))
    }
    /// Compare selected native character units. Jim counts come from the
    /// original objects, including retained cached counts; C projections are
    /// checked Unicode and retain each selected release's unit model.
    ///
    /// # Errors
    /// Refuses an unrepresented Unicode projection or access beyond owned storage.
    pub fn compare_character_units(
        &self,
        model: StringCharacterModel,
        other: &Self,
        counts: (usize, usize),
    ) -> Result<std::cmp::Ordering, NativeValueAccessRefusal> {
        if model == StringCharacterModel::Jim084Utf8 {
            return self
                .jim084_compare(counts.0, other, counts.1, false)
                .map_err(Into::into);
        }
        let left = self.unicode()?;
        let right = other.unicode()?;
        Ok(match model {
            StringCharacterModel::Utf16CodeUnits => left.encode_utf16().cmp(right.encode_utf16()),
            StringCharacterModel::UnicodeScalars => left.chars().cmp(right.chars()),
            StringCharacterModel::BmpCharsElseUtf8Bytes => {
                let units = |character: char| {
                    let mut units = [0_u32; 4];
                    let count = if u32::from(character) <= 0xffff {
                        units[0] = u32::from(character);
                        1
                    } else {
                        let mut bytes = [0_u8; 4];
                        let bytes = character.encode_utf8(&mut bytes).as_bytes();
                        for (unit, byte) in units.iter_mut().zip(bytes) {
                            *unit = u32::from(*byte);
                        }
                        bytes.len()
                    };
                    units.into_iter().take(count)
                };
                left.chars()
                    .flat_map(units)
                    .cmp(right.chars().flat_map(units))
            }
            StringCharacterModel::Jim084Utf8 => unreachable!("Jim uses original counts above"),
        })
    }

    /// Compare actual numeric character units using separately retained native
    /// cached counts. Case-sensitive byte equality is a distinct Jim operation.
    ///
    /// # Errors
    /// Refuses access beyond the retained bytes and their terminating NUL.
    pub fn jim084_compare(
        &self,
        count: usize,
        other: &Self,
        other_count: usize,
        nocase: bool,
    ) -> Result<std::cmp::Ordering, NativeStringAccessError> {
        let terminating = |raw: &Self| JimCharacter {
            codepoint: 0,
            start: raw.0.bytes.len(),
            end: raw.0.bytes.len() + 1,
        };
        let mut left = self
            .jim084_characters()
            .chain(std::iter::once(terminating(self)));
        let mut right = other
            .jim084_characters()
            .chain(std::iter::once(terminating(other)));
        for index in 0..count.min(other_count) {
            let next = |unit: Option<JimCharacter>, raw: &Self| {
                unit.map(|unit| unit.codepoint)
                    .ok_or(NativeStringAccessError::SeekBeyondStorage {
                        character_index: index,
                        byte_offset: raw.0.bytes.len() + 1,
                        byte_length: raw.0.bytes.len(),
                    })
            };
            let mut a = next(left.next(), self)?;
            let mut b = next(right.next(), other)?;
            if nocase {
                a = jim084_upper(a);
                b = jim084_upper(b);
            }
            let ordering = a.cmp(&b);
            if !ordering.is_eq() {
                return Ok(ordering);
            }
        }
        Ok(count.cmp(&other_count))
    }

    /// Compare a Jim prefix against the original candidate using independently
    /// retained character counts. The temporary candidate receives the exact
    /// count supplied to the native UTF constructor.
    pub fn jim084_prefix_compare(
        &self,
        count: usize,
        other: &Self,
        other_count: usize,
    ) -> Result<std::cmp::Ordering, NativeStringAccessError> {
        if other_count <= count {
            return self.jim084_compare(count, other, other_count, false);
        }
        let end = other.jim084_seek(0, count, count)?;
        let temporary = Self::from_bytes(other.copy_owned_bytes(0, end)?);
        self.jim084_compare(count, &temporary, count, false)
    }

    /// Number of equal decoded Jim units at the start of two retained strings.
    /// This preserves native counts and refuses reads beyond owned storage.
    pub fn jim084_common_prefix_count(
        &self,
        count: usize,
        other: &Self,
        other_count: usize,
    ) -> Result<usize, NativeStringAccessError> {
        let (mut left, mut right) = (0, 0);
        for index in 0..count.min(other_count) {
            let a = self.jim084_character_at(left, index)?;
            let b = other.jim084_character_at(right, index)?;
            if a.codepoint != b.codepoint {
                return Ok(index);
            }
            left = a.end;
            right = b.end;
        }
        Ok(count.min(other_count))
    }

    /// Apply pinned Jim084's simple BMP case tables to numeric units. Native
    /// conversion stops at NUL and emits a fresh string, re-encoding malformed
    /// bytes instead of projecting them through host Unicode.
    #[must_use]
    pub fn jim084_case_mapped(&self, mapping: JimCaseMapping) -> Self {
        let mut bytes = Vec::with_capacity(self.0.bytes.len());
        for (index, unit) in self.jim084_characters().enumerate() {
            if unit.codepoint == 0 {
                break;
            }
            let mapped = match mapping {
                JimCaseMapping::Upper => jim084_upper(unit.codepoint),
                JimCaseMapping::Title if index == 0 => jim084_title(unit.codepoint),
                JimCaseMapping::Lower | JimCaseMapping::Title => jim084_lower(unit.codepoint),
            };
            tcl_lexer::encode_jim084_unicode(&mut bytes, mapped);
        }
        Self::from_bytes(bytes)
    }

    /// Count exact native character units under an independently selected model.
    /// C Unicode projections are checked; Jim retains its numeric byte units.
    pub fn character_count(
        &self,
        model: tcl_dialect::StringCharacterModel,
    ) -> Result<usize, UnicodeAccessError> {
        if model == tcl_dialect::StringCharacterModel::Jim084Utf8 {
            Ok(self.jim084_characters().count())
        } else {
            Ok(model.count(&self.unicode()?))
        }
    }

    /// Retain an already checked Unicode view alongside its exact encoded bytes.
    #[must_use]
    pub fn from_unicode(value: impl Into<Rc<str>>) -> Self {
        let value = value.into();
        Self(Rc::new(RawStringData {
            bytes: Rc::from(value.as_bytes()),
            unicode: OnceCell::from(Ok(value)),
        }))
    }
    /// Retain bytes without decoding or replacing any sequence.
    #[must_use]
    pub fn from_bytes(bytes: impl Into<Rc<[u8]>>) -> Self {
        Self(Rc::new(RawStringData {
            bytes: bytes.into(),
            unicode: OnceCell::new(),
        }))
    }
    /// Share the exact bytes without generating a Unicode representation.
    #[must_use]
    pub fn bytes(&self) -> Rc<[u8]> {
        Rc::clone(&self.0.bytes)
    }
    /// Cache a checked Unicode projection. Failure leaves original bytes intact.
    pub fn unicode(&self) -> Result<Rc<str>, UnicodeAccessError> {
        self.0
            .unicode
            .get_or_init(|| {
                std::str::from_utf8(&self.0.bytes)
                    .map(Rc::from)
                    .map_err(|error| UnicodeAccessError {
                        valid_up_to: error.valid_up_to(),
                        error_len: error.error_len(),
                    })
            })
            .clone()
    }
    /// Construct a new string while preserving every original and appended byte.
    #[must_use]
    pub fn appended(&self, suffix: &[u8]) -> Self {
        let mut bytes = self.0.bytes.to_vec();
        bytes.extend_from_slice(suffix);
        Self::from_bytes(bytes)
    }

    /// Original byte extent of the requested pinned Jim084 character. This
    /// retains invalid leading bytes and surrogate sequences without decoding
    /// the result into host Unicode.
    pub fn jim084_character_bytes(
        &self,
        index: usize,
        character_count: usize,
    ) -> Result<Vec<u8>, NativeStringAccessError> {
        if index >= character_count {
            return Ok(Vec::new());
        }
        if character_count == self.0.bytes.len() {
            return self.copy_owned_bytes(index, index + 1);
        }
        let start = self.jim084_seek(0, index, index)?;
        let bytes = self.copy_owned_bytes(start, self.0.bytes.len() + 1)?;
        let suffix = Self::from_bytes(bytes);
        let width = suffix.jim084_characters().next().map_or(0, |unit| unit.end);
        self.copy_owned_bytes(start, start + width)
    }

    /// Exact bytes of an inclusive pinned Jim084 character range. Out-of-range
    /// and reversed ranges are empty; the final character is clamped.
    pub fn jim084_range_bytes(
        &self,
        first: usize,
        last: usize,
        character_count: usize,
    ) -> Result<(Vec<u8>, usize), NativeStringAccessError> {
        if first > last || first >= character_count {
            return Ok((Vec::new(), 0));
        }
        let last = last.min(character_count - 1);
        let count = last - first + 1;
        if character_count == self.0.bytes.len() {
            return self
                .copy_owned_bytes(first, last + 1)
                .map(|bytes| (bytes, count));
        }
        let start = self.jim084_seek(0, first, first)?;
        let end = self.jim084_seek(start, count, last + 1)?;
        self.copy_owned_bytes(start, end)
            .map(|bytes| (bytes, count))
    }

    /// Original byte offset selected by Jim's leading-byte index traversal.
    /// An offset beyond the retained counted allocation is unavailable.
    pub fn jim084_byte_offset(&self, index: usize) -> Result<usize, NativeStringAccessError> {
        let offset = self.jim084_seek(0, index, index)?;
        if offset > self.0.bytes.len() {
            return Err(NativeStringAccessError::SeekBeyondStorage {
                character_index: index,
                byte_offset: offset,
                byte_length: self.0.bytes.len(),
            });
        }
        Ok(offset)
    }

    fn jim084_seek(
        &self,
        start: usize,
        count: usize,
        character_index: usize,
    ) -> Result<usize, NativeStringAccessError> {
        let mut offset = start;
        for _ in 0..count {
            let byte = match self.0.bytes.get(offset) {
                Some(byte) => *byte,
                None if offset == self.0.bytes.len() => 0,
                None => {
                    return Err(NativeStringAccessError::SeekBeyondStorage {
                        character_index,
                        byte_offset: offset,
                        byte_length: self.0.bytes.len(),
                    });
                }
            };
            // Native utf8_index uses utf8_charlen of the leading byte; it does
            // not validate continuation bytes as utf8_tounicode does.
            offset += match byte {
                0xc0..=0xdf => 2,
                0xe0..=0xef => 3,
                0xf0..=0xf7 => 4,
                _ => 1,
            };
        }
        Ok(offset)
    }

    fn copy_owned_bytes(
        &self,
        start: usize,
        end: usize,
    ) -> Result<Vec<u8>, NativeStringAccessError> {
        let len = self.0.bytes.len();
        if start > end || start > len || end > len + 1 {
            return Err(NativeStringAccessError::CopyBeyondStorage {
                byte_start: start,
                byte_end: end,
                byte_length: len,
            });
        }
        let mut bytes = self.0.bytes[start..end.min(len)].to_vec();
        if end > len {
            bytes.push(0);
        }
        Ok(bytes)
    }

    /// Reverse the original byte extents of pinned Jim084 character units.
    /// The numeric units are never re-encoded, so malformed bytes survive.
    #[must_use]
    pub fn jim084_reversed(&self) -> Self {
        let units: Vec<_> = self.jim084_characters().collect();
        let mut bytes = Vec::with_capacity(self.0.bytes.len());
        for unit in units.into_iter().rev() {
            bytes.extend_from_slice(&self.0.bytes[unit.start..unit.end]);
        }
        Self::from_bytes(bytes)
    }
    /// Iterate using pinned Jim084's native decoder. Invalid leading bytes are
    /// single units, valid surrogate encodings are retained, and overlong
    /// encodings consume only their original leading byte. This is not a
    /// protocol choice for unknown or C interpreters.
    pub fn jim084_characters(&self) -> impl Iterator<Item = JimCharacter> + '_ {
        let mut offset = 0;
        std::iter::from_fn(move || {
            let bytes = &self.0.bytes[offset..];
            let (codepoint, consumed) = jim084_decode(bytes)?;
            let start = offset;
            offset += consumed;
            Some(JimCharacter {
                codepoint,
                start,
                end: offset,
            })
        })
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_ordering_uses_the_selected_release_units_and_original_jim_counts() {
        use std::cmp::Ordering;
        let supplementary = RawString::from_unicode("😀");
        let bmp = RawString::from_unicode("\u{e000}");
        for (model, expected) in [
            (StringCharacterModel::BmpCharsElseUtf8Bytes, Ordering::Less),
            (StringCharacterModel::Utf16CodeUnits, Ordering::Less),
            (StringCharacterModel::UnicodeScalars, Ordering::Greater),
            (StringCharacterModel::Jim084Utf8, Ordering::Greater),
        ] {
            assert_eq!(
                supplementary.compare_character_units(model, &bmp, (1, 1)),
                Ok(expected)
            );
        }
        let raw = RawString::from_bytes([0xff]);
        let unicode = RawString::from_unicode("ÿ");
        assert_eq!(
            raw.compare_character_units(StringCharacterModel::Jim084Utf8, &unicode, (1, 1)),
            Ok(Ordering::Equal)
        );
        assert_ne!(raw.bytes(), unicode.bytes());
        assert!(
            raw.compare_character_units(StringCharacterModel::UnicodeScalars, &unicode, (1, 1))
                .is_err()
        );
        let cached = RawString::from_bytes([0xc3, b'A', 0xc3, 0xa9]);
        assert_eq!(
            cached.compare_character_units(StringCharacterModel::Jim084Utf8, &cached, (2, 3)),
            Ok(Ordering::Less)
        );
    }

    #[test]
    fn jim_glob_retains_numeric_units_and_the_owned_terminating_nul() {
        let empty = RawString::from_bytes([]);
        assert_eq!(
            empty.jim084_matches(&RawString::from_bytes(b"?*".as_slice()), false),
            Ok(true)
        );
        assert_eq!(
            empty.jim084_matches(&RawString::from_bytes(b"*?".as_slice()), false),
            Ok(false)
        );
        assert!(
            empty
                .jim084_matches(&RawString::from_bytes(b"??".as_slice()), false)
                .is_err()
        );
        let unicode = RawString::from_bytes([0xc3, 0xbf]);
        assert_eq!(
            unicode.jim084_matches(&RawString::from_bytes([0xff]), false),
            Ok(true)
        );
        let upper = RawString::from_bytes([0xc5, 0xb8]);
        assert_eq!(
            upper.jim084_matches(&RawString::from_bytes([0xff]), true),
            Ok(true)
        );
        assert_eq!(
            RawString::from_bytes(b"^".as_slice())
                .jim084_matches(&RawString::from_bytes(b"[a-]".as_slice()), false),
            Ok(true)
        );
        assert_eq!(
            RawString::from_bytes(b"a".as_slice())
                .jim084_matches(&RawString::from_bytes(b"[a".as_slice()), false),
            Ok(true)
        );
    }
    #[test]
    fn jim_search_retains_byte_matches_and_exclusive_prefix_counts() {
        let hay = RawString::from_unicode("ÿ");
        let continuation = RawString::from_bytes(vec![0xbf]);
        assert_eq!(hay.jim084_first(1, &continuation, 1, 0), Ok(None));
        assert_eq!(hay.jim084_last(1, &continuation, 1), Ok(Some(1)));
        let a = RawString::from_unicode("a");
        assert_eq!(a.jim084_last(0, &a, 1), Ok(None));
        assert_eq!(a.jim084_last(1, &a, 1), Ok(Some(0)));
        assert!(a.jim084_last(3, &a, 1).is_err());
    }
    #[test]
    fn jim_comparison_and_simple_case_keep_the_native_numeric_protocol() {
        let raw = RawString::from_bytes(vec![0xff]);
        let utf8 = RawString::from_unicode("ÿ");
        assert_ne!(raw.bytes(), utf8.bytes());
        assert_eq!(
            raw.jim084_compare(1, &utf8, 1, false),
            Ok(std::cmp::Ordering::Equal)
        );
        assert_eq!(
            &*raw.jim084_case_mapped(JimCaseMapping::Upper).bytes(),
            "Ÿ".as_bytes()
        );
        assert_eq!(
            &*RawString::from_unicode("ß")
                .jim084_case_mapped(JimCaseMapping::Upper)
                .bytes(),
            "ß".as_bytes()
        );
        assert_eq!(
            &*RawString::from_unicode("𐐨")
                .jim084_case_mapped(JimCaseMapping::Upper)
                .bytes(),
            "𐐨".as_bytes()
        );
        assert_eq!(
            &*RawString::from_bytes(vec![b'A', 0, b'B'])
                .jim084_case_mapped(JimCaseMapping::Lower)
                .bytes(),
            b"a"
        );
        assert!(raw.jim084_compare(3, &raw, 3, false).is_err());
    }
    #[test]
    fn raw_bytes_are_never_unicode_replacement_or_latin1() {
        let raw = RawString::from_bytes(vec![0xff, 0xc3]);
        assert!(raw.unicode().is_err());
        assert_eq!(&*raw.bytes(), &[0xff, 0xc3]);
        let alias = raw.clone();
        assert_eq!(&*raw.appended(b"A").bytes(), &[0xff, 0xc3, b'A']);
        assert_eq!(&*alias.bytes(), &[0xff, 0xc3]);
        assert_ne!(&*raw.bytes(), "ÿÃ".as_bytes());
    }
    #[test]
    fn selected_jim_units_preserve_invalid_and_surrogate_byte_ranges() {
        let raw = RawString::from_bytes(vec![0xff, 0xc3, 0xed, 0xa0, 0x80, 0xc3, 0xa9]);
        let observed: Vec<_> = raw.jim084_characters().collect();
        assert_eq!(
            observed,
            [
                JimCharacter {
                    codepoint: 255,
                    start: 0,
                    end: 1
                },
                JimCharacter {
                    codepoint: 195,
                    start: 1,
                    end: 2
                },
                JimCharacter {
                    codepoint: 0xd800,
                    start: 2,
                    end: 5
                },
                JimCharacter {
                    codepoint: 0xe9,
                    start: 5,
                    end: 7
                }
            ]
        );
        assert!(raw.unicode().is_err());
        assert_eq!(raw.bytes().len(), 7);
    }
}

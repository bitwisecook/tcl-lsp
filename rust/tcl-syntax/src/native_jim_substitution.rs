// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original Jim substitution object construction. This projects actual token
//! purposes and counted body extents; it issues no interpreter or lookup right.

use crate::value::ValueError;
use std::ops::Range;
use tcl_lexer::{JimScriptTokenKind, JimSubstTokens};

/// The initialized ordinary Script fields. Subst deliberately does not have
/// these fields: its native allocator leaves the corresponding slots unset.
pub struct JimOrdinaryScript<V> {
    /// Actual original LINE/WORD/Source objects.
    pub objects: crate::jim_script_objects::JimScriptObjects<V>,
    /// Original filename object owned once by the backing.
    pub filename: V,
    /// Native first parser token line, including leading separators.
    pub first_line: i32,
    /// Original Source baseline used to construct LINE objects.
    pub baseline: i32,
    /// Native current/completeness line.
    pub linenr: std::cell::Cell<i32>,
}

/// Both native constructors install the SAME Script primary. Initialized
/// storage, rather than a guessed metadata value, distinguishes their layouts.
pub enum JimScriptStorage<V> {
    /// Ordinary Script, whose substFlags is zero.
    Ordinary(JimOrdinaryScript<V>),
    /// Flat substitution tokens and the genuine interpreter empty filename.
    Substitution {
        /// Selected original substitution objects and exact flags.
        objects: JimSubstitutionObjects<V>,
        /// Actual interpreter empty object, retained by this backing.
        filename: V,
    },
}

impl<V> JimScriptStorage<V> {
    /// Exact flags used by both native primary-cache hit checks.
    pub fn flags(&self) -> u8 {
        match self {
            Self::Ordinary(_) => 0,
            Self::Substitution { objects, .. } => objects.flags,
        }
    }

    /// Borrow initialized ordinary fields; no Subst line value is invented.
    pub fn ordinary(&self) -> Result<&JimOrdinaryScript<V>, ValueError> {
        match self {
            Self::Ordinary(script) => Ok(script),
            Self::Substitution { objects, .. } if objects.flags != 0 => {
                Err(ValueError::NativeFatalCondition(
                    crate::raw_string::NativeFatalCondition::JimSubstitutionScriptReentry,
                ))
            }
            Self::Substitution { .. } => Err(ValueError::CommandProtocolUnavailable(
                "Jim substitution Script metadata is not initialized",
            )),
        }
    }

    /// Original filename, initialized by both constructors.
    pub fn filename(&self) -> &V {
        match self {
            Self::Ordinary(script) => &script.filename,
            Self::Substitution { filename, .. } => filename,
        }
    }

    /// Real original token count, including ordinary LINE/WORD objects.
    pub fn len(&self) -> usize {
        match self {
            Self::Ordinary(script) => script.objects.tokens().len(),
            Self::Substitution { objects, .. } => objects.tokens().len(),
        }
    }

    /// Whether this backing contains no real interpolation objects.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Borrow an original interpolation token. None as the token purpose means
    /// native LINE/WORD, which reaches `JimSubstOneToken`'s default error branch.
    pub fn interpolation_token(&self, index: usize) -> Option<(Option<JimScriptTokenKind>, &V)> {
        match self {
            Self::Ordinary(script) => script.objects.tokens().get(index).map(|token| {
                let kind = match token.kind {
                    crate::jim_script_objects::JimScriptObjectKind::Source(kind) => Some(kind),
                    _ => None,
                };
                (kind, &token.value)
            }),
            Self::Substitution { objects, .. } => objects
                .tokens()
                .get(index)
                .map(|token| (Some(token.kind), &token.value)),
        }
    }
}

/// The original name/key extents selected by `JimDictSugarParseVarKey`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JimDictionarySubstitutionExtents {
    /// Name before the first opening parenthesis in the `CString` prefix.
    pub name: Range<usize>,
    /// Counted remainder, removing a closing parenthesis only at counted end.
    pub key: Range<usize>,
}

/// Project the actual `CString` search and counted key geometry.
/// A hidden/missing parenthesis would enter Jim's invalid-pointer frontier.
pub fn dictionary_substitution_extents(
    bytes: &[u8],
) -> Result<JimDictionarySubstitutionExtents, ValueError> {
    let c_end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    let opening = bytes[..c_end].iter().position(|byte| *byte == b'(').ok_or(
        ValueError::NativeFatalCondition(
            crate::raw_string::NativeFatalCondition::JimDictionarySubstitutionGeometry,
        ),
    )?;
    let end = bytes.len() - usize::from(bytes.last() == Some(&b')'));
    Ok(JimDictionarySubstitutionExtents {
        name: 0..opening,
        key: opening + 1..end,
    })
}

/// A plain original token owned once by the genuine Subst backing.
pub struct JimSubstitutionObject<V> {
    /// Actual native token purpose, independent of its current primary cache.
    pub kind: JimScriptTokenKind,
    /// Original object, constructed without a Source cache.
    pub value: V,
}

/// Subst's initialized fields. Ordinary Script line/missing metadata is
/// deliberately absent: `SetSubstFromAny` does not initialize those C fields.
pub struct JimSubstitutionObjects<V> {
    /// Exactly retained parser flags; flag zero can reuse an ordinary Script.
    pub flags: u8,
    tokens: Vec<JimSubstitutionObject<V>>,
}

impl<V> JimSubstitutionObjects<V> {
    /// Construct each real token once, applying only actual ESC decoding.
    pub fn prepare(
        roster: &JimSubstTokens,
        mut construct: impl FnMut(&[u8]) -> Result<V, ValueError>,
    ) -> Result<Self, ValueError> {
        let mut tokens = Vec::with_capacity(roster.tokens.len());
        for token in &roster.tokens {
            let bytes = roster.image.bytes().get(token.value.as_range()).ok_or(
                ValueError::CommandProtocolUnavailable("Jim Subst original token extent"),
            )?;
            let decoded = if token.kind == JimScriptTokenKind::Escaped {
                crate::backslash::decode_bytes_in(bytes, tcl_dialect::EscapeSyntax::Jim)
            } else {
                std::borrow::Cow::Borrowed(bytes)
            };
            tokens.push(JimSubstitutionObject {
                kind: token.kind,
                value: construct(&decoded)?,
            });
        }
        Ok(Self {
            flags: roster.flags,
            tokens,
        })
    }

    /// Borrow actual original token objects without adding token owners.
    #[must_use]
    pub fn tokens(&self) -> &[JimSubstitutionObject<V>] {
        &self.tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dictionary_substitution_preserves_counted_key_after_cstring_search() {
        assert_eq!(
            dictionary_substitution_extents(b"d(A\0B)").unwrap(),
            JimDictionarySubstitutionExtents {
                name: 0..1,
                key: 2..5
            }
        );
        assert_eq!(
            dictionary_substitution_extents(b"d(k)tail").unwrap().key,
            2..8
        );
        assert!(matches!(
            dictionary_substitution_extents(b"d\0(k)"),
            Err(ValueError::NativeFatalCondition(_))
        ));
    }
}

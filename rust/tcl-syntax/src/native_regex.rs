// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Native C compiled-pattern ownership and range allocation recipes.
//! A compiled cache retains the executable engine artifact, not a type label.

use crate::{native_object::NativeObjectSnapshot, native_tcl_utf::NativeTclUtf, value::ValueError};
use std::{cell::RefCell, rc::Rc};
use tcl_dialect::TclVersion;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NativeRegexpRecipe {
    version: TclVersion,
}
impl NativeRegexpRecipe {
    #[must_use]
    pub const fn for_version(version: TclVersion) -> Self {
        Self { version }
    }
    #[must_use]
    pub const fn version(self) -> TclVersion {
        self.version
    }
    #[must_use]
    pub fn pattern_units(self, bytes: &[u8]) -> Vec<u32> {
        NativeTclUtf::for_version(self.version).decode_units(bytes)
    }
    #[must_use]
    pub fn literal_mapping_substitution(self, substitution: &[u8]) -> bool {
        !tcl_core_types::c_string_extent(substitution)
            .iter()
            .any(|byte| b"&\\".contains(byte))
    }
    #[must_use]
    pub fn literal_mapping(
        self,
        all: bool,
        start: usize,
        pattern: &[u8],
        substitution: &[u8],
    ) -> bool {
        all && start == 0
            && !tcl_core_types::c_string_extent(substitution)
                .iter()
                .any(|byte| b"&\\".contains(byte))
            && !tcl_core_types::c_string_extent(pattern)
                .iter()
                .any(|byte| b"*+?{}()[].\\|^$".contains(byte))
    }
    #[must_use]
    pub fn equal_units(self, left: &[u32], right: &[u32], nocase: bool) -> bool {
        left.len() == right.len()
            && left.iter().zip(right).all(|(&left, &right)| {
                left == right
                    || (nocase
                        && crate::native_tcl_case::lower(self.version, left)
                            == crate::native_tcl_case::lower(self.version, right))
            })
    }
    /// Exact `TclReToGlob` conversion retained by the compiled C85+ artifact.
    /// Unsupported regexp syntax declines this optional execution recipe.
    #[must_use]
    pub fn equivalent_glob(self, source: &[u8]) -> Option<Vec<u8>> {
        if self.version == TclVersion::V8_4 {
            return None;
        }
        if let Some(literal) = source.strip_prefix(b"***=") {
            let mut result = vec![b'*'];
            for &byte in literal {
                if b"\\*[]?".contains(&byte) {
                    result.push(b'\\');
                }
                result.push(byte);
            }
            result.push(b'*');
            return Some(result);
        }
        let mut result = Vec::new();
        let mut cursor = usize::from(source.first() == Some(&b'^'));
        let mut last_star = cursor == 0;
        let mut stars = 0;
        let mut anchored_right = false;
        if last_star {
            result.push(b'*');
        }
        while let Some(&byte) = source.get(cursor) {
            match byte {
                b'\\' => {
                    cursor += 1;
                    let escaped = *source.get(cursor)?;
                    match escaped {
                        b'a' => result.push(7),
                        b'b' => result.push(8),
                        b'f' => result.push(12),
                        b'n' => result.push(10),
                        b'r' => result.push(13),
                        b't' => result.push(9),
                        b'v' => result.push(11),
                        b'B' | b'\\' => result.extend_from_slice(b"\\\\"),
                        b'*' | b'[' | b']' | b'?' => {
                            result.push(b'\\');
                            result.push(escaped);
                        }
                        b'{' | b'}' | b'(' | b')' | b'+' | b'.' | b'|' | b'^' | b'$' => {
                            result.push(escaped);
                        }
                        _ => return None,
                    }
                }
                b'.' => match source.get(cursor + 1) {
                    Some(b'*') => {
                        cursor += 2;
                        if !last_star {
                            result.push(b'*');
                            last_star = true;
                            stars += 1;
                        }
                        continue;
                    }
                    Some(b'+') => {
                        cursor += 2;
                        result.extend_from_slice(b"?*");
                        last_star = true;
                        stars += 1;
                        continue;
                    }
                    _ => result.push(b'?'),
                },
                b'$' => {
                    if cursor + 1 != source.len() {
                        return None;
                    }
                    anchored_right = true;
                }
                b'*' | b'+' | b'?' | b'|' | b'^' | b'{' | b'}' | b'(' | b')' | b'[' | b']' => {
                    return None;
                }
                _ => result.push(byte),
            }
            last_star = false;
            cursor += 1;
        }
        if stars > 1 {
            return None;
        }
        if !anchored_right && !last_star {
            result.push(b'*');
        }
        Some(result)
    }
    pub fn range(
        self,
        subject: &NativeObjectSnapshot,
        units: &[u32],
        start: usize,
        end: usize,
    ) -> Result<NativeRegexpRange, ValueError> {
        if start > end || end > units.len() {
            return Err(ValueError::CommandProtocolUnavailable(
                "native regexp range geometry",
            ));
        }
        if start == end {
            return Ok(NativeRegexpRange::Empty);
        }
        // C84/85 GetRange selects its counted byte path even when the original
        // already has Unicode. C86+ uses the existing Unicode representation.
        if self.version <= TclVersion::V8_5
            && let Some(bytes) = &subject.resident
            && bytes.len() == units.len()
        {
            return Ok(NativeRegexpRange::Bytes {
                bytes: bytes[start..end].to_vec(),
                count: end - start,
            });
        }
        Ok(NativeRegexpRange::Unicode(units[start..end].to_vec()))
    }
}

pub enum NativeRegexpRange {
    Empty,
    Bytes { bytes: Vec<u8>, count: usize },
    Unicode(Vec<u32>),
}

pub struct NativeRegexpCache<R> {
    recipe: NativeRegexpRecipe,
    flags: u32,
    compiled: Rc<RefCell<R>>,
    glob: Option<Rc<[u8]>>,
}
impl<R> Clone for NativeRegexpCache<R> {
    fn clone(&self) -> Self {
        Self {
            recipe: self.recipe,
            flags: self.flags,
            compiled: Rc::clone(&self.compiled),
            glob: self.glob.clone(),
        }
    }
}
impl<R> NativeRegexpCache<R> {
    pub fn new(
        recipe: NativeRegexpRecipe,
        flags: u32,
        compiled: Rc<RefCell<R>>,
        source: &[u8],
    ) -> Self {
        Self {
            recipe,
            flags,
            compiled,
            glob: recipe.equivalent_glob(source).map(Rc::from),
        }
    }
    #[must_use]
    pub fn compiled_for(&self, recipe: NativeRegexpRecipe, flags: u32) -> Option<Rc<RefCell<R>>> {
        (self.recipe == recipe && self.flags == flags).then(|| Rc::clone(&self.compiled))
    }
    #[must_use]
    pub fn glob_for(&self, recipe: NativeRegexpRecipe, flags: u32) -> Option<Rc<[u8]>> {
        (self.recipe == recipe && self.flags == flags)
            .then(|| self.glob.clone())
            .flatten()
    }
    #[must_use]
    pub const fn recipe(&self) -> NativeRegexpRecipe {
        self.recipe
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_object::NativeObjectCacheSnapshot;
    use crate::native_string::{NativeStringProtocol, NativeStringStorageIdentity};

    #[test]
    fn equivalent_glob_matches_actual_counted_converter_rows() {
        // Native proof: naming.regex.c-equivalent-glob-counted-conversion
        // docs/design/analysis/name-resolution-proofs/regex-c-equivalent-glob-counted-conversion.md
        let patterns: [&[u8]; 14] = [
            b"",
            b"foo",
            b"^foo$",
            b".*",
            b".+",
            b"a.*b.*c",
            b"\\B",
            b"\\n",
            b"\\*",
            b"***=x?",
            b"[a]",
            b"a$b",
            b"a\\",
            b"^.*foo.*$",
        ];
        for (version, rows) in [
            (
                TclVersion::V8_5,
                include_str!("../tests/data/native_regexp_glob/8.5.19.txt"),
            ),
            (
                TclVersion::V8_6,
                include_str!("../tests/data/native_regexp_glob/8.6.18.txt"),
            ),
            (
                TclVersion::V9_0,
                include_str!("../tests/data/native_regexp_glob/9.0.4.txt"),
            ),
            (
                TclVersion::V9_1,
                include_str!("../tests/data/native_regexp_glob/9.1.0.txt"),
            ),
        ] {
            assert_eq!(rows.lines().count(), patterns.len());
            for row in rows.lines() {
                let columns: Vec<_> = row.split('\t').collect();
                let index: usize = columns[0].parse().unwrap();
                let expected = (columns[1] == "0").then(|| {
                    columns[2]
                        .as_bytes()
                        .as_chunks::<2>()
                        .0
                        .iter()
                        .map(|hex| {
                            u8::from_str_radix(std::str::from_utf8(hex).unwrap(), 16).unwrap()
                        })
                        .collect::<Vec<_>>()
                });
                assert_eq!(
                    NativeRegexpRecipe::for_version(version).equivalent_glob(patterns[index]),
                    expected,
                    "{version:?}/{index}"
                );
            }
        }
        assert!(
            NativeRegexpRecipe::for_version(TclVersion::V8_4)
                .equivalent_glob(b"a")
                .is_none()
        );
    }

    #[test]
    fn native_counted_units_and_ranges_match_five_engine_controls() {
        for version in TclVersion::ALL {
            let recipe = NativeRegexpRecipe::for_version(version);
            for bytes in [b"a".as_slice(), b"\xff", b"\xc0\x80", b"\xf0\x9f\x98\x80"] {
                let units = recipe.pattern_units(bytes);
                let snapshot = NativeObjectSnapshot {
                    resident: Some(Rc::from(bytes)),
                    storage: Some(NativeStringStorageIdentity::Allocated),
                    cache: NativeObjectCacheSnapshot::String {
                        protocol: NativeStringProtocol::C(version),
                        num_chars: Some(units.len()),
                        unicode: Some(Rc::from(units.clone())),
                    },
                };
                let selected = recipe.range(&snapshot, &units, 0, units.len()).unwrap();
                let older = version <= TclVersion::V8_5;
                let (resident, output) = match selected {
                    NativeRegexpRange::Bytes { bytes, count } => {
                        assert_eq!(count, units.len());
                        (true, bytes)
                    }
                    NativeRegexpRange::Unicode(units) => (
                        false,
                        NativeTclUtf::for_version(version)
                            .encode_units(&units)
                            .unwrap(),
                    ),
                    NativeRegexpRange::Empty => panic!("native nonempty range control"),
                };
                assert_eq!(
                    resident,
                    older && bytes != b"\xc0\x80",
                    "{version:?}/{bytes:?}"
                );
                let expected = if bytes == b"\xff" && !older {
                    b"\xc3\xbf".as_slice()
                } else if bytes == b"\xf0\x9f\x98\x80" && version == TclVersion::V8_6 {
                    b"\xed\xa0\xbd\xed\xb8\x80".as_slice()
                } else {
                    bytes
                };
                assert_eq!(output, expected, "{version:?}/{bytes:?}");
            }
            assert_eq!(
                NativeStringProtocol::C(version).unicode_constructor_has_unicode(0),
                version >= TclVersion::V8_6
            );
        }
    }
}

/// The bundled Jim084 `CString` compiler and byte-range execution recipe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JimRegexpRecipe;
impl JimRegexpRecipe {
    #[must_use]
    pub const fn jim084() -> Self {
        Self
    }
    #[must_use]
    pub fn pattern_extent(self, bytes: &[u8]) -> &[u8] {
        tcl_core_types::c_string_extent(bytes)
    }
}

/// Lifetime-only query transport for a genuine compiled Jim program.
/// A native primary's teardown withdraws the program from every transport.
pub struct JimRegexpArtifact<R> {
    program: Rc<RefCell<Option<R>>>,
}
impl<R> Clone for JimRegexpArtifact<R> {
    fn clone(&self) -> Self {
        Self {
            program: Rc::clone(&self.program),
        }
    }
}
impl<R> JimRegexpArtifact<R> {
    pub fn with_program<T>(&self, use_program: impl FnOnce(&mut R) -> T) -> Result<T, ValueError> {
        let mut program = self.program.borrow_mut();
        let program = program
            .as_mut()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "retired shallow Jim regexp program",
            ))?;
        Ok(use_program(program))
    }
    #[must_use]
    pub fn same_program(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.program, &other.program)
    }
}
struct JimRegexpPrimary<R> {
    flags: u32,
    artifact: JimRegexpArtifact<R>,
}
impl<R> Drop for JimRegexpPrimary<R> {
    fn drop(&mut self) {
        self.artifact.program.borrow_mut().take();
    }
}
/// One physical Jim primary. Query clones share this primary's lifetime;
/// native duplication creates a second shallow primary with the same program.
pub struct JimRegexpCache<R>(Rc<JimRegexpPrimary<R>>);
impl<R> Clone for JimRegexpCache<R> {
    fn clone(&self) -> Self {
        Self(Rc::clone(&self.0))
    }
}
impl<R> JimRegexpCache<R> {
    #[must_use]
    pub fn new(flags: u32, program: R) -> Self {
        Self(Rc::new(JimRegexpPrimary {
            flags,
            artifact: JimRegexpArtifact {
                program: Rc::new(RefCell::new(Some(program))),
            },
        }))
    }
    pub fn compiled_for(&self, flags: u32) -> Result<Option<JimRegexpArtifact<R>>, ValueError> {
        if self.0.artifact.program.borrow().is_none() {
            return Err(ValueError::CommandProtocolUnavailable(
                "retired shallow Jim regexp program",
            ));
        }
        Ok((self.0.flags == flags).then(|| self.0.artifact.clone()))
    }
    #[must_use]
    pub fn duplicate(&self) -> Self {
        Self(Rc::new(JimRegexpPrimary {
            flags: self.0.flags,
            artifact: self.0.artifact.clone(),
        }))
    }
}

#[cfg(test)]
mod jim_cache_tests {
    use super::*;
    #[test]
    fn jim_shallow_duplicate_retires_program_without_query_ownership() {
        let primary = JimRegexpCache::new(0, vec![1]);
        let artifact = primary.compiled_for(0).unwrap().unwrap();
        let duplicate = primary.duplicate();
        assert!(artifact.same_program(&duplicate.compiled_for(0).unwrap().unwrap()));
        assert!(primary.compiled_for(2).unwrap().is_none());
        drop(duplicate);
        assert!(primary.compiled_for(0).is_err());
        assert!(artifact.with_program(|program| program.len()).is_err());
    }
}

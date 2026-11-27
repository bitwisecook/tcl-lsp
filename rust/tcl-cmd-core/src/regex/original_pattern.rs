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

//! Patterns compiled and executed over original objects: the native source and object traits, the prepared artifact and the native `regexp` run.

use super::regexp_command::{
    RegexpExecutionError, RegexpOptions, about_answer, regex_execution_error,
};
use super::{
    NO_MATCH, RegMatch, RegexEngine, RegexError, RegexFlags, RegexpResult, compile_error,
    completed_matches, decode_utf8, notbol_at, resolve_start_original,
};
use crate::index::NativeIndexOps;
use tcl_dialect::TclVersion;
use tcl_syntax::value::{ValueError, ValueOps};

/// Original C cache, Unicode range and actual engine artifact adapters.
/// A missing recipe is only the explicitly selected compatibility path.
pub trait NativeRegexSource: NativeIndexOps {
    /// A distinct actual Jim issuer; compatibility adapters provide none.
    fn jim_regex_recipe(
        &self,
    ) -> Result<Option<tcl_syntax::native_regex::JimRegexpRecipe>, ValueError> {
        Ok(None)
    }
    fn jim_regex_option(
        &mut self,
        _original: &Self::Value,
        _table: &'static [&'static str],
    ) -> Result<usize, crate::CmdError> {
        Err(ValueError::CommandProtocolUnavailable("original Jim regexp Enum").into())
    }
    fn jim_regex_index(&mut self, _original: &Self::Value) -> Result<i32, crate::CmdError> {
        Err(ValueError::CommandProtocolUnavailable("original Jim regexp index").into())
    }

    fn regex_recipe(
        &self,
    ) -> Result<Option<tcl_syntax::native_regex::NativeRegexpRecipe>, tcl_syntax::value::ValueError>;
}

pub trait NativeRegexObjects<E: RegexEngine>: NativeRegexSource {
    fn regex_jim_range(&mut self, bytes: &[u8], matched: bool) -> Result<Self::Value, ValueError> {
        let _ = (bytes, matched);
        Err(ValueError::CommandProtocolUnavailable(
            "original Jim regexp range String",
        ))
    }
    fn regex_cached_jim_pattern(
        &self,
        _original: &Self::Value,
        _flags: u32,
    ) -> Result<Option<tcl_syntax::native_regex::JimRegexpArtifact<E::Regex>>, ValueError> {
        Ok(None)
    }
    fn regex_install_jim_pattern(
        &mut self,
        _original: &Self::Value,
        _flags: u32,
        _program: E::Regex,
    ) -> Result<tcl_syntax::native_regex::JimRegexpArtifact<E::Regex>, ValueError> {
        Err(ValueError::CommandProtocolUnavailable(
            "original Jim regexp primary publication",
        ))
    }

    fn regex_cached_pattern(
        &self,
        original: &Self::Value,
        recipe: tcl_syntax::native_regex::NativeRegexpRecipe,
        flags: u32,
    ) -> Result<Option<std::rc::Rc<std::cell::RefCell<E::Regex>>>, tcl_syntax::value::ValueError>;
    fn regex_cached_glob(
        &self,
        original: &Self::Value,
        recipe: tcl_syntax::native_regex::NativeRegexpRecipe,
        flags: u32,
    ) -> Result<Option<std::rc::Rc<[u8]>>, tcl_syntax::value::ValueError>;
    fn regex_install_pattern(
        &mut self,
        original: &Self::Value,
        recipe: tcl_syntax::native_regex::NativeRegexpRecipe,
        flags: u32,
        compiled: std::rc::Rc<std::cell::RefCell<E::Regex>>,
    ) -> Result<(), tcl_syntax::value::ValueError>;
    fn regex_range_value(
        &mut self,
        range: tcl_syntax::native_regex::NativeRegexpRange,
    ) -> Result<Self::Value, tcl_syntax::value::ValueError>;
}

pub(super) fn native_compiled_pattern<O: NativeRegexObjects<E>, E: RegexEngine>(
    ops: &mut O,
    original: &O::Value,
    recipe: tcl_syntax::native_regex::NativeRegexpRecipe,
    flags: RegexFlags,
    version: TclVersion,
) -> Result<std::rc::Rc<std::cell::RefCell<E::Regex>>, RegexError> {
    let host = |error| RegexError::from(crate::CmdError::from(error));
    if let Some(compiled) = ops
        .regex_cached_pattern(original, recipe, flags.cache_key())
        .map_err(host)?
    {
        return Ok(compiled);
    }
    let bytes = ops.native_string_bytes(original).map_err(host)?;
    let units = recipe.pattern_units(
        &bytes,
        tcl_runtime_api::native_c_string_abi::supported_backend_plain_char(),
    );
    let compiled = E::compile_units(&units, flags)
        .ok_or_else(|| {
            host(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native regexp character-unit compiler",
            ))
        })?
        .map_err(|detail| compile_error(version, &detail))?;
    let compiled = std::rc::Rc::new(std::cell::RefCell::new(compiled));
    ops.regex_install_pattern(
        original,
        recipe,
        flags.cache_key(),
        std::rc::Rc::clone(&compiled),
    )
    .map_err(host)?;
    Ok(compiled)
}

/// Retained original-pattern engine artifact. A compatibility artifact owns
/// no native `RegExp` primary or object cache authority.
pub enum PreparedOriginalRegex<R> {
    Native {
        compiled: std::rc::Rc<std::cell::RefCell<R>>,
        recipe: tcl_syntax::native_regex::NativeRegexpRecipe,
        glob: Option<std::rc::Rc<[u8]>>,
        flags: RegexFlags,
    },
    Jim {
        artifact: tcl_syntax::native_regex::JimRegexpArtifact<R>,
        flags: RegexFlags,
    },
    Compatibility(R),
}

impl<R> PreparedOriginalRegex<R> {
    #[must_use]
    pub const fn native_recipe(&self) -> Option<tcl_syntax::native_regex::NativeRegexpRecipe> {
        match self {
            Self::Native { recipe, .. } => Some(*recipe),
            Self::Jim { .. } | Self::Compatibility(_) => None,
        }
    }
}

/// Compile an original pattern without a compatibility byte projection on C.
pub fn prepare_pattern_original<O: NativeRegexObjects<E>, E: RegexEngine>(
    ops: &mut O,
    pattern: &O::Value,
    flags: RegexFlags,
    version: TclVersion,
) -> Result<PreparedOriginalRegex<E::Regex>, RegexError> {
    if let Some(recipe) = ops
        .regex_recipe()
        .map_err(|error| RegexError::from(crate::CmdError::from(error)))?
    {
        let compiled = native_compiled_pattern::<O, E>(ops, pattern, recipe, flags, version)?;
        let glob = ops
            .regex_cached_glob(pattern, recipe, flags.cache_key())
            .map_err(|error| RegexError::from(crate::CmdError::from(error)))?;
        Ok(PreparedOriginalRegex::Native {
            compiled,
            recipe,
            glob,
            flags,
        })
    } else if ops
        .jim_regex_recipe()
        .map_err(|error| RegexError::from(crate::CmdError::from(error)))?
        .is_some()
    {
        let key = jim_flags(flags);
        let artifact = if let Some(artifact) = ops
            .regex_cached_jim_pattern(pattern, key)
            .map_err(|error| RegexError::from(crate::CmdError::from(error)))?
        {
            artifact
        } else {
            let bytes = ops
                .native_string_bytes(pattern)
                .map_err(|error| RegexError::from(crate::CmdError::from(error)))?;
            let program = E::compile_jim(&bytes, flags)
                .ok_or_else(|| {
                    RegexError::from(crate::CmdError::from(
                        ValueError::CommandProtocolUnavailable("bundled Jim regexp compiler"),
                    ))
                })?
                .map_err(|detail| {
                    RegexError::new(
                        [
                            b"couldn't compile regular expression pattern: ".as_slice(),
                            &detail,
                        ]
                        .concat(),
                    )
                })?;
            ops.regex_install_jim_pattern(pattern, key, program)
                .map_err(|error| RegexError::from(crate::CmdError::from(error)))?
        };
        // Public Jim regexp consumers deliberately reach this getter after
        // cache selection. A missing native updater is a typed host refusal.
        ops.native_string_bytes(pattern)
            .map_err(|error| RegexError::from(crate::CmdError::from(error)))?;
        Ok(PreparedOriginalRegex::Jim { artifact, flags })
    } else {
        let bytes = ops
            .native_string_bytes(pattern)
            .map_err(|error| RegexError::from(crate::CmdError::from(error)))?;
        E::compile(&bytes, flags)
            .map(PreparedOriginalRegex::Compatibility)
            .map_err(|detail| compile_error(version, &detail))
    }
}

fn jim_flags(flags: RegexFlags) -> u32 {
    u32::from(flags.nocase) * 2
        + u32::from(flags.lineanchor) * 4
        + u32::from(flags.linestop) * 8
        + u32::from(flags.expanded) * 32
}

/// Native LSEARCH tries NOSUB before list conversion and retries genuine
/// compilation failure without NOSUB. Host access refusals never retry.
pub fn prepare_search_pattern_original<O: NativeRegexObjects<E>, E: RegexEngine>(
    ops: &mut O,
    pattern: &O::Value,
    mut flags: RegexFlags,
    version: TclVersion,
) -> Result<PreparedOriginalRegex<E::Regex>, RegexError> {
    if ops
        .regex_recipe()
        .map_err(|error| RegexError::from(crate::CmdError::from(error)))?
        .is_none()
    {
        return prepare_pattern_original::<O, E>(ops, pattern, flags, version);
    }
    flags.nosub = true;
    match prepare_pattern_original::<O, E>(ops, pattern, flags, version) {
        Ok(compiled) => Ok(compiled),
        Err(error) => {
            if error.0.native_execution_refusal().is_some() {
                return Err(error);
            }
            flags.nosub = false;
            prepare_pattern_original::<O, E>(ops, pattern, flags, version)
        }
    }
}

/// Execute against the original subject using the retained artifact's input
/// owner. The executable borrow is released before any variable callback.
pub fn execute_pattern_original<O: NativeRegexObjects<E>, E: RegexEngine>(
    ops: &mut O,
    pattern: &mut PreparedOriginalRegex<E::Regex>,
    subject: &O::Value,
    offset: usize,
    notbol: bool,
) -> Result<Option<Vec<RegMatch>>, RegexError> {
    execute_pattern_original_mode::<O, E>(ops, pattern, subject, offset, notbol, true)
}

/// Execute a native boolean match without requesting capture ranges.
pub fn match_pattern_original<O: NativeRegexObjects<E>, E: RegexEngine>(
    ops: &mut O,
    pattern: &mut PreparedOriginalRegex<E::Regex>,
    subject: &O::Value,
) -> Result<bool, RegexError> {
    execute_pattern_original_mode::<O, E>(ops, pattern, subject, 0, false, false)
        .map(|result| result.is_some())
}

fn execute_pattern_original_mode<O: NativeRegexObjects<E>, E: RegexEngine>(
    ops: &mut O,
    pattern: &mut PreparedOriginalRegex<E::Regex>,
    subject: &O::Value,
    offset: usize,
    notbol: bool,
    captures: bool,
) -> Result<Option<Vec<RegMatch>>, RegexError> {
    match pattern {
        PreparedOriginalRegex::Native {
            compiled,
            recipe,
            glob,
            flags,
        } => {
            if (flags.nosub || !captures)
                && !flags.expanded
                && !flags.linestop
                && !flags.lineanchor
                && offset == 0
                && !notbol
                && let Some(glob) = glob
            {
                return native_glob_exec(ops, *recipe, glob, subject, flags.nocase)
                    .map(|matched| matched.then(Vec::new));
            }
            let units = ops
                .native_unicode_units(subject)
                .map_err(|error| RegexError::from(crate::CmdError::from(error)))?;
            let characters = units
                .iter()
                .map(|&unit| {
                    i32::try_from(unit).map_err(|_| {
                        RegexError::from(crate::CmdError::from(
                            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                                "native regexp character width",
                            ),
                        ))
                    })
                })
                .collect::<Result<Vec<_>, _>>()?;
            completed_matches(E::exec(
                &mut compiled.borrow_mut(),
                &characters,
                offset,
                notbol,
            ))
        }
        PreparedOriginalRegex::Jim { artifact, .. } => {
            let bytes = ops
                .native_string_bytes(subject)
                .map_err(|error| RegexError::from(crate::CmdError::from(error)))?;
            let raw = tcl_syntax::raw_string::RawString::from_bytes(bytes.clone());
            let byte_offset = raw.jim084_byte_offset(offset).map_err(|error| {
                RegexError::from(crate::CmdError::from(ValueError::from(error)))
            })?;
            let count = if captures {
                artifact
                    .with_program(|compiled| E::nsub(compiled) + 1)
                    .map_err(|error| RegexError::from(crate::CmdError::from(error)))?
            } else {
                1
            };
            artifact
                .with_program(|compiled| {
                    E::exec_jim(compiled, &bytes[byte_offset..], count, notbol)
                })
                .map_err(|error| RegexError::from(crate::CmdError::from(error)))?
                .map(|matches| {
                    matches.map(|mut matches| {
                        for span in &mut matches {
                            if span.so != NO_MATCH {
                                span.so += byte_offset;
                            }
                            if span.eo != NO_MATCH {
                                span.eo += byte_offset;
                            }
                        }
                        matches
                    })
                })
                .map_err(|reason| {
                    RegexError::from(crate::CmdError::from(
                        ValueError::CommandProtocolUnavailable(reason),
                    ))
                })
        }
        PreparedOriginalRegex::Compatibility(compiled) => {
            let bytes = ops
                .native_string_bytes(subject)
                .map_err(|error| RegexError::from(crate::CmdError::from(error)))?;
            let (characters, _) = decode_utf8(&bytes);
            completed_matches(E::exec(compiled, &characters, offset, notbol))
        }
    }
}

fn native_glob_exec<O: ValueOps>(
    ops: &mut O,
    recipe: tcl_syntax::native_regex::NativeRegexpRecipe,
    pattern: &[u8],
    subject: &O::Value,
    nocase: bool,
) -> Result<bool, RegexError> {
    use tcl_syntax::native_glob::NativeGlobObject as Glob;
    use tcl_syntax::native_object::NativeObjectCacheSnapshot as Cache;
    let host = |error| RegexError::from(crate::CmdError::from(error));
    let snapshot = ops.native_object_snapshot(subject).map_err(host)?;
    let unicode = matches!(&snapshot.cache, Cache::String { .. })
        || (recipe.version() >= TclVersion::V8_6 && matches!(&snapshot.cache, Cache::None));
    let units = unicode
        .then(|| ops.native_unicode_units(subject))
        .transpose()
        .map_err(host)?;
    let binary = recipe.version() == TclVersion::V8_5
        && !nocase
        && snapshot.resident.is_none()
        && matches!(&snapshot.cache, Cache::ByteArray { .. });
    let bytes = if units.is_none() && !binary {
        Some(ops.native_string_bytes(subject).map_err(host)?)
    } else {
        snapshot.resident.clone()
    };
    let original = if let Some(units) = &units {
        Glob::CachedUnicode {
            units,
            resident_bytes: snapshot.resident.as_deref(),
        }
    } else if let Cache::ByteArray { bytes: binary, .. } = &snapshot.cache {
        match &bytes {
            Some(resident) => Glob::ByteArrayWithString {
                bytes: binary,
                resident_bytes: resident,
            },
            None => Glob::PureByteArray(binary),
        }
    } else {
        match snapshot.cache {
            Cache::None => Glob::FreshString(bytes.as_deref().expect("reached string")),
            _ => Glob::OtherString(bytes.as_deref().expect("reached string")),
        }
    };
    tcl_syntax::native_glob::match_native_glob_objects(
        tcl_syntax::naming::NativeNameProtocol::C(recipe.version()),
        Glob::FreshString(pattern),
        original,
        nocase,
    )
    .map_err(|_| {
        host(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "native regexp equivalent-glob storage",
        ))
    })
}

/// Native REGEXP instruction over the same original pattern and subject.
/// It has no option argv and acquires no replacement object references.
pub fn compiled_match_original<O: NativeRegexObjects<E>, E: RegexEngine>(
    ops: &mut O,
    pattern: &O::Value,
    subject: &O::Value,
    flags: RegexFlags,
    version: TclVersion,
) -> Result<bool, RegexError> {
    let host = |error| RegexError::from(crate::CmdError::from(error));
    let recipe = ops.regex_recipe().map_err(host)?.ok_or_else(|| {
        host(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "native REGEXP instruction issuer",
        ))
    })?;
    let mut compiled = prepare_pattern_original::<O, E>(ops, pattern, flags, version)?;
    // The selected instruction requests zero ranges, independently of NOSUB.
    // The recipe query above authenticates the physical input owner.
    debug_assert_eq!(compiled.native_recipe(), Some(recipe));
    match_pattern_original::<O, E>(ops, &mut compiled, subject)
}

fn native_capture_value<O: NativeRegexObjects<E>, E: RegexEngine>(
    ops: &mut O,
    subject: &O::Value,
    units: &[u32],
    span: Option<RegMatch>,
    recipe: tcl_syntax::native_regex::NativeRegexpRecipe,
    indices: bool,
) -> Result<O::Value, ValueError> {
    if indices {
        let (start, end) = match span {
            Some(span) if span.so != NO_MATCH => (
                i64::try_from(span.so).unwrap_or(i64::MAX),
                i64::try_from(span.eo).unwrap_or(i64::MAX) - 1,
            ),
            _ => (-1, -1),
        };
        let start = ops.new_int(start);
        let end = ops.new_int(end);
        Ok(ops.new_list(vec![start, end]))
    } else {
        let range = match span {
            Some(span) if span.so != NO_MATCH && span.eo > 0 && span.eo <= units.len() => recipe
                .range(
                    &ops.native_object_snapshot(subject)?,
                    units,
                    span.so,
                    span.eo,
                )?,
            _ => tcl_syntax::native_regex::NativeRegexpRange::Empty,
        };
        ops.regex_range_value(range)
    }
}

pub(super) fn native_regex_characters(
    units: &[u32],
    reason: &'static str,
) -> Result<Vec<i32>, ValueError> {
    units
        .iter()
        .map(|&unit| {
            i32::try_from(unit).map_err(|_| ValueError::CommandProtocolUnavailable(reason))
        })
        .collect()
}

fn native_whole_match(
    matches: &[RegMatch],
    minimum: usize,
    length: usize,
    reason: &'static str,
) -> Result<RegMatch, ValueError> {
    let whole = matches
        .first()
        .copied()
        .ok_or(ValueError::CommandProtocolUnavailable(reason))?;
    if whole.so < minimum || whole.so > whole.eo || whole.eo > length {
        return Err(ValueError::CommandProtocolUnavailable(reason));
    }
    Ok(whole)
}

pub(super) fn regexp_native_selected<O: NativeRegexObjects<E>, E: RegexEngine, Err>(
    ops: &mut O,
    args: &[O::Value],
    originals: &[O::Value],
    recipe: tcl_syntax::native_regex::NativeRegexpRecipe,
    options: RegexpOptions,
    mut assign: impl FnMut(&mut O, usize, O::Value) -> Result<(), Err>,
) -> Result<RegexpResult<O::Value>, RegexpExecutionError<Err>> {
    let RegexpOptions {
        version,
        common,
        indices,
        inline,
        about,
    } = options;
    let (length, mut offset) = if about {
        (0, 0)
    } else {
        let length = ops
            .index_character_count(&originals[1])
            .map_err(regex_execution_error)?;
        let offset = resolve_start_original(ops, args, &common, length)
            .map_err(RegexpExecutionError::Regex)?;
        (length, offset)
    };
    let compiled =
        native_compiled_pattern::<O, E>(ops, &originals[0], recipe, common.flags, version)
            .map_err(RegexpExecutionError::Regex)?;
    let nsubs = E::nsub(&compiled.borrow());
    if about {
        let answer = about_answer::<O, E>(ops, &compiled.borrow());
        return Ok(RegexpResult::Inline(answer));
    }
    let mut count = 0_i64;
    let mut items = Vec::new();
    loop {
        let units = ops
            .index_c_unicode_units(&originals[1])
            .map_err(regex_execution_error)?;
        let characters = native_regex_characters(&units, "native regexp character width")
            .map_err(regex_execution_error)?;
        let matches = E::exec(
            &mut compiled.borrow_mut(),
            &characters,
            offset,
            notbol_at(&characters, offset),
        );
        let matches = completed_matches(matches).map_err(RegexpExecutionError::Regex)?;
        let Some(matches) = matches else {
            break;
        };
        // Past the end C still matches the empty remainder, positioned at the
        // caller's offset rather than the string length.
        let whole = native_whole_match(
            &matches,
            0,
            units.len().max(offset),
            "native regexp whole-match geometry",
        )
        .map_err(regex_execution_error)?;
        let total = if inline {
            nsubs + 1
        } else {
            originals.len() - 2
        };
        for index in 0..total {
            let span = (index <= nsubs)
                .then(|| matches.get(index).copied())
                .flatten();
            let value =
                native_capture_value::<O, E>(ops, &originals[1], &units, span, recipe, indices)
                    .map_err(regex_execution_error)?;
            if inline {
                items.push(
                    ops.index_created_key(value)
                        .map_err(regex_execution_error)?,
                );
            } else {
                assign(ops, index, value).map_err(RegexpExecutionError::Assignment)?;
            }
        }
        count += 1;
        if !common.all {
            break;
        }
        offset = whole.eo;
        if whole.eo == whole.so {
            offset += 1;
        }
        if offset >= length {
            break;
        }
    }
    if inline {
        let values = items
            .iter()
            .map(|owner| ops.index_key_argument(owner))
            .collect::<Result<Vec<_>, _>>()
            .map_err(regex_execution_error)?;
        Ok(RegexpResult::Inline(ops.new_list(values)))
    } else {
        Ok(RegexpResult::Count {
            assign: None,
            count,
        })
    }
}

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

//! Jim lsearch over original Enum, Index, List and callback objects.
//! The selected object adapter owns native references and publication. This
//! path preserves Jim's exact options and integer callback contract separately
//! from C lsearch grammar and compiled ARE matching.

use crate::{CmdError, regex::NativeRegexSource};
use tcl_syntax::value::{ValueError, ValueOps};

/// The retained exact Jim lsearch Enum declaration, in native ordinal order.
pub static OPTIONS: [&str; 11] = [
    "-bool", "-not", "-nocase", "-exact", "-glob", "-regexp", "-all", "-inline", "-command",
    "-stride", "-index",
];

/// Physical operations selected only after the genuine Jim issuer is checked.
pub trait NativeJimLsearchObjects: ValueOps + NativeRegexSource {
    /// One real native reference, released on every exit.
    type Hold;
    /// The fresh, unshared result List and its local cleanup owner.
    type Accumulator;
    /// Materialize the original under the already selected physical Jim recipe.
    fn jim_search_bytes(&mut self, value: &Self::Value) -> Result<std::rc::Rc<[u8]>, ValueError>;
    /// Read Jim cached UTF count and its genuine String conversion.
    fn jim_search_character_count(&mut self, value: &Self::Value) -> Result<usize, ValueError>;
    /// Resolve the original against the exact static Enum table and flags.
    fn jim_search_option(&mut self, value: &Self::Value) -> Result<usize, CmdError>;
    /// Take one actual native reference for the reached lifetime.
    fn jim_search_hold(&mut self, value: &Self::Value) -> Result<Self::Hold, ValueError>;
    /// Borrow actual current List children without adding native references.
    /// Transport one live original without adding a native reference.
    fn jim_search_borrow(&self, value: &Self::Value) -> Self::Value;
    fn jim_search_elements(&mut self, value: &Self::Value) -> Result<Vec<Self::Value>, ValueError>;
    /// Borrow the selected List backing; a callback-replaced primary refuses.
    fn jim_search_current_elements(
        &self,
        value: &Self::Value,
    ) -> Result<Vec<Self::Value>, ValueError>;
    /// Create the genuine unowned regexp command object for this loop.
    fn jim_search_regexp_command(&mut self) -> Self::Value;
    /// Create an unowned group List with the same original children.
    fn jim_search_group(&mut self, values: Vec<Self::Value>) -> Self::Value;
    /// Allocate the original empty result List and its local cleanup owner.
    fn jim_search_begin(&mut self) -> Self::Accumulator;
    /// Append original children to that same unshared result List.
    fn jim_search_append(
        &mut self,
        result: &mut Self::Accumulator,
        values: &[Self::Value],
    ) -> Result<(), ValueError>;
    /// Publish the completed original List before releasing its local owner.
    fn jim_search_finish(&mut self, result: Self::Accumulator) -> Result<Self::Value, ValueError>;
    /// Publish before the original search List's native hold is released.
    fn jim_search_publish(&mut self, value: &Self::Value) -> Result<Self::Value, ValueError>;
    /// Borrow the current result for Jim inline no-match semantics.
    fn jim_search_current_result(&self) -> Result<Self::Value, ValueError>;
}

/// A guest packet, original callback completion, negative match or usage failure.
pub enum JimLsearchError<E> {
    /// Genuine selected guest failure or typed outer host refusal.
    Command(CmdError),
    /// The actual callback completion, without result reconstruction.
    Callback(E),
    /// A negative integer callback result; the current result is preserved.
    NegativeMatch,
    /// Native argument-count failure, presented by the selected command adapter.
    Usage,
}
impl<E> From<CmdError> for JimLsearchError<E> {
    fn from(error: CmdError) -> Self {
        Self::Command(error)
    }
}
impl<E> From<ValueError> for JimLsearchError<E> {
    fn from(error: ValueError) -> Self {
        Self::Command(error.into())
    }
}

fn null_error(message: impl Into<Vec<u8>>) -> CmdError {
    CmdError::new_bytes(message.into())
}
fn index_error<O: NativeJimLsearchObjects>(
    ops: &mut O,
    index: &O::Value,
    list: &O::Value,
    encoded: i32,
    length: usize,
) -> Result<CmdError, ValueError> {
    let bytes = ops.jim_search_bytes(index)?;
    let mut message = if encoded < 0 || usize::try_from(encoded).is_ok_and(|index| index > length) {
        b"index \"".to_vec()
    } else {
        b"element ".to_vec()
    };
    message.extend_from_slice(tcl_core_types::c_string_extent(&bytes));
    if encoded < 0 || usize::try_from(encoded).is_ok_and(|index| index > length) {
        message.extend_from_slice(b"\" out of range");
    } else {
        message.extend_from_slice(b" missing from sublist \"");
        let bytes = ops.jim_search_bytes(list)?;
        message.extend_from_slice(tcl_core_types::c_string_extent(&bytes));
        message.push(b'"');
    }
    Ok(null_error(message))
}

/// Evaluate one genuine Jim lsearch invocation. Callback values are full native
/// integers; a callback failure remains the original completion for the adapter.
pub fn lsearch<O, E>(
    ops: &mut O,
    args: &[O::Value],
    mut command_match: impl FnMut(&mut O, &O::Value, &O::Value, &O::Value, bool, bool) -> Result<i64, E>,
) -> Result<O::Value, JimLsearchError<E>>
where
    O: NativeJimLsearchObjects,
{
    if ops.jim_regex_recipe()?.is_none() {
        return Err(ValueError::CommandProtocolUnavailable("Jim lsearch issuer").into());
    }
    let SearchArguments {
        source,
        pattern,
        command,
        index,
        mode,
        flags,
        all,
        stride,
    } = search_arguments(ops, args)?;
    let length = ops.jim_search_elements(source)?.len();
    if length % stride != 0 {
        return Err(
            null_error(b"list size must be a multiple of the stride length".to_vec()).into(),
        );
    }
    let mut result = all.then(|| ops.jim_search_begin());
    let fresh_command = (mode == 5).then(|| ops.jim_search_regexp_command());
    let command = if mode == 5 {
        fresh_command.as_ref()
    } else {
        command
    };
    let _command_hold = command.map(|head| ops.jim_search_hold(head)).transpose()?;
    let selection = Selection {
        pattern,
        command,
        mode,
        flags,
        stride,
    };
    for offset in (0..length).step_by(stride) {
        // Native loops reconvert the original List after callbacks. The fixed
        // initial extent must still be backed by actual current storage.
        let members = ops.jim_search_elements(source)?;
        let Some(group) = members.get(offset..offset + stride) else {
            return Err(
                ValueError::CommandProtocolUnavailable("Jim lsearch List changed extent").into(),
            );
        };
        if let Some(value) = search_group(
            ops,
            &selection,
            &SearchGroup {
                source,
                members: group,
                index,
                offset,
            },
            &mut result,
            &mut command_match,
        )? {
            return Ok(value);
        }
    }
    if let Some(result) = result {
        return Ok(ops.jim_search_finish(result)?);
    }
    if flags.contains(SearchFlags::INLINE) {
        return Ok(ops.jim_search_current_result()?);
    }
    let value = if flags.contains(SearchFlags::BOOLEAN) {
        // Jim_NewIntObj, including the -bool no-match path, owns an integer
        // primary rather than the backend's abstract Boolean representation.
        ops.new_int(i64::from(flags.contains(SearchFlags::INVERTED)))
    } else {
        ops.new_int(-1)
    };
    Ok(ops.jim_search_publish(&value)?)
}

struct SearchArguments<'a, V> {
    source: &'a V,
    pattern: &'a V,
    command: Option<&'a V>,
    index: Option<&'a V>,
    mode: usize,
    flags: SearchFlags,
    all: bool,
    stride: usize,
}
fn search_arguments<'a, O, E>(
    ops: &mut O,
    args: &'a [O::Value],
) -> Result<SearchArguments<'a, O::Value>, JimLsearchError<E>>
where
    O: NativeJimLsearchObjects,
{
    if args.len() < 2 {
        return Err(JimLsearchError::Usage);
    }
    let mut boolean = false;
    let mut inverted = false;
    let mut nocase = false;
    let mut all = false;
    let mut inline = false;
    let mut mode = 3;
    let mut command = None;
    let mut index = None;
    let mut option_end = false;
    let mut stride = 1usize;
    let mut cursor = 0;
    while cursor < args.len() - 2 {
        let selected = ops.jim_search_option(&args[cursor])?;
        match selected {
            0 => {
                boolean = true;
                inline = false;
            }
            1 => inverted = true,
            2 => nocase = true,
            3 => mode = 3,
            4 => mode = 4,
            5 => {
                mode = 5;
                option_end = true;
            }
            6 => all = true,
            7 => {
                inline = true;
                boolean = false;
            }
            8 | 10 => {
                cursor += 1;
                let Some(value) = args.get(cursor) else {
                    return Err(JimLsearchError::Usage);
                };
                if selected == 8 {
                    mode = 8;
                    command = Some(value);
                } else {
                    index = Some(value);
                }
            }
            9 => {
                cursor += 1;
                let Some(value) = args.get(cursor) else {
                    return Err(JimLsearchError::Usage);
                };
                let amount = ops.as_int(value)?;
                if amount < 1 {
                    return Err(null_error(b"stride length must be at least 1".to_vec()).into());
                }
                stride = usize::try_from(amount).map_err(|_| {
                    ValueError::CommandProtocolUnavailable("Jim native long stride extent")
                })?;
            }
            _ => {
                return Err(
                    ValueError::CommandProtocolUnavailable("Jim lsearch option receipt").into(),
                );
            }
        }
        cursor += 1;
    }
    if args.len() - cursor < 2 {
        return Err(JimLsearchError::Usage);
    }
    Ok(SearchArguments {
        source: &args[cursor],
        pattern: &args[cursor + 1],
        command,
        index,
        mode,
        flags: SearchFlags::from_options([nocase, option_end, boolean, inverted, inline]),
        all,
        stride,
    })
}

struct SearchGroup<'a, V> {
    source: &'a V,
    members: &'a [V],
    index: Option<&'a V>,
    offset: usize,
}

fn search_group<O, E>(
    ops: &mut O,
    selection: &Selection<'_, O::Value>,
    group: &SearchGroup<'_, O::Value>,
    result: &mut Option<O::Accumulator>,
    command_match: &mut impl FnMut(
        &mut O,
        &O::Value,
        &O::Value,
        &O::Value,
        bool,
        bool,
    ) -> Result<i64, E>,
) -> Result<Option<O::Value>, JimLsearchError<E>>
where
    O: NativeJimLsearchObjects,
{
    let SearchGroup {
        source,
        members: group,
        index,
        offset,
    } = *group;
    let stride = selection.stride;
    let group_object;
    let search;
    let mut candidate;
    if let Some(index) = index {
        let indices = ops.jim_search_elements(index)?;
        group_object = if stride == 1 {
            None
        } else {
            Some(ops.jim_search_group(group.to_vec()))
        };
        search = if let Some(group) = &group_object {
            group
        } else {
            &group[0]
        };
        let _search_hold = ops.jim_search_hold(search)?;
        // All original Index objects are converted before any list descent.
        let encoded: Vec<_> = indices
            .iter()
            .map(|index| ops.jim_regex_index(index))
            .collect::<Result<_, _>>()?;
        candidate = ops.jim_search_borrow(search);
        for (index, encoded) in indices.iter().zip(encoded) {
            let elements = ops.jim_search_elements(&candidate)?;
            let element_count = elements.len();
            let position = if encoded < 0 && encoded > -i32::MAX {
                i64::try_from(elements.len())
                    .map_err(|_| ValueError::CommandProtocolUnavailable("Jim List extent"))?
                    + i64::from(encoded)
            } else {
                i64::from(encoded)
            };
            let Some(element) = usize::try_from(position)
                .ok()
                .and_then(|position| elements.into_iter().nth(position))
            else {
                return Err(index_error(ops, index, &candidate, encoded, element_count)?.into());
            };
            candidate = element;
        }
        // Process while the exact search object remains pinned.
        compare_and_select(
            ops,
            selection,
            &Position {
                candidate: &candidate,
                offset,
                search,
                result_offset: 0,
            },
            result,
            command_match,
        )
    } else {
        search = source;
        let _search_hold = ops.jim_search_hold(search)?;
        compare_and_select(
            ops,
            selection,
            &Position {
                candidate: &group[0],
                offset,
                search,
                result_offset: offset,
            },
            result,
            command_match,
        )
    }
}

struct Selection<'a, V> {
    pattern: &'a V,
    command: Option<&'a V>,
    mode: usize,
    flags: SearchFlags,
    stride: usize,
}
#[derive(Clone, Copy)]
struct SearchFlags(u8);
impl SearchFlags {
    const NOCASE: u8 = 1;
    const OPTION_END: u8 = 2;
    const BOOLEAN: u8 = 4;
    const INVERTED: u8 = 8;
    const INLINE: u8 = 16;
    fn from_options(selected: [bool; 5]) -> Self {
        Self(
            [
                Self::NOCASE,
                Self::OPTION_END,
                Self::BOOLEAN,
                Self::INVERTED,
                Self::INLINE,
            ]
            .into_iter()
            .zip(selected)
            .fold(0, |flags, (flag, enabled)| {
                flags | if enabled { flag } else { 0 }
            }),
        )
    }
    fn contains(self, flag: u8) -> bool {
        self.0 & flag != 0
    }
}
struct Position<'a, V> {
    candidate: &'a V,
    offset: usize,
    search: &'a V,
    result_offset: usize,
}
fn compare_and_select<O, E>(
    ops: &mut O,
    selection: &Selection<'_, O::Value>,
    position: &Position<'_, O::Value>,
    result: &mut Option<O::Accumulator>,
    command_match: &mut impl FnMut(
        &mut O,
        &O::Value,
        &O::Value,
        &O::Value,
        bool,
        bool,
    ) -> Result<i64, E>,
) -> Result<Option<O::Value>, JimLsearchError<E>>
where
    O: NativeJimLsearchObjects,
{
    let Selection { flags, stride, .. } = *selection;
    let boolean = flags.contains(SearchFlags::BOOLEAN);
    let inverted = flags.contains(SearchFlags::INVERTED);
    let inline = flags.contains(SearchFlags::INLINE);
    let Position {
        candidate,
        offset,
        search,
        result_offset,
    } = *position;
    let all = result.is_some();
    let eq = compare_candidate(ops, selection, candidate, command_match)?;
    if eq < 0 {
        return Err(JimLsearchError::NegativeMatch);
    }
    if (!boolean && eq == i64::from(!inverted)) || (boolean && (eq != 0 || all)) {
        if inline && stride > 1 {
            let elements = ops.jim_search_current_elements(search)?;
            let members = elements.get(result_offset..result_offset + stride).ok_or(
                ValueError::CommandProtocolUnavailable(
                    "Jim lsearch callback changed selected group",
                ),
            )?;
            if let Some(result) = result {
                ops.jim_search_append(result, members)?;
            } else {
                let value = ops.new_list(members.to_vec());
                return Ok(Some(ops.jim_search_publish(&value)?));
            }
        } else {
            let value = if boolean {
                ops.new_int(eq ^ i64::from(inverted))
            } else if !inline {
                ops.new_int(i64::try_from(offset).map_err(|_| {
                    ValueError::CommandProtocolUnavailable("Jim lsearch result index")
                })?)
            } else {
                return if let Some(result) = result {
                    ops.jim_search_append(result, std::slice::from_ref(candidate))?;
                    Ok(None)
                } else {
                    Ok(Some(ops.jim_search_publish(candidate)?))
                };
            };
            if let Some(result) = result {
                if stride == 1 {
                    ops.jim_search_append(result, std::slice::from_ref(&value))?;
                }
            } else {
                return Ok(Some(ops.jim_search_publish(&value)?));
            }
        }
    }
    Ok(None)
}

fn compare_candidate<O, E>(
    ops: &mut O,
    selection: &Selection<'_, O::Value>,
    candidate: &O::Value,
    command_match: &mut impl FnMut(
        &mut O,
        &O::Value,
        &O::Value,
        &O::Value,
        bool,
        bool,
    ) -> Result<i64, E>,
) -> Result<i64, JimLsearchError<E>>
where
    O: NativeJimLsearchObjects,
{
    let nocase = selection.flags.contains(SearchFlags::NOCASE);
    let option_end = selection.flags.contains(SearchFlags::OPTION_END);
    match selection.mode {
        3 => {
            let pattern_bytes = ops.jim_search_bytes(selection.pattern)?;
            let pattern_count = ops.jim_search_character_count(selection.pattern)?;
            let subject_bytes = ops.jim_search_bytes(candidate)?;
            let subject_count = ops.jim_search_character_count(candidate)?;
            Ok(i64::from(
                tcl_syntax::raw_string::RawString::from_bytes(pattern_bytes)
                    .jim084_compare(
                        pattern_count,
                        &tcl_syntax::raw_string::RawString::from_bytes(subject_bytes),
                        subject_count,
                        nocase,
                    )
                    .map_err(ValueError::from)?
                    .is_eq(),
            ))
        }
        4 => {
            let pattern = ops.jim_search_bytes(selection.pattern)?;
            let subject = ops.jim_search_bytes(candidate)?;
            Ok(i64::from(
                tcl_syntax::raw_string::RawString::from_bytes(subject)
                    .jim084_matches(
                        &tcl_syntax::raw_string::RawString::from_bytes(pattern),
                        nocase,
                    )
                    .map_err(ValueError::from)?,
            ))
        }
        5 | 8 => command_match(
            ops,
            selection
                .command
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "Jim lsearch original command",
                ))?,
            selection.pattern,
            candidate,
            nocase,
            option_end,
        )
        .map_err(JimLsearchError::Callback),
        _ => Err(ValueError::CommandProtocolUnavailable("Jim lsearch match recipe").into()),
    }
}

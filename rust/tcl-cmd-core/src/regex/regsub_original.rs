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

//! `regsub` over original objects: option preparation, native Unicode substitution and the C9 command prefix.

use super::original_pattern::{
    NativeRegexObjects, NativeRegexSource, native_compiled_pattern, native_regex_characters,
};
use super::regexp_command::OriginalOptionArguments;
use super::regsub_command::{
    REGSUB_USAGE, RegsubError, RunFailure, regsub_option_scan, regsub_selected,
};
use super::{
    Common, NO_MATCH, RegMatch, RegexEngine, RegexError, Run, completed_matches, decode_utf8,
    notbol_at, resolve_start_checked, wrong_args,
};
use tcl_dialect::TclVersion;
use tcl_syntax::value::{ValueError, ValueOps};

/// Original-object option preparation retained through callback evaluation.
/// This packet owns no replacement option values and cannot grant Index state.
pub struct OriginalRegsubPreparation<'a, V> {
    version: TclVersion,
    common: Common,
    pub(super) command: bool,
    rest: Vec<Vec<u8>>,
    pub(super) target: Option<&'a V>,
    originals: &'a [V],
    pub(super) start: usize,
    recipe: Option<tcl_syntax::native_regex::NativeRegexpRecipe>,
}

/// Scan actual original options once before borrowing the adapter for callbacks.
pub fn regsub_prepare_original<'a, O: NativeRegexSource>(
    ops: &mut O,
    args: &'a [O::Value],
    version: TclVersion,
) -> Result<OriginalRegsubPreparation<'a, O::Value>, RegexError> {
    crate::index::run_index_output(
        ops,
        |ops| {
            let (common, command, offset) =
                regsub_option_scan(&mut OriginalOptionArguments { ops, args }, version)?;
            let originals = &args[offset..];
            if originals.len() < 3 || originals.len() > 4 {
                return Err(wrong_args(REGSUB_USAGE));
            }
            let recipe = ops
                .regex_recipe()
                .map_err(|error| RegexError::from(crate::CmdError::from(error)))?;
            let command_start = if recipe.is_some() {
                common
                    .start
                    .as_ref()
                    .map(|spec| {
                        let length = ops
                            .index_character_count(&originals[1])
                            .map_err(RegexError::from)?;
                        resolve_start_checked(ops, spec, length)
                    })
                    .transpose()?
            } else {
                None
            };
            let rest = originals[..if recipe.is_some() { 0 } else { 3 }]
                .iter()
                .map(|value| {
                    ops.native_string_bytes(value)
                        .map(|bytes| bytes.to_vec())
                        .map_err(|error| RegexError::from(crate::CmdError::from(error)))
                })
                .collect::<Result<Vec<_>, _>>()?;
            let start = if recipe.is_some() {
                command_start.unwrap_or(0)
            } else {
                let (characters, _) = decode_utf8(&rest[1]);
                common.start.as_ref().map_or(Ok(0), |spec| {
                    resolve_start_checked(ops, spec, characters.len())
                })?
            };
            Ok(OriginalRegsubPreparation {
                version,
                common,
                command,
                rest,
                target: originals.get(3),
                originals,
                start,
                recipe,
            })
        },
        |_, metadata| drop(metadata),
        RegexError::from,
    )
}

/// Result text and a borrowed same-original output target. The argv owner
/// keeps the target alive; this transport adds no native object reference.
pub struct OriginalRegsubResult<'a, V> {
    pub text: Vec<u8>,
    pub count: i64,
    pub target: Option<&'a V>,
}

/// Concrete physical edges for C9 command-prefix substitution. Object
/// transports retain lifetime separately from actual native references.
pub trait NativeRegsubObjects: ValueOps {
    type Object;
    type Error;
    /// Construct an actual Jim byte result or fresh callback argument.
    fn regex_jim_bytes(
        &mut self,
        _bytes: &[u8],
        _string_primary: bool,
    ) -> Result<Self::Object, ValueError> {
        Err(ValueError::CommandProtocolUnavailable(
            "original Jim regsub String producer",
        ))
    }
    /// Borrow the current original callback result and reach its Jim getter.
    fn regex_result_bytes(&mut self) -> Result<std::rc::Rc<[u8]>, ValueError> {
        Err(ValueError::CommandProtocolUnavailable(
            "original Jim regsub callback result",
        ))
    }
    fn regex_object<'a>(&self, value: &'a Self::Object) -> &'a Self::Value;
    fn regex_borrow(&self, value: &Self::Value) -> Self::Object;
    fn regex_duplicate(
        &mut self,
        value: &Self::Value,
    ) -> Result<Self::Object, tcl_syntax::value::ValueError>;
    fn regex_members(
        &mut self,
        value: &Self::Value,
    ) -> Result<Vec<Self::Object>, tcl_syntax::value::ValueError>;
    fn regex_unicode(
        &mut self,
        units: &[u32],
    ) -> Result<Self::Object, tcl_syntax::value::ValueError>;
    fn regex_append_unicode(
        &mut self,
        result: &Self::Object,
        units: &[u32],
    ) -> Result<(), tcl_syntax::value::ValueError>;
    fn regex_append_current_result(
        &mut self,
        result: &mut Self::Object,
    ) -> Result<(), tcl_syntax::value::ValueError>;
    fn regex_eval(
        &mut self,
        prefix: &Self::Value,
        arguments: &[Self::Object],
    ) -> Result<(), Self::Error>;
    fn regex_reset_result(&mut self) -> Result<(), tcl_syntax::value::ValueError>;
}

/// A real command callback completion is transported separately from parser errors.
pub enum OriginalRegexConsumerError<E, D = crate::CmdError> {
    Command(D),
    Callback(E),
}

/// Native result retains the genuine object producer and the same output target.
pub struct NativeRegsubResult<'a, V, T> {
    pub result: T,
    pub count: i64,
    pub target: Option<&'a V>,
}

/// Original substitution result and guest callback completion for one adapter.
pub type NativeRegsubOutcome<'a, O> = Result<
    NativeRegsubResult<'a, <O as ValueOps>::Value, <O as NativeRegsubObjects>::Object>,
    RegsubError<<O as NativeRegsubObjects>::Error>,
>;

impl<V> OriginalRegsubPreparation<'_, V> {
    /// Whether the actual selected option table entered command-prefix mode.
    #[must_use]
    pub fn is_command(&self) -> bool {
        self.command
    }
    #[must_use]
    pub fn is_native(&self) -> bool {
        self.recipe.is_some()
    }
}

type RegsubAccumulation<T, E> = Result<(Option<T>, i64), RegsubError<E>>;

fn native_literal_substitution<O: NativeRegsubObjects + NativeRegexSource>(
    ops: &mut O,
    prepared: &OriginalRegsubPreparation<'_, O::Value>,
    recipe: tcl_syntax::native_regex::NativeRegexpRecipe,
) -> RegsubAccumulation<O::Object, O::Error> {
    let host = |error| RegsubError::Regex(RegexError::from(crate::CmdError::from(error)));
    let original = prepared.originals;
    let mut count = 0;
    let mut result = None;
    let pattern = ops.native_unicode_units(&original[0]).map_err(host)?;
    let subject = ops.native_unicode_units(&original[1]).map_err(host)?;
    let substitution = ops.native_unicode_units(&original[2]).map_err(host)?;
    let mut output = Vec::new();
    if pattern.is_empty() {
        for &unit in subject.iter() {
            output.extend_from_slice(&substitution);
            output.push(unit);
            count += 1;
        }
    } else {
        let mut cursor = 0;
        let mut copied = 0;
        while cursor <= subject.len().saturating_sub(pattern.len())
            && pattern.len() <= subject.len()
        {
            if recipe.equal_units(
                &subject[cursor..cursor + pattern.len()],
                &pattern,
                prepared.common.flags.nocase,
            ) {
                output.extend_from_slice(&subject[copied..cursor]);
                output.extend_from_slice(&substitution);
                cursor += pattern.len();
                copied = cursor;
                count += 1;
            } else {
                cursor += 1;
            }
        }
        if count != 0 {
            output.extend_from_slice(&subject[copied..]);
        }
    }
    if count != 0 {
        let accumulator = ops.regex_unicode(&[]).map_err(host)?;
        ops.regex_append_unicode(&accumulator, &output)
            .map_err(host)?;
        result = Some(accumulator);
    }
    Ok((result, count))
}

fn native_regsub_matches<O: NativeRegsubObjects, E: RegexEngine>(
    ops: &mut O,
    prepared: &OriginalRegsubPreparation<'_, O::Value>,
    compiled: &std::rc::Rc<std::cell::RefCell<E::Regex>>,
    units: &[u32],
    replacement: &[u32],
    characters: &[i32],
) -> RegsubAccumulation<O::Object, O::Error> {
    let host = |error| RegsubError::Regex(RegexError::from(crate::CmdError::from(error)));
    let mut count = 0;
    let mut result = None;
    let mut offset = prepared.start;
    while offset <= units.len() {
        let matches = E::exec(
            &mut compiled.borrow_mut(),
            characters,
            offset,
            notbol_at(characters, offset),
        );
        let matches = completed_matches(matches).map_err(RegsubError::Regex)?;
        let Some(matches) = matches else {
            break;
        };
        let whole = matches.first().copied().ok_or_else(|| {
            host(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native regsub match geometry",
            ))
        })?;
        if whole.so < offset || whole.so > whole.eo || whole.eo > units.len() {
            return Err(host(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "native regsub match geometry",
                ),
            ));
        }
        if result.is_none() {
            let accumulator = ops.regex_unicode(&[]).map_err(host)?;
            ops.regex_append_unicode(&accumulator, &units[..offset])
                .map_err(host)?;
            result = Some(accumulator);
        }
        let accumulator = result.as_ref().expect("matched native result");
        ops.regex_append_unicode(accumulator, &units[offset..whole.so])
            .map_err(host)?;
        let expanded = native_substitution_units(replacement, units, &matches).map_err(host)?;
        ops.regex_append_unicode(accumulator, &expanded)
            .map_err(host)?;
        count += 1;
        offset = whole.eo;
        if whole.eo == whole.so {
            if offset < units.len() {
                ops.regex_append_unicode(accumulator, &units[offset..=offset])
                    .map_err(host)?;
            }
            offset += 1;
        }
        if !prepared.common.all {
            break;
        }
    }
    if let Some(result) = &result
        && offset < units.len()
    {
        ops.regex_append_unicode(result, &units[offset..])
            .map_err(host)?;
    }
    Ok((result, count))
}

/// Original C regsub instruction and command result over native Unicode.
/// The literal mapping route preserves its distinct original String primaries.
pub fn regsub_native_original<
    'a,
    O: NativeRegsubObjects + NativeRegexObjects<E>,
    E: RegexEngine,
>(
    ops: &mut O,
    prepared: &OriginalRegsubPreparation<'a, O::Value>,
) -> NativeRegsubOutcome<'a, O> {
    if prepared.command {
        return regsub_command_original::<O, E>(ops, prepared);
    }
    let host = |error| RegsubError::Regex(RegexError::from(crate::CmdError::from(error)));
    let recipe = prepared.recipe.ok_or_else(|| {
        host(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "native regsub source issuer",
        ))
    })?;
    let original = prepared.originals;
    let mapping = if prepared.common.all && prepared.start == 0 {
        let substitution = ops.native_string_bytes(&original[2]).map_err(host)?;
        if recipe.literal_mapping_substitution(&substitution) {
            let pattern = ops.native_string_bytes(&original[0]).map_err(host)?;
            recipe.literal_mapping(true, 0, &pattern, &substitution)
        } else {
            false
        }
    } else {
        false
    };
    let (result, count) = if mapping {
        native_literal_substitution(ops, prepared, recipe)?
    } else {
        let compiled = native_compiled_pattern::<O, E>(
            ops,
            &original[0],
            recipe,
            prepared.common.flags,
            prepared.version,
        )
        .map_err(RegsubError::Regex)?;
        let subject = if ops.same_object(&original[1], &original[0]).ok_or_else(|| {
            host(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native regsub subject identity",
            ))
        })? {
            ops.regex_duplicate(&original[1]).map_err(host)?
        } else {
            ops.regex_borrow(&original[1])
        };
        let units = ops
            .native_unicode_units(ops.regex_object(&subject))
            .map_err(host)?;
        let substitution = if ops.same_object(&original[2], &original[0]).ok_or_else(|| {
            host(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native regsub substitution identity",
            ))
        })? {
            ops.regex_duplicate(&original[2]).map_err(host)?
        } else {
            ops.regex_borrow(&original[2])
        };
        let replacement = ops
            .native_unicode_units(ops.regex_object(&substitution))
            .map_err(host)?;
        let characters =
            native_regex_characters(&units, "native regsub character width").map_err(host)?;
        native_regsub_matches::<O, E>(ops, prepared, &compiled, &units, &replacement, &characters)?
    };
    Ok(NativeRegsubResult {
        result: match result {
            Some(result) => result,
            None => ops.regex_borrow(&original[1]),
        },
        count,
        target: prepared.target,
    })
}

fn native_substitution_units(
    specification: &[u32],
    subject: &[u32],
    matches: &[RegMatch],
) -> Result<Vec<u32>, tcl_syntax::value::ValueError> {
    let mut output = Vec::new();
    let mut cursor = 0;
    while cursor < specification.len() {
        let unit = specification[cursor];
        let mut group = (unit == u32::from(b'&')).then_some(0);
        if unit == u32::from(b'\\')
            && let Some(&next) = specification.get(cursor + 1)
        {
            if (u32::from(b'0')..=u32::from(b'9')).contains(&next) {
                group = Some((next - u32::from(b'0')) as usize);
                cursor += 1;
            } else if matches!(next, 38 | 92) {
                output.push(next);
                cursor += 2;
                continue;
            }
        }
        if let Some(group) = group {
            if let Some(span) = matches.get(group)
                && span.so != NO_MATCH
            {
                let range = subject.get(span.so..span.eo).ok_or(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "native regsub submatch geometry",
                    ),
                )?;
                output.extend_from_slice(range);
            }
        } else {
            output.push(unit);
        }
        cursor += 1;
    }
    Ok(output)
}

struct RegsubCommandInputs<T, R> {
    prefix: T,
    subject: T,
    pub(super) compiled: std::rc::Rc<std::cell::RefCell<R>>,
}

type RegsubCommandPreparation<O, E> = Result<
    RegsubCommandInputs<<O as NativeRegsubObjects>::Object, <E as RegexEngine>::Regex>,
    RegsubError<<O as NativeRegsubObjects>::Error>,
>;

fn prepare_regsub_command<O: NativeRegsubObjects + NativeRegexObjects<E>, E: RegexEngine>(
    ops: &mut O,
    prepared: &OriginalRegsubPreparation<'_, O::Value>,
) -> RegsubCommandPreparation<O, E> {
    let host = |error| RegsubError::Regex(RegexError::from(crate::CmdError::from(error)));
    let original = prepared.originals;
    let recipe = ops.regex_recipe().map_err(host)?.ok_or_else(|| {
        host(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "native regsub compiled pattern issuer",
        ))
    })?;
    let compiled = native_compiled_pattern::<O, E>(
        ops,
        &original[0],
        recipe,
        prepared.common.flags,
        prepared.version,
    )
    .map_err(RegsubError::Regex)?;
    let members = ops.regex_members(&original[2]).map_err(host)?;
    if members.is_empty() {
        return Err(RegsubError::Regex(RegexError::from(
            crate::CmdError::with_error_code_bytes(
                b"command prefix must be a list of at least one element",
                b"TCL OPERATION REGSUB CMDEMPTY",
            ),
        )));
    }
    drop(members);
    // Prefix conversion can replace the original pattern representation.
    drop(compiled);
    let compiled = native_compiled_pattern::<O, E>(
        ops,
        &original[0],
        recipe,
        prepared.common.flags,
        prepared.version,
    )
    .map_err(RegsubError::Regex)?;
    let subject = if ops.same_object(&original[1], &original[0]).ok_or_else(|| {
        host(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "original regsub subject identity",
        ))
    })? {
        ops.regex_duplicate(&original[1]).map_err(host)?
    } else {
        ops.regex_borrow(&original[1])
    };
    let prefix = if ops.same_object(&original[2], &original[0]).ok_or_else(|| {
        host(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
            "original regsub prefix identity",
        ))
    })? {
        ops.regex_duplicate(&original[2]).map_err(host)?
    } else {
        ops.regex_borrow(&original[2])
    };
    Ok(RegsubCommandInputs {
        prefix,
        subject,
        compiled,
    })
}

fn native_callback_arguments<O: NativeRegsubObjects>(
    ops: &mut O,
    nsubs: usize,
    matches: &[RegMatch],
    units: &[u32],
) -> Result<Vec<O::Object>, ValueError> {
    (0..=nsubs)
        .map(|index| {
            let capture_units = match matches.get(index) {
                Some(span)
                    if span.so != NO_MATCH && span.so <= span.eo && span.eo <= units.len() =>
                {
                    &units[span.so..span.eo]
                }
                Some(span) if span.so != NO_MATCH => {
                    return Err(ValueError::CommandProtocolUnavailable(
                        "native regex submatch geometry",
                    ));
                }
                _ => &[],
            };
            ops.regex_unicode(capture_units)
        })
        .collect()
}

/// C9 prefix substitution over original Unicode storage and List members.
/// Subject/prefix aliasing to the pattern selects genuine duplicates. Callback
/// result append precedes result reset and original subject Unicode refetch.
pub fn regsub_command_original<
    'a,
    O: NativeRegsubObjects + NativeRegexObjects<E>,
    E: RegexEngine,
>(
    ops: &mut O,
    prepared: &OriginalRegsubPreparation<'a, O::Value>,
) -> NativeRegsubOutcome<'a, O> {
    let host = |error| RegsubError::Regex(RegexError::from(crate::CmdError::from(error)));
    if !prepared.command || prepared.version < TclVersion::V9_0 {
        return Err(host(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native C9 regsub command prefix",
            ),
        ));
    }
    let original = prepared.originals;
    let RegsubCommandInputs {
        compiled,
        subject,
        prefix,
    } = prepare_regsub_command::<O, E>(ops, prepared)?;
    let mut units = ops
        .native_unicode_units(ops.regex_object(&subject))
        .map_err(host)?;
    let mut offset = prepared.start;
    let nsubs = E::nsub(&compiled.borrow());
    let mut result = None;
    let mut count = 0;
    while offset <= units.len() {
        let characters =
            native_regex_characters(&units, "native regex character width").map_err(host)?;
        let notbol = offset > 0 && units[offset - 1] != u32::from(b'\n');
        let matches = completed_matches(E::exec(
            &mut compiled.borrow_mut(),
            &characters,
            offset,
            notbol,
        ))
        .map_err(RegsubError::Regex)?;
        let Some(matches) = matches else {
            break;
        };
        let whole = matches.first().copied().ok_or_else(|| {
            host(tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "native regex match geometry",
            ))
        })?;
        if whole.so < offset || whole.eo < whole.so || whole.eo > units.len() {
            return Err(host(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "native regex match geometry",
                ),
            ));
        }
        if result.is_none() {
            let accumulator = ops.regex_unicode(&[]).map_err(host)?;
            ops.regex_append_unicode(&accumulator, &units[..offset])
                .map_err(host)?;
            result = Some(accumulator);
        }
        let accumulator = result.as_mut().expect("matched result owner");
        ops.regex_append_unicode(accumulator, &units[offset..whole.so])
            .map_err(host)?;
        let arguments = native_callback_arguments(ops, nsubs, &matches, &units).map_err(host)?;
        ops.regex_eval(ops.regex_object(&prefix), &arguments)
            .map_err(RegsubError::Eval)?;
        drop(arguments);
        ops.regex_append_current_result(accumulator).map_err(host)?;
        ops.regex_reset_result().map_err(host)?;
        units = ops
            .native_unicode_units(ops.regex_object(&subject))
            .map_err(host)?;
        count += 1;
        offset = whole.eo;
        if whole.eo == whole.so {
            if offset < units.len() {
                ops.regex_append_unicode(accumulator, &units[offset..=offset])
                    .map_err(host)?;
            }
            offset += 1;
        }
        if !prepared.common.all {
            break;
        }
    }
    let result = match result {
        Some(result) => {
            if offset < units.len() {
                ops.regex_append_unicode(&result, &units[offset..])
                    .map_err(host)?;
            }
            result
        }
        None => ops.regex_borrow(&original[1]),
    };
    Ok(NativeRegsubResult {
        result,
        count,
        target: prepared.target,
    })
}

/// Execute the already selected original option plan without another scan.
pub fn regsub_eval_original<'a, E: RegexEngine, Err, V>(
    prepared: &OriginalRegsubPreparation<'a, V>,
    eval: impl FnMut(&[Vec<u8>]) -> Result<Vec<u8>, Err>,
) -> Result<OriginalRegsubResult<'a, V>, RegsubError<Err>>
where
    E::Regex: 'static,
{
    if prepared.command {
        return Err(RegsubError::Regex(RegexError::from(crate::CmdError::from(
            tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                "original native regsub callback owner",
            ),
        ))));
    }
    let rest = prepared.rest.iter().map(Vec::as_slice).collect::<Vec<_>>();
    let result = regsub_selected::<E, Err>(
        &rest,
        prepared.version,
        prepared.start,
        &prepared.common,
        prepared.command,
        eval,
        &mut Run::Runtime,
    )
    .map_err(|failure| match failure {
        RunFailure::Regex(error) => RegsubError::Regex(error),
        RunFailure::Eval(error) => RegsubError::Eval(error),
        RunFailure::Declined(decline) => RegsubError::Regex(decline.into_error()),
    })?;
    Ok(OriginalRegsubResult {
        text: result.text,
        count: result.count,
        target: prepared.target,
    })
}

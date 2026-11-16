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

//! Jim's `regexp` and `regsub` over original objects: options, captures and substitution callbacks.

use super::original_pattern::{
    NativeRegexObjects, NativeRegexSource, PreparedOriginalRegex, prepare_pattern_original,
};
use super::regexp_command::{REGEXP_USAGE, RegexpExecutionError, regex_execution_error};
use super::regsub_command::{REGSUB_USAGE, RegsubError};
use super::regsub_original::{NativeRegsubObjects, NativeRegsubOutcome, NativeRegsubResult};
use super::{
    NO_MATCH, RegMatch, RegexEngine, RegexError, RegexFlags, RegexpResult, UNMATCHED, wrong_args,
};
use tcl_dialect::TclVersion;
use tcl_syntax::value::ValueError;

static JIM_REGEXP_NAMES: [&str; 10] = [
    "-indices",
    "-nocase",
    "-line",
    "-linestop",
    "-lineanchor",
    "-all",
    "-inline",
    "-start",
    "-expanded",
    "--",
];

static JIM_REGSUB_NAMES: [&str; 9] = [
    "-nocase",
    "-line",
    "-linestop",
    "-lineanchor",
    "-all",
    "-start",
    "-command",
    "-expanded",
    "--",
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum JimCaptureOutput {
    Strings,
    Indices,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum JimResultDelivery {
    Variables,
    Inline,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum JimReplacement {
    Template,
    Command,
}

struct JimOptions {
    pub(super) flags: RegexFlags,
    pub(super) all: bool,
    indices: JimCaptureOutput,
    inline: JimResultDelivery,
    pub(super) command: JimReplacement,
    pub(super) start: i32,
    pub(super) offset: usize,
}

fn jim_options<O: NativeRegexSource>(
    ops: &mut O,
    args: &[O::Value],
    version: TclVersion,
    substitution: bool,
) -> Result<JimOptions, RegexError> {
    let names = if substitution {
        JIM_REGSUB_NAMES.as_slice()
    } else {
        JIM_REGEXP_NAMES.as_slice()
    };
    let mut selected = JimOptions {
        flags: RegexFlags::for_release(version),
        all: false,
        indices: JimCaptureOutput::Strings,
        inline: JimResultDelivery::Variables,
        command: JimReplacement::Template,
        start: 0,
        offset: 0,
    };
    while selected.offset < args.len() {
        let bytes = ops
            .native_string_bytes(&args[selected.offset])
            .map_err(|error| RegexError::from(crate::CmdError::from(error)))?;
        if bytes.first() != Some(&b'-') {
            break;
        }
        let index = ops
            .jim_regex_option(&args[selected.offset], names)
            .map_err(RegexError::from)?;
        selected.offset += 1;
        match names[index] {
            "-indices" => selected.indices = JimCaptureOutput::Indices,
            "-nocase" => selected.flags.nocase = true,
            "-line" => {
                selected.flags.lineanchor = true;
                selected.flags.linestop = true;
            }
            "-linestop" => selected.flags.linestop = true,
            "-lineanchor" => selected.flags.lineanchor = true,
            "-all" => selected.all = true,
            "-inline" => selected.inline = JimResultDelivery::Inline,
            "-command" => selected.command = JimReplacement::Command,
            "-expanded" => selected.flags.expanded = true,
            "-start" => {
                let original = args.get(selected.offset).ok_or_else(|| {
                    wrong_args(if substitution {
                        REGSUB_USAGE
                    } else {
                        REGEXP_USAGE
                    })
                })?;
                selected.start = ops.jim_regex_index(original).map_err(RegexError::from)?;
                selected.offset += 1;
            }
            _ => break,
        }
    }
    Ok(selected)
}

/// Jim's `-start` index, with a negative index counted back from one past the end.
fn jim_adjusted_start(length: usize, index: i32) -> Result<i64, ValueError> {
    Ok(if index < 0 {
        i64::from(index)
            + i64::try_from(length)
                .map_err(|_| ValueError::CommandProtocolUnavailable("Jim regexp source length"))?
            + 1
    } else {
        i64::from(index)
    })
}

fn jim_byte_offset(bytes: &[u8], index: i32) -> Result<usize, ValueError> {
    let adjusted = jim_adjusted_start(bytes.len(), index)?;
    if adjusted <= 0 {
        return Ok(0);
    }
    let index = usize::try_from(adjusted)
        .map_err(|_| ValueError::CommandProtocolUnavailable("Jim regexp start width"))?;
    if index > bytes.len() {
        return Ok(bytes.len());
    }
    tcl_syntax::raw_string::RawString::from_bytes(bytes)
        .jim084_byte_offset(index)
        .map_err(Into::into)
}

fn jim_capture_value<O: NativeRegexObjects<E>, E: RegexEngine>(
    ops: &mut O,
    subject: &[u8],
    span: RegMatch,
    offset: usize,
    character_offset: i64,
    output: JimCaptureOutput,
) -> Result<O::Value, ValueError> {
    if span.so != NO_MATCH && (span.eo < span.so || span.eo > subject.len() - offset) {
        return Err(ValueError::CommandProtocolUnavailable(
            "Jim regexp capture geometry",
        ));
    }
    if output == JimCaptureOutput::Indices {
        let (start, end) = if span.so == NO_MATCH {
            (-1, -1)
        } else {
            let start = jim_character_count(&subject[offset..offset + span.so])?;
            let end = jim_character_count(&subject[offset..offset + span.eo])?;
            (start + character_offset, end + character_offset - 1)
        };
        let start = ops.new_int(start);
        let end = ops.new_int(end);
        Ok(ops.new_list(vec![start, end]))
    } else if span.so == NO_MATCH {
        ops.regex_jim_range(b"", false)
    } else {
        ops.regex_jim_range(&subject[offset + span.so..offset + span.eo], true)
    }
}

fn jim_character_count(bytes: &[u8]) -> Result<i64, ValueError> {
    i64::try_from(
        tcl_syntax::raw_string::RawString::from_bytes(bytes)
            .jim084_characters()
            .count(),
    )
    .map_err(|_| ValueError::CommandProtocolUnavailable("Jim regexp index width"))
}

fn advance_jim_regexp(
    subject: &[u8],
    whole: RegMatch,
    offset: &mut usize,
    character_offset: &mut i64,
) -> Result<(), ValueError> {
    if whole.eo != 0 {
        *character_offset += jim_character_count(&subject[*offset..*offset + whole.eo])?;
        *offset += whole.eo;
    } else {
        *offset += 1;
        *character_offset += 1;
    }
    Ok(())
}

fn jim_whole_match(matches: &[RegMatch], length: usize) -> Result<RegMatch, ValueError> {
    let whole = matches
        .first()
        .copied()
        .ok_or(ValueError::CommandProtocolUnavailable(
            "Jim regexp whole match geometry",
        ))?;
    if whole.so == NO_MATCH || whole.eo < whole.so || whole.eo > length {
        return Err(ValueError::CommandProtocolUnavailable(
            "Jim regexp byte match geometry",
        ));
    }
    Ok(whole)
}

pub(super) fn regexp_jim_original<O: NativeRegexObjects<E>, E: RegexEngine, Err>(
    ops: &mut O,
    args: &[O::Value],
    version: TclVersion,
    mut assign: impl FnMut(&mut O, &O::Value, O::Value) -> Result<(), Err>,
) -> Result<RegexpResult<O::Value>, RegexpExecutionError<Err>> {
    let selected = jim_options(ops, args, version, false).map_err(RegexpExecutionError::Regex)?;
    let original = &args[selected.offset..];
    if original.len() < 2 {
        return Err(RegexpExecutionError::Regex(wrong_args(REGEXP_USAGE)));
    }
    let prepared = prepare_pattern_original::<O, E>(ops, &original[0], selected.flags, version)
        .map_err(RegexpExecutionError::Regex)?;
    let PreparedOriginalRegex::Jim { artifact, .. } = prepared else {
        return Err(regex_execution_error(
            ValueError::CommandProtocolUnavailable("Jim regexp prepared artifact"),
        ));
    };
    let pattern = ops
        .native_string_bytes(&original[0])
        .map_err(regex_execution_error)?;
    let subject = ops
        .native_string_bytes(&original[1])
        .map_err(regex_execution_error)?;
    let vars = original.len() - 2;
    if selected.inline == JimResultDelivery::Inline && vars != 0 {
        return Err(RegexpExecutionError::Regex(RegexError::new(
            b"regexp match variables not allowed when using -inline".to_vec(),
        )));
    }
    let count = if selected.inline == JimResultDelivery::Inline {
        artifact
            .with_program(|re| E::nsub(re) + 1)
            .map_err(regex_execution_error)?
    } else {
        vars
    };
    let mut offset = jim_byte_offset(&subject, selected.start).map_err(regex_execution_error)?;
    let mut character_offset =
        jim_adjusted_start(subject.len(), selected.start).map_err(regex_execution_error)?;
    let mut notbol = selected.start != 0;
    let mut matches_count = 0i64;
    let mut output = Vec::new();
    loop {
        let matches = artifact
            .with_program(|re| E::exec_jim(re, &subject[offset..], count + 1, notbol))
            .map_err(regex_execution_error)?
            .map_err(|reason| {
                regex_execution_error(ValueError::CommandProtocolUnavailable(reason))
            })?;
        let Some(matches) = matches else {
            break;
        };
        let whole =
            jim_whole_match(&matches, subject.len() - offset).map_err(regex_execution_error)?;
        matches_count += 1;
        for index in 0..count {
            let span = matches.get(index).copied().unwrap_or(UNMATCHED);
            let value = jim_capture_value::<O, E>(
                ops,
                &subject,
                span,
                offset,
                character_offset,
                selected.indices,
            )
            .map_err(regex_execution_error)?;
            if selected.inline == JimResultDelivery::Inline {
                output.push(
                    ops.index_created_key(value)
                        .map_err(regex_execution_error)?,
                );
            } else {
                assign(ops, &original[index + 2], value)
                    .map_err(RegexpExecutionError::Assignment)?;
            }
        }
        if !selected.all
            || (pattern.first() == Some(&b'^') && !selected.flags.lineanchor)
            || subject.get(offset).is_none_or(|&byte| byte == 0)
        {
            break;
        }
        advance_jim_regexp(&subject, whole, &mut offset, &mut character_offset)
            .map_err(regex_execution_error)?;
        if subject.get(offset).is_none_or(|&byte| byte == 0) {
            break;
        }
        notbol = true;
    }
    if selected.inline == JimResultDelivery::Inline {
        let values = output
            .iter()
            .map(|owner| ops.index_key_argument(owner))
            .collect::<Result<Vec<_>, _>>()
            .map_err(regex_execution_error)?;
        Ok(RegexpResult::Inline(ops.new_list(values)))
    } else {
        Ok(RegexpResult::Count {
            assign: None,
            count: matches_count,
        })
    }
}

fn jim_substitution_bytes(
    output: &mut Vec<u8>,
    replacement: &[u8],
    subject: &[u8],
    offset: usize,
    matched: &[RegMatch],
) {
    let mut cursor = 0;
    while cursor < replacement.len() {
        let byte = replacement[cursor];
        cursor += 1;
        let capture = if byte == b'&' {
            Some(0)
        } else if byte == b'\\' && replacement.get(cursor).is_some_and(u8::is_ascii_digit) {
            let group = usize::from(replacement[cursor] - b'0');
            cursor += 1;
            Some(group)
        } else {
            None
        };
        if let Some(group) = capture {
            if let Some(span) = matched.get(group).filter(|span| span.so != NO_MATCH) {
                output.extend_from_slice(&subject[offset + span.so..offset + span.eo]);
            }
        } else if byte == b'\\'
            && replacement
                .get(cursor)
                .is_some_and(|&byte| matches!(byte, b'&' | b'\\'))
        {
            output.push(replacement[cursor]);
            cursor += 1;
        } else {
            output.push(byte);
        }
    }
}

fn jim_regsub_callback<O: NativeRegsubObjects>(
    ops: &mut O,
    prefix: &O::Object,
    subject: &[u8],
    offset: usize,
    matched: &[RegMatch],
    output: &mut Vec<u8>,
) -> Result<(), RegsubError<O::Error>> {
    let host = |error| RegsubError::Regex(RegexError::from(crate::CmdError::from(error)));
    let mut arguments = Vec::new();
    for span in matched {
        if span.so == NO_MATCH {
            break;
        }
        arguments.push(
            ops.regex_jim_bytes(&subject[offset + span.so..offset + span.eo], false)
                .map_err(host)?,
        );
    }
    let call = ops
        .regex_duplicate(ops.regex_object(prefix))
        .map_err(host)?;
    ops.regex_eval(ops.regex_object(&call), &arguments)
        .map_err(RegsubError::Eval)?;
    let current = ops.regex_result_bytes().map_err(host)?;
    output.extend_from_slice(tcl_core_types::c_string_extent(&current));
    Ok(())
}

/// Bundled Jim regsub uses a shallow original-pattern duplicate, `CString`
/// matching, counted replacement bytes and original callback result getters.
pub fn regsub_jim_original<'a, O: NativeRegsubObjects + NativeRegexObjects<E>, E: RegexEngine>(
    ops: &mut O,
    args: &'a [O::Value],
    version: TclVersion,
) -> NativeRegsubOutcome<'a, O> {
    let host = |error| RegsubError::Regex(RegexError::from(crate::CmdError::from(error)));
    let selected = jim_options(ops, args, version, true).map_err(RegsubError::Regex)?;
    let originals = &args[selected.offset..];
    if !(3..=4).contains(&originals.len()) {
        return Err(RegsubError::Regex(wrong_args(REGSUB_USAGE)));
    }
    let duplicate = ops.regex_duplicate(&originals[0]).map_err(host)?;
    let original_duplicate = ops.regex_object(&duplicate);
    let prepared =
        prepare_pattern_original::<O, E>(ops, original_duplicate, selected.flags, version)
            .map_err(RegsubError::Regex)?;
    let PreparedOriginalRegex::Jim { artifact, .. } = prepared else {
        return Err(host(ValueError::CommandProtocolUnavailable(
            "Jim regsub prepared artifact",
        )));
    };
    let pattern = ops.native_string_bytes(&originals[0]).map_err(host)?;
    let subject = ops.native_string_bytes(&originals[1]).map_err(host)?;
    let prefix = if selected.command == JimReplacement::Command {
        let members = ops.regex_members(&originals[2]).map_err(host)?;
        if members.is_empty() {
            return Err(RegsubError::Regex(RegexError::new(
                b"command prefix must be a list of at least one element".to_vec(),
            )));
        }
        Some(ops.regex_borrow(&originals[2]))
    } else {
        None
    };
    let replacement = if selected.command == JimReplacement::Command {
        None
    } else {
        Some(ops.native_string_bytes(&originals[2]).map_err(host)?)
    };
    let mut offset = jim_byte_offset(&subject, selected.start).map_err(host)?;
    let mut output = subject[..offset].to_vec();
    let mut count = 0i64;
    let mut notbol = false;
    while offset < subject.len() || !tcl_core_types::c_string_extent(&pattern).is_empty() {
        let matched = artifact
            .with_program(|program| E::exec_jim(program, &subject[offset..], 50, notbol))
            .map_err(host)?
            .map_err(|reason| host(ValueError::CommandProtocolUnavailable(reason)))?;
        let Some(matched) = matched else {
            break;
        };
        let whole = *matched.first().ok_or_else(|| {
            host(ValueError::CommandProtocolUnavailable(
                "Jim regsub whole match",
            ))
        })?;
        for span in &matched {
            if span.so != NO_MATCH && (span.eo < span.so || span.eo > subject.len() - offset) {
                return Err(host(ValueError::CommandProtocolUnavailable(
                    "Jim regsub original range geometry",
                )));
            }
        }
        if whole.so == NO_MATCH {
            return Err(host(ValueError::CommandProtocolUnavailable(
                "Jim regsub missing whole match",
            )));
        }
        output.extend_from_slice(&subject[offset..offset + whole.so]);
        if let Some(prefix) = &prefix {
            jim_regsub_callback(ops, prefix, &subject, offset, &matched, &mut output)?;
        } else if let Some(replacement) = &replacement {
            jim_substitution_bytes(&mut output, replacement, &subject, offset, &matched);
        }
        count += 1;
        offset += whole.eo;
        notbol = false;
        if !selected.all || offset == subject.len() {
            break;
        }
        if whole.eo == whole.so {
            if pattern.first() == Some(&b'^') {
                notbol = true;
            } else {
                let width = tcl_syntax::raw_string::RawString::from_bytes(&subject[offset..])
                    .jim084_byte_offset(1)
                    .map_err(ValueError::from)
                    .map_err(host)?;
                output.extend_from_slice(&subject[offset..offset + width]);
                offset += width;
            }
        }
    }
    output.extend_from_slice(tcl_core_types::c_string_extent(&subject[offset..]));
    let result = ops.regex_jim_bytes(&output, true).map_err(host)?;
    Ok(NativeRegsubResult {
        result,
        count,
        target: originals.get(3),
    })
}

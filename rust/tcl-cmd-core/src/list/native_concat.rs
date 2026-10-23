// SPDX-License-Identifier: AGPL-3.0-or-later
//! Selected original concat eligibility, native headers and getter chronology.

use crate::CmdError;
use tcl_dialect::{ConcatPolicy, TclVersion};
use tcl_syntax::value::{NativeConcatListShape as Shape, ValueOps};

pub(super) fn concatenate<O: ValueOps>(
    ops: &mut O,
    args: &[O::Value],
) -> Result<O::Value, CmdError> {
    let policy = ops.concat_policy().ok_or({
        tcl_syntax::value::ValueError::CommandProtocolUnavailable("native concat protocol")
    })?;
    if lists_are_eligible(ops, args, policy)?
        && let Some(value) = concatenate_lists(ops, args, policy)?
    {
        return Ok(value);
    }
    concatenate_strings(ops, args, policy)
}

fn accepts_shape(shape: Shape, policy: ConcatPolicy) -> bool {
    match (policy, shape) {
        (ConcatPolicy::JimRepresentationSensitive, Shape::List { .. })
        | (
            ConcatPolicy::Tcl(TclVersion::V8_4),
            Shape::List {
                resident_length: None,
                ..
            },
        ) => true,
        (
            ConcatPolicy::Tcl(version),
            Shape::List {
                canonical,
                resident_length,
                ..
            },
        ) if version >= TclVersion::V8_5 => canonical || resident_length.is_none(),
        (ConcatPolicy::Tcl(version), Shape::Indexed) => version >= TclVersion::V9_0,
        _ => false,
    }
}

fn lists_are_eligible<O: ValueOps>(
    ops: &mut O,
    args: &[O::Value],
    policy: ConcatPolicy,
) -> Result<bool, CmdError> {
    for value in args {
        if accepts_shape(ops.native_concat_list_shape(value)?, policy) {
            continue;
        }
        if !matches!(policy, ConcatPolicy::Tcl(version) if version >= TclVersion::V8_5)
            || !ops.native_concat_string_bytes(value)?.is_empty()
        {
            return Ok(false);
        }
    }
    Ok(true)
}

struct Working<'a, O: ValueOps> {
    ops: &'a mut O,
    value: Option<O::Value>,
}

impl<O: ValueOps> Drop for Working<'_, O> {
    fn drop(&mut self) {
        if let Some(value) = self.value.take() {
            self.ops.discard_native_concat_result(value);
        }
    }
}

fn concatenate_lists<O: ValueOps>(
    ops: &mut O,
    args: &[O::Value],
    policy: ConcatPolicy,
) -> Result<Option<O::Value>, CmdError> {
    let modern = matches!(policy, ConcatPolicy::Tcl(version) if version >= TclVersion::V8_5);
    let mut result = Working { ops, value: None };
    if !modern {
        result.value = Some(result.ops.native_concat_empty_list()?);
    }
    for source in args {
        let shape = result.ops.native_concat_list_shape(source)?;
        if modern && skip_empty(shape, source, result.ops, policy)? {
            continue;
        }
        if let Some(target) = result.value.as_ref() {
            if matches!(policy, ConcatPolicy::Tcl(version) if version >= TclVersion::V8_6) {
                let Some(first) = result.ops.native_concat_first_bytes(source)? else {
                    continue;
                };
                if first.bytes.first() == Some(&b'#') {
                    result.ops.discard_native_concat_result(
                        result.value.take().expect("partial native concat"),
                    );
                    result.ops.release_native_concat_first(first);
                    return Ok(None);
                }
                let appended = result.ops.native_concat_append_list(target, source);
                result.ops.release_native_concat_first(first);
                appended?;
            } else {
                result.ops.native_concat_append_list(target, source)?;
            }
        } else {
            // C8.5 ignores empty Lists before choosing its first backing.
            if policy == ConcatPolicy::Tcl(TclVersion::V8_5)
                && matches!(shape, Shape::List { length: 0, .. })
            {
                continue;
            }
            result.value = Some(result.ops.native_concat_copy_list(source)?);
        }
    }
    if result.value.is_none() {
        result.value = Some(result.ops.native_concat_empty_list()?);
    }
    Ok(result.value.take())
}

fn skip_empty<O: ValueOps>(
    shape: Shape,
    source: &O::Value,
    ops: &mut O,
    policy: ConcatPolicy,
) -> Result<bool, CmdError> {
    if matches!(
        policy,
        ConcatPolicy::Tcl(TclVersion::V9_0 | TclVersion::V9_1)
    ) {
        return Ok(!accepts_shape(shape, policy));
    }
    Ok(matches!(
        shape,
        Shape::List {
            resident_length: Some(0),
            ..
        }
    ) || (!accepts_shape(shape, policy) && ops.native_concat_string_bytes(source)?.is_empty()))
}

fn concatenate_strings<O: ValueOps>(
    ops: &mut O,
    args: &[O::Value],
    policy: ConcatPolicy,
) -> Result<O::Value, CmdError> {
    // Native allocation sizing gets every original before the assembly pass.
    for value in args {
        let _ = ops.native_concat_string_bytes(value)?;
    }
    let values = args
        .iter()
        .map(|value| ops.native_concat_string_bytes(value))
        .collect::<Result<Vec<_>, _>>()?;
    let bytes = match policy {
        ConcatPolicy::Tcl(_) => tcl_syntax::list::concat_bytes(values.iter().map(AsRef::as_ref)),
        ConcatPolicy::JimRepresentationSensitive => {
            tcl_syntax::list::concat_bytes_jim(values.iter().map(AsRef::as_ref))
        }
    };
    Ok(ops.native_concat_string_result(&bytes)?)
}

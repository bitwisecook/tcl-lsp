// SPDX-License-Identifier: AGPL-3.0-or-later
//! Genuine Jim switch selection over borrowed original argv and List members.
use crate::CmdError;
use tcl_syntax::native_jim_switch::{NativeJimSwitchOption as OptionStep, NativeJimSwitchProtocol};

/// Native static immediate literals, with independent `ComparedString` identities.
#[derive(Clone, Copy)]
pub enum Immediate {
    /// Every reached pattern is checked against this, including nonterminal ones.
    Default,
    /// Only selected body/fallthrough members are checked against this.
    Dash,
}
/// Concrete original-object doors; returned members are lifetime-only borrows.
pub trait NativeJimSwitchObjects {
    /// Original header handle. Borrowing must add no native object reference.
    type Value;
    /// Actual callback completion, independently from private host refusal.
    type Callback;
    /// Reach the selected original string getter.
    fn switch_bytes(&mut self, original: &Self::Value) -> Result<Vec<u8>, CmdError>;
    /// Borrow the original header without introducing native ownership.
    fn switch_borrow(&self, original: &Self::Value) -> Self::Value;
    /// Convert/refetch the same original case-list backing and return its count.
    fn switch_list_length(&mut self, original: &Self::Value) -> Result<usize, CmdError>;
    /// Borrow one current original member; never clone a member's native owner.
    fn switch_list_member(
        &mut self,
        original: &Self::Value,
        index: usize,
    ) -> Result<Self::Value, CmdError>;
    /// Genuine static-literal comparison/cache on the same original header.
    fn switch_immediate(
        &mut self,
        original: &Self::Value,
        literal: Immediate,
    ) -> Result<bool, CmdError>;
    /// Exact Jim counted-object equality, including actual pointer identity.
    fn switch_equal(
        &mut self,
        subject: &Self::Value,
        pattern: &Self::Value,
    ) -> Result<bool, CmdError>;
    /// Actual Jim object glob matching; no C Unicode/cache donation.
    fn switch_glob(
        &mut self,
        pattern: &Self::Value,
        subject: &Self::Value,
    ) -> Result<bool, CmdError>;
    /// Original callback return code, before Jim negates it into the match integer.
    fn switch_callback_code(&self, callback: &Self::Callback) -> i32;
    /// Return the original current result with the native negated match code.
    fn switch_negative_match(&mut self, code: i32) -> Self::Callback;
    /// Invoke a borrowed original command; None manufactures a fresh regexp head.
    /// Return the full native long (not Boolean) and preserve actual completions.
    fn switch_command(
        &mut self,
        command: Option<&Self::Value>,
        pattern: &Self::Value,
        subject: &Self::Value,
        option_end: bool,
    ) -> Result<i64, Self::Callback>;
}
/// Selected original body, or the native normal empty-result path.
pub enum Selection<V> {
    /// Borrowed same original body object, not reconstructed script text.
    Body(V),
    /// No match.
    Empty,
}
/// An actual guest command failure or a returned callback completion.
pub enum Failure<C> {
    /// Selected switch usage/diagnostic or typed object-access refusal.
    Command(CmdError),
    /// The match command's unchanged returned completion.
    Callback(C),
}
impl<C> From<CmdError> for Failure<C> {
    fn from(error: CmdError) -> Self {
        Self::Command(error)
    }
}
const USAGE: &str =
    "switch ?options? string pattern body ... ?default body? or pattern body ?pattern body ...?";
fn usage() -> CmdError {
    CmdError::wrong_args(USAGE)
}
#[derive(Clone, Copy)]
enum Mode {
    Exact,
    Glob,
    Regexp,
    Command(usize),
}
enum Cases<'a, V> {
    Inline(&'a [V]),
    List(&'a V),
}
impl<V> Cases<'_, V> {
    fn length<O: NativeJimSwitchObjects<Value = V>>(&self, ops: &mut O) -> Result<usize, CmdError> {
        match self {
            Self::Inline(values) => Ok(values.len()),
            Self::List(root) => ops.switch_list_length(root),
        }
    }
    fn member<O: NativeJimSwitchObjects<Value = V>>(
        &self,
        ops: &mut O,
        index: usize,
    ) -> Result<V, CmdError> {
        match self {
            Self::Inline(values) => {
                values
                    .get(index)
                    .map(|v| ops.switch_borrow(v))
                    .ok_or_else(|| {
                        tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                            "Jim switch current inline member",
                        )
                        .into()
                    })
            }
            Self::List(root) => ops.switch_list_member(root, index),
        }
    }
}
/// Execute Jim's original option/selection algorithm before the selected body.
///
/// # Errors
/// Guest usage/matching failures retain native presentation; host unavailability
/// remains typed. Callback errors still refetch the same case List before return.
pub fn select<O: NativeJimSwitchObjects>(
    ops: &mut O,
    protocol: NativeJimSwitchProtocol,
    args: &[O::Value],
) -> Result<Selection<O::Value>, Failure<O::Callback>> {
    let (mut at, mode, option_end) = switch_options(ops, protocol, args)?;
    let subject = args.get(at).ok_or_else(usage)?;
    at += 1;
    let rest = &args[at..];
    let cases = if rest.len() == 1 {
        Cases::List(&rest[0])
    } else {
        Cases::Inline(rest)
    };
    let mut count = cases.length(ops)?;
    if count == 0 || count % 2 != 0 {
        return Err(usage().into());
    }
    let mut index = 0;
    let mut selected = None;
    while selected.is_none() && index < count {
        let pattern = cases.member(ops, index)?;
        let fallback = ops.switch_immediate(&pattern, Immediate::Default)? && index + 2 == count;
        let matched = if fallback {
            true
        } else {
            match mode {
                Mode::Exact => ops.switch_equal(subject, &pattern)?,
                Mode::Glob => ops.switch_glob(&pattern, subject)?,
                Mode::Regexp | Mode::Command(_) => {
                    let head = match mode {
                        Mode::Command(command) => Some(&args[command]),
                        _ => None,
                    };
                    let result = ops.switch_command(head, &pattern, subject, option_end);
                    if matches!(&cases, Cases::List(_)) {
                        count = cases.length(ops)?;
                    }
                    {
                        let matched = match result {
                            Ok(value) => tcl_syntax::number::native_int32_low_bits(value),
                            Err(callback) => {
                                let matched = ops.switch_callback_code(&callback).wrapping_neg();
                                if matched < 0 {
                                    return Err(Failure::Callback(callback));
                                }
                                matched
                            }
                        };
                        if matched < 0 {
                            return Err(Failure::Callback(
                                ops.switch_negative_match(matched.wrapping_neg()),
                            ));
                        }
                        matched != 0
                    }
                }
            }
        };
        if matched {
            selected = Some(cases.member(ops, index + 1)?);
        }
        index += 2;
    }
    while index < count {
        let Some(body) = selected.as_ref() else { break };
        if !ops.switch_immediate(body, Immediate::Dash)? {
            break;
        }
        selected = Some(cases.member(ops, index + 1)?);
        index += 2;
    }
    if let Some(body) = selected {
        if ops.switch_immediate(&body, Immediate::Dash)? {
            let pattern = cases.member(ops, index - 2)?;
            let bytes = ops.switch_bytes(&pattern)?;
            let message = [
                b"no body specified for pattern \"".as_slice(),
                tcl_core_types::c_string_extent(&bytes),
                b"\"",
            ]
            .concat();
            return Err(CmdError::new_bytes(message).into());
        }
        Ok(Selection::Body(body))
    } else {
        Ok(Selection::Empty)
    }
}

fn switch_options<O: NativeJimSwitchObjects>(
    ops: &mut O,
    protocol: NativeJimSwitchProtocol,
    args: &[O::Value],
) -> Result<(usize, Mode, bool), Failure<O::Callback>> {
    let mut at = 0;
    let mut mode = Mode::Exact;
    let mut option_end = false;
    while at < args.len() {
        let word = ops.switch_bytes(&args[at])?;
        match protocol.option(&word) {
            OptionStep::Subject => break,
            OptionStep::End => {
                at += 1;
                break;
            }
            OptionStep::Exact => mode = Mode::Exact,
            OptionStep::Glob => mode = Mode::Glob,
            OptionStep::Regexp => {
                mode = Mode::Regexp;
                option_end = true;
            }
            OptionStep::Command => {
                at += 1;
                if at >= args.len() {
                    return Err(usage().into());
                }
                mode = Mode::Command(at);
            }
            OptionStep::Invalid => {
                let message = [
                    b"bad option \"".as_slice(),
                    tcl_core_types::c_string_extent(&word),
                    b"\": must be -exact, -glob, -regexp, -command procname or --",
                ]
                .concat();
                return Err(CmdError::new_bytes(message).into());
            }
        }
        if args.len() - at < 2 {
            return Err(usage().into());
        }
        at += 1;
    }
    Ok((at, mode, option_end))
}

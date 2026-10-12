// SPDX-License-Identifier: AGPL-3.0-or-later
//! Jim private return state, independent of the latest command completion.

use crate::{Code, completion_options::OptionValue};

/// Interpreter-local counters written by Jim's return command.
/// Ordinary commands and catch/try settlement leave these counters unchanged.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct JimReturnState {
    /// Code selected by the most recent reached return command.
    pub code: i32,
    /// Remaining procedure boundaries selected by that command.
    pub level: i64,
}

impl JimReturnState {
    /// Decrement at an actual procedure boundary receiving raw Return.
    /// The selected code takes effect when the remaining level reaches zero.
    pub fn settle_procedure(&mut self, raw: Code) -> Code {
        if raw != Code::Return {
            return raw;
        }
        self.level = self.level.saturating_sub(1);
        if self.level > 0 {
            return Code::Return;
        }
        let code = Code::from_int(self.code);
        *self = Self::default();
        code
    }
}

/// Owned private state captured at one actual Jim interpreter boundary.
/// The result and raw exit code remain separate from this metadata.
#[derive(Debug, Clone)]
pub struct JimReturnReceipt<V> {
    /// Live return counters, including counters retained after an ordinary command.
    pub pending: JimReturnState,
    /// Actual global errorCode object; absent if the variable has been removed.
    pub error_code: Option<V>,
    /// Actual explicit or captured stackTrace object.
    pub stack_trace: V,
}

impl<V: Clone> JimReturnReceipt<V> {
    /// Construct catch/try options using the raw exit code and live private state.
    /// Error metadata belongs only to raw Error, even when pending code is Error.
    #[must_use]
    pub fn option_pairs(&self, exit: Code) -> Vec<(&'static [u8], OptionValue<V>)> {
        let code = if exit == Code::Return {
            i64::from(self.pending.code)
        } else {
            exit.as_int()
        };
        let mut pairs = vec![
            (b"-code".as_slice(), OptionValue::Integer(code)),
            (
                b"-level".as_slice(),
                OptionValue::Integer(self.pending.level),
            ),
        ];
        if exit == Code::Error {
            pairs.push((b"-errorinfo", OptionValue::Value(self.stack_trace.clone())));
            if let Some(code) = &self.error_code {
                pairs.push((b"-errorcode", OptionValue::Value(code.clone())));
            }
        }
        pairs
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ordinary_completion_retains_private_return_counters() {
        let mut state = JimReturnState { code: 1, level: 2 };
        assert_eq!(state.settle_procedure(Code::Ok), Code::Ok);
        assert_eq!(state.level, 2);
        assert_eq!(state.settle_procedure(Code::Return), Code::Return);
        assert_eq!(state.settle_procedure(Code::Return), Code::Error);
        assert_eq!(state, JimReturnState::default());
    }
    #[test]
    fn raw_error_options_use_live_state_after_finally() {
        let receipt = JimReturnReceipt {
            pending: JimReturnState { code: 7, level: 3 },
            error_code: Some("CUSTOM"),
            stack_trace: "TRACE",
        };
        let pairs = receipt.option_pairs(Code::Error);
        assert!(matches!(pairs[0].1, OptionValue::Integer(1)));
        assert!(matches!(pairs[1].1, OptionValue::Integer(3)));
        assert_eq!(receipt.option_pairs(Code::Return).len(), 2);
        assert!(matches!(
            receipt.option_pairs(Code::Return)[0].1,
            OptionValue::Integer(7)
        ));
    }
}

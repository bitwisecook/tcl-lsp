//! `exp_continue` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "exp_continue",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::new(0, 1),
        hover: Some(HoverSnippet::brief(
            "Continue matching within an expect body instead of returning.",
            &["exp_continue ?-continue_timer?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}

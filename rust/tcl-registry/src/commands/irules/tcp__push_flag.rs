//! `TCP::push_flag` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::push_flag",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command can be used to set/get the PUSH flag mode of a TCP connection.",
            &["TCP::push_flag ('default' | 'none' | 'one' | 'auto')?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}

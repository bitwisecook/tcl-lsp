//! `TCP::rexmt_thresh` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::rexmt_thresh",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "This command can be used to set/get the retransmission threshold of a TCP connec",
            &["TCP::rexmt_thresh (TCP_REXMT_THRESH_VALUE)?"],
            "F5 iRules",
        )),
        ..CommandSpec::DEFAULT
    }
}

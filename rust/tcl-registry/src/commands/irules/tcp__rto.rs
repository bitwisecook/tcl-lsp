//! `TCP::rto` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::rto",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the current value of Retransmission timeout.",
            synopsis: &["TCP::rto"],
            snippet: "Returns the last setting to which the retransmit timer was set in milliseconds. It does not include time elapsed since the timer was set.",
            source: "https://clouddocs.f5.com/api/irules/TCP__rto.html",
            examples: "when CLIENT_CLOSED {\n    set rto [TCP::rto]\n    log local0. \"Final RTO value is $rto\"\n}",
            return_value: "Retransmit timer value in milliseconds.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "TCP::rto" },
        ],
        ..CommandSpec::DEFAULT
    }
}

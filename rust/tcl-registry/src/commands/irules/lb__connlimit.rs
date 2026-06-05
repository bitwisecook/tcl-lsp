//! `LB::connlimit` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "LB::connlimit",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Set the connection limit for virtual/node/poolmember.",
            synopsis: &[
                "LB::connlimit ('virtual' | 'node' | 'poolmember') ?limit <value>? ?key <value>?",
            ],
            snippet: "Set the connection limit for virtual/node/poolmember",
            source: "https://clouddocs.f5.com/api/irules/LB__connlimit.html",
            examples: "",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "LB::connlimit <target> ?args?",
        }],
        ..CommandSpec::DEFAULT
    }
}

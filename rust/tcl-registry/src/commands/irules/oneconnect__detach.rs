//! `ONECONNECT::detach` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ONECONNECT::detach",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Detaches server-side OneConnect connections.",
            synopsis: &["ONECONNECT::detach BOOL_VALUE"],
            snippet: "Controls the behavior of a server-side connection when a OneConnect\nprofile is on the virtual server. The default behavior is that the\nserver-side connection detaches after each response is completed, and a\nnew load balancing decision and persistence look-up are performed for\nevery request.\nDisabling detaching prevents this behavior.\nNote: the use of the terms \"request\" and \"response\" imply the presence\nof a supported layer 7 profile (e.g. the HTTP profile) on the virtual\nserver. An iRule can also detaching the server-side connection using\nthe LB::detach command.",
            source: "https://clouddocs.f5.com/api/irules/ONECONNECT__detach.html",
            examples: "when HTTP_RESPONSE {\n    if { $headreq } {\n        # Response to HEAD request. Detach after done.\n        ONECONNECT::detach enable\n        ONECONNECT::reuse enable\n    }\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "ONECONNECT::detach BOOL_VALUE",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ConnectionControl,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Server,
        }],
        ..CommandSpec::DEFAULT
    }
}

//! `MR::retry` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::retry",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Send the current message to the router for routing.",
            synopsis: &["MR::retry"],
            snippet: "The MR::retry command instructs the Message Routing Framework submit the current message to the router to retry routing. This will clear the message's route status. The script may need to clear the message's existing nexthop and route fields if a new routetable lookup is desired. If a persistence record exists for this message, it may also need to be reset.",
            source: "https://clouddocs.f5.com/api/irules/MR__retry.html",
            examples: "when MR_FAILED {\n    if {[MR::retry_count] < 2} {\n        MR::message nexthop none\n        MR::message route config tc1 host \"10.1.1.1:1234\"\n        MR::retry\n    }\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["MR"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "MR::retry" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::MessageState,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}

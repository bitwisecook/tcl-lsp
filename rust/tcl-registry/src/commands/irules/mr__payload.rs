//! `MR::payload` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::payload",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Access data collected using MR::collect command.",
            synopsis: &["MR::payload ( 'length' )?"],
            snippet: "This command can be used to access payload collected using the COLLECT command.\n\nSYNTAX\n\nMR::payload [length]\n\nMR::payload\n    Returns the collected payload obtained as a result of a prior call to MR::collect.\n\nMR::payload length\n    Returns the length of payload of a MR message.",
            source: "https://clouddocs.f5.com/api/irules/MR__payload.html",
            examples: "when MR_DATA {\n                log local0 \"Payload: [MR::payload]\"\n            }",
            return_value: "When called without an argument, this command returns the collected payload of an MR message.",
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
            FormSpec { kind: FormKind::Default, synopsis: "MR::payload ( 'length' )?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::MessageState,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        taint_source: Some(TaintColour::TAINTED),
        ..CommandSpec::DEFAULT
    }
}

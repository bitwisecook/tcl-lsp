//! `MR::release` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::release",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Releases the data collected via MR::collect iRule command.",
            synopsis: &["MR::release"],
            snippet: "Releases the payload data collected via MR::collect iRule command for further processing.\n\nThis command is valid only when MR::collect has been called.",
            source: "https://clouddocs.f5.com/api/irules/MR__release.html",
            examples: "",
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
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "MR::release",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::MessageState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}

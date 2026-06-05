//! `VDI::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "VDI::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Enable VDI plugin.",
            synopsis: &["VDI::enable"],
            snippet: "The VDI::enable command enables VDI plugin in the flow.",
            source: "https://clouddocs.f5.com/api/irules/VDI__enable.html",
            examples: "",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["FASTHTTP"],
            also_in: &["CLIENT_ACCEPTED"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "VDI::enable",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ConnectionControl,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}

//! `VDI::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "VDI::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Disable VDI plugin.",
            synopsis: &["VDI::disable"],
            snippet: "The VDI::disable command disables VDI plugin in the flow.",
            source: "https://clouddocs.f5.com/api/irules/VDI__disable.html",
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
            synopsis: "VDI::disable",
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

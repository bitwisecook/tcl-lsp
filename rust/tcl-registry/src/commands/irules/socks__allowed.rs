//! `SOCKS::allowed` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SOCKS::allowed",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "This command allows you to change whether the SOCKS request is allowed or not.",
            synopsis: &["SOCKS::allowed ('0' | '1')?"],
            snippet: "This command allows you to reject a SOCKS request during the SOCKS_REQUEST event.\n\nDetails (Syntax):\nSOCKS::allowed '0' | '1'\n    Sets the state of SOCKS based on the Boolean value.",
            source: "https://clouddocs.f5.com/api/irules/SOCKS__allowed.html",
            examples: "# Reject all SOCKS requests:\nwhen SOCKS_REQUEST {\n    SOCKS::allowed 0\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["SOCKS"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "SOCKS::allowed ('0' | '1')?",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}

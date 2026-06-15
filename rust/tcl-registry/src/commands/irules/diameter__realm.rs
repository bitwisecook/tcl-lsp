//! `DIAMETER::realm` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::realm",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Gets or sets the value of the origin-realm or destination-realm AVP.",
            synopsis: &["DIAMETER::realm ( ('origin' | 'dest' ) (DIAMETER_REALM)? )"],
            snippet: "This iRule command gets or sets the value of the origin-realm (code 296) or\ndestination-realm (code 283) AVP in the current Diameter message.",
            source: "https://clouddocs.f5.com/api/irules/DIAMETER__realm.html",
            examples: "when DIAMETER_INGRESS {\n    log local0. \"Received a DIAMETER message with origin realm [DIAMETER::realm origin]\"\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["DIAMETER", "MR"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "DIAMETER::realm ( ('origin' | 'dest' ) (DIAMETER_REALM)? )",
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

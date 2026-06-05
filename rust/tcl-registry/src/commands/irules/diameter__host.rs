//! `DIAMETER::host` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::host",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Gets or sets the value of the origin-host or destination-host AVP.",
            synopsis: &["DIAMETER::host ( ('origin' | 'dest' ) (DIAMETER_HOST)? )"],
            snippet: "This iRule command gets or sets the value of the origin-host (code\n264) or destination-host (code 293) AVP in the current message.",
            source: "https://clouddocs.f5.com/api/irules/DIAMETER__host.html",
            examples: "when DIAMETER_INGRESS {\n    log local0. \"Received a DIAMETER message with origin host [DIAMETER::host origin]\"\n}",
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
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "DIAMETER::host ( ('origin' | 'dest' ) (DIAMETER_HOST)? )" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::NetworkIo,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}

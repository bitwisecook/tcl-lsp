//! `SIP::via` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SIP::via",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Gets SIP via header information.",
            synopsis: &["SIP::via ?field? ?INDEX?", "SIP::via (proto | sent_by | received | branch | maddr | ttl) ?INDEX?"],
            snippet: "This set of commands allows you to get information in the SIP via header.",
            source: "https://clouddocs.f5.com/api/irules/SIP__via.html",
            examples: "when SIP_RESPONSE {\n  log local0. [SIP::via 0]\n  SIP::header remove Via 0\n  SIP::response rewrite 123 \"no xxx\"\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["SIP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "SIP::via ?field? ?INDEX?" },
        ],
        ..CommandSpec::DEFAULT
    }
}

//! `DNS::drop` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DNS::drop",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Drops the current DNS packet after the execution of the event.",
            synopsis: &["DNS::drop"],
            snippet: "This iRules command drops the current DNS packet after the execution of\nthe event.\n\nNote: This command functions only in the context of LTM iRules and\nrequires the DNS Profile, which is only enabled as part of GTM or the\nDNS Services add-on.",
            source: "https://clouddocs.f5.com/api/irules/DNS__drop.html",
            examples: "quests from a specific IP\n            when DNS_REQUEST {\n                if { [IP::client_addr] equals \"192.168.1.245\" } {\n                    DNS::drop\n                }\n            }",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["DNS"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "DNS::drop" },
        ],
        ..CommandSpec::DEFAULT
    }
}

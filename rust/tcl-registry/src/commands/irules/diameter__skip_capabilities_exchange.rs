//! `DIAMETER::skip_capabilities_exchange` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::skip_capabilities_exchange",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Instructs DIAMETER protocol to skip capabilities exchange when establishing a peering relationship.",
            synopsis: &["DIAMETER::skip_capabilities_exchange ( HOSTNAME )?"],
            snippet: "Once called, the current connection will skip DIAMETER capabilities exchange message communication with the peer device and will immediately be able to receive DIAMETER messaegs.\n\nIf the HOSTNAME parameter is provided, the provided name will be used as the peer device's origin-host attribute for logging.",
            source: "https://clouddocs.f5.com/api/irules/DIAMETER__skip_capabilities_exchange.html",
            examples: "when CLIENT_ACCEPTED {\n                if { ([IP::address] starts_with \"192.168.\") } {\n                    DIAMETER::skip_capabilities_exchange [IP::address].somesp.com\n                }\n            }",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &[],
            also_in: &["CLIENT_ACCEPTED"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "DIAMETER::skip_capabilities_exchange ( HOSTNAME )?" },
        ],
        ..CommandSpec::DEFAULT
    }
}

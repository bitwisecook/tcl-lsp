//! `SIP::discard` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SIP::discard",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Discards the current SIP message.",
            synopsis: &["SIP::discard"],
            snippet:
                "Discards a SIP message\n\nSIP::discard\n\n     * Discards the current SIP message",
            source: "https://clouddocs.f5.com/api/irules/SIP__discard.html",
            examples: "when SIP_RESPONSE {\n  SIP::discard\n}",
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
        ..CommandSpec::DEFAULT
    }
}

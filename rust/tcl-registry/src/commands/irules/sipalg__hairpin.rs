//! `SIPALG::hairpin` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SIPALG::hairpin",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Gets or sets the value of hairpin flag for the current message.",
            synopsis: &[
                "SIPALG::hairpin",
                "SIPALG::hairpin (detect | disable | enable)",
            ],
            snippet: "Returns the value of the hairpin flag for the current message.",
            source: "https://clouddocs.f5.com/api/irules/SIPALG__hairpin.html",
            examples:
                "when SIP_REQUEST {\n    log local0. \"message hairpin mode [SIPALG::hairpin]\"\n}",
            return_value: "Returns 'detect', 'disable', or 'enable'",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["MR", "SIP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}

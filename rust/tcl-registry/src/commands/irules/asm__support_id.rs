//! `ASM::support_id` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::support_id",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the support id of the HTTP transaction.",
            synopsis: &["ASM::support_id"],
            snippet: "Returns the support id of the HTTP transaction, a unique\nidentifier assigned by ASM to the transaction, regardless of whether\nviolations were found in the transaction or not. The support id can be\nused to correlate the transaction with its corresponding entry in the\nrequest log and with the blocking page returned to the user in case of\nblocking violations",
            source: "https://clouddocs.f5.com/api/irules/ASM__support_id.html",
            examples: "",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["ASM"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        ..CommandSpec::DEFAULT
    }
}

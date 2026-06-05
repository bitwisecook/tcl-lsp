//! `MR::store` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::store",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Stores a tcl variable with the mr_message object.",
            synopsis: &["MR::store (VAR)*"],
            snippet: "The MR::store command stores one or more named Tcl variables with the message so that they are available on egress even if stored on ingress. If no name is provided, it stores all local variables in the current message context. Storing variables does not affect the content of the message.",
            source: "https://clouddocs.f5.com/api/irules/MR__store.html",
            examples: "when MR_EGRESS {\n    MR::restore client_addr\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["MR"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "MR::store (VAR)*" },
        ],
        ..CommandSpec::DEFAULT
    }
}

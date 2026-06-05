//! `DIAMETER::header` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::header",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Gets or sets the DIAMETER header fields.",
            synopsis: &["DIAMETER::header <field> ?value?", "DIAMETER::header command_code ?value?", "DIAMETER::header application_id ?value?"],
            snippet: "This iRule command is used to get and set header fields in the current DIAMETER message.",
            source: "https://clouddocs.f5.com/api/irules/DIAMETER__header.html",
            examples: "when DIAMETER_INGRESS {\n    if { [DIAMETER::header tflag] } {\n        log local0. \"Received a potentially retransmitted Diameter message\"\n    }\n}",
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
            FormSpec { kind: FormKind::Default, synopsis: "DIAMETER::header <field> ?value?" },
        ],
        ..CommandSpec::DEFAULT
    }
}

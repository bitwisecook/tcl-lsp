//! `MR::stream` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::stream",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Start egressing bytes previously collected and stored.",
            synopsis: &["MR::stream ( 'end' )? (BYTES)"],
            snippet: "Start egressing bytes previously collected and stored say in sessionDB. If payload has been split in multiple segments, use end to indicate the final segment.\n\nSYNTAX\n\nMR::stream <payload>\n    Stream payload segment.\n\nMR::stream end <payload>\n    Stream payload segement. End indicates final segment.",
            source: "https://clouddocs.f5.com/api/irules/MR__stream.html",
            examples: "when MR_EGRESS {\n    MR::stream end \"abcd\"\n}",
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
            FormSpec { kind: FormKind::Default, synopsis: "MR::stream ( 'end' )? (BYTES)" },
        ],
        ..CommandSpec::DEFAULT
    }
}

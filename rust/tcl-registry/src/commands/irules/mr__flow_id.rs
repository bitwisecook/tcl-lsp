//! `MR::flow_id` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::flow_id",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns a unique identifier for the current connection.",
            synopsis: &["MR::flow_id"],
            snippet: "Returns a unique identifier for the current connection. This identifier can be used to generate the lasthop and nexthop of a message.",
            source: "https://clouddocs.f5.com/api/irules/MR__flow_id.html",
            examples: "when MR_INGRESS {\n    set orig_flowid [MR::flow_id]\n    MR::store orig_flowid\n}",
            return_value: "Returns a unique identifier for the current connection.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "MR::flow_id" },
        ],
        ..CommandSpec::DEFAULT
    }
}

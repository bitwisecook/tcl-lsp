//! `MR::max_retries` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "MR::max_retries",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the number of retries allows for this router instance.",
            synopsis: &["MR::max_retries"],
            snippet: "returns the number of retries allowed",
            source: "https://clouddocs.f5.com/api/irules/MR__max_retries.html",
            examples: "when MR_FAILED {\n    if {[MR::message retry_count] < [MR::max_retries]} {\n        MR::message nexthop none\n        MR::retry\n    }\n}",
            return_value: "returns the number of retries allowed",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "MR::max_retries" },
        ],
        ..CommandSpec::DEFAULT
    }
}

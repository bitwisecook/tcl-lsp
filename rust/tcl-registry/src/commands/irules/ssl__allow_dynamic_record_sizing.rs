//! `SSL::allow_dynamic_record_sizing` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::allow_dynamic_record_sizing",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Get or set dynamic record sizing.",
            synopsis: &["SSL::allow_dynamic_record_sizing (ZERO_ONE)?"],
            snippet: "SSL::allow_dynamic_record_sizing\n  Returns the currently set value for allowing dynamic record sizing\nSSL::allow_dynamic_record_sizing ( 0 | 1 )\n  0 disables dynamic record sizing, 1 enables it.\n  Dynamic record sizing, when using protocols such as HTTP, can increase respnonsiveness of a website.",
            source: "https://clouddocs.f5.com/api/irules/SSL__allow_dynamic_record_sizing.html",
            examples: "when CLIENT_ACCEPTED {\n    SSL::allow_dynamic_record_sizing 1\n}",
            return_value: "SSL::allow_dynamic_record_sizing Returns the currently set dynamic record sizing value. SSL::allow_dynamic_record_sizing [0|1] There is no return value.",
        }),
        ..CommandSpec::DEFAULT
    }
}

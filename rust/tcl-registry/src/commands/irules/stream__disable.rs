//! `STREAM::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "STREAM::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Disables the stream filter for a connection.",
            synopsis: &["STREAM::disable"],
            snippet: "Disables the stream filter for this connection.",
            source: "https://clouddocs.f5.com/api/irules/STREAM__disable.html",
            examples: "# This section only logs matches, and should be removed before using the rule in production.\nwhen STREAM_MATCHED {\n    log local0. \"Matched: [STREAM::match]\"\n}",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}

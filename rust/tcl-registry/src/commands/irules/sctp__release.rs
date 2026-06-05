//! `SCTP::release` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SCTP::release",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Resumes processing and flushes collected data.",
            synopsis: &["SCTP::release (RELEASE_BYTES)?"],
            snippet: "Causes SCTP to resume processing the connection and flush collected data.",
            source: "https://clouddocs.f5.com/api/irules/SCTP__release.html",
            examples: "when CLIENT_ACCEPTED {\n  SCTP::collect 15\n}",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}

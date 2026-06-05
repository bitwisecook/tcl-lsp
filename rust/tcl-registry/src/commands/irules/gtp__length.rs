//! `GTP::length` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "GTP::length",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "This value is returned as read from the message header.",
            synopsis: &["GTP::length ('-message' MESSAGE)?"],
            snippet: "This value is returned as read from the message header.",
            source: "https://clouddocs.f5.com/api/irules/GTP__length.html",
            examples:
                "when GTP_SIGNALLING_INGRESS {\n    log local0. \"GTP length [GTP::length]\"\n}",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}

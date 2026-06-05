//! `md4` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "md4",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the RSA MD4 Message Digest Algorithm message digest of the specified string.",
            synopsis: &["md4"],
            snippet: "Returns the RSA Data Security, Inc.",
            source: "https://clouddocs.f5.com/api/irules/md4.html",
            examples: "",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}

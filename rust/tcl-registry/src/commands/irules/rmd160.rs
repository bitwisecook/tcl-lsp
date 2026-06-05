//! `rmd160` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "rmd160",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the RIPEMD-160 message digest of the specified string.",
            synopsis: &["rmd160 ANY_CHARS"],
            snippet: "Returns the RIPEMD-160 (RACE Integrity Primitives Evaluation Message Digest) message digest of the specified string, or an empty string if an error occurs. Used to ensure data integrity.",
            source: "https://clouddocs.f5.com/api/irules/rmd160.html",
            examples: "when HTTP_REQUEST {\n    binary scan [rmd160 [HTTP::host]] w1 key\n\n    set key [expr {$key & 1}]\n    switch $key {\n        0 { pool my_pool member 1.2.3.4:80 }\n        1 { pool my_pool member 5.6.7.8:80 }\n    }\n}",
            return_value: "rmd160 <string> Returns the RIPEMD-160 message digest of the specified string, or an empty string if an error occurs.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "rmd160 ANY_CHARS" },
        ],
        ..CommandSpec::DEFAULT
    }
}

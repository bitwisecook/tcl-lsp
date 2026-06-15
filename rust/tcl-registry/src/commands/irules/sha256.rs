//! `sha256` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "sha256",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the Secure Hash Algorithm (SHA2) 256-bit message digest of the specified string.",
            synopsis: &["sha256 ANY_CHARS"],
            snippet: "Returns the Secure Hash Algorithm (SHA2) 256-bit message digest of the specified string. If an error occurs, an empty string is returned. Used to ensure data integrity.",
            source: "https://clouddocs.f5.com/api/irules/sha256.html",
            examples: "when HTTP_REQUEST {\n    binary scan [sha256 [HTTP::host]] w1 key\n\n    set key [expr {$key & 1}]\n    switch $key {\n        0 { pool my_pool member 1.2.3.4:80 }\n        1 { pool my_pool member 5.6.7.8:80 }\n    }\n}",
            return_value: "sha256 <string> Returns the Secure Hash Algorithm version 2.0 (SHA2) message digest of the specified string using 256 bit digest length. If an error occurs, an empty string is returned.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "sha256 ANY_CHARS",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::Unknown,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Global,
        }],
        ..CommandSpec::DEFAULT
    }
}

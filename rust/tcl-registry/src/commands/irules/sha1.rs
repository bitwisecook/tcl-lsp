//! `sha1` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "sha1",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the SHA version 1.0 message digest of the specified string.",
            synopsis: &["sha1 ANY_CHARS"],
            snippet: "Returns the Secure Hash Algorithm version 1.0 (SHA1) message digest of the specified string, or if an error occurs, an empty string. Used to ensure data integrity.",
            source: "https://clouddocs.f5.com/api/irules/sha1.html",
            examples: "when HTTP_REQUEST {\n    binary scan [sha1 [HTTP::host]] w1 key\n\n    set key [expr {$key & 1}]\n    switch $key {\n        0 { pool my_pool member 1.2.3.4:80 }\n        1 { pool my_pool member 5.6.7.8:80 }\n    }\n}",
            return_value: "sha1 <string> Returns the Secure Hash Algorithm version 1.0 (SHA1) message digest of the specified string, or if an error occurs, an empty string.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "sha1 ANY_CHARS" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::Unknown,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Global,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}

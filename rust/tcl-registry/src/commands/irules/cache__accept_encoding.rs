//! `CACHE::accept_encoding` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "CACHE::accept_encoding",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Overrides the accept_encoding value used by the cache to store the cached content.",
            synopsis: &["CACHE::accept_encoding ENCODING_STRING"],
            snippet: "Overrides the accept_encoding value used by the cache to store the\ncached content. You can use this command to group various user encoding\nvalues into a single group, to minimize duplicated cached content.\n\nCACHE::accept_encoding <string>\n\n     * Overrides the accept_encoding value used by the cache to store the\n       cached content, according to the specified string.",
            source: "https://clouddocs.f5.com/api/irules/CACHE__accept_encoding.html",
            examples: "",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "CACHE::accept_encoding ENCODING_STRING" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::StreamProfile,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}

//! `CACHE::age` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "CACHE::age",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the age of the document in the cache.",
            synopsis: &["CACHE::age"],
            snippet: "Returns the age of the document in the cache, in seconds.\n\nCACHE::age\n\n     * Returns the age of the document in the cache, in seconds.",
            source: "https://clouddocs.f5.com/api/irules/CACHE__age.html",
            examples: "when CACHE_REQUEST {\n  if { [CACHE::age] > 60 } {\n    CACHE::expire\n    log local0. \"Expiring content: Age > 60 seconds\"\n   }\n}",
            return_value: "Returns the age of the document in the cache, in seconds.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "CACHE::age" },
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

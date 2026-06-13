//! `HTML::comment` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTML::comment",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Query and update HTML comment.",
            synopsis: &["HTML::comment ((append STRING) | (prepend STRING) | remove)?"],
            snippet: "Queries, removes HTML comment or appends/prepends it by a string.\n\nHTML::comment\nReturn the entire HTML comment, including the opening and the closing delimiter.\n\nHTML::comment append <string>\nInsert a string after the closing delimiter of the HTML comment; when multiple appends are issued, the inserted strings are ordered according to the sequence of the append commands as they are issued for the given comment.",
            source: "https://clouddocs.f5.com/api/irules/HTML__comment.html",
            examples: "when HTML_COMMENT_MATCHED {\n    HTML::comment append \"some_string\"\n}",
            return_value: "HTML::comment returns the entire HTML comment; others do not return anything.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["HTML"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "HTML::comment ((append STRING) | (prepend STRING) | remove)?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::StreamProfile,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}

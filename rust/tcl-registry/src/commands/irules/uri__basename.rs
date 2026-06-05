//! `URI::basename` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "URI::basename",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Extracts the basename part of a given uri string.",
            synopsis: &["URI::basename URI_STRING"],
            snippet: "Extracts the basename part of a given uri string.\nFor the following URI:\n/main/index.jsp?user=test&login=check\n\nThe basename is:\n\nindex.jsp",
            source: "https://clouddocs.f5.com/api/irules/URI__basename.html",
            examples: "when HTTP_REQUEST {\n  set base [URI::basename [HTTP::uri]]\n  log local0. \"Basename of uri [HTTP::uri] is $base\"\n}",
            return_value: "Return the basename part of a given uri string.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "URI::basename URI_STRING" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::HttpUri,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Global,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}

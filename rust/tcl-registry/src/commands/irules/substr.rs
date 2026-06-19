//! `substr` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "substr",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns a substring from a string.",
            synopsis: &["substr STRING SKIP_COUNT (TERMINATOR)?"],
            snippet: "A custom iRule function which returns a substring named <string>,\nbased on the values of the <skip_count> and <terminator> arguments.\nNote the following:\n  * The <skip_count> and <terminator> arguments are used in the same\n    way as they are for the findstr command.\n  * The <skip_count> argument is the index into <string> of the first\n    character to be returned, where 0 indicates the first character of\n    <string>.\n  * The <terminator> argument can be either the subtring length or the\n    substring terminating string.",
            source: "https://clouddocs.f5.com/api/irules/substr.html",
            examples: "when HTTP_REQUEST {\n  set uri [substr $uri 1 \"?\"]\n  log local0. \"Uri Part = $uri\"\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "substr STRING SKIP_COUNT (TERMINATOR)?",
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

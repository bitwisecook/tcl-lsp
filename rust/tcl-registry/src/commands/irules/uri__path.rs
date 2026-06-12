//! `URI::path` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "URI::path",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the path portion of the given URI.",
            synopsis: &["URI::path URI_STRING (depth | START | (START END))?"],
            snippet: "Returns the path portion of the given URI.",
            source: "https://clouddocs.f5.com/api/irules/URI__path.html",
            examples: "when RULE_INIT {\n\n    # You can use URI::query against a static string and not in a client-triggered event!\n    log local0. \"\\[URI::query \\\"?param1=val1&param2=val2\\\" param1\\]: [URI::query \"?param1=val1&param2=val2\" param1]\"\n\n    # This doesn't work, as URI::query expects a query string to start with a question mark\n    log local0. \"\\[URI::query \\\"param1=val1&param2=val2\\\" param1\\]: [URI::query \"param1=val1&param2=val2\" param1]\"\n}",
            return_value: "Returns the path portion of the given URI.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "URI::path URI_STRING (depth | START | (START END))?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::HttpUri,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Global,
            },
        ],
        taint_source: Some(TaintColour::TAINTED),
        ..CommandSpec::DEFAULT
    }
}

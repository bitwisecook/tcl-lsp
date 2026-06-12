//! `URI::protocol` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "URI::protocol",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the protocol of the given URI.",
            synopsis: &["URI::protocol URI_STRING"],
            snippet: "Returns the protocol of the given URI.",
            source: "https://clouddocs.f5.com/api/irules/URI__protocol.html",
            examples: "when RULE_INIT {\n        # Loop through some test URLs and URIs and log the URI::protocol value\n        foreach uri [list \\\n                http://test.com \\\n                https://test.com \\\n                ftp://test.com \\\n                sip://test.com \\\n                myproto://test.com \\\n                /test.com \\\n                /uri?url=http://test.example.com/uri \\\n        ] {\n                log local0. \"\\[URI::protocol $uri\\]: [URI::protocol $uri]\"\n        }\n}",
            return_value: "Returns the protocol of the given URI.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "URI::protocol URI_STRING" },
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

//! `URI::port` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "URI::port",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the host port from the given URI.",
            synopsis: &["URI::port URI_STRING"],
            snippet: "Returns the host port from the given URI.",
            source: "https://clouddocs.f5.com/api/irules/URI__port.html",
            examples: "when HTTP_REQUEST {\n  set port [URI::port [HTTP::uri]]\n  log local0. \"Host port of uri [HTTP::uri] is $port\"\n}",
            return_value: "Returns the host port from the given URI.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "URI::port URI_STRING" },
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

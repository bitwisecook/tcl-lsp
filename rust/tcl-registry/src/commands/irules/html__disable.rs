//! `HTML::disable` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "HTML::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Disable the processing of HTML for this transaction.",
            synopsis: &["HTML::disable"],
            snippet: "Disable the processing of HTML for this transaction.",
            source: "https://clouddocs.f5.com/api/irules/HTML__disable.html",
            examples: "when HTTP_RESPONSE {\n    if {$host == \"www.f5.com\"} {\n        HTML::disable\n    }\n    log local0. \"host: $host\"\n}",
            return_value: "empty return code.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "HTML::disable",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::StreamProfile,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}

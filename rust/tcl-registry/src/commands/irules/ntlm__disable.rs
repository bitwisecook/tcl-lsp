//! `NTLM::disable` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "NTLM::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Disables processing for NTLM.",
            synopsis: &["NTLM::disable"],
            snippet: "Disables processing for NTLM",
            source: "https://clouddocs.f5.com/api/irules/NTLM__disable.html",
            examples: "when HTTP_RESPONSE {\n    if { [string tolower [HTTP::header values \"WWW-Authenticate\"]] contains \"negotiate\"} {\n        ONECONNECT::detach disable\n        NTLM::disable\n    }\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "NTLM::disable",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ApmState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}

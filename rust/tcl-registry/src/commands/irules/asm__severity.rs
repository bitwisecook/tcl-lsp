//! `ASM::severity` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::severity",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Returns the overall severity of the violations found in the transaction (both request and response).",
            synopsis: &["ASM::severity"],
            snippet: "Returns the overall severity of the violations found in the transaction\n(both request and response). This equals to the maximum severity of all\nthese violations",
            source: "https://clouddocs.f5.com/api/irules/ASM__severity.html",
            examples: "when ASM_REQUEST_DONE {\n   if {[ASM::violation count] > 3 and [ASM::severity] eq \"Error\"} {\n      ASM::raise VIOLATION_TOO_MANY_VIOLATIONS\n   }\n}",
            return_value: "+ Null string (in case there was no violation until the time the command is invoked) + Emergency + Alert + Critical + Error + Warning + Notice + Informational",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["ASM"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "ASM::severity",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::AsmState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Client,
        }],
        ..CommandSpec::DEFAULT
    }
}

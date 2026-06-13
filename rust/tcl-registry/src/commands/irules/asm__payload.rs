//! `ASM::payload` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::payload",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Retrieves or replaces the payload collected by ASM.",
            synopsis: &["ASM::payload (LENGTH | (OFFSET LENGTH))?", "ASM::payload length", "ASM::payload replace OFFSET LENGTH ASM_PAYLOAD"],
            snippet: "This command retrieves or replaces the payload collected by ASM.",
            source: "https://clouddocs.f5.com/api/irules/ASM__payload.html",
            examples: "when ASM_REQUEST_VIOLATION\n{\n  set x [ASM::violation_data]\n  if {([lindex $x 0] contains \"VIOLATION_EVASION_DETECTED\")}\n   {\n      ASM::payload replace 0 0 \"1234567890\"\n   }\n}",
            return_value: "",
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
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "ASM::payload (LENGTH | (OFFSET LENGTH))?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::AsmState,
                reads: true,
                writes: true,
                connection_side: ConnectionSide::Client,
            },
        ],
        taint_source: Some(TaintColour::TAINTED),
        ..CommandSpec::DEFAULT
    }
}

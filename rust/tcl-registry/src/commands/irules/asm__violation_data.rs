//! `ASM::violation_data` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::violation_data",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "This command exposes violation data using a multiple buffers instance.",
            synopsis: &["ASM::violation_data"],
            snippet: "This command exposes violation data using a multiple buffers instance.\n\nNote: Starting version 11.5.0 this command is deprecated and replaced by\nASM::violation, ASM::support_id, ASM::severity and ASM::client_ip, which\nhave more convenient syntax and enhanced options. It is kept for\nbackward compatibility.",
            source: "https://clouddocs.f5.com/api/irules/ASM__violation_data.html",
            examples: "when ASM_REQUEST_VIOLATION\n{\n    set x [ASM::violation_data]\n\n    foreach i $x {\n      log local0. \"i=$i\"\n    }\n}",
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
            FormSpec { kind: FormKind::Default, synopsis: "ASM::violation_data" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::AsmState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Client,
            },
        ],
        deprecated_replacement: Some("ASM::violation"),
        ..CommandSpec::DEFAULT
    }
}

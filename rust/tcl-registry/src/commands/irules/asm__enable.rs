//! `ASM::enable` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Enables plugin processing on the connection.",
            synopsis: &["ASM::enable ASM_POLICY"],
            snippet: "Enables the ASM plugin processing for the current TCP connection.\nASM will remain enabled on the current TCP connection until it is closed or\nASM::disable is called.",
            source: "https://clouddocs.f5.com/api/irules/ASM__enable.html",
            examples: "#Enabling an asm policy called asmb in the Common partition in v11.4.x and Later\nwhen HTTP_REQUEST {\n    ASM::enable \"/Common/asmb\"\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["FASTHTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "ASM::enable ASM_POLICY",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::AsmState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Client,
        }],
        xc_translatable: Some(true),
        ..CommandSpec::DEFAULT
    }
}

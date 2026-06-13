//! `ASM::disable` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Disables plugin processing on the connection.",
            synopsis: &["ASM::disable"],
            snippet: "Disables the ASM plugin processing for the current TCP connection.\nASM will remain disabled on the current TCP connection until it is closed or\nASM::enable is called.",
            source: "https://clouddocs.f5.com/api/irules/ASM__disable.html",
            examples: "# for 11.4.0+ the command should be used in HTTP_REQUEST event\nwhen HTTP_CLASS_SELECTED {\n  ASM::enable\n  # Disable ASM for HTTP paths ending in .jpg\n  if { [HTTP::path] ends_with \".jpg\" } {\n    ASM::disable\n  }\n}",
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
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "ASM::disable" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::AsmState,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Client,
            },
        ],
        xc_translatable: Some(true),
        ..CommandSpec::DEFAULT
    }
}

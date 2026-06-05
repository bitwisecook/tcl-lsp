//! `WAM::enable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "WAM::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Enables Web Accelerator plugin processing on the connection.",
            synopsis: &["WAM::enable"],
            snippet: "Enables the WAM plugin for the current TCP connection. WAM will remain\nenabled on the current TCP connection until it is closed or\nWAM::disable is called.",
            source: "https://clouddocs.f5.com/api/irules/WAM__enable.html",
            examples: "# Disable WAM for HTTP paths ending in .php\nwhen HTTP_REQUEST {\n  WAM::enable\n  if { [HTTP::path] ends_with \".php\" } {\n    WAM::disable\n  }\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["HTTP"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "WAM::enable" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::StreamProfile,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        deprecated_replacement: Some("(removed)"),
        ..CommandSpec::DEFAULT
    }
}

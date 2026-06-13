//! `DIAMETER::command` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::command",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Gets or sets the command-code.",
            synopsis: &["DIAMETER::command (DIAMETER_COMMAND_CODE)?"],
            snippet: "The DIAMETER::command gets or sets the command code in the Diameter message header.",
            source: "https://clouddocs.f5.com/api/irules/DIAMETER__command.html",
            examples: "when DIAMETER_INGRESS {\n    log local0. \"Received a DIAMETER command, with code [DIAMETER::command]\"\n}",
            return_value: "If new command-code value is not provided, returns command code of current Diameter message",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["DIAMETER", "MR"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "DIAMETER::command (DIAMETER_COMMAND_CODE)?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::NetworkIo,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}

//! `PEM::enable` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "PEM::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "PEM iRule command to enable PEM feature on current flow.",
            synopsis: &["PEM::enable"],
            snippet: "Enable PEM for the current flow. Note that the config must already contain a Policy Enforcement Profile.",
            source: "https://clouddocs.f5.com/api/irules/PEM__enable.html",
            examples: "when HTTP_REQUEST {\n    PEM::enable;\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "PEM::enable" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::ConnectionControl,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}

//! `TCP::enhanced_loss_recovery` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::enhanced_loss_recovery",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Toggles enhanced loss recovery.",
            synopsis: &["TCP::enhanced_loss_recovery BOOL_VALUE"],
            snippet: "Enables or disables enhanced loss recovery which recovers from random packet loss more effectively.",
            source: "https://clouddocs.f5.com/api/irules/TCP__enhanced_loss_recovery.html",
            examples: "when CLIENT_ACCEPTED {\n    TCP::enhanced_loss_recovery enable\n}",
            return_value: "None.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "TCP::enhanced_loss_recovery BOOL_VALUE" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::TcpState,
                reads: true,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}

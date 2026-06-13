//! `TCP::naglemode` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::naglemode",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns setting of Nagle mode.",
            synopsis: &["TCP::naglemode"],
            snippet: "Returns the Nagle mode of a TCP flow.",
            source: "https://clouddocs.f5.com/api/irules/TCP__naglemode.html",
            examples: "# Get the TCP Nagle mode of the TCP flow.\nwhen CLIENT_ACCEPTED {\n    log local0. \"TCP Nagle mode: [TCP::naglemode]\"\n}",
            return_value: "- enabled - disabled - auto",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "TCP::naglemode" },
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

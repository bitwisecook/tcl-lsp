//! `TCP::proxybuffer` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::proxybuffer",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Sets proxy buffer low and high thresholds.",
            synopsis: &["TCP::proxybuffer ('auto' | (LOW HIGH))"],
            snippet: "Sets thresholds at which the proxy buffer accepts (low) and stops accepting (high) new data, in bytes.",
            source: "https://clouddocs.f5.com/api/irules/TCP__proxybuffer.html",
            examples: "when SERVER_CONNECTED {\n    TCP::proxybuffer 100000 500000\n}",
            return_value: "None.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: true,
            transport: Some("tcp"),
            profiles: &[],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "TCP::proxybuffer ('auto' | (LOW HIGH))",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}

//! `TCP::push_flag` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::push_flag",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "This command can be used to set/get the PUSH flag mode of a TCP connection.",
            synopsis: &["TCP::push_flag ('default' | 'none' | 'one' | 'auto')?"],
            snippet: "TCP::push_flag returns the PUSH flag mode of a TCP connection.\nTCP::push_flag mode sets the PUSH flag mode to specified mode.",
            source: "https://clouddocs.f5.com/api/irules/TCP__push_flag.html",
            examples: "# get/set the PUSH flag mode of the TCP flow.\nwhen CLIENT_ACCEPTED {\n    log local0. \"TCP set PUSH flag mode: [TCP::push_flag auto]\"\n    log local0. \"TCP get PUSH flag more: [TCP::push_flag]\"\n}",
            return_value: "TCP::push_flag returns the PUSH flag mode.",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "TCP::push_flag ('default' | 'none' | 'one' | 'auto')?",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::TcpState,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}

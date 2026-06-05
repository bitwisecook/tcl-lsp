//! `TCP::rcv_size` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::rcv_size",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the maximum allowed advertised window size by BIG-IP.",
            synopsis: &["TCP::rcv_size"],
            snippet: "TCP configuration limits the advertised received window to control the memory impact of any single connection.",
            source: "https://clouddocs.f5.com/api/irules/TCP__rcv_size.html",
            examples: "when CLIENT_CLOSED {\n    # Get BIGIP's receive window size.\n    log local0. \"BIGIP's rcv wnd size: [TCP::rcv_size]\"\n}",
            return_value: "The maximum receive window in bytes.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "TCP::rcv_size" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::TcpState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}

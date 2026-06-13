//! `BIGTCP::release_flow` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "BIGTCP::release_flow",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Releases a flow from BIGTCP's control.",
            synopsis: &["BIGTCP::release_flow"],
            snippet: "This command releases a flow from BIGTCP's control. After calling this method, the flow will be in passthrough mode.  In this mode, no more data will be processed by any filters or iRules.",
            source: "https://clouddocs.f5.com/api/irules/BIGTCP__release_flow.html",
            examples: "when CLIENT_ACCEPTED {\n    BIGTCP::release_flow;\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "BIGTCP::release_flow" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::TcpState,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}

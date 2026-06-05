//! `TCP::lossfilterburst` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "TCP::lossfilterburst",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Gets the TCP Loss Ignore Burst Parameter.",
            synopsis: &["TCP::lossfilterburst"],
            snippet: "Gets the maximum size burst loss (in packets) before triggering congestion response.\n  * Burst range is valid from 0 to 32. Higher values decrease the\n    chance of performing congestion control.",
            source: "https://clouddocs.f5.com/api/irules/TCP__lossfilterburst.html",
            examples: "when SERVER_CONNECTED {\n    # Set loss filter burst to a maximum of 3\n    if { [TCP::lossfilterburst] > 3 } {\n        TCP::lossfilter [TCP::lossfilterrate] 3\n    }\n}",
            return_value: "TCP Loss Ignore Burst in packets.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "TCP::lossfilterburst" },
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

//! `SSL::mode` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::mode",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Gets the enabled/disabled state of SSL.",
            synopsis: &["SSL::mode"],
            snippet: "Gets the enabled/disabled state of SSL",
            source: "https://clouddocs.f5.com/api/irules/SSL__mode.html",
            examples: "when CLIENT_ACCEPTED {\n    if { [TCP::local_port] != 443 } {\n        SSL::disable\n    }\n}",
            return_value: "SSL::mode Gets the enabled/disabled state of SSL. Returns 1 if it is enabled, and 0 if it is disabled.",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "SSL::mode" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::SslState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}

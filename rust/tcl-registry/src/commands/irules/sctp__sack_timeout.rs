//! `SCTP::sack_timeout` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "SCTP::sack_timeout",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the SCTP's delayed selective acknowledgement timeout.",
            synopsis: &["SCTP::sack_timeout (clientside | serverside)?"],
            snippet: "Returns the SCTP's delayed selective acknowledgement timeout. Can specify the value on clientside or serverside.",
            source: "https://clouddocs.f5.com/api/irules/SCTP__sack_timeout.html",
            examples: "when SERVER_CONNECTED {\n        log local0.info \"SCTP selective acknowledgement timeout value is [SCTP::sack_timeout]\"\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "SCTP::sack_timeout (clientside | serverside)?" },
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

//! `SCTP::collect` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SCTP::collect",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Collects the specified amount of content data.",
            synopsis: &["SCTP::collect (COLLECT_BYTES)?"],
            snippet: "Causes SCTP to start collecting the specified amount of content data. After collecting the data, event CLIENT_DATA will be triggered.\n\nSCTP::collect <length>\n    Causes SCTP to start collecting the specified amount of content data. The parameter specifies the minimum number of bytes to collect.\n\nSCTP::collect\n    When length is not specified, CLIENT_DATA will be triggered for every received packet. To stop collecting data, use SCTP::release.",
            source: "https://clouddocs.f5.com/api/irules/SCTP__collect.html",
            examples: "when CLIENT_ACCEPTED {\n  SCTP::collect 15\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "SCTP::collect (COLLECT_BYTES)?" },
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

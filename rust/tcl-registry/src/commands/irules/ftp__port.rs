//! `FTP::port` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "FTP::port",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Controls the range of passive mode FTP ephemeral ports.",
            synopsis: &["FTP::port FIRST (LAST)?"],
            snippet:
                "This command allows control over the range of passive mode FTP\nephemeral ports.",
            source: "https://clouddocs.f5.com/api/irules/FTP__port.html",
            examples: "when SERVER_CONNECTED {\n  FTP::port 5000 5999\n}",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "FTP::port FIRST (LAST)?",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::FtpState,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}

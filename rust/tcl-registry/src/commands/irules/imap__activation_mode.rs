//! `IMAP::activation_mode` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "IMAP::activation_mode",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Get or set the activation mode for IMAP STARTTLS.",
            synopsis: &["IMAP::activation_mode (none | allow | require)?"],
            snippet: "Sets the IMAP activation mode to none (IMAP STARTTLS detection will not activate), allow (IMAP will optionally activate TLS if client or server support STARTTLS), or require (IMAP will require that both client and server support STARTTLS). Returns the current activation mode if no option is specified.",
            source: "https://clouddocs.f5.com/api/irules/IMAP__activation_mode.html",
            examples: "when CLIENT_ACCEPTED {\n                if { ([IP::addr [IP::client_addr] equals 10.0.0.0/8]) } {\n                    IMAP::activation_mode require\n                }\n\n                if { ([IP::addr [IP::client_addr] equals 10.0.0.0/8]) } {\n                    set mode [IMAP::activation_mode]\n                }\n            }",
            return_value: "",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "IMAP::activation_mode (none | allow | require)?",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}

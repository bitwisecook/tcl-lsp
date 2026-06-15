//! `DIAMETER::retry` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::retry",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Tries to send the Diameter message contained in the binary array \"binary_message\".",
            synopsis: &["DIAMETER::retry DIAMETER_MESSAGE (BOOL_ACROSS)?"],
            snippet: "This iRule command tries to send the Diameter message contained in the\nbinary array \"binary_message\".  This command, in conjunction with the\nDIAMETER::message command, can be used to write an iRule that will\nhold and retry messages.\n\nIf the optional argument \"across\" is specified as 1, the message will\nbe sent through the proxy and trigger the various iRule events.  If it\nis specified as 0, or not specified, the message will be sent directly\nand not experience any iRules, persistence, or other processing.",
            source: "https://clouddocs.f5.com/api/irules/DIAMETER__retry.html",
            examples: "when DIAMETER_EGRESS {\n   if { [DIAMETER::is_request] } {\n      set saved_message([DIAMETER::header hopid]) [DIAMETER::message]\n   }\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["DIAMETER", "MR"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "DIAMETER::retry DIAMETER_MESSAGE (BOOL_ACROSS)?",
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

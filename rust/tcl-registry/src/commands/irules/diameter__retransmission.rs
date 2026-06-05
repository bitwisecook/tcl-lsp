//! `DIAMETER::retransmission` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::retransmission",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Gets of sets the current message's retransmission settings.",
            synopsis: &["DIAMETER::retransmission action"],
            snippet: "This command allows the setting or getting of the current message\\'s\nretransmission settings.\n        \nGets the current message\\'s retransmission action.\nPossible actions are:\n\n* \"disabled\" - this request messages will not be queued for\n  retransmission\n\n* \"busy\" - when retransmission is triggered for this request message\n  an answer message with a 'busy' result code will be returned to the\n  originator.\n\n* \"unable\" - when retransmission is triggered for this request message\n  an answer message with a 'unable to deliver' result code will be\n  returned to the originator.",
            source: "https://clouddocs.f5.com/api/irules/DIAMETER__retransmission.html",
            examples: "when DIAMETER_INGRESS {\n    DIAMETER::retransmission action retransmit\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["DIAMETER"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "DIAMETER::retransmission action" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::NetworkIo,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}

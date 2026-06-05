//! `DIAMETER::persist` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::persist",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns the persistence key being used for the current message.",
            synopsis: &["DIAMETER::persist", "DIAMETER::persist reset", "DIAMETER::persist use"],
            snippet: "This iRule command returns the persistence key being used for the\ncurrent message. If new persist key is provided, the existing\npersistence key will be replaced. The value of the new key MUST be the\nvalue of a valid AVP in the message. An AVP attribute name should not\nbe given as the new key value.\n\nIf bidirection is specified as false, disable(d), no, 0, or is\nunspecified, then persistence is not bidirectional. If bidirection is\nspecified as true, enable(d), yes, or 1 this persistence entry is\nbidirectional.",
            source: "https://clouddocs.f5.com/api/irules/DIAMETER__persist.html",
            examples: "when DIAMETER_INGRESS {\n    log local0. \"Received a DIAMETER message, persistence key is [DIAMETER::persist]\"\n}",
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
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "DIAMETER::persist" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::PersistenceTable,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}

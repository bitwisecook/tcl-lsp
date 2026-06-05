//! `DIAMETER::result` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "DIAMETER::result",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Gets or sets the value of the result-code attribute-value pair.",
            synopsis: &["DIAMETER::result (DIAMETER_RESULT_CODE)?"],
            snippet: "This iRule command gets or sets the value of the result-code (code\n268) attribute-value pair.",
            source: "https://clouddocs.f5.com/api/irules/DIAMETER__result.html",
            examples: "when DIAMETER_INGRESS {\n    log local0. \"Received a DIAMETER message with result code [DIAMETER::result]\"\n}",
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
            FormSpec { kind: FormKind::Default, synopsis: "DIAMETER::result (DIAMETER_RESULT_CODE)?" },
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

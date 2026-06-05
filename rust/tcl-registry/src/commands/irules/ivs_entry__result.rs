//! `IVS_ENTRY::result` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "IVS_ENTRY::result",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Sends a result code to the IVS client.",
            synopsis: &["IVS_ENTRY::result (noop | modified | response)"],
            snippet: "Send a result code to the IVS (Internal Virtual Server) client\n(usually ADAPT). The intent is to allow an IVS to be used in a\nuser-defined way without a specific IVS profile like \"icap\". If an\n\"icap\" profile is present, IVS_ENTRY::result should not be used as\nit would cause a second result to be sent to the IVS client\n(usually ADAPT), with undefined effect.",
            source: "https://clouddocs.f5.com/api/irules/IVS_ENTRY__result.html",
            examples: "when IVS_ENTRY_REQUEST {\n                # Tell primary virtual the IVS will not handle this request\n                IVS_ENTRY::result noop\n            }",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["ICAP", "IVS_ENTRY"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "IVS_ENTRY::result (noop | modified | response)" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::Unknown,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}

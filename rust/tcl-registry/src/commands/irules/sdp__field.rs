//! `SDP::field` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SDP::field",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Gets or Sets the value in a given SDP field.",
            synopsis: &["SDP::field FIELD_NAME ((INDEX) (NEW_VALUE)?)?"],
            snippet: "This command will get or set the value of a specific SDP field",
            source: "https://clouddocs.f5.com/api/irules/SDP__field.html",
            examples: "when SIP_REQUEST {\n    log local0. \"SDP field b: [SDP::field b]\"\n    SDP::field c 0 \"IN IP4 10.10.1.150\"\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "SDP::field FIELD_NAME ((INDEX) (NEW_VALUE)?)?" },
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

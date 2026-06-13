//! `ECA::status` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ECA::status",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns NTLM authentication result.",
            synopsis: &["ECA::status"],
            snippet: "The ECA::status command returns NTLM authentication result such as NTLM_STATUS_OK, NTLM_STATUS_WRONG_PASSWORD, NTLM_STATUS_NO_SUCH_USER.",
            source: "https://clouddocs.f5.com/api/irules/ECA__status.html",
            examples: "",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "ECA::status" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::ApmState,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}

//! `ECA::client_machine_name` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ECA::client_machine_name",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns NTLM authenticating user's machine name.",
            synopsis: &["ECA::client_machine_name"],
            snippet: "The ECA::client_machine_name command returns NTLM returns authenticating user's machine name",
            source: "https://clouddocs.f5.com/api/irules/ECA__client_machine_name.html",
            examples: "",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "ECA::client_machine_name" },
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

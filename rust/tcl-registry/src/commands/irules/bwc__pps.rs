//! `BWC::pps` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "BWC::pps",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "This command is used to modify pps value for the policy.",
            synopsis: &["BWC::pps BW_VALUE"],
            snippet: "After a policy is created, irule can modify the pps for the session. The value is specified in packets per second.",
            source: "https://clouddocs.f5.com/api/irules/BWC__pps.html",
            examples: "when CLIENT_ACCEPTED {\n    set mycookie [IP::remote_addr]:[TCP::remote_port]\n    BWC::policy attach gold_user $mycookie\n    BWC::pps 77\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "BWC::pps BW_VALUE" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::ConnectionControl,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}

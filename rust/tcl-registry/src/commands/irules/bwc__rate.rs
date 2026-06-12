//! `BWC::rate` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "BWC::rate",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "This command is used to modify max-user rate for dynamic policy.",
            synopsis: &["BWC::rate SESSION_ID BW_VALUE", "BWC::rate SESSION_ID APPLICATION_NAME BW_VALUE"],
            snippet: "This command is used to modify max-user rate for dynamic policy after it is created. This irule can modify the rate for a session or category.",
            source: "https://clouddocs.f5.com/api/irules/BWC__rate.html",
            examples: "when CLIENT_ACCEPTED {\n    set mycookie [IP::remote_addr]:[TCP::remote_port]\n    BWC::policy attach gold_user $mycookie\n    BWC::color set gold_user p2p\n    BWC::rate $mycookie p2p 1000000bps\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "BWC::rate SESSION_ID BW_VALUE" },
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

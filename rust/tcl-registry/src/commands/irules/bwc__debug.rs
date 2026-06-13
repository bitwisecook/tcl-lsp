//! `BWC::debug` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "BWC::debug",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "This command is used for troubleshooting a bwc policy instance.",
            synopsis: &["BWC::debug ('start')", "BWC::debug ('stop')"],
            snippet: "This command enables debug logs on per policy instance. However the bwc sys db variables for bwc trace need to be enabled and appropriate levels needs to be set as required.",
            source: "https://clouddocs.f5.com/api/irules/BWC__debug.html",
            examples: "when CLIENT_ACCEPTED {\n    set mycookie [IP::remote_addr]:[TCP::remote_port]\n    BWC::policy attach test_pol $mycookie\n    log  local0. \"BWC::policy attach  $mycookie\"\n    BWC::debug start session\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "BWC::debug ('start')" },
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

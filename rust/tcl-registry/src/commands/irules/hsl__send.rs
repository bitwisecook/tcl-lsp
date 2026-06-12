//! `HSL::send` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "HSL::send",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Sends data via High Speed Logging.",
            synopsis: &["HSL::send HANDLE DATA"],
            snippet: "Send data via High Speed Logging",
            source: "https://clouddocs.f5.com/api/irules/HSL__send.html",
            examples: "when CLIENT_ACCEPTED {\n    set hsl [HSL::open -proto UDP -pool syslog_server_pool]\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "HSL::send HANDLE DATA" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::LogIo,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        arg_roles: &[(0, ArgRole::Channel)],
        ..CommandSpec::DEFAULT
    }
}

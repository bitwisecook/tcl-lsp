//! `AVR::enable` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "AVR::enable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Enables the AVR plugin for the current connection.",
            synopsis: &["AVR::enable"],
            snippet: "Enables the AVR plugin for the current connection. AVR will remain\nenabled on the current connection until it is closed or\nAVR::disable is called.\n\nNote that enabling AVR alone within the iRule only ensures the\nmessage reaches the AVR plugin, it doesn't ensure that statistics\nwill be gathered.",
            source: "https://clouddocs.f5.com/api/irules/AVR__enable.html",
            examples: "",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "AVR::enable" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::LogIo,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}

//! `HSL::open` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "HSL::open",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        return_type: Some(TclType::Channel),
hover: Some(HoverSnippet {
            summary: "Opens a handle for High Speed Logging communication.",
            synopsis: &["HSL::open ('-publisher' | '-pub') PUBLISHER", "HSL::open '-proto' ('UDP' | 'TCP') '-pool' POOL_OBJ"],
            snippet: "Open a handle for High Speed Logging communication. After creating the\nconnection, send data on the connection using HSL::send.",
            source: "https://clouddocs.f5.com/api/irules/HSL__open.html",
            examples: "#2\nwhen CLIENT_ACCEPTED {\n    set hsl [HSL::open -publisher /Common/lpAll]\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "HSL::open ('-publisher' | '-pub') PUBLISHER" },
        ],
        options: &[
            OptionSpec { name: "-publisher", takes_value: true, value_hint: "", detail: "Option -publisher.", dialects: None },
            OptionSpec { name: "-pub", takes_value: true, value_hint: "", detail: "Option -pub.", dialects: None },
            OptionSpec { name: "-proto", takes_value: true, value_hint: "", detail: "Option -proto.", dialects: None },
            OptionSpec { name: "-pool", takes_value: true, value_hint: "", detail: "Option -pool.", dialects: None },
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

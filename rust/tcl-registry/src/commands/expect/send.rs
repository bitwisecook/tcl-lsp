//! `send` command.
use crate::prelude::*;
const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-i",
        takes_value: true,
        value_hint: "spawn_id",
        detail: "Send to the specified spawn id.",
        dialects: None,
    },
    OptionSpec {
        name: "-raw",
        takes_value: false,
        value_hint: "",
        detail: "Send without any translation.",
        dialects: None,
    },
    OptionSpec {
        name: "-null",
        takes_value: false,
        value_hint: "",
        detail: "Send null characters.",
        dialects: None,
    },
    OptionSpec {
        name: "-break",
        takes_value: false,
        value_hint: "",
        detail: "Send a break condition.",
        dialects: None,
    },
    OptionSpec {
        name: "-s",
        takes_value: false,
        value_hint: "",
        detail: "Send slowly (obey send_slow parameters).",
        dialects: None,
    },
    OptionSpec {
        name: "-h",
        takes_value: false,
        value_hint: "",
        detail: "Send as if a human were typing (obey send_human parameters).",
        dialects: None,
    },
    OptionSpec {
        name: "--",
        takes_value: false,
        value_hint: "",
        detail: "End of options.",
        dialects: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "send ?-flags? string",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "send",
        dialects: Some(DialectSet::EXPECT),
        arity: Arity::at_least(1),
hover: Some(HoverSnippet {
            summary: "Send a string to the current spawned process.",
            synopsis: &["send ?-flags? string"],
            snippet: "Sends *string* to the process identified by the current ``spawn_id``. Use ``-s`` for slow sending or ``-h`` for human-like typing.",
            source: "Expect send(1)",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        options: OPTIONS,
        ..CommandSpec::DEFAULT
    }
}

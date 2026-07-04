//! `control::control` command.
use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "control::control command option ?arg arg ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "control::control",
        dialects: None,
        arity: Arity::at_least(2),
        hover: Some(HoverSnippet {
            summary: "Configure the customizable commands of the control package.",
            synopsis: &["control::control command option ?arg arg ...?"],
            snippet: "Configuration command for customizing other public commands of the control package (for example, enabling/disabling control::assert). The valid option and arguments depend on the named command.",
            source: "tcllib control package",
            examples: "",
            return_value: "",
        }),
        forms: FORMS,
        tcllib_package: Some("control"),
        required_package: Some("control"),
        ..CommandSpec::DEFAULT
    }
}

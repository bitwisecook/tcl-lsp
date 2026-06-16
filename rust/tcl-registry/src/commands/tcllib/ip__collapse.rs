//! `ip::collapse` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "ip::collapse addressList",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ip::collapse",
        traits: Traits::PURE,
        dialects: None,
        arity: Arity::exact(1),
        return_type: Some(TclType::List),
        hover: Some(HoverSnippet {
            summary: "Collapse a list of IP addresses or subnets into the minimal set.",
            synopsis: &["ip::collapse addressList"],
            snippet: "",
            source: "tcllib ip package",
            examples: "",
            return_value: "A list of collapsed address ranges.",
        }),
        forms: FORMS,
        tcllib_package: Some("ip"),
        required_package: Some("ip"),
        ..CommandSpec::DEFAULT
    }
}

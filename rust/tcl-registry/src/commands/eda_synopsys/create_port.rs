//! `create_port` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "create_port port_name ?-direction direction?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "create_port",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Create a new port in the design.",
            &["create_port port_name ?-direction direction?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}

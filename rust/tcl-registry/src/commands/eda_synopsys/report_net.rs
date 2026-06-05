//! `report_net` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "report_net ?-nosplit? ?-connections? ?-verbose? ?net_list?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_net",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Report net information.",
            &["report_net ?-nosplit? ?-connections? ?-verbose? ?net_list?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}

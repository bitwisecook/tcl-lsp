//! `upgrade_ip` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "upgrade_ip ?-srcset srcset? ?-quiet? ?objects?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "upgrade_ip",
        dialects: Some(DialectSet::XILINX),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Upgrade IP cores to a newer version.",
            &["upgrade_ip ?-srcset srcset? ?-quiet? ?objects?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}

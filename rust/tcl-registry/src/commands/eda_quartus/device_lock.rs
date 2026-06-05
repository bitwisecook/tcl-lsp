//! `device_lock` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "device_lock ?-timeout seconds?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "device_lock",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Lock a JTAG device for exclusive access.",
            &["device_lock ?-timeout seconds?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}

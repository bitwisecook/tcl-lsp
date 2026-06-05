//! `device_unlock` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "device_unlock",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "device_unlock",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Unlock a previously locked JTAG device.",
            &["device_unlock"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}

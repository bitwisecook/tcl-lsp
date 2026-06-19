//! `set_scan_configuration` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "set_scan_configuration ?-chain_count n? ?-clock_mixing mix_type? ?-style style?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "set_scan_configuration",
        dialects: Some(DialectSet::SYNOPSYS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Configure scan chain parameters.",
            &["set_scan_configuration ?-chain_count n? ?-clock_mixing mix_type? ?-style style?"],
            "F5",
        )),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}

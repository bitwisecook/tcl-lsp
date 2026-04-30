//! `report_min_pulse_width` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "report_min_pulse_width",
        dialects: Some(DialectSet::QUARTUS),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief(
            "Report minimum pulse width violations.",
            &["report_min_pulse_width ?-nworst n? ?-file file?"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}

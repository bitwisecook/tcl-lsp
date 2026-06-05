//! `report_min_pulse_width` command.
use crate::prelude::*;
const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "report_min_pulse_width ?-nworst n? ?-file file?",
}];

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
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}

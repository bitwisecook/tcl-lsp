//! `create_analysis_view` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "create_analysis_view",
        dialects: Some(DialectSet::CADENCE),
        arity: Arity::at_least(1),
        hover: Some(HoverSnippet::brief(
            "Create an analysis view combining mode and corner.",
            &["create_analysis_view -name name -constraint_mode mode -delay_corner corner"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}

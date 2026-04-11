//! `ASM::signature` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ASM::signature",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Returns the list of signatures.", &["ASM::signature (ids | names | set_names | staged_ids | staged_names | staged_set_names)"], "F5 iRules")),
        ..CommandSpec::DEFAULT
    }
}

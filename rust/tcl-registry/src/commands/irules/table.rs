//! `table` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "table",
        traits: Traits::DIAGRAM_ACTION,
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Provides enhanced access to the session table.", &["table set (((-mustexist | -excl) -notouch ((-subtable TABLE_NAME) | -georedundancy))# ('--')?)? KEY "], "F5 iRules")),
        ..CommandSpec::DEFAULT
    }
}

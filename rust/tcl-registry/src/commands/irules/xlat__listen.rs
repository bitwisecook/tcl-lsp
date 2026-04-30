//! `XLAT::listen` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "XLAT::listen",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet::brief("Creates a related ephemeral listener.", &["XLAT::listen (-hairpin)? (-inherit-main-rules)? (-single-connection)? (-translation-loose)? (XLAT_LI"], "F5 iRules")),
        ..CommandSpec::DEFAULT
    }
}

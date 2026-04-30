//! `base64::encode` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "base64::encode",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::new(1, 5),
        hover: Some(HoverSnippet::brief(
            "Encode binary data as a base64 string.",
            &["base64::encode ?-maxlen maxlen? ?-wrapchar wrapchar? data"],
            "F5",
        )),
        ..CommandSpec::DEFAULT
    }
}

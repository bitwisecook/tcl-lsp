//! `http::config` command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "http::config",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Get or set http package configuration options.",
            synopsis: &["http::config", "http::config ?options?"],
            snippet: "Without arguments returns a list of all configuration option/value pairs.  With arguments sets one or more options: ``-accept``, ``-proxyhost``, ``-proxyport``, ``-proxyfilter``, ``-urlencoding``, ``-useragent``.",
            source: "Tcl stdlib http package",
            examples: "",
            return_value: "",
        }),
        required_package: Some("http"),
        ..CommandSpec::DEFAULT
    }
}

//! `WAM::disable` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "WAM::disable",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Disables Web Accelerator plugin processing on the connection.",
            synopsis: &["WAM::disable"],
            snippet: "Disables the WAM plugin for the current TCP connection. WAM will remain\ndisabled on the current TCP connection until it is closed or\nWAM::enable is called.",
            source: "https://clouddocs.f5.com/api/irules/WAM__disable.html",
            examples: "# Disable WAM for HTTP paths ending in .php\nwhen HTTP_REQUEST {\n  if { [HTTP::path] ends_with \".php\" } {\n    WAM::disable\n  } else {\n    WAM::enable\n  }\n}",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}

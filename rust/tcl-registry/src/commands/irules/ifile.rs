//! `ifile` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ifile",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Returns content and attributes from external files on the BIG-IP system.",
            synopsis: &["ifile 'listall'", "ifile ("],
            snippet: "This iRules command returns content and attributes from external files\non the BIG-IP system",
            source: "https://clouddocs.f5.com/api/irules/ifile.html",
            examples: "when HTTP_REQUEST {\n   # Retrieve the file contents, send it in an HTTP 200 response and clear the temporary variable\n   set ifileContent [ifile get \"/Common/iFile-index.html\"]\n   HTTP::respond 200 content $ifileContent\n   unset ifileContent\n}",
            return_value: "",
        }),
        ..CommandSpec::DEFAULT
    }
}

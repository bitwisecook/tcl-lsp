//! `SSL::session` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "SSL::session",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Drops a session from the SSL session cache.",
            synopsis: &["SSL::session invalidate ( drop | nodrop )?"],
            snippet: "Invalidates the current session. If no parameter is specified, or the \"drop\" parameter is specified, this commands drops the current SSL session ID from the session cache to prevent reuse of the session. If \"nodrop\" is specified, the current session will be invalidated but the session will not be dropped from the session cache.",
            source: "https://clouddocs.f5.com/api/irules/SSL__session.html",
            examples: "when HTTP_REQUEST {\n    if { [HTTP::uri] contains \"/maint_mode\" } {\n        ## send content and die\n        HTTP::respond 200 content $::error_html Connection Close\n        event HTTP_REQUEST disable\n        SSL::session invalidate\n    }\n}",
            return_value: "SSL::session invalidate Invalidates the current session. Specifically, this command drops the current SSL session ID from the session cache to prevent reuse of the session.",
        }),
        ..CommandSpec::DEFAULT
    }
}

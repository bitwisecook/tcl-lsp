//! `ONECONNECT::select` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "ONECONNECT::select",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Instruct the proxy to use persistence data as a OneConnect keying label when connecting to a server.",
            synopsis: &["ONECONNECT::select (persist | none)"],
            snippet: "The 'select persist' command instructs the proxy to use persistence data as the\nOneConnect keying label when connecting to the server. NTLM connection pooling\nleverages these commands internally, and it is not necessary for the user to\nuse them directly.  Persistance data should be established via the 'persist'\ncommand.",
            source: "https://clouddocs.f5.com/api/irules/ONECONNECT__select.html",
            examples: "when HTTP_REQUEST_SEND {\n     if { $keymatch == \"/myuri\"} {\n     ONECONNECT::label update $keymatch\n     }\n   }",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "ONECONNECT::select (persist | none)" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::ConnectionControl,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::Server,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}

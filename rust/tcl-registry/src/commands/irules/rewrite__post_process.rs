//! `REWRITE::post_process` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "REWRITE::post_process",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Toggle post processing functionality.",
            synopsis: &["REWRITE::post_process (SWITCH)?"],
            snippet: "When REWRITE::post_process is called (without any arguments), it\nwill return a \"0\" to signify that it is off, or an \"1\" to signify that\nit is on. By default, it is off. Use the command \"REWRITE::post_process\n1\" to turn on the post process functionality and \"REWRITE::post_process\n0\" to turn it off. When post_process is on, the\nREWRITE_RESPONSE_DONE event is triggered. Otherwise, the\nREWRITE_RESPONSE_DONE event is ignored.",
            source: "https://clouddocs.f5.com/api/irules/REWRITE__post_process.html",
            examples: "when REWRITE_REQUEST_DONE {\n  if { \"[HTTP::host][HTTP::path]\" eq \"www.external.com/contents.php\" } {\n    # Found the file we wanted to modify\n    REWRITE::post_process 1\n  }\n}",
            return_value: "",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &["REWRITE"],
            also_in: &[],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "REWRITE::post_process (SWITCH)?" },
        ],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::StreamProfile,
                reads: true,
                writes: true,
                connection_side: ConnectionSide::Both,
            },
        ],
        ..CommandSpec::DEFAULT
    }
}

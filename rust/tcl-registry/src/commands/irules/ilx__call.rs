//! `ILX::call` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "ILX::call",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
        hover: Some(HoverSnippet {
            summary: "Calls an ILX method.",
            synopsis: &["ILX::call HANDLE ?-timeout ms? ?--? METHOD ?args ...?"],
            snippet: "Make a call to a method defined within the plugin extension referenced by the handle.  Provide the method with the arguments listed in ARGS, do not continue processing the iRule until a response is received.",
            source: "https://clouddocs.f5.com/api/irules/ILX__call.html",
            examples: "when CLIENT_ACCEPTED {\n    # Get a handle to the running extension instance to call into.\n    set RPC_HANDLE [ILX::init my_plugin my_extension]\n    # Make the call and store the response in $rpc_response\n    set rpc_response [ILX::call $RPC_HANDLE my_js_function arg1 arg2]\n}",
            return_value: "The return value is the argument passed to response.reply() call on the extension side (eg. an array, a string, etc).",
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "ILX::call HANDLE ?-timeout ms? ?--? METHOD ?args ...?",
        }],
        options: &[
            OptionSpec {
                name: "-timeout",
                takes_value: true,
                value_hint: "MSEC",
                detail: "Timeout in milliseconds.",
                dialects: None,
            },
            OptionSpec {
                name: "--",
                takes_value: false,
                value_hint: "",
                detail: "",
                dialects: None,
            },
        ],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..CommandSpec::DEFAULT
    }
}

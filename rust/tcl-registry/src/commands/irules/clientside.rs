//! `clientside` iRules command.
use crate::prelude::*;
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "clientside",
        traits: Traits::IS_SIDE_SWITCH,
        dialects: Some(DialectSet::IRULES),
        // `clientside (NESTING_SCRIPT)?` — the bare query form (0 args,
        // returns 1/0) or a single optional nesting-script body (#501).
        arity: Arity::new(0, 1),
        // The optional nesting script (index 0) is a body evaluated in
        // the client-side context; it runs synchronously in the
        // caller's frame, so the default `BodyKind::Plain` applies.  The
        // 0-arg query form has no arg at index 0, so the role simply
        // does not apply there.
        arg_roles: &[(0, ArgRole::Body)],
hover: Some(HoverSnippet {
            summary: "Causes the specified iRule commands to be evaluated under the client-side context.",
            synopsis: &["clientside (NESTING_SCRIPT)?"],
            snippet: "Causes the specified iRule commands to be evaluated under the client-side context. This command has no effect if the iRule is already being evaluated under the client-side context. If there is no argument, the command returns 1 if the current event is in the clientside context or 0 if not.",
            source: "https://clouddocs.f5.com/api/irules/clientside.html",
            examples: "when SERVER_CONNECTED {\n   # Check if the client IP address is 10.1.1.80\n   # [clientside {IP::remote_addr}] is equivalent to [IP::client_addr]\n   if { [IP::addr [clientside {IP::remote_addr}] equals 10.1.1.80] } {\n      # Do something like drop the packets in this example\n      discard\n   }\n}",
            return_value: "clientside Returns 1 if the current event is in the clientside context or 0 if not.",
        }),
        ..CommandSpec::DEFAULT
    }
}

//! `serverside` iRules command.
use crate::prelude::*;
pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "serverside",
        traits: Traits::IS_SIDE_SWITCH,
        dialects: Some(DialectSet::IRULES),
        // `serverside (NESTING_SCRIPT)?` — the bare query form (0 args,
        // returns 1/0) or a single optional nesting-script body (#501).
        arity: Arity::new(0, 1),
        // The optional nesting script (index 0) is a body evaluated in
        // the server-side context; it runs synchronously in the
        // caller's frame, so the default `BodyKind::Plain` applies.  The
        // 0-arg query form has no arg at index 0, so the role simply
        // does not apply there.
        arg_roles: &[(0, ArgRole::Body)],
        hover: Some(HoverSnippet {
            summary: "Causes the specified iRule command to be evaluated under the server-side context.",
            synopsis: &["serverside (NESTING_SCRIPT)?"],
            snippet: "Causes the specified iRule command or commands to be evaluated under the server-side context. This command has no effect if the iRule is already being evaluated under the server-side context. If there is no argument, the command returns 1 if the current event is in the serverside context or 0 if not.",
            source: "https://clouddocs.f5.com/api/irules/serverside.html",
            examples: "when CLIENT_ACCEPTED {\n\n   # Check if the server (pool member) IP address is 10.1.1.80\n   # [serverside {IP::remote_addr}] is equivalent to [IP::server_addr]\n   if { [IP::addr [serverside {IP::remote_addr}] equals 10.1.1.80] } {\n\n      # Do something like drop the packets in this example\n      discard\n   }\n}",
            return_value: "serverside Returns 1 if the current event is in the serverside context or 0 if not.",
        }),
        event_requires: Some(EventRequires {
            client_side: false,
            server_side: false,
            transport: None,
            profiles: &[],
            also_in: &["CLIENT_ACCEPTED"],
            init_only: false,
            flow: false,
            capability: None,
        }),
        forms: &[FormSpec {
            kind: FormKind::Default,
            synopsis: "serverside (NESTING_SCRIPT)?",
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::ConnectionControl,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Server,
        }],
        ..CommandSpec::DEFAULT
    }
}

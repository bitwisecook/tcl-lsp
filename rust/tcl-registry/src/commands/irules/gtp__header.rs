//! `GTP::header` iRules command.
use crate::prelude::*;

/// Subcommands ported from the Python source of truth.
const SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "version",
        arity: Arity::exact(0),
        detail: "Get GTP version.",
        synopsis: "GTP::header version ?-message msg?",
        pure: true,
        options: &[OptionSpec {
            name: "-message",
            takes_value: true,
            value_hint: "MESSAGE",
            detail: "Operate on specific message.",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "type",
        arity: Arity::exact(0),
        detail: "Get GTP type.",
        synopsis: "GTP::header type ?-message msg?",
        pure: true,
        options: &[OptionSpec {
            name: "-message",
            takes_value: true,
            value_hint: "MESSAGE",
            detail: "Operate on specific message.",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::Both,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "npdu",
        arity: Arity::at_least(0),
        detail: "Get/set/remove GTP npdu.",
        synopsis: "GTP::header npdu ?set|remove? ?-message msg? ?value?",
        pure: true,
        mutator: true,
        options: &[OptionSpec {
            name: "-message",
            takes_value: true,
            value_hint: "MESSAGE",
            detail: "Operate on specific message.",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "teid",
        arity: Arity::at_least(0),
        detail: "Get/set/remove GTP teid.",
        synopsis: "GTP::header teid ?set|remove? ?-message msg? ?value?",
        pure: true,
        mutator: true,
        options: &[OptionSpec {
            name: "-message",
            takes_value: true,
            value_hint: "MESSAGE",
            detail: "Operate on specific message.",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "sequence",
        arity: Arity::at_least(0),
        detail: "Get/set/remove GTP sequence.",
        synopsis: "GTP::header sequence ?set|remove? ?-message msg? ?value?",
        pure: true,
        mutator: true,
        options: &[OptionSpec {
            name: "-message",
            takes_value: true,
            value_hint: "MESSAGE",
            detail: "Operate on specific message.",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "extension",
        arity: Arity::at_least(0),
        detail: "Access GTP extension headers.",
        synopsis: "GTP::header extension ?args?",
        pure: true,
        mutator: true,
        options: &[OptionSpec {
            name: "-message",
            takes_value: true,
            value_hint: "MESSAGE",
            detail: "Operate on specific message.",
            dialects: None,
        }],
        side_effects: &[SideEffect {
            target: SideEffectTarget::NetworkIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::Both,
        }],
        ..SubCommand::DEFAULT
    },
];

pub const fn spec() -> CommandSpec {
    CommandSpec {
        name: "GTP::header",
        dialects: Some(DialectSet::IRULES),
        arity: Arity::at_least(0),
hover: Some(HoverSnippet {
            summary: "Allows for the parsing of GTP header information.",
            synopsis: &["GTP::header ('version' | 'type') ('-message' MESSAGE)?", "GTP::header ('teid' | 'npdu' | 'sequence') ('-message' MESSAGE)?", "GTP::header ('teid' | 'npdu' | 'sequence') 'set' ('-message' MESSAGE)? VALUE", "GTP::header ('teid' | 'npdu' | 'sequence') 'remove' ('-message' MESSAGE)?"],
            snippet: "Allows for the parsing of GTP header information. UINT -- Unsigned\ninteger value of n bits. For n > 8, appropriate network to host byte\norder conversion happens transparently.",
            source: "https://clouddocs.f5.com/api/irules/GTP__header.html",
            examples: "when GTP_SIGNALLING_INGRESS {\n    log local0. \"GTP version [GTP::header version]\"\n    log local0. \"GTP type [GTP::header type]\"\n}",
            return_value: "",
        }),
        forms: &[
            FormSpec { kind: FormKind::Default, synopsis: "GTP::header <field> ?set|remove? ?-message msg? ?value?" },
        ],
        options: &[
            OptionSpec { name: "-message", takes_value: true, value_hint: "MESSAGE", detail: "Operate on specific message.", dialects: None },
        ],
        subcommands: SUBCOMMANDS,
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::NetworkIo,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::Both,
            },
        ],
        taint_source: Some(TaintColour::TAINTED),
        ..CommandSpec::DEFAULT
    }
}

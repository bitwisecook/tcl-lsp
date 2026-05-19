//! `chan` — channel operations.
use crate::prelude::*;

/// Shared option table for `chan configure` — same shape as
/// `fconfigure` options.  The Tcl 9.0+ entries (`-nodelay`,
/// `-keepalive`, `-inputmode`) carry their own dialect gate so
/// W004 fires when they're used on pre-9.0 dialects.
static CONFIGURE_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-blocking",
        takes_value: true,
        value_hint: "boolean",
        detail: "",
        dialects: None,
    },
    OptionSpec {
        name: "-buffering",
        takes_value: true,
        value_hint: "mode",
        detail: "",
        dialects: None,
    },
    OptionSpec {
        name: "-buffersize",
        takes_value: true,
        value_hint: "size",
        detail: "",
        dialects: None,
    },
    OptionSpec {
        name: "-encoding",
        takes_value: true,
        value_hint: "encoding",
        detail: "",
        dialects: None,
    },
    OptionSpec {
        name: "-eofchar",
        takes_value: true,
        value_hint: "chars",
        detail: "",
        dialects: None,
    },
    OptionSpec {
        name: "-translation",
        takes_value: true,
        value_hint: "mode",
        detail: "",
        dialects: None,
    },
    // Tcl 9.0+ socket / terminal options (TIPs 528 / 160).
    OptionSpec {
        name: "-nodelay",
        takes_value: true,
        value_hint: "boolean",
        detail: "Disable Nagle's algorithm on TCP sockets (Tcl 9.0+).",
        dialects: Some(DialectSet::TCL90),
    },
    OptionSpec {
        name: "-keepalive",
        takes_value: true,
        value_hint: "boolean",
        detail: "Enable TCP keepalive on sockets (Tcl 9.0+).",
        dialects: Some(DialectSet::TCL90),
    },
    OptionSpec {
        name: "-inputmode",
        takes_value: true,
        value_hint: "mode",
        detail: "Terminal input mode: normal/password/raw (Tcl 9.0+).",
        dialects: Some(DialectSet::TCL90),
    },
];

static SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "blocked",
        arity: Arity::exact(1),
        detail: "Test whether last input exhausted all data.",
        synopsis: "chan blocked channelId",
        pure: true,
        return_type: Some(TclType::Boolean),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "close",
        arity: Arity::new(1, 2),
        detail: "Close a channel.",
        synopsis: "chan close channelId ?direction?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "configure",
        arity: Arity::at_least(1),
        detail: "Get or set channel configuration.",
        synopsis: "chan configure channelId ?-option value ...?",
        options: CONFIGURE_OPTIONS,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "copy",
        arity: Arity::at_least(2),
        detail: "Copy data between channels.",
        synopsis: "chan copy inputChan outputChan ?-option value ...?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "create",
        arity: Arity::exact(2),
        detail: "Create a channel from a Tcl command.",
        synopsis: "chan create mode cmdPrefix",
        return_type: Some(TclType::Channel),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "eof",
        arity: Arity::exact(1),
        detail: "Test for end of file.",
        synopsis: "chan eof channelId",
        pure: true,
        return_type: Some(TclType::Boolean),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "event",
        arity: Arity::new(2, 3),
        detail: "Set up channel event handler.",
        synopsis: "chan event channelId event ?script?",
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "flush",
        arity: Arity::exact(1),
        detail: "Flush buffered output.",
        synopsis: "chan flush channelId",
        mutator: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "gets",
        traits: Traits::TAINT_SOURCE,
        arity: Arity::new(1, 2),
        detail: "Read a line.",
        synopsis: "chan gets channelId ?varName?",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "names",
        arity: Arity::new(0, 1),
        detail: "List open channels.",
        synopsis: "chan names ?pattern?",
        pure: true,
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "pending",
        arity: Arity::exact(2),
        detail: "Return pending bytes.",
        synopsis: "chan pending mode channelId",
        pure: true,
        return_type: Some(TclType::Int),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "pipe",
        arity: Arity::any(),
        detail: "Create a connected pair of channels.",
        synopsis: "chan pipe",
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "pop",
        arity: Arity::exact(1),
        detail: "Remove the top channel transformation.",
        synopsis: "chan pop channelId",
        mutator: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "push",
        arity: Arity::exact(2),
        detail: "Add a channel transformation.",
        synopsis: "chan push channelId cmdPrefix",
        mutator: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "puts",
        arity: Arity::at_least(1),
        detail: "Write to a channel.",
        synopsis: "chan puts ?-nonewline? ?channelId? string",
        mutator: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "read",
        traits: Traits::TAINT_SOURCE,
        arity: Arity::new(1, 2),
        detail: "Read from a channel.",
        synopsis: "chan read channelId ?numChars?",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "seek",
        arity: Arity::new(2, 3),
        detail: "Set access position.",
        synopsis: "chan seek channelId offset ?origin?",
        mutator: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "tell",
        arity: Arity::exact(1),
        detail: "Return current position.",
        synopsis: "chan tell channelId",
        pure: true,
        return_type: Some(TclType::Int),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "truncate",
        arity: Arity::new(1, 2),
        detail: "Truncate a channel.",
        synopsis: "chan truncate channelId ?length?",
        ..SubCommand::DEFAULT
    },
];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "chan",
        traits: Traits::CONFIGURES_CHANNEL | Traits::HAS_DESTRUCTIVE_OPS,
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::at_least(1),
        subcommands: SUBCOMMANDS,
        hover: Some(HoverSnippet::brief(
            "Channel operations.",
            &["chan subcommand ?arg ...?"],
            "Tcl chan(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}

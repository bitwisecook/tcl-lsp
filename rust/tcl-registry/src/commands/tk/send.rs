// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! `send` command.
use crate::prelude::*;
use tcl_dialect::model::{SpecSurface};

const SIDE_EFFECTS: &[SideEffect] = &[
    SideEffect {
        // The target interpreter is external to the caller, but it is still
        // a Tcl interpreter whose command may change state.
        target: SideEffectTarget::InterpState,
        writes: true,
        ..SideEffect::DEFAULT
    },
    SideEffect {
        // Tk transports `send` through its inter-application transport
        // (typically the X server); model the externally observable I/O as
        // well as the remote interpreter effect.
        target: SideEffectTarget::NetworkIo,
        writes: true,
        ..SideEffect::DEFAULT
    },
];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-async",
        value: OptionValue::flag(),
        detail: "Invoke the command asynchronously and return without waiting for its result.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-displayof",
        value: OptionValue::value("window"),
        detail: "Select the display containing the target application's main window.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "--",
        value: OptionValue::flag(),
        detail: "End option processing before the target application name.",
        ..OptionSpec::DEFAULT
    },
];

/// `send ?options? app cmd ?arg ...?`: options precede the application name,
/// and the command plus trailing words form a remotely evaluated script.  The
/// resolver marks the application name and, only where it is an honest whole
/// Tcl script, the remote body. `send app script` can expose its sole script
/// word as a body. In `send app word ?word ...?`, Tcl concatenates every
/// trailing word before remote evaluation; marking only the first would parse
/// a truncated, fictional script. The generic registry has no concatenated
/// multi-word body representation, so it intentionally abstains there (the
/// same rule as `after` and `uplevel`). `--` terminates option processing but
/// is not part of the remote command.
fn send_command_start(args: &[&str]) -> (usize, bool) {
    let mut index = 0usize;
    let mut asynchronous = false;
    while index < args.len() {
        match args[index] {
            "-async" => {
                asynchronous = true;
                index += 1;
            }
            "-displayof" if index + 1 < args.len() => index += 2,
            "--" => {
                index += 1;
                break;
            }
            _ => break,
        }
    }
    (index, asynchronous)
}

pub(crate) fn send_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    let (index, _) = send_command_start(args);
    match args.len().saturating_sub(index) {
        0 => Vec::new(),
        1 => u8::try_from(index)
            .ok()
            .map(|index| vec![(index, ArgRole::Name)])
            .unwrap_or_default(),
        2 => match (u8::try_from(index), u8::try_from(index + 1)) {
            (Ok(app), Ok(body)) => vec![(app, ArgRole::Name), (body, ArgRole::Body)],
            _ => Vec::new(),
        },
        _ => u8::try_from(index)
            .ok()
            .map(|index| vec![(index, ArgRole::Name)])
            .unwrap_or_default(),
    }
}

fn send_script_timing(args: &[&str]) -> Vec<(u8, ScriptTiming)> {
    let (index, asynchronous) = send_command_start(args);
    if args.len().saturating_sub(index) != 2 {
        return Vec::new();
    }
    u8::try_from(index + 1)
        .ok()
        .map(|body| {
            vec![(
                body,
                if asynchronous {
                    ScriptTiming::Deferred
                } else {
                    ScriptTiming::SameInvocation
                },
            )]
        })
        .unwrap_or_default()
}

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "send ?options? app cmd ?arg ...?",
    ..FormSpec::DEFAULT
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "send",
        surface: Some(SpecSurface::TK_AND_TCL),
        arity: Arity::at_least(2),
        arg_role_resolver: Some(send_arg_roles),
        script_timing_resolver: Some(send_script_timing),
        traits: Traits::EVALUATES_CODE
            | Traits::SCRIPT_CONCATENATES_ARGS
            | Traits::HAS_INTERP_EVAL
            | Traits::TAINT_SINK
            | Traits::CREATES_DYNAMIC_BARRIER
            | Traits::DYNAMIC_EVAL_BODY,
        body_kind: BodyKind::Structural,
        return_type: Some(TclType::String),
        hover: Some(HoverSnippet {
            summary: "Execute a command in a different Tk application.",
            synopsis: &["send ?options? app cmd ?arg ...?"],
            snippet: "The command words are concatenated and evaluated in the named Tk application's interpreter, not in the sender's frame. Without `-async`, the remote result or error is returned synchronously. With `-async`, send returns immediately with an empty result and remote errors are not reported to the sender. Because remotely evaluated text is code, untrusted command words are a code-injection sink; construct them with `list` rather than string concatenation.",
            source: "Tk man page send.n",
            examples: "",
            return_value: "Without -async, the remote command's result or error. With -async, an empty result after queueing the request.",
        }),
        required_package: Some("Tk"),
        warn_missing_import: false,
        forms: FORMS,
        options: OPTIONS,
        side_effects: SIDE_EFFECTS,
        taint_sink_safe_colour: Some(TaintColour::LIST_CANONICAL),
        ..CommandSpec::DEFAULT
    }
}

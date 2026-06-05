//! `tcltest::test` command.
use crate::prelude::*;
const SIDE_EFFECTS: &[SideEffect] = &[SideEffect {
    target: SideEffectTarget::InterpState,
    reads: true,
    writes: true,
    connection_side: ConnectionSide::None,
}];

const OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-body",
        takes_value: true,
        value_hint: "script",
        detail: "",
        dialects: None,
    },
    OptionSpec {
        name: "-result",
        takes_value: true,
        value_hint: "",
        detail: "",
        dialects: None,
    },
    OptionSpec {
        name: "-output",
        takes_value: true,
        value_hint: "",
        detail: "",
        dialects: None,
    },
    OptionSpec {
        name: "-errorOutput",
        takes_value: true,
        value_hint: "",
        detail: "",
        dialects: None,
    },
    OptionSpec {
        name: "-returnCodes",
        takes_value: true,
        value_hint: "",
        detail: "",
        dialects: None,
    },
    OptionSpec {
        name: "-errorCode",
        takes_value: true,
        value_hint: "",
        detail: "",
        dialects: None,
    },
    OptionSpec {
        name: "-match",
        takes_value: true,
        value_hint: "",
        detail: "",
        dialects: None,
    },
    OptionSpec {
        name: "-setup",
        takes_value: true,
        value_hint: "script",
        detail: "",
        dialects: None,
    },
    OptionSpec {
        name: "-cleanup",
        takes_value: true,
        value_hint: "script",
        detail: "",
        dialects: None,
    },
    OptionSpec {
        name: "-constraints",
        takes_value: true,
        value_hint: "",
        detail: "",
        dialects: None,
    },
];

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "test name description ?option value ...?",
}];

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "tcltest::test",
        dialects: Some(DialectSet::ALL_TCL),
        arity: Arity::at_least(2),
hover: Some(HoverSnippet {
            summary: "Define and run a single test case.",
            synopsis: &["tcltest::test name description ?option value ...?", "tcltest::test name description ?constraints? body result"],
            snippet: "The primary command for defining tests.  Options include ``-body``, ``-result``, ``-output``, ``-errorOutput``, ``-returnCodes``, ``-match``, ``-setup``, ``-cleanup``, and ``-constraints``.",
            source: "Tcl stdlib tcltest package",
            examples: "",
            return_value: "",
        }),
        required_package: Some("tcltest"),
        forms: FORMS,
        options: OPTIONS,
        side_effects: SIDE_EFFECTS,
        ..CommandSpec::DEFAULT
    }
}

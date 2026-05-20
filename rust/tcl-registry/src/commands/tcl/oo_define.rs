//! `oo::define` — define class members.
use crate::prelude::*;

/// Subcommands recognised by ``oo::define`` / ``oo::objdefine``.
/// Used to disambiguate the script-form (`oo::define Target {body}`)
/// from a subcommand call where `args[1]` is one of these words.
const OO_DEFINE_SUBCOMMANDS: &[&str] = &[
    "constructor",
    "destructor",
    "method",
    "classmethod",
    "initialise",
    "initialize",
    "private",
    "self",
    "property",
    "filter",
    "export",
    "unexport",
    "deletemethod",
    "renamemethod",
    "forward",
    "mixin",
    "superclass",
    "variable",
];

/// Resolve body argument indices for `oo::define` / `oo::objdefine`.
///
/// * `oo::define Target body` (script form, when `args[1]` is not a
///   recognised subcommand) → body at index 1.
/// * `oo::define Target constructor args body` → body at index 3.
/// * `oo::define Target destructor body` → body at index 2.
/// * `oo::define Target method name args body` → body at last index.
/// * `oo::define Target initialise body` / `initialize body` /
///   `private body` → body at index 2.
/// * `oo::define Target self constructor args body` → body at index 4.
/// * `oo::define Target self destructor body` → body at index 3.
/// * `oo::define Target self method name args body` → body at last
///   index.
/// * `oo::define Target property -set BODY ?-get BODY?` →
///   bodies after each `-set` / `-get` flag.
//
// `match_same_arms`: keeping each subcommand on its own arm reads
// as a lookup table — collapsing the `initialise` / `initialize` /
// `private` / `destructor` arms loses that and makes adding new
// subcommands harder to spot.
#[allow(clippy::match_same_arms)]
pub(crate) fn oo_define_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    let n = args.len();
    if n == 2 && !OO_DEFINE_SUBCOMMANDS.contains(&args[1]) {
        return vec![(1, ArgRole::Body)];
    }
    if n < 2 {
        return Vec::new();
    }
    let Ok(last) = u8::try_from(n - 1) else {
        return Vec::new();
    };
    match args[1] {
        "constructor" if n >= 4 => vec![(3, ArgRole::Body)],
        "destructor" if n >= 3 => vec![(2, ArgRole::Body)],
        "method" | "classmethod" if n >= 5 => vec![(last, ArgRole::Body)],
        "initialise" | "initialize" if n >= 3 => vec![(2, ArgRole::Body)],
        "private" if n >= 3 => vec![(2, ArgRole::Body)],
        "self" if n >= 3 => match args[2] {
            "constructor" if n >= 5 => vec![(4, ArgRole::Body)],
            "destructor" if n >= 4 => vec![(3, ArgRole::Body)],
            "method" | "classmethod" if n >= 6 => vec![(last, ArgRole::Body)],
            _ => Vec::new(),
        },
        "property" => collect_property_body_roles(args, 2),
        _ => Vec::new(),
    }
}

/// `oo::define Target property name ?-set BODY? ?-get BODY?` →
/// flag-keyed bodies. `start` is the index of the first option flag
/// (2 for `oo::define Target property`, 0 for inner `property` —
/// which folding handles separately).
pub(crate) fn collect_property_body_roles(args: &[&str], start: usize) -> Vec<(u8, ArgRole)> {
    let n = args.len();
    if n == 0 {
        return Vec::new();
    }
    args.iter()
        .enumerate()
        .skip(start)
        .take(n.saturating_sub(start + 1))
        .filter_map(|(i, &a)| {
            if (a == "-set" || a == "-get") && i + 1 < n {
                u8::try_from(i + 1).ok().map(|idx| (idx, ArgRole::Body))
            } else {
                None
            }
        })
        .collect()
}

pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "oo::define",
        traits: Traits::NOT_PROC_FACTORY | Traits::LANGUAGE_KEYWORD | Traits::NEVER_INLINE_BODY,
        dialects: Some(DialectSet::TCL86_PLUS),
        arity: Arity::at_least(2),
        arg_roles: &[(0, ArgRole::Name)],
        arg_role_resolver: Some(oo_define_arg_roles),
        return_type: Some(TclType::String),
        // SYNC2: every body argument that `oo_define_arg_roles`
        // surfaces is a TclOO definition / dispatch body, never a
        // caller-frame body.  Stamping `Structural` here covers all
        // the script-bearing forms (constructor / destructor /
        // method / classmethod / initialise / initialize / private /
        // self.* / property -set / -get) plus the bare-script form
        // `oo::define Cls {body}`.
        body_kind: BodyKind::Structural,
        hover: Some(HoverSnippet::brief(
            "Define class members.",
            &[
                "oo::define className ?definition?",
                "oo::define className subcommand ?arg ...?",
            ],
            "Tcl oo::define(1)",
        )),
        ..CommandSpec::DEFAULT
    }
}

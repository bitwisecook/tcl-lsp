// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! tcllib data-structure / archive packages — `tar`, `pki`, `map::slippy`,
//! and the `struct::*` container *creator* commands.
//!
//! The `struct::graph` / `struct::tree` / `struct::matrix` / … commands
//! construct an instance command whose methods live on the returned object.
//! Most creators are modelled statically here for recognition, package
//! gating, and hover alone; `struct::graph` and `struct::tree` additionally
//! bind an [`ObjectClassSpec`] so their walker methods' callbacks resolve.
//! Command names, arity bounds, synopses, and summaries are derived from the
//! upstream tcllib 2.0 manual pages.  Requires Tcl 8.5+.
//!
//! ## P5 — `struct::tree` across its two trains
//!
//! `struct::tree` is the redesign's flagship adversarial module: tcllib
//! 2.0's `struct/pkgIndex.tcl` offers **1.2.3** (`tree1.tcl`) and **2.1.3**
//! (`tree_tcl.tcl`) side by side, and their walker interfaces are
//! incompatible.  Both shapes are modelled on the one class, each with the
//! lifecycle that says which train it belongs to:
//!
//! - **2.x `walk node ?-order o? ?-type t? ?--? loopvar script`** — a
//!   loop-variable list and a *script* `uplevel 2`-ed in the caller's frame.
//!   The script's index depends on how many option pairs precede it, so it
//!   is resolved rather than fixed.
//! - **1.x `walk node ?…? -command cmd`** — `tree1.tcl`'s `WalkCall` does
//!   `string map {%n … %a … %t … %% %} $cmd` and then `uplevel 2`, so
//!   **nothing is appended** and the placeholders carry the payload.  The
//!   option is `retired: "2.0"`, per `struct_tree.man`'s "Changes for 2.0".
//! - **2.x `walkproc node ?…? cmdprefix`** — a genuine command prefix with
//!   three appended words, `introduced: "2.0"`.
//!
//! `struct::graph`, by contrast, has **no** cross-train walker delta:
//! `graph1.tcl:1875` and `graph_tcl.tcl:2675` carry the identical
//! `?-dir? ?-order? ?-type? -command cmd` usage and the identical
//! `lappend cmdcpy <action> $name $node; uplevel` call, so one descriptor
//! serves both.
//!
//! **What this module cannot say** (the P5 limits, each with its field):
//!
//! - *Scoped completion codes.*  `::struct::tree::prune` is `return -code 5`,
//!   meaningful only inside a `walk` body.  The producer half is expressible
//!   ([`CompletionCode::Other`]); the consumer half is not.  It needs
//!   `body_completion_codes: &[(u8, CompletionCode, &str)]` on the body
//!   slot — deep-dive ruling candidate E-R6.
//! - *Callback substitution sets.*  Nothing can declare that 1.x
//!   `-command` is `string map`-substituted with `%n`/`%a`/`%t`/`%%`; a
//!   `-substitutions {%n node …}` qualifier on the prefix slot would.
//!   [`CallbackTaintInput::TkPercent`] names such a spelling, but only as a
//!   taint colour.
//! - *Body-scoped legality.*  `prune` is an error under
//!   `-order post`/`-order in` — a relation between an option's value and a
//!   command used *inside the walk body*, which is the body-scoped half of the
//!   same E-R6 gap as `prune`'s completion code, not an option relation.
//!   (`-order in` with `-type bfs`, the *other* half of what used to be
//!   recorded here, **is** expressible under E-R14 and is declared below.)
//! - *Instance-method version gating.*  The lifecycles on `walkproc` and on
//!   `walk -command` are declared but unread: the analyser has no diagnostic
//!   site on the instance-method dispatch path.  That is a missing consumer,
//!   not a missing field.

use crate::prelude::*;

/// A row in a flat package command table: name, arity, synopsis, and summary.
type Row = (&'static str, Arity, &'static [&'static str], &'static str);

/// Build command specs for a flat package from its table.
fn rows(pkg: &'static str, table: &'static [Row]) -> Vec<CommandSpec> {
    table
        .iter()
        .map(|&(name, arity, synopsis, summary)| CommandSpec {
            name,
            arity,
            hover: Some(HoverSnippet::brief(summary, synopsis, "tcllib package")),
            tcllib_package: Some(pkg),
            required_package: Some(pkg),
            ..CommandSpec::DEFAULT
        })
        .collect()
}

/// A `struct::*` container creator command.  The package name is the command
/// name (matching the `struct::list` / `struct::queue` convention).
fn creator(
    name: &'static str,
    synopsis: &'static [&'static str],
    summary: &'static str,
) -> CommandSpec {
    CommandSpec {
        name,
        arity: Arity::any(),
        hover: Some(HoverSnippet::brief(
            summary,
            synopsis,
            "tcllib struct package",
        )),
        tcllib_package: Some(name),
        required_package: Some(name),
        ..CommandSpec::DEFAULT
    }
}

/// Like [`creator`] but binds the created object command to an
/// [`ObjectClassSpec`] whose instance methods carry command-prefix callbacks.
/// `struct::graph name` / `struct::tree name` name the new object command
/// positionally (index 0 — the `?name?`), so `creates_instance_at` types it and
/// a later `name walk … -command cb` resolves the callback through the class.
fn object_creator(
    name: &'static str,
    synopsis: &'static [&'static str],
    summary: &'static str,
    class: &'static ObjectClassSpec,
) -> CommandSpec {
    CommandSpec {
        creates_instance_at: Some(0),
        object_class: Some(class),
        ..creator(name, synopsis, summary)
    }
}

/// `struct::graph` instance `walk node … -command cmd` (`graph_tcl.tcl` `_walk`,
/// 2675).  At each visited node it runs `lappend cmdcpy <enter|leave> $name
/// $node; uplevel 1 $cmdcpy`, appending three words — the action keyword, the
/// graph name, and the node — so `-command` is a prefix of `Exactly(3)`.
const STRUCT_GRAPH_WALK_OPTIONS: &[OptionSpec] = &[
    OptionSpec {
        name: "-dir",
        value: OptionValue::value("forward|backward"),
        detail: "Walk direction: forward (default) or backward.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-order",
        value: OptionValue::value("pre|post|both"),
        detail: "Visit order: pre (default), post, or both.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-type",
        value: OptionValue::value("bfs|dfs"),
        detail: "Traversal type: dfs (default) or bfs.",
        ..OptionSpec::DEFAULT
    },
    OptionSpec {
        name: "-command",
        value: OptionValue::command_prefix_n("prefix", AppendedArity::Exactly(3)),
        detail: "Command prefix invoked at each node with (action graphName node) appended.",
        ..OptionSpec::DEFAULT
    },
];

const STRUCT_GRAPH_METHODS: &[SubCommand] = &[SubCommand {
    name: "walk",
    arity: Arity::at_least(1),
    detail: "Walk the graph from node, invoking -command at each visited node.",
    synopsis: "graphName walk node ?-dir forward|backward? ?-order pre|post|both? ?-type bfs|dfs? -command cmd",
    options: STRUCT_GRAPH_WALK_OPTIONS,
    ..SubCommand::DEFAULT
}];

static STRUCT_GRAPH_CLASS: ObjectClassSpec = ObjectClassSpec {
    class_name: "struct::graph",
    instance_methods: STRUCT_GRAPH_METHODS,
    superclasses: &[],
    // Only `walk` is modelled for its callback; every other method
    // (`node insert`, `arc …`, `get`, …) passes through unflagged.
    allow_unknown_methods: true,
    method_prefix_matching: PrefixMatching::Strict,
};

/// `struct::tree` instance `walkproc node … cmdprefix` (`tree_tcl.tcl` `_walkproc`,
/// 1856).  The trailing `cmdprefix` is a positional command prefix invoked as
/// `lappend cmd $tree $node $action; uplevel 2 $cmd` — three appended words
/// (tree, node, action) ⇒ `Exactly(3)`.
fn struct_tree_walkproc_command_prefixes(
    args: CommandPrefixArguments<'_>,
) -> Vec<(u8, AppendedArity)> {
    // args are the words after `walkproc`: `node ?-order o? ?-type t? ?--?
    // cmdprefix`.  The prefix is always the final word (node is required, so
    // len ≥ 2 for a real prefix).
    match u8::try_from(args.len()) {
        Ok(n) if n >= 2 => vec![(n - 1, AppendedArity::Exactly(3))],
        _ => Vec::new(),
    }
}

/// Traversal orders both trains accept (`struct_tree.man`; `WalkOptions`
/// in `tree_tcl.tcl`).  `in` is illegal with `-type bfs` — declared as
/// [`STRUCT_TREE_WALK_RELATIONS`] under E-R14 — and `post`/`in` are the two
/// orders `::struct::tree::prune` may not be used with, which is body-scoped
/// and still has no field (see the module note).
const STRUCT_TREE_ORDER_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "pre",
        detail: "visit a parent before its children (default)",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "post",
        detail: "visit a parent after its children",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "in",
        detail: "visit a parent between its first and second child (dfs only)",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "both",
        detail: "visit a parent before and after its children",
        ..ArgValue::DEFAULT
    },
];

/// `bfs`/`dfs`, the two traversal types.
const STRUCT_TREE_TYPE_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "dfs",
        detail: "depth-first (default)",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "bfs",
        detail: "breadth-first",
        ..ArgValue::DEFAULT
    },
];

/// The `walk` method's options across **both** `struct::tree` trains.
///
/// `-order` and `-type` exist in both.  `-command` is the 1.x walker
/// interface and was **removed at 2.0**: `struct_tree.man`'s "Changes for
/// 2.0" says the walker API was "streamlined and made more similar to the
/// command `foreach`" — "The superfluous option `-command` has been
/// removed.  Ditto for the place holders."  So it carries
/// `retired: "2.0"` on the `struct::tree` axis, which is what makes a
/// document declaring a 2.x floor (or a range reaching 2.x) get told the
/// option is gone.
const STRUCT_TREE_WALK_OPTIONS: &[OptionSpec] = &[
    STRUCT_TREE_ORDER_OPTION,
    STRUCT_TREE_TYPE_OPTION,
    OptionSpec {
        name: "-command",
        // `tree1.tcl` `WalkCall` is `string map {%n … %a … %t … %% %} $cmd`
        // followed by `uplevel 2` — the substituted text is *evaluated*, and
        // **nothing is appended**, so the appended arity is `Exactly(0)`
        // rather than `walkproc`'s `Exactly(3)`.  The placeholder set itself
        // has no field; see the module note on what would express it.
        value: OptionValue::command_prefix_n("cmd", AppendedArity::Exactly(0)),
        detail: "1.x only: command with %n (node), %a (action), %t (tree) substituted before evaluation. Removed in struct::tree 2.0.",
        lifecycle: Lifecycle {
            retired: Some("2.0"),
            ..Lifecycle::UNSPECIFIED
        },
        ..OptionSpec::DEFAULT
    },
];

/// The cross-option **value** legality rule both walkers share.
///
/// `tree_tcl.tcl`'s `WalkOptions` closes with
/// `if {[string equal $order "in"] && [string equal $type "bfs"]} { return
/// -code error "unable to do a ${order}-order breadth first walk" }` — a
/// relation between one option's *value* and another option's *value*, which
/// is exactly [`OptionTerm::OptionValue`]'s reason to exist.  It is written
/// directionally (`-order in` forbids `-type bfs`) because that is how the
/// library phrases the failure, and because `-order in` is the word the
/// author has to change.
const STRUCT_TREE_WALK_RELATIONS: &[OptionRelation] = &[OptionRelation {
    kind: RelationKind::Forbids,
    subject: Some(OptionTerm::OptionValue("-order", "in")),
    terms: &[OptionTerm::OptionValue("-type", "bfs")],
    message: Some("unable to do a in-order breadth first walk"),
    ..OptionRelation::DEFAULT
}];

/// `walkproc`'s options: `WalkOptions` (`tree_tcl.tcl`) accepts only
/// `-order`, `-type` and `--`.  `-command` is deliberately absent — it is
/// the 1.x `walk` interface, and `walkproc` postdates its removal.
const STRUCT_TREE_WALKPROC_OPTIONS: &[OptionSpec] =
    &[STRUCT_TREE_ORDER_OPTION, STRUCT_TREE_TYPE_OPTION];

/// The shared `-order` descriptor (both methods, both trains).
const STRUCT_TREE_ORDER_OPTION: OptionSpec = OptionSpec {
    name: "-order",
    value: OptionValue::enumerated(STRUCT_TREE_ORDER_VALUES, true, "pre|post|in|both"),
    detail: "Visit order: pre (default), post, in, or both.",
    ..OptionSpec::DEFAULT
};

/// The shared `-type` descriptor (both methods, both trains).
const STRUCT_TREE_TYPE_OPTION: OptionSpec = OptionSpec {
    name: "-type",
    value: OptionValue::enumerated(STRUCT_TREE_TYPE_VALUES, true, "bfs|dfs"),
    detail: "Traversal type: dfs (default) or bfs.",
    ..OptionSpec::DEFAULT
};

/// `walk`'s script argument is the **last** word, and the option prefix in
/// front of it is variable-length, so the body position cannot be a fixed
/// index.
///
/// 2.x shape: `node ?-order o? ?-type t? ?--? loopvar script` — the final
/// word is a script evaluated with `uplevel 2` (in the caller's frame), and
/// the word before it is the loop-variable list, written not read.
/// 1.x shape: `node ?-type t? ?-order o? -command cmd` — the callback
/// arrives through the option, so there is no trailing body at all and this
/// resolver correctly declines.
fn struct_tree_walk_arg_roles(args: &[&str]) -> Vec<(u8, ArgRole)> {
    // Anything after the `node` word that begins an option pair is consumed
    // two words at a time; the remainder is `loopvar script`.
    let mut index = 1usize;
    while index + 1 < args.len() && matches!(args[index], "-order" | "-type") {
        index += 2;
    }
    if index < args.len() && args[index] == "--" {
        index += 1;
    }
    // The 1.x `-command` form leaves nothing trailing.
    if args.get(index).is_some_and(|word| word.starts_with('-')) {
        return Vec::new();
    }
    let (Ok(loopvar), Ok(script)) = (u8::try_from(index), u8::try_from(index + 1)) else {
        return Vec::new();
    };
    if usize::from(script) + 1 != args.len() {
        return Vec::new();
    }
    vec![(loopvar, ArgRole::VarWrite), (script, ArgRole::Body)]
}

const STRUCT_TREE_METHODS: &[SubCommand] = &[
    SubCommand {
        name: "walk",
        arity: Arity::at_least(2),
        detail: "Walk the tree from node, evaluating a script (2.x) or a %-substituted command (1.x) at each visited node.",
        synopsis: "treeName walk node ?-order pre|post|in|both? ?-type bfs|dfs? ?--? loopvar script",
        options: STRUCT_TREE_WALK_OPTIONS,
        option_relations: STRUCT_TREE_WALK_RELATIONS,
        // `_walk {name node args}` takes the node **positionally** and then
        // reads its option run out of `$args`, so the options are not leading
        // in the invocation as written (`$t walk root -order in …`).
        option_placement: OptionPlacement::Anywhere,
        arg_role_resolver: Some(struct_tree_walk_arg_roles),
        arg_role_resolver_roles: &[ArgRole::VarWrite, ArgRole::Body],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "walkproc",
        arity: Arity::at_least(2),
        detail: "Walk the tree from node, invoking the trailing command prefix at each node.",
        synopsis: "treeName walkproc node ?-order pre|post|in|both? ?-type bfs|dfs? cmdprefix",
        options: STRUCT_TREE_WALKPROC_OPTIONS,
        option_relations: STRUCT_TREE_WALK_RELATIONS,
        option_placement: OptionPlacement::Anywhere,
        command_prefix_resolver: Some(struct_tree_walkproc_command_prefixes),
        // `walkproc` arrived with the 2.0 walker rework: `tree1.tcl` (the
        // 1.2.3 train) defines `_walk` and no `_walkproc` at all, while
        // `struct_tree.man` documents it beside `walk`.
        lifecycle: Lifecycle {
            introduced: Some("2.0"),
            ..Lifecycle::UNSPECIFIED
        },
        ..SubCommand::DEFAULT
    },
];

static STRUCT_TREE_CLASS: ObjectClassSpec = ObjectClassSpec {
    class_name: "struct::tree",
    instance_methods: STRUCT_TREE_METHODS,
    superclasses: &[],
    allow_unknown_methods: true,
    method_prefix_matching: PrefixMatching::Strict,
};

/// The completion `::struct::tree::prune` produces: **code 5**, and only
/// code 5.
///
/// `tree_tcl.tcl:181` is the whole implementation —
/// `proc ::struct::tree::prune_tcl {} { return -code 5 }` — and
/// `WalkCall`/`WalkCallProc` are the only consumers, switching on `5` to
/// turn it into a `continue` for the walker's own loop (and into an error
/// when the order is `post` or `in`, which visit children first).
///
/// The **producer** half is expressible today: a library-defined
/// completion code is exactly [`CompletionCode::Other`].  The
/// **consumer** half is the deep dive's E-R6 ruling candidate and stays a
/// recorded limit — see the `prune` spec's own note.
const STRUCT_TREE_PRUNE_CODES: &[CompletionCode] = &[CompletionCode::Other(5)];

/// `::struct::tree::prune` — the scoped completion code, modelled as far
/// as the model goes.
///
/// **What is modelled.** The command exists only in the 2.x train
/// (`tree1.tcl`, the 1.2.3 train, has no `prune` proc; `tree_tcl.tcl` and
/// `struct_tree.man` both carry it, and the tcllib `struct` `ChangeLog`
/// dates it to 2004-08-14 "Added a prune operation to the tree walk
/// command", well after the 2003-07-14 bump of the module to 2.0), so it
/// carries `introduced: "2.0"` on the **`struct::tree`** axis — its own
/// package's axis, never the Tcl core's (invariant I2).  Its completion
/// domain is the exact singleton `{5}`.
///
/// **What is not, and the exact field it needs.**  `prune` is only
/// meaningful *inside a `treeName walk` body*: there, code 5 means "skip
/// this node's children and go on", a loop-adjacent completion the walker
/// consumes.  Anywhere else, `return -code 5` propagates as an ordinary
/// non-standard completion.  Nothing in [`SubCommand`] can say that,
/// because a body slot ([`ArgRole::Body`]) carries a *timing* and a
/// *kind*, never a set of completion codes the enclosing command
/// handles.  The missing field is E-R6's, on the body slot rather than
/// on the open CFG vocabulary:
///
/// ```text
/// body_completion_codes: &'static [(u8, CompletionCode, &'static str)]
/// ```
///
/// — "at argument *n* of this invocation, completion code *c* is a named,
/// loop-adjacent completion called *name*, consumed here" — so the
/// existing `BREAKS_LOOP`-family machinery could be scoped to that body
/// and nowhere else.  Until it exists, `prune` deliberately carries **no**
/// control-flow trait: `CONTINUES_LOOP` would be a lie the CFG builder
/// would act on, lowering a `continue` edge into whatever ordinary loop
/// happened to enclose the call.
fn struct_tree_prune_spec() -> CommandSpec {
    CommandSpec {
        name: "struct::tree::prune",
        arity: Arity::exact(0),
        completion: Some(CompletionDescriptor::exact(STRUCT_TREE_PRUNE_CODES)),
        hover: Some(HoverSnippet {
            summary: "Abort the current struct::tree walk script and skip the current node's children.",
            synopsis: &["::struct::tree::prune"],
            snippet: "Provided outside the tree methods because it is not a tree method: it returns completion code 5, which `treeName walk` and `treeName walkproc` interpret as \"ignore this node's children and continue\". It is an error to use it with `-order post` or `-order in`, which visit children before their parent; the only applicable orders are `pre` and `both`. Outside a walk script, code 5 simply propagates as a non-standard completion.",
            source: "tcllib struct::tree package",
            examples: "",
            return_value: "Nothing — the call completes with Tcl code 5.",
        }),
        tcllib_package: Some("struct::tree"),
        required_package: Some("struct::tree"),
        lifecycle: Lifecycle {
            introduced: Some("2.0"),
            ..Lifecycle::UNSPECIFIED
        },
        ..CommandSpec::DEFAULT
    }
}

/// The `tar` package.
const TAR_CMDS: &[Row] = &[
    (
        "tar::contents",
        Arity::at_least(1),
        &["tar::contents tarball"],
        "Returns a list of the files contained in tarball.",
    ),
    (
        "tar::stat",
        Arity::at_least(1),
        &["tar::stat tarball file"],
        "Returns a nested dict containing information on the named file in tarball, or all files if none is specified.",
    ),
    (
        "tar::untar",
        Arity::at_least(2),
        &["tar::untar tarball args"],
        "Extracts tarball.",
    ),
    (
        "tar::get",
        Arity::at_least(2),
        &["tar::get tarball fileName"],
        "Returns the contents of fileName from the tarball.",
    ),
    (
        "tar::create",
        Arity::at_least(3),
        &["tar::create tarball files args"],
        "Creates a new tar file containing the files.",
    ),
    (
        "tar::add",
        Arity::at_least(3),
        &["tar::add tarball files args"],
        "Appends files to the end of the existing tarball.",
    ),
    (
        "tar::remove",
        Arity::exact(2),
        &["tar::remove tarball files"],
        "Removes files from the tarball.",
    ),
];

/// The `pki` package.
const PKI_CMDS: &[Row] = &[
    (
        "pki::encrypt",
        Arity::at_least(2),
        &["pki::encrypt input key"],
        "Encrypt a message using PKI (probably RSA).",
    ),
    (
        "pki::decrypt",
        Arity::at_least(2),
        &["pki::decrypt input key"],
        "Decrypt a message using PKI (probably RSA).",
    ),
    (
        "pki::sign",
        Arity::at_least(2),
        &["pki::sign input key"],
        "Digitally sign message input using the private key.",
    ),
    (
        "pki::verify",
        Arity::at_least(3),
        &["pki::verify signedmessage plaintext key"],
        "Verify a digital signature using a public key.",
    ),
    (
        "pki::key",
        Arity::at_least(1),
        &["pki::key key"],
        "Convert a key structure into a serialized PEM (default) or DER encoded private key suitable for other applications.",
    ),
    (
        "pki::pkcs::parse_key",
        Arity::at_least(1),
        &["pki::pkcs::parse_key key"],
        "Convert a PKCS#1 private key into a usable key, i.e.",
    ),
    (
        "pki::x509::parse_cert",
        Arity::exact(1),
        &["pki::x509::parse_cert cert"],
        "Convert an X.509 certificate to a usable (public) key.",
    ),
    (
        "pki::rsa::generate",
        Arity::at_least(1),
        &["pki::rsa::generate bitlength"],
        "Generate a new RSA key pair, the parts of which can be used as argument for ::pki::encrypt, ::pki::decrypt.",
    ),
    (
        "pki::x509::verify_cert",
        Arity::at_least(2),
        &["pki::x509::verify_cert cert trustedcerts"],
        "Verify that a trust can be found between the certificate specified in the cert argument and one of the.",
    ),
    (
        "pki::x509::validate_cert",
        Arity::at_least(1),
        &["pki::x509::validate_cert cert"],
        "Validate that a certificate is valid to be used in some capacity.",
    ),
    (
        "pki::pkcs::create_csr",
        Arity::at_least(2),
        &["pki::pkcs::create_csr keylist namelist"],
        "Generate a certificate signing request from a key pair specified in the keylist argument.",
    ),
    (
        "pki::pkcs::parse_csr",
        Arity::exact(1),
        &["pki::pkcs::parse_csr csr"],
        "Parse a Certificate Signing Request.",
    ),
    (
        "pki::x509::create_cert",
        Arity::at_least(7),
        &[
            "pki::x509::create_cert signreqlist cakeylist serial_number notBefore notAfter isCA extensions",
        ],
        "Sign a signing request (usually from ::pki::pkcs::create_csr or ::pki::pkcs::parse_csr) with a Certificate.",
    ),
];

/// The `map::slippy` package (tile-geometry helper).
const MAP_SLIPPY_CMDS: &[Row] = &[(
    "map",
    Arity::at_least(0),
    &["map {slippy geo box 2point} zoom geobox"],
    "The command converts the geographical box geobox to a point box in the canvas, for the specified zoom level, and.",
)];

/// All data-structure / archive command specs.
pub fn specs() -> Vec<CommandSpec> {
    let mut specs = rows("tar", TAR_CMDS);
    specs.extend(rows("pki", PKI_CMDS));
    specs.extend(rows("map::slippy", MAP_SLIPPY_CMDS));
    specs.extend([
        object_creator(
            "struct::graph",
            &["struct::graph ?name? ?=|:=|as|deserialize source?"],
            "Create a directed-graph object command.",
            &STRUCT_GRAPH_CLASS,
        ),
        object_creator(
            "struct::tree",
            &["struct::tree ?name? ?=|:=|as|deserialize source?"],
            "Create a tree object command.",
            &STRUCT_TREE_CLASS,
        ),
        struct_tree_prune_spec(),
        creator(
            "struct::matrix",
            &["struct::matrix ?name? ?=|:=|as|deserialize source?"],
            "Create a two-dimensional matrix object command.",
        ),
        creator(
            "struct::prioqueue",
            &["struct::prioqueue ?-ascii|-integer|-real? name"],
            "Create a priority-queue object command.",
        ),
        creator(
            "struct::pool",
            &["struct::pool ?name? ?maxsize?"],
            "Create a resource-pool object command.",
        ),
        creator(
            "struct::disjointset",
            &["struct::disjointset name"],
            "Create a disjoint-set (union-find) object command.",
        ),
        creator(
            "struct::map",
            &["struct::map name"],
            "Create a bidirectional-map object command.",
        ),
    ]);
    specs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree_class() -> &'static ObjectClassSpec {
        specs()
            .into_iter()
            .find(|spec| spec.name == "struct::tree")
            .and_then(|spec| spec.object_class)
            .expect("the struct::tree class")
    }

    /// **P5.** The two walker interfaces coexist on one class, each
    /// carrying the lifecycle that says which `struct::tree` train it
    /// belongs to — the multi-train case as declaration data.
    #[test]
    fn both_walker_trains_are_declared_with_their_lifecycles() {
        let class = tree_class();
        let walk = class.instance_method("walk").expect("walk");
        let walkproc = class.instance_method("walkproc").expect("walkproc");

        // 2.x `walkproc` did not exist in the 1.2.3 train.
        assert_eq!(walkproc.lifecycle.introduced, Some("2.0"));
        assert_eq!(walkproc.lifecycle.retired, None);
        // …and it never accepted the 1.x `-command`.
        assert!(
            walkproc
                .options
                .iter()
                .all(|option| option.name != "-command"),
            "walkproc's WalkOptions accepts only -order/-type",
        );

        // 1.x `walk -command` was removed at 2.0, and appends nothing —
        // it is `string map`ped, not a prefix with runtime arguments.
        let command = walk
            .options
            .iter()
            .find(|option| option.name == "-command")
            .expect("walk -command");
        assert_eq!(command.lifecycle.retired, Some("2.0"));
        assert_eq!(command.lifecycle.introduced, None);
        assert_eq!(
            command.value_appended_arity(),
            AppendedArity::Exactly(0),
            "the %-substituted 1.x callback appends no words",
        );
        // The 2.x prefix twin does append three.
        assert_eq!(
            struct_tree_walkproc_command_prefixes(CommandPrefixArguments::literals(&[
                "root", "cb",
            ])),
            vec![(1, AppendedArity::Exactly(3))],
        );
    }

    /// `walk`'s body is the last word and its index moves with the option
    /// prefix, so it is resolved; the 1.x `-command` form has no trailing
    /// body at all and the resolver declines rather than guessing.
    #[test]
    fn the_walk_body_position_follows_the_option_prefix() {
        let roles = |args: &[&str]| struct_tree_walk_arg_roles(args);
        assert_eq!(
            roles(&["root", "n", "body"]),
            vec![(1, ArgRole::VarWrite), (2, ArgRole::Body)],
        );
        assert_eq!(
            roles(&["root", "-order", "both", "n", "body"]),
            vec![(3, ArgRole::VarWrite), (4, ArgRole::Body)],
        );
        assert_eq!(
            roles(&["root", "-order", "pre", "-type", "bfs", "--", "n", "body"]),
            vec![(6, ArgRole::VarWrite), (7, ArgRole::Body)],
        );
        // The 1.x callback form: no trailing script, so no body role.
        assert!(roles(&["root", "-command", "puts %n"]).is_empty());
        // Truncated calls never invent a position.
        assert!(roles(&["root"]).is_empty());
        assert!(roles(&["root", "n"]).is_empty());
    }

    /// `::struct::tree::prune` is a real command on the `struct::tree`
    /// axis whose completion domain is exactly the library-defined code
    /// 5 — the producer half of E-R6, which the model *can* say.
    #[test]
    fn prune_declares_the_library_completion_code_and_its_train() {
        let prune = struct_tree_prune_spec();
        assert_eq!(prune.owning_package(), Some("struct::tree"));
        assert_eq!(prune.lifecycle.introduced, Some("2.0"));
        assert_eq!(prune.arity, Arity::exact(0));
        let completion = prune.completion.expect("a completion descriptor");
        assert_eq!(
            completion.codes,
            CompletionCodeDomain::Exact(&[CompletionCode::Other(5)]),
        );
        // The consumer half is *not* expressible, so no control-flow
        // trait may be claimed: `CONTINUES_LOOP` would make the CFG
        // builder lower a continue edge into any enclosing ordinary loop.
        assert!(
            !prune
                .traits
                .intersects(Traits::CONTINUES_LOOP | Traits::BREAKS_LOOP),
            "prune's code 5 is scoped to a walk body, and nothing can say so",
        );
    }

    /// `struct::graph`'s walker is identical in both trains, so its one
    /// descriptor is correct for each — verified against `graph1.tcl` and
    /// `graph_tcl.tcl`.
    #[test]
    fn the_graph_walker_has_no_cross_train_delta() {
        let class = specs()
            .into_iter()
            .find(|spec| spec.name == "struct::graph")
            .and_then(|spec| spec.object_class)
            .expect("the struct::graph class");
        let walk = class.instance_method("walk").expect("walk");
        let mut names: Vec<&str> = walk.options.iter().map(|option| option.name).collect();
        names.sort_unstable();
        assert_eq!(names, ["-command", "-dir", "-order", "-type"]);
        for option in walk.options {
            assert!(
                option.lifecycle.is_unspecified(),
                "{}: both graph trains carry it",
                option.name,
            );
        }
    }
}

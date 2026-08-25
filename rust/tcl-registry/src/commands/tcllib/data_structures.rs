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
//! construct an instance command whose methods live on the returned object;
//! only the creator command is modelled statically here (recognition, package
//! gating, and hover).  Command names, arity bounds, synopses, and summaries
//! are derived from the upstream tcllib 2.0 manual pages.  Requires Tcl 8.5+.

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
/// (tree, node, action) ⇒ `Exactly(3)`.  (`walk … loopvar script` is a
/// loop-variable *script*, not a prefix, so it is not modelled here.)
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

const STRUCT_TREE_METHODS: &[SubCommand] = &[SubCommand {
    name: "walkproc",
    arity: Arity::at_least(2),
    detail: "Walk the tree from node, invoking the trailing command prefix at each node.",
    synopsis: "treeName walkproc node ?-order pre|post|in|both? ?-type bfs|dfs? cmdprefix",
    command_prefix_resolver: Some(struct_tree_walkproc_command_prefixes),
    ..SubCommand::DEFAULT
}];

static STRUCT_TREE_CLASS: ObjectClassSpec = ObjectClassSpec {
    class_name: "struct::tree",
    instance_methods: STRUCT_TREE_METHODS,
    superclasses: &[],
    allow_unknown_methods: true,
    method_prefix_matching: PrefixMatching::Strict,
};

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

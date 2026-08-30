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

//! Scoped command environments — curated command sets exposed inside a
//! command's script body.
//!
//! Some commands run their [`ArgRole::Body`](crate::arg_role::ArgRole::Body)
//! argument in a context that makes a *closed set of extra commands*
//! available — commands that exist only inside that body and nowhere else.
//! The archetype is tcllib `report::defstyle styleName arguments script`,
//! whose `script` is evaluated in a safe interpreter that aliases the report
//! configuration methods (`top`, `data`, `bottom`, `topdatasep`, `columns`,
//! `size`, …) as commands, plus every previously-defined style.
//!
//! Rather than teach the analyser / LSP that `top` / `data` / … are "known"
//! by name — which would false-suppress them *everywhere* — the command set is
//! registry **data** attached to the definer's [`CommandSpec::body_scope`].
//! The compiler and LSP are generic consumers: they push the environment while
//! walking the body and resolve heads against it, exactly as they push a
//! [`DefinitionBodyGrammar`](crate::definer::DefinitionBodyGrammar) for a class
//! definer.  Adding another scoped-body command (a safe-interp `eval` wrapper,
//! a bespoke DSL) is a matter of writing a [`ScopedCommandEnv`] and hanging it
//! off the command's spec — **never** editing a walker with a command name.
//!
//! [`CommandSpec::body_scope`]: crate::spec::CommandSpec::body_scope

use crate::arity::Arity;
use crate::hover::HoverSnippet;
use crate::spec::SubCommand;

/// A single command made available inside a scoped body.
///
/// Models an ensemble: [`Self::arity`] bounds the whole call (subcommand word
/// plus any operands), and [`Self::subcommands`] (possibly empty) are the
/// ensemble operations.  A plain command like `columns` carries no subcommands
/// and an `arity` over its positional operands directly.
#[derive(Debug)]
pub struct ScopedCommand {
    /// The command name as invoked inside the body (`top`, `data`, `columns`).
    pub name: &'static str,
    /// Argument-count bounds over the words after the command name.  For an
    /// ensemble this spans the subcommand word plus its operands; for a plain
    /// command it is the operand count directly.
    pub arity: Arity,
    /// Ensemble operations (`set` / `get` / `enable` / `disable` / `enabled`),
    /// each a reused [`SubCommand`].  Empty for a plain command.
    pub subcommands: &'static [SubCommand],
    /// Whether an unrecognised ensemble operation is accepted without W001.
    pub allow_unknown_subcommands: bool,
    /// One-line completion detail.
    pub detail: &'static str,
    /// Hover documentation for the command head.
    pub hover: Option<HoverSnippet>,
}

impl ScopedCommand {
    /// Resolve an ensemble operation word to its [`SubCommand`], accepting a
    /// unique non-empty prefix the way Tcl's ensemble dispatch does.  An exact
    /// match wins; an ambiguous prefix resolves to `None`.
    #[must_use]
    pub fn subcommand(&self, word: &str) -> Option<&SubCommand> {
        if word.is_empty() {
            return None;
        }
        if let Some(exact) = self.subcommands.iter().find(|s| s.name == word) {
            return Some(exact);
        }
        let mut hits = self.subcommands.iter().filter(|s| s.name.starts_with(word));
        let first = hits.next()?;
        if hits.next().is_some() {
            return None; // ambiguous prefix
        }
        Some(first)
    }
}

/// A curated command environment exposed inside a command's
/// [`ArgRole::Body`](crate::arg_role::ArgRole::Body) argument.
#[derive(Debug)]
pub struct ScopedCommandEnv {
    /// Human-facing environment name, for hover / diagnostics
    /// (`"report style definition"`).
    pub name: &'static str,
    /// The commands ambient inside the body, in canonical order.
    pub commands: &'static [ScopedCommand],
    /// When `true`, names introduced by the *owning definer command itself*
    /// (its [`ArgRole::Name`](crate::arg_role::ArgRole::Name) argument, across
    /// the whole document) are also ambient inside every scoped body — e.g. a
    /// previously-defined `report::defstyle` style is callable as a command in
    /// a later style body.  The analyser collects these names dynamically.
    pub include_sibling_definitions: bool,
    /// When `true`, an unrecognised bare command head inside the body is *not*
    /// flagged as unknown (W123) — the environment may inject commands the
    /// registry cannot enumerate.  Default `false` so genuine typos still
    /// surface; a closed environment (report) keeps this `false`.
    pub allow_unknown_commands: bool,
}

impl ScopedCommandEnv {
    /// Resolve a command head to its [`ScopedCommand`] by exact name.
    #[must_use]
    pub fn command(&self, name: &str) -> Option<&ScopedCommand> {
        self.commands.iter().find(|c| c.name == name)
    }

    /// Whether `name` is a command in this environment.
    #[must_use]
    pub fn is_command(&self, name: &str) -> bool {
        self.commands.iter().any(|c| c.name == name)
    }
}

// report — the `report::defstyle styleName arguments script` style-definition
// environment.
//
// VERIFIED against tcllib report(n) + report.tcl: the style script runs in a
// safe interpreter aliasing the report configuration methods.  Line commands
// (`top`, `data`, separators, …) are ensembles: template/data lines support
// `set`/`get`; separators additionally support `enable`/`disable`/`enabled`.
// Column/caption methods (`columns`, `size`, `pad`, `justify`, `tcaption`,
// `bcaption`, `printmatrix`, …) are plain commands.

/// `<line> set value` — install a template / separator string.
const OP_SET: SubCommand = SubCommand {
    name: "set",
    arity: Arity::exact(1),
    detail: "set the line template",
    synopsis: "<line> set template",
    ..SubCommand::DEFAULT
};
/// `<line> get` — return the current template string.
const OP_GET: SubCommand = SubCommand {
    name: "get",
    arity: Arity::exact(0),
    detail: "return the line template",
    synopsis: "<line> get",
    ..SubCommand::DEFAULT
};
/// `<sep> enable` — draw this separator line.
const OP_ENABLE: SubCommand = SubCommand {
    name: "enable",
    arity: Arity::exact(0),
    detail: "enable the separator line",
    synopsis: "<separator> enable",
    ..SubCommand::DEFAULT
};
/// `<sep> disable` — do not draw this separator line.
const OP_DISABLE: SubCommand = SubCommand {
    name: "disable",
    arity: Arity::exact(0),
    detail: "disable the separator line",
    synopsis: "<separator> disable",
    ..SubCommand::DEFAULT
};
/// `<sep> enabled` — query whether this separator is drawn.
const OP_ENABLED: SubCommand = SubCommand {
    name: "enabled",
    arity: Arity::exact(0),
    detail: "query whether the separator is drawn",
    synopsis: "<separator> enabled",
    ..SubCommand::DEFAULT
};

/// Operations of a data / template line (no enable/disable).
const TEMPLATE_OPS: &[SubCommand] = &[OP_SET, OP_GET];
/// Operations of a separator line.
const SEPARATOR_OPS: &[SubCommand] = &[OP_SET, OP_GET, OP_ENABLE, OP_DISABLE, OP_ENABLED];

/// Build a hover snippet for a report line command.
const fn line_hover(summary: &'static str, synopsis: &'static str) -> HoverSnippet {
    HoverSnippet {
        summary,
        synopsis: &[],
        snippet: synopsis,
        source: "tcllib report package",
        examples: "",
        return_value: "",
    }
}

/// A template / data line command (`set` / `get`).
const fn template_line(name: &'static str, detail: &'static str) -> ScopedCommand {
    ScopedCommand {
        name,
        // subcommand word plus the optional `set` value.
        arity: Arity::new(1, 2),
        subcommands: TEMPLATE_OPS,
        allow_unknown_subcommands: false,
        detail,
        hover: Some(line_hover(detail, "<line> set template | <line> get")),
    }
}

/// A separator line command (`set` / `get` / `enable` / `disable` / `enabled`).
const fn separator_line(name: &'static str, detail: &'static str) -> ScopedCommand {
    ScopedCommand {
        name,
        arity: Arity::new(1, 2),
        subcommands: SEPARATOR_OPS,
        allow_unknown_subcommands: false,
        detail,
        hover: Some(line_hover(
            detail,
            "<separator> set template | get | enable | disable | enabled",
        )),
    }
}

/// A plain configuration command (`columns`, `size`, …).
const fn config_command(
    name: &'static str,
    arity: Arity,
    detail: &'static str,
    synopsis: &'static str,
) -> ScopedCommand {
    ScopedCommand {
        name,
        arity,
        subcommands: &[],
        allow_unknown_subcommands: false,
        detail,
        hover: Some(line_hover(detail, synopsis)),
    }
}

/// The report style-definition command set (19 report configuration methods).
const REPORT_COMMANDS: &[ScopedCommand] = &[
    // Separator lines (top-to-bottom).
    separator_line("top", "topmost separator line of the report"),
    separator_line(
        "topdatasep",
        "separator between rows of the top caption region",
    ),
    separator_line(
        "topcapsep",
        "separator between the top caption and data regions",
    ),
    separator_line("datasep", "separator between rows of the data region"),
    separator_line(
        "botcapsep",
        "separator between the data and bottom caption regions",
    ),
    separator_line(
        "botdatasep",
        "separator between rows of the bottom caption region",
    ),
    separator_line("bottom", "bottommost separator line of the report"),
    // Data / template lines.
    template_line(
        "topdata",
        "template for data lines in the top caption region",
    ),
    template_line("data", "template for data lines in the data region"),
    template_line(
        "botdata",
        "template for data lines in the bottom caption region",
    ),
    // Column / caption configuration.
    config_command(
        "columns",
        Arity::exact(0),
        "number of columns in the report",
        "columns",
    ),
    config_command(
        "size",
        Arity::new(1, 2),
        "get/set a column's width",
        "size column ?number|dyn?",
    ),
    config_command(
        "sizes",
        Arity::new(0, 1),
        "get/set all column widths",
        "sizes ?size-list?",
    ),
    config_command(
        "pad",
        Arity::new(1, 3),
        "get/set a column's padding",
        "pad column ?where ?string??",
    ),
    config_command(
        "justify",
        Arity::new(1, 2),
        "get/set a column's justification",
        "justify column ?left|right|center?",
    ),
    config_command(
        "tcaption",
        Arity::new(0, 1),
        "get/set the top caption region size",
        "tcaption ?size?",
    ),
    config_command(
        "bcaption",
        Arity::new(0, 1),
        "get/set the bottom caption region size",
        "bcaption ?size?",
    ),
    config_command(
        "printmatrix",
        Arity::exact(1),
        "format a matrix using this report",
        "printmatrix matrix",
    ),
    config_command(
        "printmatrix2channel",
        Arity::exact(2),
        "format a matrix to a channel",
        "printmatrix2channel matrix chan",
    ),
];

/// The scoped command environment of a `report::defstyle` style script.
///
/// Sibling styles defined earlier in the document are also callable as
/// commands (`include_sibling_definitions`); the environment is otherwise
/// closed, so a mistyped method still draws a W123.
pub const REPORT_DEFSTYLE_ENV: ScopedCommandEnv = ScopedCommandEnv {
    name: "report style definition",
    commands: REPORT_COMMANDS,
    include_sibling_definitions: true,
    allow_unknown_commands: false,
};

// tclpkg — the `tclpkg.tcl` package-manifest directive environment.
//
// A manifest is a Tcl script evaluated in a deprivileged interpreter that
// defines exactly these directive commands (see
// `rust/tcl-pkg/src/manifest.rs`, `DIRECTIVES` + `apply_directive`, which is
// the runtime source of truth for names and arities — a drift test in
// `tcl-pkg` asserts the two stay aligned).  Several directives shadow real
// Tcl/Tk commands (`package`, `entry`), so the environment must *replace*
// normal resolution for its names inside a manifest file rather than merge
// with it — exactly what a scoped environment does.

/// Build a hover snippet for a tclpkg manifest directive.
const fn manifest_hover(summary: &'static str, synopsis: &'static str) -> HoverSnippet {
    HoverSnippet {
        summary,
        synopsis: &[],
        snippet: synopsis,
        source: "tclpkg manifest (tcl pkg)",
        examples: "",
        return_value: "",
    }
}

/// A tclpkg manifest directive (all are plain commands, no subcommands).
const fn manifest_directive(
    name: &'static str,
    arity: Arity,
    detail: &'static str,
    synopsis: &'static str,
) -> ScopedCommand {
    ScopedCommand {
        name,
        arity,
        subcommands: &[],
        allow_unknown_subcommands: false,
        detail,
        hover: Some(manifest_hover(detail, synopsis)),
    }
}

/// The tclpkg manifest directive set — names and arities mirror
/// `tcl-pkg`'s `manifest.rs` (`require_arity` calls in `apply_directive`).
const TCLPKG_MANIFEST_COMMANDS: &[ScopedCommand] = &[
    manifest_directive(
        "package",
        Arity::exact(1),
        "declare the package name",
        "package <name>",
    ),
    manifest_directive(
        "version",
        Arity::exact(1),
        "declare the package version",
        "version <semver>",
    ),
    manifest_directive(
        "description",
        Arity::exact(1),
        "one-line package description",
        "description <text>",
    ),
    manifest_directive(
        "license",
        Arity::exact(1),
        "SPDX licence identifier",
        "license <spdx-id>",
    ),
    manifest_directive(
        "author",
        Arity::exact(1),
        "package author (repeatable)",
        "author <name-and-email>",
    ),
    manifest_directive(
        "homepage",
        Arity::exact(1),
        "project homepage URL",
        "homepage <url>",
    ),
    manifest_directive(
        "tcl",
        Arity::exact(1),
        "required Tcl version constraint",
        "tcl <constraint>  (e.g. tcl >=8.6)",
    ),
    manifest_directive(
        "require",
        Arity::new(2, 4),
        "production dependency",
        "require <name> <minimum> ?-source <url>?",
    ),
    manifest_directive(
        "dev-require",
        Arity::new(2, 4),
        "development-only dependency",
        "dev-require <name> <minimum> ?-source <url>?",
    ),
    manifest_directive(
        "replace",
        Arity::exact(3),
        "replace a transitive dependency's source",
        "replace <name> <version> <source-url>",
    ),
    manifest_directive(
        "exclude",
        Arity::exact(2),
        "exclude a transitive dependency version",
        "exclude <name> <version>",
    ),
    manifest_directive(
        "provides",
        Arity::exact(1),
        "namespace or capability provided (repeatable)",
        "provides <name>",
    ),
    manifest_directive(
        "entry",
        Arity::exact(1),
        "application entry-point script",
        "entry <script.tcl>",
    ),
    manifest_directive(
        "build",
        Arity::new(1, 2),
        "declarative build script (never auto-run)",
        "build <script.tcl> ?-network?",
    ),
];

/// The whole-file environment of a `tclpkg.tcl` package manifest.
///
/// Closed (`allow_unknown_commands: false`): the manifest evaluator rejects
/// non-directive commands, so a typo'd directive should still draw W123.
pub const TCLPKG_MANIFEST_ENV: ScopedCommandEnv = ScopedCommandEnv {
    name: "tclpkg manifest",
    commands: TCLPKG_MANIFEST_COMMANDS,
    include_sibling_definitions: false,
    allow_unknown_commands: false,
};

/// File-name-keyed ambient environments: a document whose file *basename*
/// matches is analysed with the environment ambient across the whole
/// document — the file-level analogue of [`ScopedCommandEnv`] on a command
/// body.  Purely registry data; consumers look the file up here rather than
/// matching names themselves.
pub const FILE_SCOPED_ENVS: &[(&str, &ScopedCommandEnv)] = &[("tclpkg.tcl", &TCLPKG_MANIFEST_ENV)];

/// The whole-file scoped environment for a document at `file_path`, if any.
///
/// Matches on the path's final component (`/` or `\` separated), so both
/// `/proj/tclpkg.tcl` and a bare `tclpkg.tcl` resolve.
#[must_use]
pub fn file_scope_env(file_path: &str) -> Option<&'static ScopedCommandEnv> {
    let base = file_path.rsplit(['/', '\\']).next().unwrap_or(file_path);
    FILE_SCOPED_ENVS
        .iter()
        .find(|(name, _)| *name == base)
        .map(|(_, env)| *env)
}

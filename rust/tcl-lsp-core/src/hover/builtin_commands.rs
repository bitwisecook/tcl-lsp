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

//! The card for a built-in command and its iRules valid-event guidance.

#[cfg(test)]
use super::*;

/// Standalone fixture lookup through global imports before the cursor.
/// Newest eligible import wins; qualified source names decline.
#[cfg(test)]
fn resolve_imported_command<'r>(
    registry: &'r CommandRegistry,
    name: &str,
    analysis: &AnalysisResult,
    cursor_offset: u32,
) -> Option<(String, &'r tcl_registry::CommandSpec)> {
    if tcl_syntax::naming::is_qualified(name.as_bytes()) {
        return None;
    }
    // Iterate imports newest-first (`namespace_imports` is in source order): when
    // several in-scope imports could provide the same bare name, Tcl's later
    // `namespace import` (notably `-force`) wins, so the most recent one before
    // the cursor is the effective binding.
    for imp in analysis.namespace_imports.iter().rev() {
        if imp.ns != "::" || imp.range.end() > cursor_offset {
            continue;
        }
        let Some(candidate) = imp
            .source
            .as_ref()
            .and_then(|source| source.candidate(name))
        else {
            continue;
        };
        if let Some(spec) = registry.get(&candidate) {
            return Some((candidate, spec));
        }
    }
    None
}

/// Render a hover snippet for a built-in command name.
/// Looks up `name` in the registry, uses the matched spec's
/// `hover.summary` / `synopsis` to produce a markdown block.
#[cfg(test)]
pub(super) fn builtin_command_hover_text(
    registry: &CommandRegistry,
    name: &str,
    analysis: &AnalysisResult,
    cursor_offset: u32,
) -> Option<String> {
    use std::borrow::Cow;
    use std::fmt::Write;
    // Resolve the head through the document's `namespace import`s: a bare
    // `test` under `namespace import ::tcltest::*` hovers as `tcltest::test`
    // (mirrors the analyser's imported-command body resolution, scoped + ordered
    // to imports actually in effect at the cursor).
    let (name, spec): (Cow<'_, str>, _) = if let Some(spec) = registry.get(name) {
        (Cow::Borrowed(name), spec)
    } else {
        let (qual, spec) = resolve_imported_command(registry, name, analysis, cursor_offset)?;
        (Cow::Owned(qual), spec)
    };
    let name = name.as_ref();
    // A command whose *bare* spelling only works inside a `TclOO` method
    // context (`link` / `my` / `next` / `nextto` / `self` /
    // `classvariable`) has no hover anywhere else: at the top level real Tcl
    // answers `invalid command name`, so there is nothing to describe.
    // Registry data decides which commands those are; the frame classifier
    // decides where the cursor is. Hover keys on *callability*, not mere
    // resolution, so a Tcl 9 class `initialise` body — where the family
    // resolves but raises `… may only be called from inside a method` —
    // hovers only `my`, the one member a class-object frame can call. The
    // separately-registered qualified spelling (`::oo::Helpers::link`) is
    // not scoped and still hovers everywhere.
    if !crate::oo_dispatch::OoFrame::at(analysis, cursor_offset).admits(registry, name) {
        return None;
    }
    let hover = spec.hover.as_ref()?;
    let mut out = format!("**`{name}`** — built-in command\n");
    if !hover.summary.is_empty() {
        let _ = write!(out, "\n{}\n", hover.summary);
    }
    if let Some(synopsis) = hover.synopsis.first() {
        let _ = write!(out, "\n```tcl\n{synopsis}\n```\n");
    }
    if !spec.subcommands.is_empty() {
        let mut names: Vec<&str> = spec.subcommands.iter().map(|s| s.name).collect();
        names.sort_unstable();
        let joined = names.join(", ");
        let _ = write!(out, "\nSubcommands: {joined}\n");
    }
    // Import hint: when the command needs a `package require` the
    // document hasn't imported, append a `**Requires**` line.  Gated on
    // the dialect supporting `package require` at all (the `package`
    // command must exist — e.g. iRules bans it), and never shown for a
    // package the profile ships ambiently (an F5 surface is part of the
    // runtime, §7.1 axis C — there is nothing to require).
    if let Some(pkg) = spec.required_package
        && !registry.is_ambient_package(pkg)
    {
        let pkg_available = registry.get("package").is_some();
        let imported = analysis.package_requires.iter().any(|pr| pr.name == pkg);
        if pkg_available && !imported {
            let _ = write!(out, "\n**Requires**: `package require {pkg}`");
        }
    }
    let release = analysis
        .resolved_input
        .as_ref()
        .map_or(EventRelease::Unconfigured, |input| {
            EventRelease::of(input.availability_context())
        });
    append_command_event_metadata(&mut out, spec, release);
    Some(out)
}

/// The BIG-IP release a command's **Valid events** guidance is filtered to:
/// the document's configured iRules placement floor, when it has one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EventRelease<'a> {
    /// Events are listed only where the declared lifecycle has them at this
    /// release.
    Configured(&'a str),
    /// No release is configured; the list is the requirement match alone and
    /// says so.
    Unconfigured,
}

impl<'a> EventRelease<'a> {
    /// The release `context` places the iRules command surface at.
    pub(super) fn of(context: &'a tcl_registry::model::ResolvedContext) -> Self {
        context
            .placement_floor("f5-irules-cmds")
            .map_or(Self::Unconfigured, |floor| Self::Configured(floor.as_str()))
    }
}

pub(super) fn append_command_event_metadata(
    out: &mut String,
    spec: &tcl_registry::CommandSpec,
    release: EventRelease<'_>,
) {
    use std::fmt::Write;
    if let Some(requires) = spec.event_requires.as_ref() {
        append_valid_events(out, &effective_event_requires(spec, requires), release);
    } else {
        let profiles = tcl_registry::profiles::ProfileRegistry::build();
        if let Some(namespace) = profiles.namespace_for_command_spec(spec)
            && !namespace.profiles.is_empty()
        {
            let _ = write!(
                out,
                "\n\n**Requires**: {} profile",
                namespace.profiles.join(", ")
            );
        }
    }
}

/// Augment a command's `EventRequires` with namespace-backed profile
/// metadata.  For an iRules `NS::cmd` whose own `profiles` are empty, the
/// profiles declared by the `NS` protocol namespace are substituted, so
/// the **Valid events** list narrows and a profile **Requires** line is
/// shown (e.g. `DIAMETER::*`, `MQTT::*`, `LDAP::*`, `IMAP::*`).
/// The selected command row retains its declared F5 surface; context and source
/// applicability stay with the caller.
pub(super) fn effective_event_requires(
    spec: &tcl_registry::CommandSpec,
    requires: &tcl_registry::events::EventRequires,
) -> tcl_registry::events::EventRequires {
    // A command with its own profiles, or no `NS::` qualifier, is unchanged.
    if !requires.profiles.is_empty() {
        return requires.clone();
    }
    let profile_reg = tcl_registry::profiles::ProfileRegistry::build();
    match profile_reg.namespace_for_command_spec(spec) {
        Some(ns) if !ns.profiles.is_empty() => tcl_registry::events::EventRequires {
            profiles: ns.profiles,
            ..*requires
        },
        _ => requires.clone(),
    }
}

/// Append the iRules "**Valid events**" list (first 8 + total) and the
/// event "**Requires**" line (sides / transport / profiles) for a
/// command's `EventRequires`. The list names the release it was filtered to,
/// or that none was configured.
pub(super) fn append_valid_events(
    out: &mut String,
    requires: &tcl_registry::events::EventRequires,
    release: EventRelease<'_>,
) {
    use std::fmt::Write;
    let matching = valid_events(requires, release);
    if !matching.is_empty() {
        let mut list = matching
            .iter()
            .take(8)
            .map(|e| format!("`{e}`"))
            .collect::<Vec<_>>()
            .join(", ");
        if matching.len() > 8 {
            let _ = write!(list, ", ... ({} total)", matching.len());
        }
        match release {
            EventRelease::Configured(version) => {
                let _ = write!(out, "\n\n**Valid events** (BIG-IP {version}): {list}");
            }
            EventRelease::Unconfigured => {
                let _ = write!(
                    out,
                    "\n\n**Valid events** (event requirements only; no BIG-IP release configured): {list}"
                );
            }
        }
    }
    let mut reqs: Vec<String> = Vec::new();
    if requires.client_side {
        reqs.push("client-side".to_owned());
    }
    if requires.server_side {
        reqs.push("server-side".to_owned());
    }
    if let Some(t) = requires.transport {
        reqs.push(t.to_uppercase());
    }
    if !requires.profiles.is_empty() {
        let mut profs: Vec<&str> = requires.profiles.to_vec();
        profs.sort_unstable();
        reqs.push(format!("profile {}", profs.join(" or ")));
    }
    if !reqs.is_empty() {
        let _ = write!(out, "\n\n**Requires**: {}", reqs.join(", "));
    }
}

/// Sorted event names whose properties satisfy `requires` and, at a
/// configured release, whose declared lifecycle has them at that release.
pub(super) fn valid_events(
    requires: &tcl_registry::events::EventRequires,
    release: EventRelease<'_>,
) -> Vec<String> {
    let event_reg = tcl_registry::events::EventRegistry::build();
    let profile_reg = tcl_registry::profiles::ProfileRegistry::build();
    let mut out: Vec<String> = event_reg
        .all_event_names()
        .into_iter()
        .filter(|name| match release {
            EventRelease::Configured(version) => event_reg.event_available_at(name, version),
            EventRelease::Unconfigured => true,
        })
        .filter(|name| {
            event_reg.get_props(name).is_some_and(|p| {
                tcl_registry::events::event_satisfies(p, requires, name, &profile_reg)
            })
        })
        .map(ToOwned::to_owned)
        .collect();
    out.sort_unstable();
    out
}

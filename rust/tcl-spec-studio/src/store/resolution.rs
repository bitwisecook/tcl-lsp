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

//! The merged world every surface queries: built-ins with the pack laid over them, its views and validation report, and their JSON.

use super::patch_packs::StandingOverride;
use super::{Builtins, PackStore, line_reasons_json};
use crate::draft;
use crate::draft::Draft;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::sync::Arc;
use tcl_registry::CommandRegistry;
use tcl_spectcl::loader::{FileExtension, HookOwner, HookSource, Notice};
use tcl_spectcl::{Tier, loader};

// The resolution facade

/// Where a name's *effective* definition comes from once the collision policy
/// has been applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Origin {
    /// Only the shipped registry defines it.
    Builtin,
    /// Only the pack defines it.
    Pack,
    /// Both define it and the pack said `-override`; the pack's wins.
    Override,
    /// Both define it and the pack did not say `-override`; the shipped one
    /// wins and the pack's declaration is inert.
    Shadowed,
}

impl Origin {
    /// The word the report uses.
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            Self::Builtin => "builtin",
            Self::Pack => "pack",
            Self::Override => "override",
            Self::Shadowed => "shadowed",
        }
    }

    /// Whether the pack's declaration is the one an editor would use.
    #[must_use]
    pub fn pack_wins(self) -> bool {
        matches!(self, Self::Pack | Self::Override)
    }
}

/// The one merged world every surface queries: built-ins with the pack laid
/// over them, collision policy applied.
pub struct Resolution<'a> {
    builtins: Builtins,
    store: &'a PackStore,
}

impl<'a> Resolution<'a> {
    /// Merge `store` over `builtins`.
    #[must_use]
    pub fn new(builtins: Builtins, store: &'a PackStore) -> Self {
        Self { builtins, store }
    }

    /// The built-ins half.
    #[must_use]
    pub fn builtins(&self) -> Builtins {
        self.builtins
    }

    /// The user-definitions half.
    #[must_use]
    pub fn store(&self) -> &PackStore {
        self.store
    }

    /// The **live registry** this merged world resolves to: the shipped one
    /// for the dialect with the pack installed over it.
    ///
    /// This is the seam that makes the Test tab honest. Rather than the studio
    /// merging two drafts and calling that "what an editor would see", the
    /// pack goes through `tcl_spectcl::install::registry_with_packs` — the very
    /// function the language server calls at workspace init — so the collision
    /// policy, the vendor package gate, and hook specialisation are all the
    /// shipped behaviour rather than a re-implementation of it.
    ///
    /// Cached on `(profile, key)` inside the registry cache, so repeated
    /// queries against an unchanged document cost a lookup. An edit is a new
    /// key and therefore a new registry: callers should debounce, which is why
    /// the Test tab re-analyses on a settle timer rather than per keystroke.
    ///
    /// An owning handle: an edit supersedes the previous key, and the
    /// generation behind it is freed once the last reader lets go.
    #[must_use]
    pub fn registry(&self) -> Arc<CommandRegistry> {
        tcl_spectcl::install::registry_with_packs(
            crate::environment::profile_for_dialect(self.builtins.dialect()),
            &self.store.pack_set(),
        )
    }

    /// The pack overlay identity [`Self::registry`] installed under — the value
    /// `Analyser::with_pack_overlay` takes. `0` when the pack declares nothing.
    #[must_use]
    pub fn overlay_key(&self) -> u64 {
        self.store.overlay_key()
    }

    /// Where `name` resolves.
    ///
    /// This is `tcl_spectcl::pack::installs_over`'s rule — *shipped wins unless
    /// the pack says `-override`* — restated as a four-way answer so a surface
    /// can show the author which of the two definitions is live.
    ///
    /// The **effective** draft decides, patch pack included (E-R12): an edit
    /// to a programmed document lands in the patch, the patch is what the
    /// installed registry answers with, and a command the patch alone
    /// declares is therefore a command the pack has. Asking the document
    /// alone made every such write invisible the moment it was made — the
    /// surface that had just written it was told the name is in neither the
    /// pack nor the registry.
    #[must_use]
    pub fn origin(&self, name: &str) -> Option<Origin> {
        let in_pack = self.store.effective_draft(name).is_some();
        let in_builtins = self.builtins.contains(name);
        // A patch declaration is written `-override` whatever the form asked
        // for — a patch that does not override is inert — so a standing patch
        // over a shipped name is an override of it.
        let overrides =
            self.store.overrides_shipped(name) || self.store.patch_draft(name).is_some();
        match (in_pack, in_builtins) {
            (false, false) => None,
            (false, true) => Some(Origin::Builtin),
            (true, false) => Some(Origin::Pack),
            (true, true) if overrides => Some(Origin::Override),
            (true, true) => Some(Origin::Shadowed),
        }
    }

    /// The merged view of `name`: which definition is live, and both of them
    /// when there are two.
    #[must_use]
    pub fn view(&self, name: &str) -> Option<Value> {
        let origin = self.origin(name)?;
        // The **effective** pack draft: a standing patch override is what the
        // registry answers with, so it is what the form must show (E-R12).
        let pack = self.store.effective_draft(name);
        let builtin = self.builtins.draft(name);
        let effective = if origin.pack_wins() {
            pack.cloned()
        } else {
            builtin.clone()
        };
        Some(json!({
            "name": name,
            "origin": origin.key(),
            "editable": origin.pack_wins() || pack.is_some(),
            "dialect": self.builtins.dialect(),
            // Prefixed: in a *command* view every unprefixed key is the
            // command's, and `pack` is already this command's pack draft.
            "pack_display_name": self.store.display_name(),
            "pack_file_extensions": file_extensions_json(self.store.file_extensions()),
            "override": self.store.overrides_shipped(name),
            // Whether what is shown comes from a `StudioOverride` patch pack
            // rather than from the document itself.
            "patched": self.store.patch_draft(name).is_some(),
            "effective": effective,
            "pack": pack,
            "builtin": builtin,
            "notices": notices_json(&self.store.notices_for(name)),
        }))
    }

    /// One row per pack command, with the state a sidebar shows at a glance.
    #[must_use]
    pub fn pack_index(&self) -> Vec<Value> {
        let defaults = draft::default_command_draft();
        self.store
            .commands()
            .iter()
            .map(|(name, d)| {
                let origin = self.origin(name).unwrap_or(Origin::Pack);
                let notices = self.store.notices_for(name);
                let site = self.store.declaration_site(name);
                json!({
                    "name": name,
                    "origin": origin.key(),
                    "override": self.store.overrides_shipped(name),
                    // Where the declaration is, and whether the author can
                    // edit it at all: an expanded command is a program's
                    // output (E-R12 — the studio never rewrites a programmed
                    // pack), so a surface labels it rather than opening it.
                    "declared_at": site.as_ref().map(|site| json!({
                        "line": site.line,
                        "file": site.file.as_ref().map(|f| f.display().to_string()),
                        "expanded": site.expanded,
                    })),
                    // A standing patch override (E-R12): the row a surface
                    // labels "patched" and offers a "revert" on.
                    "patched": self.store.patch_draft(name).is_some(),
                    "summary": summary_of(d),
                    "fields_set": fields_set(d, &defaults).len(),
                    "subcommands": count_of(d, "subcommands"),
                    "options": count_of(d, "options"),
                    "notices": notices.len(),
                    "unrenderable": d
                        .get(draft::UNRENDERABLE_KEY)
                        .and_then(Value::as_array)
                        .map_or(0, Vec::len),
                })
            })
            .collect()
    }

    /// Every name where the pack and the shipped registry both speak.
    #[must_use]
    pub fn collisions(&self) -> Vec<Value> {
        self.store
            .commands()
            .iter()
            .filter_map(|(name, _)| {
                let origin = self.origin(name)?;
                if !matches!(origin, Origin::Override | Origin::Shadowed) {
                    return None;
                }
                Some(json!({
                    "name": name,
                    "override": matches!(origin, Origin::Override),
                    "effect": if matches!(origin, Origin::Override) {
                        "pack-spec-wins"
                    } else {
                        "shipped-spec-wins"
                    },
                    "reason": if matches!(origin, Origin::Override) {
                        format!("`{name}` replaces the shipped command of that name (`-override`)")
                    } else {
                        format!(
                            "`{name}` is already a shipped command; the shipped spec wins \
                             (declare it `-override` to replace it)"
                        )
                    },
                }))
            })
            .collect()
    }

    /// The whole-store view a sidebar and a status bar are built from.
    #[must_use]
    pub fn store_view(&self) -> Value {
        let collisions = self.collisions();
        let shadowed = collisions
            .iter()
            .filter(|c| c["effect"] == "shipped-spec-wins")
            .count();
        json!({
            "pack": self.store.name(),
            "display_name": self.store.display_name(),
            "file_extensions": file_extensions_json(self.store.file_extensions()),
            "dsl_version": self.store.dsl_version(),
            "dialect": self.builtins.dialect(),
            // Two facts only the evaluation loader can report: whether this
            // surface is one analysis target's answer (E-R1), and what an
            // untrusted tier would refuse the document for (E-R2).
            "target_dependent": self.store.target_dependent(),
            "untrusted_tier_refusal": self
                .store
                .untrusted_tier_refusal()
                .map(|(line, why)| json!({ "line": line, "reason": why })),
            // The codegen-axis stamps a workspace load drops (the stamp
            // rejection rule): reported here, never removed from the text.
            "stamp_refusals": line_reasons_json(&self.store.stamp_refusals()),
            // E-R12: whether this document is a program the studio must not
            // rewrite, and what patch pack currently stands over it.
            "programmed": self.store.programmed().map(|why| json!({
                "why": why.key(),
                "reason": why.reason(),
            })),
            "patch": self.store.patch_source().map(|source| json!({
                "pack": self.store.patch_name(),
                "tier": Tier::StudioOverride.label(),
                "source": source,
                "untrusted_tier_refusal": self
                    .store
                    .patch_untrusted_tier_refusal()
                    .map(|(line, why)| json!({ "line": line, "reason": why })),
                "stamp_refusals": line_reasons_json(&self.store.patch_stamp_refusals()),
            })),
            "standing_overrides": standing_overrides_json(&self.store.standing_overrides()),
            "commands": self.pack_index(),
            "notices": notices_json(&self.store.notices().iter().collect::<Vec<_>>()),
            "collisions": collisions,
            "summary": {
                "commands": self.store.commands().len(),
                "notices": self.store.notices().len(),
                "collisions": collisions.len(),
                "shadowed_commands": shadowed,
                "standing_overrides": self.store.standing_overrides().len(),
                "bytes": self.store.source().len(),
            },
        })
    }

    /// The structured validation report — the studio's twin of `tcl spec
    /// check` and the MCP server's `spectcl_check`.
    ///
    /// Same three questions: what did the pack declare, what did the loader
    /// drop, and what collides with a shipped spec.
    #[must_use]
    pub fn validate(&self) -> Value {
        let defaults = draft::default_command_draft();
        let sub_defaults = draft::default_subcommand_draft();
        let commands: Vec<Value> = self
            .store
            .commands()
            .iter()
            .map(|(name, d)| {
                let subcommands: Vec<Value> = d
                    .get("subcommands")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_object)
                    .map(|sub| {
                        json!({
                            "name": sub.get("name").cloned().unwrap_or(Value::Null),
                            "fields_set": fields_set(sub, &sub_defaults),
                        })
                    })
                    .collect();
                let hooks: Vec<Value> = self
                    .store
                    .pack
                    .command(name)
                    .map_or_default(|c| c.hooks.iter().map(hook_json).collect());
                json!({
                    "name": name,
                    "origin": self.origin(name).map_or("pack", Origin::key),
                    "override": self.store.overrides_shipped(name),
                    "fields_set": fields_set(d, &defaults),
                    "unrenderable": d.get(draft::UNRENDERABLE_KEY).cloned().unwrap_or(Value::Null),
                    "subcommands": subcommands,
                    // The declared grammar itself — the draft's value, plain
                    // data — or `null` when the command declares none.
                    "clause_grammar": d.get("clause_grammar").cloned().unwrap_or(Value::Null),
                    "hooks": hooks,
                    // Structured `example` blocks failing the checks the
                    // built-in examples pass: text off its line, arrows out of
                    // execution order.
                    "example_errors": self.store.pack.command(name).map_or_default(|c| {
                        c.spec
                            .documentation_examples
                            .iter()
                            .enumerate()
                            .flat_map(|(index, example)| {
                                crate::examples::documentation_example_errors(
                                    *example,
                                    &format!("{name} example {}", index + 1),
                                )
                            })
                            .collect::<Vec<_>>()
                    }),
                })
            })
            .collect();

        let mut report = self.store_view();
        if let Some(map) = report.as_object_mut() {
            map.insert("commands".to_owned(), Value::Array(commands));
            let hooks: usize = self.store.pack.commands.iter().map(|c| c.hooks.len()).sum();
            if let Some(summary) = map.get_mut("summary").and_then(Value::as_object_mut) {
                summary.insert("hooks".to_owned(), json!(hooks));
            }
        }
        report
    }
}

// JSON helpers

/// The pack's `file_extension` rows as a surface reads them: what the file
/// type is called and where a file of it routes, minus the declaring line
/// (which only the DSL pane's notices need).
/// The standing patch overrides as a surface reads them: which command, from
/// which patch, over which base declaration (design E §15.2 — "`spec check`
/// reports standing overrides so they cannot rot silently").
#[must_use]
pub fn standing_overrides_json(rows: &[StandingOverride]) -> Value {
    Value::Array(
        rows.iter()
            .map(|row| {
                json!({
                    "command": row.command,
                    "patch_pack": row.patch_pack,
                    "base_pack": row.base_pack,
                    "base_line": row.base_line,
                    "base_expanded": row.base_expanded,
                })
            })
            .collect(),
    )
}

fn file_extensions_json(rows: &[FileExtension]) -> Value {
    Value::Array(
        rows.iter()
            .map(|row| {
                json!({
                    "extension": row.extension,
                    "display_name": row.display_name,
                    "dialect": row.dialect,
                })
            })
            .collect(),
    )
}

fn notices_json(notices: &[&Notice]) -> Value {
    Value::Array(
        notices
            .iter()
            .map(|n| json!({ "line": n.line, "context": n.context, "reason": n.message }))
            .collect(),
    )
}

/// The draft keys `d` says something about — [`fields_set`] against a
/// brand-new command's defaults, for callers outside this module.
#[must_use]
pub fn fields_set_of(d: &Draft) -> Vec<String> {
    fields_set(d, &draft::default_command_draft())
}

/// The draft keys this declaration actually said something about, sorted.
///
/// Two kinds, because the draft model has two ways of saying "set": a key
/// whose *value* differs from a brand-new command's, and a key listed under
/// [`draft::UNRENDERABLE_KEY`] — precisely the fields that *are* set but whose
/// defining Rust expression could not be recovered, every hook among them.
/// `name` is excluded because it always differs and is reported separately.
pub(super) fn fields_set(d: &Draft, defaults: &Draft) -> Vec<String> {
    let mut set: BTreeSet<String> = d
        .iter()
        .filter(|(key, _)| key.as_str() != "name" && key.as_str() != draft::UNRENDERABLE_KEY)
        .filter(|(key, value)| defaults.get(key.as_str()) != Some(*value))
        .map(|(key, _)| key.to_owned())
        .collect();
    set.extend(
        d.get(draft::UNRENDERABLE_KEY)
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .map(str::to_owned),
    );
    set.into_iter().collect()
}

/// The hover summary a sidebar row shows, if the draft has one.
fn summary_of(d: &Draft) -> String {
    d.get("hover")
        .and_then(Value::as_object)
        .and_then(|h| h.get("summary"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

fn count_of(d: &Draft, key: &str) -> usize {
    d.get(key).and_then(Value::as_array).map_or(0, Vec::len)
}

/// The schema spelling of a hook family — the word an author writes.
fn family_key(family: loader::HookFamily) -> &'static str {
    use loader::HookFamily as F;
    match family {
        F::ArgRoleResolver => "arg_role_resolver",
        F::CommandPrefixResolver => "command_prefix_resolver",
        F::ScriptTimingResolver => "script_timing_resolver",
        F::ConstFold => "const_fold",
        F::ConstFoldVersioned => "const_fold_versioned",
        F::TaintSinkGate => "taint_sink_gate",
        F::ContextGate => "context_gate",
        F::LiteralArgumentValidator => "literal_argument_validator",
        F::ClauseShapeCheck => "clause_shape_check",
        F::Constraints => "constraints",
        F::OptionArity => "-arity-hook",
        F::Evaluate => "evaluate",
        F::StateTransitionResolver => "state_transitions.resolver",
    }
}

fn hook_json(hook: &loader::HookDecl) -> Value {
    let (kind, detail) = match &hook.source {
        HookSource::Body { params, .. } => ("body", params.join(" ")),
        HookSource::Native { id } => ("native", id.clone()),
        HookSource::Derived { keyword } => ("derived", keyword.clone()),
    };
    let owner = match &hook.owner {
        HookOwner::Command => "command".to_owned(),
        HookOwner::Subcommand(name) => format!("subcommand {name}"),
        HookOwner::Option { subcommand, option } => match subcommand {
            Some(sub) => format!("subcommand {sub} option {option}"),
            None => format!("option {option}"),
        },
    };
    json!({
        "owner": owner,
        "field": hook.field,
        "family": family_key(hook.family),
        "source": kind,
        "detail": detail,
    })
}

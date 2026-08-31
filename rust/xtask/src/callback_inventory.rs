// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Registry-backed inventory of executable and callback-valued surfaces.
//!
//! Static rows come directly from `CommandSpec`, `SubCommand`, `OptionSpec`,
//! and object-class instance-method metadata. Resolver-bearing surfaces are
//! retained as `dynamic` rows: the generator must not guess a concrete
//! position from a function pointer. The small seed file supplies audited
//! facts which do not have a Tcl callback shape at all (notably iRulesLX
//! remote dispatch) or whose value has an intentionally ambiguous shape.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::fs;
use std::process::ExitCode;
use tcl_dialect::model::surface_admits;

use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use tcl_registry::{
    AppendedArity, ArgRole, CallbackTaintInput, CommandSpec, ScriptTiming, SubCommand, Traits,
    hover::{OptionSpec, OptionValue},
    lifecycle::Lifecycle,
};

use crate::callback_coverage::SurfaceRow;
use crate::util::repo_root;
use tcl_dialect::model::SpecSurface;

const SEED_PATH: &str = "docs/references/command-spec/callback-surface-catalogue.json";
const JSON_PATH: &str = "docs/generated/callback-surfaces.json";
const REPORT_PATH: &str = "docs/generated/callback-surfaces.md";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SurfaceKind {
    CommandPrefix,
    BodyScript,
    ReferenceOnly,
    Dynamic,
    ExternalDispatch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Timing {
    SameInvocation,
    Deferred,
    ReferenceOnly,
    Dynamic,
    BlockingExternalRpc,
    FireAndForgetExternal,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct InventoryRow {
    id: String,
    owner: String,
    surface: String,
    kind: SurfaceKind,
    provenance: String,
    dialects: Vec<String>,
    lifecycle: String,
    timing: Timing,
    appended_arity: Option<String>,
    callback_taint_inputs: Vec<String>,
    forms: Vec<String>,
    registry_derived: bool,
    registry_owner: Option<String>,
    notes: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Seed {
    schema_version: u8,
    rows: Vec<InventoryRow>,
}

pub fn run(check: bool) -> Result<ExitCode> {
    let root = repo_root();
    let seed_path = root.join(SEED_PATH);
    let seed: Seed = serde_json::from_str(
        &fs::read_to_string(&seed_path)
            .with_context(|| format!("reading {}", seed_path.display()))?,
    )
    .with_context(|| format!("parsing {}", seed_path.display()))?;
    if seed.schema_version != 1 {
        bail!(
            "unsupported callback inventory seed schema {}",
            seed.schema_version
        );
    }

    let mut rows = discover_registry_rows()?;
    validate_seed(&seed.rows)?;
    rows.extend(seed.rows);
    rows.sort_by(|a, b| a.id.cmp(&b.id).then(a.surface.cmp(&b.surface)));
    reject_duplicate_ids(&rows)?;

    // Before the projection is written *or* compared: a generated file cannot
    // tell a retired callback from a lost one, so the authored manifest is
    // what makes a downgrade fail in write mode too (issue #1706).
    let view: Vec<SurfaceRow<'_>> = rows.iter().map(surface_row).collect();
    crate::callback_coverage::enforce(&root, &view)?;

    let json = format!("{}\n", serde_json::to_string_pretty(&rows)?);
    let markdown = render_markdown(&rows);
    if check {
        check_file(&root.join(JSON_PATH), &json)?;
        check_file(&root.join(REPORT_PATH), &markdown)?;
    } else {
        fs::write(root.join(JSON_PATH), json).context("writing callback inventory JSON")?;
        fs::write(root.join(REPORT_PATH), markdown).context("writing callback inventory report")?;
    }
    Ok(ExitCode::SUCCESS)
}

/// The row as the authored manifest reads it.
///
/// A registry row's id carries the merged dialect list (`fcopy/option
/// -command value@tcl8.4+…`), which changes whenever a surface reaches one
/// more profile. The manifest pins owner and location, so the suffix is
/// stripped here rather than being written into every authored row.
fn surface_row(row: &InventoryRow) -> SurfaceRow<'_> {
    let tail = row
        .id
        .strip_prefix(&format!("{}/", row.owner))
        .unwrap_or(&row.id);
    let location = tail
        .strip_suffix(&format!("@{}", row.dialects.join("+")))
        .unwrap_or(tail);
    SurfaceRow {
        owner: &row.owner,
        location,
        kind: row.kind,
        timing: row.timing,
        appended_arity: row.appended_arity.as_deref(),
        dialects: &row.dialects,
    }
}

fn discover_registry_rows() -> Result<Vec<InventoryRow>> {
    let mut rows: BTreeMap<String, InventoryRow> = BTreeMap::new();
    for profile in tcl_dialect::DialectProfile::all()
        .iter()
        .chain(std::iter::once(crate::environment::profile_for_dialect(
            "tk",
        )))
    {
        let registry = crate::environment::store_for_profile(profile);
        let mut names: Vec<_> = registry.command_names().collect();
        names.sort_unstable();
        for name in names {
            let Some(spec) = registry.get(name) else {
                continue;
            };
            collect_spec(&mut rows, profile.name, name, spec)?;
        }
    }
    collect_bundled_packs(&mut rows)?;
    Ok(rows
        .into_values()
        .map(|mut row| {
            row.id = format!("{}@{}", row.id, row.dialects.join("+"));
            row
        })
        .collect())
}

/// The EDA vendor libraries ship as bundled `SpecTcl` loadables rather than
/// native specs (AGENTS.md § *Command registry*), so the per-profile stores
/// above never carry them and a vendor callback would be invisible to this
/// audit rather than merely unclassified.
///
/// Walking the pack-installed store closes that. Four surfaces reach the
/// inventory only this way today — Vivado's `add_condition` block, UPF's
/// `create_upf_library -contents` and its `define_power_model` role resolver,
/// and SDC's `foreach_in_collection` body — and the vendor `-rule_body`
/// checker procedures stay waived in the coverage manifest, which is only a
/// waiver worth having because this pass would see the classification arrive.
fn collect_bundled_packs(rows: &mut BTreeMap<String, InventoryRow>) -> Result<()> {
    let packs = tcl_spectcl::bundled::load_from(&repo_root().join("specs"));
    for profile in tcl_dialect::DialectProfile::all() {
        let registry = tcl_spectcl::bundled::registry_for_dialect_from(profile.name, &packs);
        let mut names: Vec<_> = registry.command_names().collect();
        names.sort_unstable();
        for name in names {
            let Some(spec) = registry.get(name) else {
                continue;
            };
            collect_spec(rows, profile.name, name, spec)?;
        }
    }
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn collect_spec(
    rows: &mut BTreeMap<String, InventoryRow>,
    dialect: &str,
    registered_name: &str,
    spec: &CommandSpec,
) -> Result<()> {
    let owner = registered_name;
    let command_provenance = provenance(spec.hover.map(|hover| hover.source), spec);
    let forms = command_forms(spec);
    collect_static(
        rows,
        dialect,
        owner,
        "command",
        spec.arg_roles,
        spec.command_prefixes,
        spec.callback_taint_inputs,
        spec.traits,
        &format_lifecycle(spec.lifecycle),
        &command_provenance,
        &forms,
    )?;
    collect_options(
        rows,
        dialect,
        owner,
        "command-option",
        spec.options,
        &format_lifecycle(spec.lifecycle),
        &command_provenance,
        &forms,
    )?;
    collect_dynamic(
        rows,
        dialect,
        owner,
        "command",
        spec.arg_role_resolver.is_some(),
        spec.command_prefix_resolver.is_some(),
        spec.script_timing_resolver.is_some(),
        &format_lifecycle(spec.lifecycle),
        &command_provenance,
        &forms,
    )?;
    for form in spec.command_forms {
        if !visible_in(form.surface, dialect) {
            continue;
        }
        let form_owner = format!("{registered_name} form {}", form.name);
        let form_context = vec![format_form(form)];
        collect_static(
            rows,
            dialect,
            &form_owner,
            "form",
            form.arg_roles,
            &[],
            &[],
            form.traits.unwrap_or(spec.traits),
            &format_lifecycle(spec.lifecycle),
            &command_provenance,
            &form_context,
        )?;
        collect_options(
            rows,
            dialect,
            &form_owner,
            "form-option",
            form.options,
            &format_lifecycle(spec.lifecycle),
            &command_provenance,
            &form_context,
        )?;
    }

    for sub in spec.subcommands {
        if !visible_in(sub.surface, dialect) {
            continue;
        }
        let sub_owner = format!("{registered_name} {}", sub.name);
        let source = provenance(sub.hover.map(|hover| hover.source), spec);
        let sub_forms = subcommand_forms(sub);
        collect_static(
            rows,
            dialect,
            &sub_owner,
            "subcommand",
            sub.arg_roles,
            sub.command_prefixes,
            sub.callback_taint_inputs,
            spec.traits | sub.traits,
            &combined_lifecycle(spec.lifecycle, sub.lifecycle),
            &source,
            &sub_forms,
        )?;
        collect_options(
            rows,
            dialect,
            &sub_owner,
            "subcommand-option",
            sub.options,
            &combined_lifecycle(spec.lifecycle, sub.lifecycle),
            &source,
            &sub_forms,
        )?;
        collect_dynamic(
            rows,
            dialect,
            &sub_owner,
            "subcommand",
            sub.arg_role_resolver.is_some(),
            sub.command_prefix_resolver.is_some(),
            sub.script_timing_resolver.is_some(),
            &combined_lifecycle(spec.lifecycle, sub.lifecycle),
            &source,
            &sub_forms,
        )?;
        for form in sub.subcommand_forms {
            if !visible_in(form.surface, dialect) {
                continue;
            }
            let form_owner = format!("{sub_owner} form {}", form.name);
            let form_context = vec![format_form(form)];
            let inherited_traits = spec.traits | sub.traits;
            collect_static(
                rows,
                dialect,
                &form_owner,
                "form",
                form.arg_roles,
                &[],
                &[],
                form.traits.unwrap_or(inherited_traits),
                &combined_lifecycle(spec.lifecycle, sub.lifecycle),
                &source,
                &form_context,
            )?;
            collect_options(
                rows,
                dialect,
                &form_owner,
                "form-option",
                form.options,
                &combined_lifecycle(spec.lifecycle, sub.lifecycle),
                &source,
                &form_context,
            )?;
        }
    }

    if let Some(class) = spec.object_class {
        for method in class.instance_methods {
            if !visible_in(method.surface, dialect) {
                continue;
            }
            let method_owner = format!("{registered_name} instance {}", method.name);
            let source = provenance(method.hover.map(|hover| hover.source), spec);
            let method_forms = subcommand_forms(method);
            collect_static(
                rows,
                dialect,
                &method_owner,
                "instance-method",
                method.arg_roles,
                method.command_prefixes,
                method.callback_taint_inputs,
                method.traits,
                &combined_lifecycle(spec.lifecycle, method.lifecycle),
                &source,
                &method_forms,
            )?;
            collect_options(
                rows,
                dialect,
                &method_owner,
                "instance-method-option",
                method.options,
                &combined_lifecycle(spec.lifecycle, method.lifecycle),
                &source,
                &method_forms,
            )?;
            collect_dynamic(
                rows,
                dialect,
                &method_owner,
                "instance-method",
                method.arg_role_resolver.is_some(),
                method.command_prefix_resolver.is_some(),
                method.script_timing_resolver.is_some(),
                &combined_lifecycle(spec.lifecycle, method.lifecycle),
                &source,
                &method_forms,
            )?;
            for form in method.subcommand_forms {
                if !visible_in(form.surface, dialect) {
                    continue;
                }
                let form_owner = format!("{method_owner} form {}", form.name);
                let form_context = vec![format_form(form)];
                collect_static(
                    rows,
                    dialect,
                    &form_owner,
                    "instance-method-form",
                    form.arg_roles,
                    &[],
                    &[],
                    form.traits.unwrap_or(method.traits),
                    &combined_lifecycle(spec.lifecycle, method.lifecycle),
                    &source,
                    &form_context,
                )?;
                collect_options(
                    rows,
                    dialect,
                    &form_owner,
                    "instance-method-form-option",
                    form.options,
                    &combined_lifecycle(spec.lifecycle, method.lifecycle),
                    &source,
                    &form_context,
                )?;
            }
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn collect_static(
    rows: &mut BTreeMap<String, InventoryRow>,
    dialect: &str,
    owner: &str,
    surface: &str,
    roles: &[(u8, ArgRole)],
    prefixes: &[(u8, AppendedArity)],
    taint: &[(u8, &'static [CallbackTaintInput])],
    traits: Traits,
    lifecycle: &str,
    provenance: &str,
    forms: &[String],
) -> Result<()> {
    for &(index, appended) in prefixes {
        let timing = if traits.contains(Traits::DEFERS_BODY) {
            Timing::Deferred
        } else {
            Timing::SameInvocation
        };
        insert_row(
            rows,
            dialect,
            row(
                owner,
                &format!("arg[{index}]"),
                surface,
                SurfaceKind::CommandPrefix,
                timing,
                lifecycle,
                provenance,
                Some(format_appended(appended)),
                taint_at(taint, index),
                forms,
                "static CommandSpec command-prefix position",
            ),
        )?;
    }
    for &(index, role) in roles {
        if role == ArgRole::CommandPrefix {
            if prefixes.iter().any(|(at, _)| *at == index) {
                continue;
            }
            let timing = if traits.contains(Traits::DEFERS_BODY) {
                Timing::Deferred
            } else {
                Timing::SameInvocation
            };
            insert_row(
                rows,
                dialect,
                row(
                    owner,
                    &format!("arg[{index}]"),
                    surface,
                    SurfaceKind::CommandPrefix,
                    timing,
                    lifecycle,
                    provenance,
                    Some("unknown".to_owned()),
                    taint_at(taint, index),
                    forms,
                    "CommandPrefix role without a more precise appended-arity table",
                ),
            )?;
            continue;
        }
        if !matches!(role, ArgRole::Body | ArgRole::LambdaLiteral) {
            continue;
        }
        let timing = if traits.contains(Traits::DEFERS_BODY) {
            Timing::Deferred
        } else {
            Timing::SameInvocation
        };
        insert_row(
            rows,
            dialect,
            row(
                owner,
                &format!("arg[{index}]"),
                surface,
                SurfaceKind::BodyScript,
                timing,
                lifecycle,
                provenance,
                None,
                taint_at(taint, index),
                forms,
                match role {
                    ArgRole::LambdaLiteral => "static anonymous-lambda literal position",
                    _ => "static script-body position",
                },
            ),
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn collect_options(
    rows: &mut BTreeMap<String, InventoryRow>,
    dialect: &str,
    owner: &str,
    surface: &str,
    options: &[OptionSpec],
    parent_lifecycle: &str,
    provenance: &str,
    forms: &[String],
) -> Result<()> {
    for option in options {
        if !visible_in(option.surface, dialect) {
            continue;
        }
        let Some((kind, timing, appended, taint)) = classify_option(option) else {
            continue;
        };
        insert_row(
            rows,
            dialect,
            row(
                owner,
                &format!("option {} value", option.name),
                surface,
                kind,
                timing,
                &combined_lifecycle_string(parent_lifecycle, option.lifecycle),
                provenance,
                appended,
                taint,
                forms,
                "OptionSpec value descriptor",
            ),
        )?;
    }
    Ok(())
}

fn classify_option(
    option: &OptionSpec,
) -> Option<(SurfaceKind, Timing, Option<String>, Vec<String>)> {
    let OptionValue::Takes(arg) = option.value else {
        return None;
    };
    let role = if arg.role.has_script_timing() {
        arg.role
    } else if arg.also_role.is_some_and(ArgRole::has_script_timing) {
        arg.also_role.expect("checked Some")
    } else {
        return None;
    };
    let timing = timing(arg.script_timing);
    let kind = if arg.script_timing == ScriptTiming::ReferenceOnly {
        SurfaceKind::ReferenceOnly
    } else if role == ArgRole::CommandPrefix {
        SurfaceKind::CommandPrefix
    } else {
        SurfaceKind::BodyScript
    };
    let appended = (role == ArgRole::CommandPrefix).then(|| format_appended(arg.appended_arity));
    let taint = arg
        .callback_taint_inputs
        .iter()
        .map(|input| input.spelling().to_owned())
        .collect();
    Some((kind, timing, appended, taint))
}

#[allow(clippy::too_many_arguments)]
fn collect_dynamic(
    rows: &mut BTreeMap<String, InventoryRow>,
    dialect: &str,
    owner: &str,
    surface: &str,
    arg_roles: bool,
    prefixes: bool,
    timing_resolver: bool,
    lifecycle: &str,
    provenance: &str,
    forms: &[String],
) -> Result<()> {
    for (suffix, present, note) in [
        (
            "dynamic-arg-role",
            arg_roles,
            "argument-role resolver; executable positions, if any, are invocation-dependent",
        ),
        (
            "dynamic-command-prefix",
            prefixes,
            "command-prefix resolver; position and appended arity are invocation-dependent",
        ),
        (
            "dynamic-script-timing",
            timing_resolver,
            "script-timing resolver; timing is invocation-dependent",
        ),
    ] {
        if !present {
            continue;
        }
        insert_row(
            rows,
            dialect,
            row(
                owner,
                suffix,
                surface,
                SurfaceKind::Dynamic,
                Timing::Dynamic,
                lifecycle,
                provenance,
                None,
                Vec::new(),
                forms,
                note,
            ),
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn row(
    owner: &str,
    location: &str,
    surface: &str,
    kind: SurfaceKind,
    timing: Timing,
    lifecycle: &str,
    provenance: &str,
    appended_arity: Option<String>,
    callback_taint_inputs: Vec<String>,
    forms: &[String],
    notes: &str,
) -> InventoryRow {
    InventoryRow {
        id: format!("{owner}/{location}"),
        owner: owner.to_owned(),
        surface: surface.to_owned(),
        kind,
        provenance: provenance.to_owned(),
        dialects: Vec::new(),
        lifecycle: lifecycle.to_owned(),
        timing,
        appended_arity,
        callback_taint_inputs,
        forms: forms.to_vec(),
        registry_derived: true,
        registry_owner: Some(owner.split(' ').next().unwrap_or(owner).to_owned()),
        notes: notes.to_owned(),
    }
}

fn insert_row(
    rows: &mut BTreeMap<String, InventoryRow>,
    dialect: &str,
    mut candidate: InventoryRow,
) -> Result<()> {
    // The serialization is an internal grouping key over every semantic
    // field except dialects. This merges only genuinely identical rows; a
    // lifecycle, source, form, timing, arity, or taint difference creates a
    // separate row and remains visible in the report.
    let key = serde_json::to_string(&candidate)?;
    if let Some(existing) = rows.get_mut(&key) {
        if !existing.dialects.iter().any(|item| item == dialect) {
            existing.dialects.push(dialect.to_owned());
            existing.dialects.sort();
        }
    } else {
        candidate.dialects.push(dialect.to_owned());
        rows.insert(key, candidate);
    }
    Ok(())
}

fn visible_in(surface: Option<&'static [SpecSurface]>, profile_name: &str) -> bool {
    // The resolved environment's document authoring point, through the seam
    // — the same point for every name this projection passes (the catalogue
    // ids plus `tk`).
    surface.is_none_or(|rows| {
        surface_admits(
            rows,
            Some(&crate::environment::surface_point_for_dialect(profile_name)),
        )
    })
}

fn provenance(source: Option<&str>, spec: &CommandSpec) -> String {
    if let Some(source) = source.filter(|source| !source.is_empty()) {
        return source.to_owned();
    }
    if let Some(package) = spec.tcllib_package {
        return format!("tcllib {package} manual/source (2.0 corpus)");
    }
    match spec.required_package {
        Some("Tk") => "Tk manual/source (8.4-9.0 corpus)".to_owned(),
        Some(package) => format!("{package} package documentation/source"),
        None => "Tcl manual/source (8.4-9.1 corpus)".to_owned(),
    }
}

fn command_forms(spec: &CommandSpec) -> Vec<String> {
    let mut forms: Vec<String> = spec
        .forms
        .iter()
        .map(|form| format!("{} [{}]", form.synopsis, format_lifecycle(form.lifecycle)))
        .collect();
    if forms.is_empty()
        && let Some(hover) = spec.hover
    {
        forms.extend(hover.synopsis.iter().map(|synopsis| (*synopsis).to_owned()));
    }
    forms.sort();
    forms.dedup();
    forms
}

fn subcommand_forms(sub: &SubCommand) -> Vec<String> {
    let mut forms = Vec::new();
    if !sub.synopsis.is_empty() {
        forms.push(sub.synopsis.to_owned());
    }
    if let Some(hover) = sub.hover {
        forms.extend(hover.synopsis.iter().map(|synopsis| (*synopsis).to_owned()));
    }
    forms.sort();
    forms.dedup();
    forms
}

fn format_form(form: &tcl_registry::forms::CommandForm) -> String {
    let selector = form.literal_argument_prefix.map_or_else(
        || "arity-selected".to_owned(),
        |prefix| format!("selector {}", prefix.words.join(" ")),
    );
    format!("{} ({selector})", form.name)
}

fn taint_at(table: &[(u8, &'static [CallbackTaintInput])], index: u8) -> Vec<String> {
    table
        .iter()
        .find_map(|(at, inputs)| (*at == index).then_some(*inputs))
        .unwrap_or_default()
        .iter()
        .map(|input| input.spelling().to_owned())
        .collect()
}

fn timing(value: ScriptTiming) -> Timing {
    match value {
        ScriptTiming::SameInvocation => Timing::SameInvocation,
        ScriptTiming::Deferred => Timing::Deferred,
        ScriptTiming::ReferenceOnly => Timing::ReferenceOnly,
    }
}

fn format_appended(value: AppendedArity) -> String {
    match value {
        AppendedArity::Exactly(n) => format!("exactly {n}"),
        AppendedArity::OneOf(set) => format!("one of {:?}", set.counts()),
        AppendedArity::AtLeast(n) => format!("at least {n}"),
        AppendedArity::Unknown => "unknown".to_owned(),
        _ => "unknown future arity kind".to_owned(),
    }
}

fn combined_lifecycle(parent: Lifecycle, child: Lifecycle) -> String {
    combined_lifecycle_string(&format_lifecycle(parent), child)
}

fn combined_lifecycle_string(parent: &str, child: Lifecycle) -> String {
    if child == Lifecycle::UNSPECIFIED {
        parent.to_owned()
    } else if parent == "all known owning-package versions" {
        format_lifecycle(child)
    } else {
        format!("owner ({parent}); surface ({})", format_lifecycle(child))
    }
}

fn format_lifecycle(value: Lifecycle) -> String {
    let mut parts = Vec::new();
    if let Some(version) = value.introduced {
        parts.push(format!("introduced {version}"));
    }
    if let Some(version) = value.deprecated {
        parts.push(format!("deprecated {version}"));
    }
    if let Some(version) = value.retired {
        parts.push(format!("retired {version}"));
    }
    if parts.is_empty() {
        "all known owning-package versions".to_owned()
    } else {
        parts.join("; ")
    }
}

fn validate_seed(seed: &[InventoryRow]) -> Result<()> {
    let owners: BTreeSet<&str> = tcl_dialect::DialectProfile::all()
        .iter()
        .chain(std::iter::once(crate::environment::profile_for_dialect(
            "tk",
        )))
        .flat_map(|profile| crate::environment::store_for_profile(profile).command_names())
        .collect();
    for row in seed {
        if row.registry_derived {
            bail!("seed row {} must not claim to be registry-derived", row.id);
        }
        let Some(owner) = row.registry_owner.as_deref() else {
            bail!(
                "seed row {} needs a registry_owner for stale-entry checking",
                row.id
            );
        };
        if !owners.contains(owner) {
            bail!("seed row {} names missing registry owner {owner}", row.id);
        }
        if row.provenance.trim().is_empty() {
            bail!("seed row {} has no provenance", row.id);
        }
    }
    Ok(())
}

fn reject_duplicate_ids(rows: &[InventoryRow]) -> Result<()> {
    let mut ids = BTreeSet::new();
    for row in rows {
        if !ids.insert(&row.id) {
            bail!("duplicate callback inventory id {}", row.id);
        }
    }
    Ok(())
}

fn render_markdown(rows: &[InventoryRow]) -> String {
    let mut out = String::from(
        "<!-- Generated by `cargo xtask callback-inventory`; do not edit. -->\n\n\
         # Executable and callback surface inventory\n\n\
         This report is a human-readable projection of [`callback-surfaces.json`](callback-surfaces.json). \
         Registry rows are derived from command, subcommand, option, form, and instance-method metadata; \
         audited non-structural facts come from the catalogue seed.\n\n\
         | Surface | Kind | Timing | Dialects | Appended arity | Callback taint inputs | Forms | Lifecycle | Provenance |\n\
         |---|---|---|---|---|---|---|---|---|\n",
    );
    for row in rows {
        let taint = if row.callback_taint_inputs.is_empty() {
            "—".to_owned()
        } else {
            row.callback_taint_inputs.join(", ")
        };
        let forms = if row.forms.is_empty() {
            "—".to_owned()
        } else {
            row.forms.join("<br>").replace('|', "\\|")
        };
        let _ = writeln!(
            out,
            "| `{}` | `{:?}` | `{:?}` | {} | {} | {} | {} | {} | {} |",
            row.id.replace('|', "\\|"),
            row.kind,
            row.timing,
            row.dialects.join(", "),
            row.appended_arity.as_deref().unwrap_or("—"),
            taint,
            forms,
            row.lifecycle,
            row.provenance.replace('|', "\\|")
        );
    }
    out
}

fn check_file(path: &std::path::Path, expected: &str) -> Result<()> {
    let actual = fs::read_to_string(path).unwrap_or_default();
    if actual != expected {
        bail!(
            "{} is stale — run `cargo xtask callback-inventory`",
            path.display()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn replacing_callback_option_with_plain_value_changes_inventory() {
        let callback = OptionSpec {
            name: "-command",
            value: OptionValue::command_prefix_n("callback", AppendedArity::Exactly(1)),
            ..OptionSpec::DEFAULT
        };
        let plain = OptionSpec {
            value: OptionValue::value("value"),
            ..callback.clone()
        };
        assert!(classify_option(&callback).is_some());
        assert!(classify_option(&plain).is_none());
    }

    #[test]
    fn seed_row_without_classification_is_rejected() {
        let json = r#"{
            "schema_version": 1,
            "rows": [{
                "id":"x", "owner":"x", "surface":"x",
                "provenance":"source", "dialects":[],
                "lifecycle":"all", "timing":"dynamic",
                "appended_arity":null, "callback_taint_inputs":[],
                "forms":[], "registry_derived":false,
                "registry_owner":"x", "notes":"x"
            }]
        }"#;
        assert!(serde_json::from_str::<Seed>(json).is_err());
    }
}

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

//! Generate the documentation and the AI prompt manifest that name the
//! selectable environments, from the compiled environment registry
//! ([`tcl_dialect::model::EnvironmentRegistry::compiled_selectable`]).
//!
//! - the README's two dialect tables (`Dialects` and `Tcl + packages`), with
//!   the file types, shebang words and aliases each environment answers to;
//! - `docs/generated/environments.md`, the full reference table;
//! - the dialect-selection KCS note's picker groups and detection lists;
//! - `ai/prompts/manifest.json`, which prompt fragments each environment
//!   reads. The rule that assigns a fragment is data over the environment
//!   (F5 core, Tk), so an environment added tomorrow reaches a prompt, and
//!   `--check` fails on a manifest that lists otherwise.
//!
//! Generated prose sits between `<generated: name>` and `</generated>` marker
//! lines written as HTML comments; the prose around a region is kept.
//!
//! Run `cargo xtask gen-environment-docs`; `--check` makes every output a
//! drift gate.

use std::fmt::Write as _;
use std::fs;
use std::process::ExitCode;
use std::sync::Arc;

use anyhow::{Context, Result, bail};
use tcl_dialect::model::{
    DEFAULT_ENVIRONMENT_ID, EnvironmentDefinition, EnvironmentKind, Family, LENIENT_ENVIRONMENT_ID,
    Provenance,
};

use crate::editor_extensions::language_of_environment;
use crate::gen_editor_configs::{Model, Registration, code_sentence_list, wrap_text};
use crate::util::{replace_generated_region, repo_root};

const README: &str = "README.md";
const KCS_DIALECT_SELECTION: &str = "docs/kcs/features/kcs-feature-dialect-selection.md";
const REFERENCE: &str = "docs/generated/environments.md";
const PROMPT_MANIFEST: &str = "ai/prompts/manifest.json";

/// One prompt fragment and the environments that read it.
struct PromptFragment {
    file: &'static str,
    reads: fn(&EnvironmentDefinition) -> bool,
}

/// Whether the environment is hosted on an F5 core: the iRules offshoot, the
/// BIG-IP Tcl trunk under iApps and tmsh, or the BIG-IP configuration surface
/// (the one environment with no Tcl core).
fn on_an_f5_core(environment: &EnvironmentDefinition) -> bool {
    environment
        .core
        .is_none_or(|core| matches!(core.family, Family::F5Tcl | Family::F5Irules))
}

/// Whether the environment is a Tcl release or shell that places Tk: the
/// release ladder hosts it, and the `tk` shell has it ambiently.
fn has_tk(environment: &EnvironmentDefinition) -> bool {
    environment
        .core
        .is_some_and(|core| core.family == Family::Tcl)
        && environment
            .expected_packages
            .iter()
            .any(|placement| placement.package.as_ref() == "Tk")
}

/// The prompt fragments, in the order the manifest lists them. Every
/// selectable environment reads at least one: the Tcl prompt is the answer
/// for everything that is not on an F5 core.
const PROMPTS: &[PromptFragment] = &[
    PromptFragment {
        file: "irules_system.md",
        reads: on_an_f5_core,
    },
    PromptFragment {
        file: "tcl_system.md",
        reads: |environment| !on_an_f5_core(environment),
    },
    PromptFragment {
        file: "tk_system.md",
        reads: has_tk,
    },
];

fn prompt_manifest(model: &Model) -> String {
    let mut out = String::from("{\n  \"prompts\": [\n");
    for (index, fragment) in PROMPTS.iter().enumerate() {
        let ids: Vec<String> = model
            .environments
            .iter()
            .filter(|environment| (fragment.reads)(environment))
            .map(|environment| format!("\"{}\"", environment.id))
            .collect();
        let comma = if index + 1 == PROMPTS.len() { "" } else { "," };
        let _ = write!(
            out,
            "    {{\n      \"file\": \"{}\",\n      \"dialects\": [{}]\n    }}{comma}\n",
            fragment.file,
            ids.join(", ")
        );
    }
    out.push_str("  ]\n}\n");
    out
}

/// Inline code for each of `items`, comma separated.
fn code_join<I, S>(items: I) -> String
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    items
        .into_iter()
        .map(|item| format!("`{}`", item.as_ref()))
        .collect::<Vec<_>>()
        .join(", ")
}

fn cell(text: &str) -> String {
    text.replace('|', "\\|")
}

/// The aliases an environment also answers to, or `none`.
fn aliases_of(environment: &EnvironmentDefinition) -> String {
    if environment.aliases.is_empty() {
        "none".to_owned()
    } else {
        code_join(environment.aliases.iter().map(AsRef::as_ref))
    }
}

/// `label` with an `s` when there is more than one of `items`.
fn labelled(label: &str, items: &[String]) -> String {
    let plural = if items.len() == 1 { "" } else { "s" };
    format!("{label}{plural} {}", code_join(items))
}

/// What detection keys the environment on, or `none`.
fn detected_from(registration: &Registration) -> String {
    let mut parts = Vec::new();
    if !registration.extensions.is_empty() {
        let extensions: Vec<String> = registration
            .extensions
            .iter()
            .map(|ext| format!(".{ext}"))
            .collect();
        parts.push(labelled("extension", &extensions));
    }
    if !registration.filenames.is_empty() {
        parts.push(labelled("file name", &registration.filenames));
    }
    if !registration.shebang_words.is_empty() {
        parts.push(labelled("shebang", &registration.shebang_words));
    }
    if parts.is_empty() {
        "none".to_owned()
    } else {
        parts.join("; ")
    }
}

/// The Tcl release and packages of a `Packages` environment: its description
/// without the leading display name.
fn tcl_and_packages(environment: &EnvironmentDefinition) -> String {
    let description = environment.description();
    description
        .strip_prefix(&format!("{} — ", environment.display_name))
        .map_or(description.clone(), str::to_owned)
}

fn of_kind(
    model: &Model,
    kind: EnvironmentKind,
) -> impl Iterator<Item = &Arc<EnvironmentDefinition>> {
    model
        .environments
        .iter()
        .filter(move |environment| environment.kind == kind)
}

fn dialect_cell(environment: &EnvironmentDefinition) -> String {
    if environment.id.as_str() == DEFAULT_ENVIRONMENT_ID {
        format!("`{}` (default)", environment.id)
    } else {
        format!("`{}`", environment.id)
    }
}

/// The two README tables and the sentence about plain Tcl files.
fn readme_tables(model: &Model) -> Result<String> {
    let mut out = String::from("**Dialects**\n\n");
    out.push_str("| Dialect | Name | Also accepted as | Detected from |\n|---|---|---|---|\n");
    for environment in of_kind(model, EnvironmentKind::Language) {
        let registration = model.registration(environment);
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} |",
            dialect_cell(environment),
            cell(&environment.display_name),
            aliases_of(environment),
            detected_from(&registration)
        );
    }
    out.push_str("\n**Tcl + packages**\n\n");
    out.push_str(
        "| Dialect | Name | Tcl and packages | Also accepted as | Detected from |\n|---|---|---|---|---|\n",
    );
    for environment in of_kind(model, EnvironmentKind::Packages) {
        let registration = model.registration(environment);
        let _ = writeln!(
            out,
            "| {} | {} | {} | {} | {} |",
            dialect_cell(environment),
            cell(&environment.display_name),
            cell(&tcl_and_packages(environment)),
            aliases_of(environment),
            detected_from(&registration)
        );
    }
    let plain = model
        .langs
        .iter()
        .find(|lang| lang.id == "tcl")
        .context("no plain tcl language")?;
    let extensions: Vec<String> = plain
        .extensions
        .iter()
        .map(|ext| format!(".{ext}"))
        .collect();
    out.push('\n');
    out.push_str(&wrap_text(&format!(
        "Files with the extensions {} name no dialect of their own. Detection, a \
         `# tcl-dialect:` comment, or the `tclLsp.dialect` setting chooses one. \
         [docs/generated/environments.md](docs/generated/environments.md) lists every \
         environment with its core, packages, and language id.",
        code_sentence_list(&extensions)
    )));
    Ok(out)
}

fn render_readme(original: &str, model: &Model) -> Result<String> {
    replace_generated_region(original, "environment-tables", &readme_tables(model)?)
}

fn provenance_word(provenance: Provenance) -> &'static str {
    match provenance {
        Provenance::BuiltIn => "built in",
        Provenance::BundledPack => "bundled pack",
        _ => "other",
    }
}

fn reference_row(model: &Model, environment: &EnvironmentDefinition) -> String {
    let registration = model.registration(environment);
    let core = environment.core.map_or_else(
        || "none".to_owned(),
        |core| {
            let family = match core.family {
                Family::Tcl => "Tcl",
                other => other.name(),
            };
            format!("{family} {}", core.default_release)
        },
    );
    let ambient: Vec<&str> = environment
        .expected_packages
        .iter()
        .filter(|placement| placement.ambient)
        .map(|placement| placement.package.as_ref())
        .collect();
    let optional = |items: Vec<String>| {
        if items.is_empty() {
            "none".to_owned()
        } else {
            code_join(items)
        }
    };
    let language_id = language_of_environment(&model.langs, environment.id.as_str())
        .map_or_else(|| "none".to_owned(), |lang| format!("`{}`", lang.id));
    format!(
        "| `{}` | {} | {} | {core} | {} | {} | {language_id} | {} | {} | {} | {} |\n",
        environment.id,
        cell(&environment.display_name),
        cell(&environment.short_name),
        optional(ambient.iter().map(|name| (*name).to_owned()).collect()),
        aliases_of(environment),
        optional(
            registration
                .extensions
                .iter()
                .map(|ext| format!(".{ext}"))
                .collect()
        ),
        optional(registration.filenames.clone()),
        optional(registration.shebang_words.clone()),
        provenance_word(environment.provenance),
    )
}

fn reference_table(model: &Model, kind: EnvironmentKind) -> String {
    let mut out = String::from(
        "| Id | Name | Short name | Core | Ambient packages | Also accepted as | Language id | Extensions | File names | Shebang words | Source |\n\
         |---|---|---|---|---|---|---|---|---|---|---|\n",
    );
    for environment in of_kind(model, kind) {
        out.push_str(&reference_row(model, environment));
    }
    out
}

fn render_reference(model: &Model) -> String {
    let mut out = String::from(
        "# Environments\n\n\
         > Auto-generated by `cargo xtask gen-environment-docs`. Do not edit by hand.\n\
         > The gate (`make xtask-check`) fails on drift.\n\n",
    );
    out.push_str(&wrap_text(&format!(
        "Each environment here is a value that `tclLsp.dialect`, `# tcl-dialect:`, and \
         `--dialect` accept. Source of truth: the compiled environment registry. A \
         *Dialects* environment is a language or a Tcl release; a *Tcl + packages* environment \
         is a Tcl release with library packages loaded. The default is `{DEFAULT_ENVIRONMENT_ID}`. \
         `{LENIENT_ENVIRONMENT_ID}` is the lenient environment every unknown, unstated, or plain \
         `{LENIENT_ENVIRONMENT_ID}` name resolves to; it is a fallback rather than a choice, so \
         it has no row.",
    )));
    out.push_str("\n## Dialects\n\n");
    out.push_str(&reference_table(model, EnvironmentKind::Language));
    out.push_str("\n## Tcl + packages\n\n");
    out.push_str(&reference_table(model, EnvironmentKind::Packages));
    out
}

/// The picker groups, as the nested bullets under the VS Code entry.
fn kcs_picker_groups(model: &Model) -> String {
    let names = |kind| {
        of_kind(model, kind)
            .map(|environment| environment.id.as_str())
            .collect::<Vec<_>>()
    };
    format!(
        "  - **Dialects**: {}.\n  - **Tcl + packages**: {}.\n",
        code_join(names(EnvironmentKind::Language)),
        code_join(names(EnvironmentKind::Packages)),
    )
}

/// What each environment owns, as the nested bullets under the automatic
/// entry: extensions and file names, shebang words, and the environments
/// with neither.
fn kcs_detection(model: &Model) -> String {
    let mut owned = Vec::new();
    let mut shebangs = Vec::new();
    let mut neither = Vec::new();
    for environment in model.environments {
        let registration = model.registration(environment);
        let mut names: Vec<String> = registration
            .extensions
            .iter()
            .map(|ext| format!("`.{ext}`"))
            .collect();
        names.extend(
            registration
                .filenames
                .iter()
                .map(|name| format!("`{name}`")),
        );
        if !names.is_empty() {
            owned.push(format!("{} → {}", names.join("/"), environment.id));
        }
        if !registration.shebang_words.is_empty() {
            shebangs.push(format!(
                "{} → {}",
                code_join(&registration.shebang_words).replace(", ", "/"),
                environment.id
            ));
        }
        if names.is_empty() && registration.shebang_words.is_empty() {
            neither.push(environment.id.as_str().to_owned());
        }
    }
    let mut out = format!(
        "  - **Extensions and file names**: {}.\n  - **Shebang line**: {}.\n",
        owned.join("; "),
        shebangs.join("; ")
    );
    if !neither.is_empty() {
        let _ = writeln!(
            out,
            "  - **No automatic route**: {} own no extension, file name, or shebang word. Pick \
             them with the setting or a `# tcl-dialect:` comment.",
            code_sentence_list(&neither)
        );
    }
    out
}

fn render_kcs(original: &str, model: &Model) -> Result<String> {
    let text = replace_generated_region(original, "kcs-dialect-groups", &kcs_picker_groups(model))?;
    replace_generated_region(&text, "kcs-dialect-detection", &kcs_detection(model))
}

type RenderText = fn(&str, &Model) -> Result<String>;

/// The files this generator edits in place, each with the render that
/// rebuilds its regions.
const EDITED: &[(&str, RenderText)] =
    &[(README, render_readme), (KCS_DIALECT_SELECTION, render_kcs)];

/// Every file this generator owns, with the content it must hold.
fn artifacts(model: &Model) -> Result<Vec<(&'static str, String)>> {
    let root = repo_root();
    let mut out = Vec::new();
    for &(rel, render) in EDITED {
        let path = root.join(rel);
        let original =
            fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
        out.push((
            rel,
            render(&original, model).with_context(|| format!("rendering {rel}"))?,
        ));
    }
    out.push((REFERENCE, render_reference(model)));
    out.push((PROMPT_MANIFEST, prompt_manifest(model)));
    Ok(out)
}

pub fn run(check: bool) -> Result<ExitCode> {
    let root = repo_root();
    let model = Model::load()?;
    let mut drift = Vec::new();
    for (rel, content) in artifacts(&model)? {
        let path = root.join(rel);
        let current = fs::read_to_string(&path).unwrap_or_default();
        if current == content {
            continue;
        }
        if check {
            drift.push(rel);
        } else {
            crate::util::write_if_changed(&path, &content)?;
            eprintln!("wrote {rel}");
        }
    }
    if !drift.is_empty() {
        eprintln!(
            "{} environment documentation file(s) are stale — run `cargo xtask gen-environment-docs`:",
            drift.len()
        );
        for rel in drift {
            eprintln!("  - {rel}");
        }
        return Ok(ExitCode::from(1));
    }
    if check {
        let gaps = prompt_gaps(&model);
        if !gaps.is_empty() {
            bail!("no prompt fragment reads {gaps:?}");
        }
        eprintln!("OK: environment documentation and the prompt manifest match the registry.");
    }
    Ok(ExitCode::SUCCESS)
}

/// The selectable environments no prompt fragment reads.
fn prompt_gaps(model: &Model) -> Vec<String> {
    model
        .environments
        .iter()
        .filter(|environment| !PROMPTS.iter().any(|fragment| (fragment.reads)(environment)))
        .map(|environment| environment.id.to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn committed(rel: &str) -> String {
        fs::read_to_string(repo_root().join(rel)).unwrap_or_else(|e| panic!("reading {rel}: {e}"))
    }

    fn fragments_read_by(model: &Model, id: &str) -> Vec<&'static str> {
        let environment = model
            .environments
            .iter()
            .find(|environment| environment.id.as_str() == id)
            .unwrap_or_else(|| panic!("{id} is not selectable"));
        PROMPTS
            .iter()
            .filter(|fragment| (fragment.reads)(environment))
            .map(|fragment| fragment.file)
            .collect()
    }

    #[test]
    fn committed_files_match_the_generated_output() {
        let model = Model::load().unwrap();
        for (rel, content) in artifacts(&model).unwrap() {
            assert_eq!(committed(rel), content, "{rel} is stale");
        }
    }

    #[test]
    fn every_selectable_environment_reads_a_prompt_fragment() {
        let model = Model::load().unwrap();
        assert_eq!(prompt_gaps(&model), Vec::<String>::new());
        // The generated prompts live under `ai/prompts`; the Tk prompt is
        // static and lives beside the skills.
        for fragment in PROMPTS {
            let root = repo_root();
            assert!(
                root.join("ai/prompts").join(fragment.file).exists()
                    || root
                        .join("ai/claude/skills/_prompts")
                        .join(fragment.file)
                        .exists(),
                "no prompt file {} exists",
                fragment.file
            );
        }
    }

    #[test]
    fn prompts_follow_the_core_and_the_packages() {
        let model = Model::load().unwrap();
        assert_eq!(fragments_read_by(&model, "f5-irules"), ["irules_system.md"]);
        assert_eq!(fragments_read_by(&model, "f5-bigip"), ["irules_system.md"]);
        assert_eq!(fragments_read_by(&model, "jim"), ["tcl_system.md"]);
        assert_eq!(
            fragments_read_by(&model, "tk"),
            ["tcl_system.md", "tk_system.md"]
        );
        assert_eq!(
            fragments_read_by(&model, "tcl8.6"),
            ["tcl_system.md", "tk_system.md"]
        );
        assert_eq!(
            fragments_read_by(&model, "xilinx-eda-tcl"),
            ["tcl_system.md"]
        );
    }

    #[test]
    fn the_manifest_names_every_selectable_environment() {
        let manifest = prompt_manifest(&Model::load().unwrap());
        let value: serde_json::Value = serde_json::from_str(&manifest).unwrap();
        let listed: Vec<&str> = value["prompts"]
            .as_array()
            .unwrap()
            .iter()
            .flat_map(|entry| entry["dialects"].as_array().unwrap())
            .map(|id| id.as_str().unwrap())
            .collect();
        for id in ["jim", "tk", "tcl8.6", "f5-irules", "xilinx-eda-tcl"] {
            assert!(listed.contains(&id), "{id} reaches no fragment");
        }
    }

    /// A render must repair a broken region, not merely reproduce a file that
    /// already matches.
    #[test]
    fn every_declared_region_is_written_by_its_render() {
        let model = Model::load().unwrap();
        let mut seen = 0;
        for &(rel, render) in EDITED {
            let original = committed(rel);
            let names: Vec<String> = original
                .lines()
                .filter_map(|line| {
                    let start = line.find("<generated: ")? + "<generated: ".len();
                    let end = line[start..].find('>')? + start;
                    Some(line[start..end].to_owned())
                })
                .collect();
            for name in names {
                let blanked = replace_generated_region(&original, &name, "").unwrap();
                assert_eq!(
                    render(&blanked, &model).unwrap(),
                    original,
                    "{rel}: region {name} is not restored"
                );
                seen += 1;
            }
        }
        assert_eq!(seen, 3, "the edited files declare 3 regions");
    }

    #[test]
    fn the_readme_tables_show_jim_and_the_tool_shells_apart() {
        let model = Model::load().unwrap();
        let tables = readme_tables(&model).unwrap();
        let (dialects, packages) = tables.split_once("**Tcl + packages**").unwrap();
        assert!(dialects.contains("| `jim` | Jim Tcl |"), "{dialects}");
        assert!(dialects.contains("shebang `jimsh`"), "{dialects}");
        assert!(dialects.contains("extension `.tmsh`"), "{dialects}");
        assert!(
            dialects.contains("extensions `.exp`, `.expect`"),
            "{dialects}"
        );
        assert!(dialects.contains("`tcl8.6` (default)"), "{dialects}");
        assert!(
            packages.contains("| `xilinx-eda-tcl` | Xilinx Vivado | Tcl 8.5 + vivado, sdc, upf |"),
            "{packages}"
        );
        assert!(!packages.contains("`jim`"));
    }
}

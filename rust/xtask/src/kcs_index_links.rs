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

//! KCS / design-docs link + index-coverage check.
//!
//! Validates five things, producing the expected stdout and exit
//! codes:
//!
//! 1. every local markdown link and its Markdown heading fragment in
//!    `docs/` (+ `CONTRIBUTING.md` / `AGENTS.md`) resolves;
//! 2. every design doc under `docs/design/` is linked from the design
//!    index (or a parent `README.md` that is itself indexed);
//! 3. every top-level KCS note under `docs/kcs/` is linked from the KCS
//!    index (and the `features/` subindex);
//! 4. every `kcs-*.md` note carries a `> **Audience:**` header (warning
//!    only — does not fail the build).
//! 5. every tagged diagnostic KCS page uses only the controlled Applies-to
//!    vocabulary.
//!
//! Exit 1 if any of checks 1–3 fail; otherwise exit 0.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use regex::Regex;

use crate::util::repo_root;

/// Applies-to tags accepted on diagnostic KCS pages. Keep this in lockstep
/// with the tables in `docs/kcs/STYLE.md` and the short vocabulary in
/// `AGENTS.md`. Diagnostic pages are deliberately checked here rather than
/// relying on the CLI help builder, which only indexes feature pages and
/// historically accepted arbitrary tokens.
const DIAGNOSTIC_KCS_TAGS: &[&str] = &[
    "all-editors",
    "analyser",
    "cfg",
    "command-walk",
    "const-fold",
    "dataflow",
    "dce",
    "diagnostic",
    "lexing",
    "liveness",
    "mcp",
    "naming",
    "lowering",
    "sccp",
    "ssa",
    "shimmer",
    "taint",
    "tcl-lsp-cli",
    "tcloo",
    "tclpkg",
];

/// Compiler pass that owns the emission of the diagnostic. Keep this table
/// keyed by code rather than accepting any globally-valid pass tag: a page
/// must describe the pass that actually produces its diagnostic.
const EXPECTED_DIAGNOSTIC_STAGES: &[(&str, &str)] = &[
    ("IRULE4004", "cfg"),
    ("IRULE4005", "ssa"),
    ("IRULE5002", "lowering"),
    ("IRULE5004", "lowering"),
];

/// Run the docs link/index check.
pub fn run() -> Result<ExitCode> {
    let root = repo_root();
    let docs = root.join("docs");
    let link_re = Regex::new(r"\[[^\]]+\]\(([^)]+)\)").expect("static regex is valid");
    let audience_re = Regex::new(r"(?m)^>\s*\*\*Audience:\*\*").expect("static regex is valid");

    let mut problems = Vec::new();
    problems.extend(check_local_markdown_links(&root, &docs, &link_re)?);
    problems.extend(check_design_index_coverage(&docs, &link_re)?);
    problems.extend(check_kcs_index_coverage(&docs, &link_re)?);
    problems.extend(check_diagnostic_kcs_tags(&root, &docs)?);
    let warnings = check_kcs_audience_headers(&root, &docs, &audience_re)?;

    if !problems.is_empty() {
        println!("KCS docs checks failed:");
        for problem in &problems {
            println!("- {problem}");
        }
        if !warnings.is_empty() {
            println!();
            println!("(plus {} audience-header warnings)", warnings.len());
        }
        return Ok(ExitCode::from(1));
    }

    if !warnings.is_empty() {
        println!("KCS docs checks passed with {} warnings:", warnings.len());
        for warning in &warnings {
            println!("- {warning}");
        }
    }
    println!("KCS docs checks passed.");
    Ok(ExitCode::SUCCESS)
}

/// Remove fenced code blocks (triple-backtick and tilde fences) so links
/// inside code examples are not validated. An unterminated fence extends
/// to EOF.
fn strip_fenced_code(text: &str) -> String {
    let mut out = String::new();
    let mut in_fence = false;
    let mut marker = "";
    for line in text.split_inclusive('\n') {
        let stripped = line.trim_start();
        if in_fence {
            if stripped.starts_with(marker) {
                in_fence = false;
                marker = "";
            }
            // Drop the line either way (the fence body and its closer).
        } else if stripped.starts_with("```") || stripped.starts_with("~~~") {
            in_fence = true;
            marker = if stripped.starts_with("```") {
                "```"
            } else {
                "~~~"
            };
        } else {
            out.push_str(line);
        }
    }
    out
}

/// The set of link targets in a markdown file, each truncated at the
/// first `#` anchor. Missing files yield an empty set.
fn extract_link_targets(link_re: &Regex, md_path: &Path) -> Result<BTreeSet<String>> {
    if !md_path.exists() {
        return Ok(BTreeSet::new());
    }
    let text = strip_fenced_code(&read(md_path)?);
    let mut targets = BTreeSet::new();
    for caps in link_re.captures_iter(&text) {
        targets.insert(link_before_anchor(&caps[1]).to_owned());
    }
    Ok(targets)
}

/// Check 1 — every local markdown link resolves.
fn check_local_markdown_links(root: &Path, docs: &Path, link_re: &Regex) -> Result<Vec<String>> {
    let mut files = Vec::new();
    collect_md_recursive(docs, &mut files)?;
    files.sort();
    files.push(root.join("CONTRIBUTING.md"));
    files.push(root.join("AGENTS.md"));

    let mut problems = Vec::new();
    for file in &files {
        // docs/archive/ holds historical snapshots whose links are
        // expected to rot; skip any path with an `archive` segment.
        if file.components().any(|c| c.as_os_str() == "archive") {
            continue;
        }
        if !file.exists() {
            continue;
        }
        let parent = file.parent().unwrap_or(root);
        let text = strip_fenced_code(&read(file)?);
        for caps in link_re.captures_iter(&text) {
            let link = &caps[1];
            if is_external_link(link) {
                continue;
            }
            let (link_path, fragment) = split_link_fragment(link);
            let target = if link_path.is_empty() {
                file.clone()
            } else {
                parent.join(link_path)
            };
            if !target.exists() {
                let rel = rel_to(root, file);
                problems.push(format!("broken link in {rel}: {link}"));
                continue;
            }
            if let Some(fragment) = fragment.filter(|fragment| !is_template_placeholder(fragment))
                && target
                    .extension()
                    .is_some_and(|extension| extension == "md")
            {
                let headings = heading_slugs(&read(&target)?);
                if !headings.contains(fragment) {
                    let rel = rel_to(root, file);
                    problems.push(format!("broken fragment in {rel}: {link}"));
                }
            }
        }
    }
    Ok(problems)
}

/// Whether a link points outside the local Markdown tree.
fn is_external_link(link: &str) -> bool {
    link.starts_with("http://")
        || link.starts_with("https://")
        || link.starts_with("mailto:")
        || link.starts_with("//")
}

/// Split a Markdown destination into the path and an optional heading
/// fragment. An empty path denotes an intra-file link.
fn split_link_fragment(link: &str) -> (&str, Option<&str>) {
    match link.split_once('#') {
        Some((path, fragment)) => (path, Some(fragment)),
        None => (link, None),
    }
}

/// Unfilled KCS templates intentionally point at a placeholder heading. They
/// are instructions to an author, not a broken link in published docs.
fn is_template_placeholder(fragment: &str) -> bool {
    fragment.contains('<') && fragment.contains('>')
}

/// Return the GitHub-style heading slugs in `text`, excluding fenced code.
///
/// GitHub's heading ids lowercase the visible inline-Markdown text, discard
/// punctuation other than spaces, hyphens, and underscores, turn spaces into
/// hyphens, and suffix duplicates with `-1`, `-2`, and so on.
fn heading_slugs(text: &str) -> BTreeSet<String> {
    let text = strip_fenced_code(text);
    let mut used = BTreeSet::new();
    let mut duplicates = BTreeMap::<String, u32>::new();

    for line in text.lines() {
        let Some(heading) = atx_heading_text(line) else {
            continue;
        };
        let (heading, explicit_id) = split_explicit_heading_id(heading);
        if let Some(explicit_id) = explicit_id {
            used.insert(explicit_id.to_owned());
        }
        let base = github_heading_slug(heading);
        if base.is_empty() {
            continue;
        }
        let count = duplicates.entry(base.clone()).or_insert(0);
        let mut suffix = *count;
        loop {
            let slug = if suffix == 0 {
                base.clone()
            } else {
                format!("{base}-{suffix}")
            };
            *count = suffix + 1;
            if used.insert(slug) {
                break;
            }
            suffix += 1;
        }
    }
    used
}

/// Split the Pandoc-style `{#id}` syntax used by a few generated reference
/// pages. The target is an explicit author-provided anchor rather than a
/// GitHub-generated slug, so it is admitted alongside the normal heading id.
fn split_explicit_heading_id(heading: &str) -> (&str, Option<&str>) {
    let Some((visible, id)) = heading.rsplit_once(" {#") else {
        return (heading, None);
    };
    let Some(id) = id.strip_suffix('}') else {
        return (heading, None);
    };
    if id.is_empty() {
        (heading, None)
    } else {
        (visible, Some(id))
    }
}

/// Extract the visible text of an ATX heading. Setext headings are not used
/// in the repository, while GitHub-generated KCS/design anchors are all ATX.
fn atx_heading_text(line: &str) -> Option<&str> {
    let line = line.trim_start();
    let hashes = line.chars().take_while(|ch| *ch == '#').count();
    if !(1..=6).contains(&hashes) || !line[hashes..].starts_with(char::is_whitespace) {
        return None;
    }
    Some(line[hashes..].trim().trim_end_matches('#').trim_end())
}

/// Convert visible heading Markdown to the GitHub heading-id shape used by
/// this repository. Inline links contribute their label, while ordinary
/// formatting markers disappear as punctuation during slugging.
fn github_heading_slug(heading: &str) -> String {
    let visible = strip_inline_markdown(heading);
    visible
        .chars()
        .filter_map(|ch| {
            if ch.is_alphanumeric() || matches!(ch, ' ' | '-' | '_') {
                Some(ch.to_ascii_lowercase())
            } else if ch.is_whitespace() {
                Some(' ')
            } else {
                None
            }
        })
        .collect::<String>()
        .replace(' ', "-")
}

/// Remove the destination from inline links/images, preserving their visible
/// label. The small scanner intentionally handles nested brackets, which is
/// enough for headings without imposing a second Markdown parser dependency.
fn strip_inline_markdown(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    let mut out = String::new();
    let mut index = 0;
    while index < chars.len() {
        let is_image = chars[index] == '!' && chars.get(index + 1) == Some(&'[');
        let starts_link = chars[index] == '[' || is_image;
        let open = index + usize::from(is_image);
        if starts_link && chars.get(open) == Some(&'[') {
            let mut depth = 1;
            let mut close = open + 1;
            while close < chars.len() && depth > 0 {
                match chars[close] {
                    '[' => depth += 1,
                    ']' => depth -= 1,
                    _ => {}
                }
                close += 1;
            }
            if depth == 0 {
                out.push_str(&strip_inline_markdown(
                    &chars[open + 1..close - 1].iter().collect::<String>(),
                ));
                if chars.get(close) == Some(&'(') {
                    let mut parens = 1;
                    close += 1;
                    while close < chars.len() && parens > 0 {
                        match chars[close] {
                            '(' => parens += 1,
                            ')' => parens -= 1,
                            _ => {}
                        }
                        close += 1;
                    }
                }
                index = close;
                continue;
            }
        }
        out.push(chars[index]);
        index += 1;
    }
    out
}

/// Check 2 — every design doc is reachable from the design index.
fn check_design_index_coverage(docs: &Path, link_re: &Regex) -> Result<Vec<String>> {
    let design_dir = docs.join("design");
    if !design_dir.exists() {
        return Ok(Vec::new());
    }
    let top_targets = extract_link_targets(link_re, &design_dir.join("README.md"))?;

    let mut notes = Vec::new();
    collect_md_recursive(&design_dir, &mut notes)?;
    notes.sort();

    let mut problems = Vec::new();
    for note in &notes {
        if note.file_name().is_some_and(|n| n == "README.md") {
            continue;
        }
        let rel_str = rel_to(&design_dir, note);
        if top_targets.contains(&rel_str) {
            continue;
        }
        // Accept a link from a parent-directory README that is itself
        // indexed at the top level.
        if let Some(parent) = note.parent() {
            let parent_readme = parent.join("README.md");
            let parent_rel = rel_to(&design_dir, &parent_readme);
            if parent_readme.exists() && top_targets.contains(&parent_rel) {
                let parent_targets = extract_link_targets(link_re, &parent_readme)?;
                if note
                    .file_name()
                    .is_some_and(|n| parent_targets.contains(&n.to_string_lossy().into_owned()))
                {
                    continue;
                }
            }
        }
        problems.push(format!(
            "design index missing link to docs/design/{rel_str}"
        ));
    }
    Ok(problems)
}

/// Check 3 — every top-level KCS note is reachable from the KCS index.
fn check_kcs_index_coverage(docs: &Path, link_re: &Regex) -> Result<Vec<String>> {
    const SKIP_NAMES: [&str; 2] = ["README.md", "STYLE.md"];

    let kcs_dir = docs.join("kcs");
    if !kcs_dir.exists() {
        return Ok(Vec::new());
    }
    let top_targets = extract_link_targets(link_re, &kcs_dir.join("README.md"))?;

    let mut problems = Vec::new();
    let mut notes = list_md_depth1(&kcs_dir)?;
    notes.sort();
    for note in &notes {
        let name = file_name(note);
        if SKIP_NAMES.contains(&name.as_str()) || top_targets.contains(&name) {
            continue;
        }
        problems.push(format!("KCS index missing link to docs/kcs/{name}"));
    }

    let features_dir = kcs_dir.join("features");
    if features_dir.exists() {
        let features_targets = extract_link_targets(link_re, &features_dir.join("README.md"))?;
        let mut feature_notes = list_md_depth1(&features_dir)?;
        feature_notes.sort();
        for note in &feature_notes {
            let name = file_name(note);
            if name == "README.md" || features_targets.contains(&name) {
                continue;
            }
            problems.push(format!(
                "features index missing link to docs/kcs/features/{name}"
            ));
        }
    }
    Ok(problems)
}

/// Check 4 — tagged diagnostic KCS pages use controlled tags. This prevents a
/// misspelt tag, or the obsolete `lowering` tag copied onto a command-walk
/// diagnostic, from silently becoming another help-index facet.
fn check_diagnostic_kcs_tags(root: &Path, docs: &Path) -> Result<Vec<String>> {
    let codes_dir = docs.join("kcs/codes");
    if !codes_dir.exists() {
        return Ok(Vec::new());
    }

    let mut notes = list_md_depth1(&codes_dir)?;
    notes.retain(|note| file_name(note).starts_with("kcs-diagnostic-"));
    notes.sort();

    let mut problems = Vec::new();
    for note in notes {
        let rel = rel_to(root, &note);
        let Some(tags) = applies_to_tags(&read(&note)?) else {
            // The ordinary KCS style check predates this tag gate and has
            // existing pages without an Applies-to heading. Do not turn this
            // targeted vocabulary guard into an unrelated documentation
            // migration; whenever a page does declare tags, they are strict.
            continue;
        };
        for tag in &tags {
            if !DIAGNOSTIC_KCS_TAGS.contains(&tag.as_str()) {
                problems.push(format!("unknown diagnostic KCS tag `{tag}` in {rel}"));
            }
        }
        if let Some(code) = diagnostic_code(&note)
            && let Some(expected) = EXPECTED_DIAGNOSTIC_STAGES
                .iter()
                .find_map(|(known, stage)| (*known == code).then_some(*stage))
            && !tags.iter().any(|tag| tag == expected)
        {
            problems.push(format!(
                "diagnostic {code} must use `{expected}` stage in {rel}"
            ));
        }
    }
    Ok(problems)
}

/// Extract the stable diagnostic code from a `kcs-diagnostic-<code>-*.md`
/// filename. The code is kept separate from prose so stage ownership remains
/// explicit even when a page's title or wording changes.
fn diagnostic_code(path: &Path) -> Option<String> {
    let stem = path.file_stem()?.to_str()?;
    let code = stem.strip_prefix("kcs-diagnostic-")?.split('-').next()?;
    (!code.is_empty()).then(|| code.to_ascii_uppercase())
}

/// Parse the one-line comma-separated Applies-to list from a KCS note.
fn applies_to_tags(text: &str) -> Option<Vec<String>> {
    let mut lines = text.lines();
    while let Some(line) = lines.next() {
        if line.trim() != "## Applies to" {
            continue;
        }
        let value = lines.find(|line| !line.trim().is_empty())?;
        if value.starts_with('#') {
            return None;
        }
        return Some(
            value
                .split(',')
                .filter_map(|tag| {
                    let tag = normalise_tag(tag);
                    (!tag.is_empty()).then_some(tag)
                })
                .collect(),
        );
    }
    None
}

/// The normalisation used by the CLI help index.
fn normalise_tag(raw: &str) -> String {
    raw.to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-")
}

/// Check 5 (warning only) — every `kcs-*.md` note has an Audience header.
fn check_kcs_audience_headers(
    root: &Path,
    docs: &Path,
    audience_re: &Regex,
) -> Result<Vec<String>> {
    let kcs_dir = docs.join("kcs");
    if !kcs_dir.exists() {
        return Ok(Vec::new());
    }
    let mut candidates = list_kcs_notes(&kcs_dir)?;
    candidates.extend(list_kcs_notes(&kcs_dir.join("features"))?);
    candidates.sort();

    let mut warnings = Vec::new();
    for note in &candidates {
        let text = read(note)?;
        if !audience_re.is_match(&text) {
            let rel = rel_to(root, note);
            warnings.push(format!("KCS note missing `> **Audience:**` header: {rel}"));
        }
    }
    Ok(warnings)
}

// helpers

/// The portion of a link before its `#` anchor.
fn link_before_anchor(link: &str) -> &str {
    link.split('#').next().unwrap_or("")
}

/// `path` relative to `base`, as a forward-slash string.
fn rel_to(base: &Path, path: &Path) -> String {
    path.strip_prefix(base)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}

/// The file name of `path` as an owned string.
fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

fn read(path: &Path) -> Result<String> {
    std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))
}

/// Recursively collect `*.md` files under `dir`.
fn collect_md_recursive(dir: &Path, out: &mut Vec<PathBuf>) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        let path = entry?.path();
        if path.is_dir() {
            collect_md_recursive(&path, out)?;
        } else if path.extension().is_some_and(|ext| ext == "md") {
            out.push(path);
        }
    }
    Ok(())
}

/// Non-recursive `*.md` files directly under `dir`.
fn list_md_depth1(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    if !dir.is_dir() {
        return Ok(out);
    }
    for entry in std::fs::read_dir(dir).with_context(|| format!("reading {}", dir.display()))? {
        let path = entry?.path();
        if path.is_file() && path.extension().is_some_and(|ext| ext == "md") {
            out.push(path);
        }
    }
    Ok(out)
}

/// Non-recursive `kcs-*.md` notes directly under `dir`.
fn list_kcs_notes(dir: &Path) -> Result<Vec<PathBuf>> {
    Ok(list_md_depth1(dir)?
        .into_iter()
        .filter(|p| {
            p.file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with("kcs-"))
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fenced_code_is_stripped() {
        let md = "before\n```\n[x](broken.md)\n```\nafter\n";
        let stripped = strip_fenced_code(md);
        assert!(stripped.contains("before"));
        assert!(stripped.contains("after"));
        assert!(!stripped.contains("broken.md"));
    }

    #[test]
    fn tilde_fences_and_unterminated_fence() {
        let md = "keep\n~~~\nhidden\n~~~\nkeep2\n```\ntrailing\n";
        let stripped = strip_fenced_code(md);
        assert!(stripped.contains("keep"));
        assert!(stripped.contains("keep2"));
        assert!(!stripped.contains("hidden"));
        assert!(!stripped.contains("trailing"));
    }

    #[test]
    fn link_targets_drop_anchors() {
        let re = Regex::new(r"\[[^\]]+\]\(([^)]+)\)").unwrap();
        let targets = {
            let mut t = BTreeSet::new();
            for caps in re.captures_iter("[a](foo.md#sec) [b](http://x) [c](bar.md)") {
                t.insert(link_before_anchor(&caps[1]).to_owned());
            }
            t
        };
        assert!(targets.contains("foo.md"));
        assert!(targets.contains("bar.md"));
        assert!(targets.contains("http://x"));
    }

    #[test]
    fn audience_regex_matches_blockquote_header() {
        let re = Regex::new(r"(?m)^>\s*\*\*Audience:\*\*").unwrap();
        assert!(re.is_match("# Title\n\n> **Audience:** engineers\n"));
        assert!(!re.is_match("# Title\n\nno audience line\n"));
    }

    #[test]
    fn heading_slugs_follow_github_rules_and_deduplicate() {
        let headings = heading_slugs(
            "## [Command *walk*](command.md) — `args` & `CommandSpec`!\n\
             ## Repeat\n\
             ## Repeat\n\
             ## Grammar {#grammar}\n",
        );
        assert!(headings.contains("command-walk--args--commandspec"));
        assert!(headings.contains("repeat"));
        assert!(headings.contains("repeat-1"));
        assert!(headings.contains("grammar"));
        assert!(!headings.contains("command-walk-args-commandspec"));
        assert_eq!(
            github_heading_slug("Example 6: `if {1} { ... } else { ... }` (constant condition)"),
            "example-6-if-1----else----constant-condition"
        );
    }

    #[test]
    fn heading_slugs_skip_occupied_generated_suffixes() {
        let headings = heading_slugs("# Foo\n# Foo-1\n# Foo\n");
        assert!(headings.contains("foo"));
        assert!(headings.contains("foo-1"));
        assert!(headings.contains("foo-2"));
    }

    #[test]
    fn fragments_cover_intra_file_and_ignore_fences_and_templates() {
        let headings = heading_slugs("# Current section\n```md\n# Hidden section\n```\n");
        let (path, fragment) = split_link_fragment("#current-section");
        assert!(path.is_empty());
        assert!(headings.contains(fragment.unwrap()));
        assert!(!headings.contains("hidden-section"));
        assert!(is_template_placeholder("<pass-anchor>"));
        assert!(!is_template_placeholder("current-section"));
    }

    #[test]
    fn diagnostic_tags_are_normalised_and_closed() {
        assert_eq!(normalise_tag("Command walk"), "command-walk");
        assert!(DIAGNOSTIC_KCS_TAGS.contains(&"command-walk"));
        assert!(!DIAGNOSTIC_KCS_TAGS.contains(&"command-wlak"));
        assert!(DIAGNOSTIC_KCS_TAGS.contains(&"lowering"));
    }

    #[test]
    fn diagnostic_stage_mapping_is_code_specific() {
        assert_eq!(
            diagnostic_code(Path::new("kcs-diagnostic-irule4004-page.md")),
            Some("IRULE4004".to_owned())
        );
        assert!(EXPECTED_DIAGNOSTIC_STAGES.contains(&("IRULE4005", "ssa")));
        assert!(EXPECTED_DIAGNOSTIC_STAGES.contains(&("IRULE5002", "lowering")));
    }
}

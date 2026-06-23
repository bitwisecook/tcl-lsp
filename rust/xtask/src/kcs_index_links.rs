//! KCS / design-docs link + index-coverage check.
//!
//! Validates four things, producing the expected stdout and exit
//! codes:
//!
//! 1. every local markdown link in `docs/` (+ `CONTRIBUTING.md` /
//!    `AGENTS.md`) resolves;
//! 2. every design doc under `docs/design/` is linked from the design
//!    index (or a parent `README.md` that is itself indexed);
//! 3. every top-level KCS note under `docs/kcs/` is linked from the KCS
//!    index (and the `features/` subindex);
//! 4. every `kcs-*.md` note carries a `> **Audience:**` header (warning
//!    only — does not fail the build).
//!
//! Exit 1 if any of checks 1–3 fail; otherwise exit 0.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::{Context, Result};
use regex::Regex;

use crate::util::repo_root;

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
            if link.starts_with("http://")
                || link.starts_with("https://")
                || link.starts_with('#')
                || link.starts_with("mailto:")
            {
                continue;
            }
            let target = parent.join(link_before_anchor(link));
            if !target.exists() {
                let rel = rel_to(root, file);
                problems.push(format!("broken link in {rel}: {link}"));
            }
        }
    }
    Ok(problems)
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

/// Check 4 (warning only) — every `kcs-*.md` note has an Audience header.
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

// ── helpers ─────────────────────────────────────────────────────────────

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
}

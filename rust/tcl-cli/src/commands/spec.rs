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

//! `tcl spec` verb group — authoring `.tclspec` command packs.
//!
//! Today one sub-action: `tcl spec import`, which reads *several releases* of
//! a Tcl package and renders a pack whose `introduced_version` /
//! `retired_version` fields carry the ranges those releases actually witness.
//! A single-snapshot import cannot do that — it can only stamp every command
//! with whatever version the newest sources declare, which is a fabrication.
//!
//! Two ways to name the releases:
//!
//! - `--snapshot VERSION=PATH` (repeatable) — a directory, `.zip` or
//!   `.tar.gz` per release, already on disk. No network at all.
//! - `--github OWNER/REPO` — enumerate the repository's tags through the
//!   GitHub REST API and fetch each release's tarball from codeload.
//!
//! Everything after "sources in hand" is
//! [`tcl_cli_support::spec_import`], shared verbatim with the MCP
//! `spec_import` tool; the tag enumeration and the downloads live here,
//! because the MCP server deliberately has no fetcher.

// Handlers return `anyhow::Result<u8>` for a uniform dispatch signature even
// when a given verb cannot fail; the wrap is the interface contract.
#![allow(clippy::unnecessary_wraps)]

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use anyhow::{Context, anyhow, bail};
use serde_json::{Value, json};
use tcl_cli_support::chrome::{eprint_status, warn_style};
use tcl_cli_support::spec_import::{
    SpecImport, SpecImportOptions, import_snapshots, load_snapshot, parse_snapshot_arg,
};
use tcl_cli_support::{OutputTarget, write_text_output};
use tcl_spec_studio::versions::VersionedSnapshot;

use crate::cli::{SpecCommand, SpecImportArgs};

/// GitHub's maximum page size for `/tags`; fewer requests, same answer.
const TAGS_PER_PAGE: usize = 100;

/// How many pages of tags to walk before giving up. A thousand tags is far
/// more history than a spec import can use, and an unbounded loop against a
/// paginated API is a way to hang on a hostile or broken endpoint.
const MAX_TAG_PAGES: usize = 10;

/// Dispatch a `tcl spec` sub-action.
pub fn run(action: &SpecCommand) -> anyhow::Result<u8> {
    match action {
        SpecCommand::Import(args) => run_import(args),
    }
}

/// `tcl spec import` — derive version ranges from several releases.
pub fn run_import(args: &SpecImportArgs) -> anyhow::Result<u8> {
    // `--partial-history` is the default, and is accepted so a script can say
    // what it means rather than relying on the default staying put. The safe
    // default is the modest claim: snapshots are only ever *some* releases
    // until the caller says otherwise, and a wrong `introduced_version` is a
    // fact nobody can tell from a derived one later.
    let complete_history = args.history.complete_history;

    let (snapshots, fetched) = if let Some(repo) = &args.github {
        let repo = GithubRepo::parse(repo)?;
        let picks = select_tags(
            &github_tags(&repo, args.timeout)?,
            args.tag_pattern.as_deref(),
            args.limit,
        );
        if picks.is_empty() {
            bail!(
                "no tags of {repo} matched{}",
                args.tag_pattern
                    .as_deref()
                    .map_or_else(String::new, |p| format!(" `{p}`"))
            );
        }
        if args.list_tags {
            return list_tags(&repo, &picks, args.json, args.out.as_deref());
        }
        (fetch_snapshots(&repo, &picks, args.timeout)?, true)
    } else {
        (local_snapshots(&args.snapshot)?, false)
    };

    let import = import_snapshots(
        &snapshots,
        &SpecImportOptions {
            dialect: &args.dialect,
            package: args.package.as_deref(),
            complete_history,
        },
    );

    let target = OutputTarget::from_arg(args.out.as_deref());
    if args.json {
        write_text_output(&target, &format!("{:#}\n", import.to_json()))?;
    } else {
        write_text_output(&target, &import.pack)?;
    }
    summarise(&import, fetched);
    Ok(0)
}

/// Read every `--snapshot VERSION=PATH` into a labelled snapshot.
fn local_snapshots(args: &[String]) -> anyhow::Result<Vec<VersionedSnapshot>> {
    if args.is_empty() {
        bail!("pass --snapshot VERSION=PATH (repeatable) or --github OWNER/REPO");
    }
    let mut snapshots = Vec::with_capacity(args.len());
    for arg in args {
        let (version, path) = parse_snapshot_arg(arg)?;
        snapshots.push(load_snapshot(&version, &path)?);
    }
    Ok(snapshots)
}

// ---------------------------------------------------------------------------
// GitHub mode
// ---------------------------------------------------------------------------

/// A validated `OWNER/REPO`.
///
/// Both halves are checked against GitHub's own name alphabet before they are
/// pasted into a URL: a `..` or a `?` in a repository name would otherwise
/// build a request to somewhere the user did not ask for.
#[derive(Debug, Clone)]
struct GithubRepo {
    owner: String,
    repo: String,
}

impl GithubRepo {
    fn parse(spec: &str) -> anyhow::Result<Self> {
        let (owner, repo) = spec
            .split_once('/')
            .ok_or_else(|| anyhow!("--github expects OWNER/REPO, got `{spec}`"))?;
        let repo = repo.strip_suffix(".git").unwrap_or(repo);
        for (label, part) in [("owner", owner), ("repository", repo)] {
            if part.is_empty() || !part.chars().all(is_name_char) {
                bail!("--github: `{part}` is not a usable GitHub {label} name");
            }
        }
        Ok(Self {
            owner: owner.to_owned(),
            repo: repo.to_owned(),
        })
    }
}

impl std::fmt::Display for GithubRepo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}/{}", self.owner, self.repo)
    }
}

/// The characters GitHub allows in an owner or repository name.
fn is_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')
}

/// The characters a tag may contain to be safe in a codeload path.
///
/// `/` is allowed because `release/1.2` is a real tag shape and codeload
/// resolves it; a `..` segment is rejected separately.
fn is_tag_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/' | '+')
}

/// One tag we would fetch, and the version label derived from it.
#[derive(Debug, Clone, PartialEq, Eq)]
struct TagPick {
    tag: String,
    version: String,
}

impl TagPick {
    /// The codeload tarball for this tag.
    fn url(&self, repo: &GithubRepo) -> String {
        format!(
            "https://codeload.github.com/{}/{}/tar.gz/refs/tags/{}",
            repo.owner, repo.repo, self.tag
        )
    }
}

/// Every tag name of `repo`, walking the REST API's pages.
///
/// `GITHUB_TOKEN`, when set, is sent as `Authorization: Bearer …` — the
/// unauthenticated rate limit (60 requests/hour/IP) is easy to exhaust and the
/// failure looks like "the repository has no tags". The request goes through
/// [`tcl_pkg::fetchers::fetch_bytes`], so `HTTPS_PROXY` / `NO_PROXY` and the
/// rest of the standard proxy environment are honoured.
fn github_tags(repo: &GithubRepo, timeout: u64) -> anyhow::Result<Vec<String>> {
    let token = std::env::var("GITHUB_TOKEN").ok().filter(|t| !t.is_empty());
    let mut headers: Vec<(&str, &str)> = vec![
        ("Accept", "application/vnd.github+json"),
        ("X-GitHub-Api-Version", "2022-11-28"),
    ];
    let authorization;
    if let Some(token) = &token {
        authorization = format!("Bearer {token}");
        headers.push(("Authorization", &authorization));
    }

    let mut tags = Vec::new();
    for page in 1..=MAX_TAG_PAGES {
        let url = format!(
            "https://api.github.com/repos/{}/{}/tags?per_page={TAGS_PER_PAGE}&page={page}",
            repo.owner, repo.repo
        );
        let body = tcl_pkg::fetchers::fetch_bytes(&url, timeout, &headers)
            .with_context(|| format!("listing tags of {repo}"))?;
        let value: Value = serde_json::from_slice(&body)
            .with_context(|| format!("listing tags of {repo}: response was not JSON"))?;
        let Some(entries) = value.as_array() else {
            // The API answers an error with an object carrying `message`.
            let message = value
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("unexpected response shape");
            bail!("listing tags of {repo}: {message}");
        };
        let page_len = entries.len();
        tags.extend(
            entries
                .iter()
                .filter_map(|e| e.get("name").and_then(Value::as_str))
                .map(ToOwned::to_owned),
        );
        if page_len < TAGS_PER_PAGE {
            return Ok(tags);
        }
    }
    Ok(tags)
}

/// The version label a tag names.
///
/// Two reductions, both common and both reversible by eye:
///
/// 1. A leading project prefix — everything before the first `-` or `_` when
///    that head carries no digit and the tail starts a version
///    (`tcllib-1.20` → `1.20`, `release_2.0` → `2.0`). `v1.2-rc1` keeps its
///    tail, because `v1.2` does carry a digit.
/// 2. A leading `v` or `V` directly in front of a digit (`v1.2` → `1.2`).
///
/// Anything else is the label verbatim, so an unrecognised scheme is passed
/// through rather than mangled.
fn version_label(tag: &str) -> String {
    let trimmed = tag.trim();
    let mut label = trimmed;
    if let Some((head, tail)) = trimmed.split_once(['-', '_'])
        && !head.chars().any(|c| c.is_ascii_digit())
        && starts_a_version(tail)
    {
        label = tail;
    }
    if let Some(rest) = label.strip_prefix(['v', 'V'])
        && rest.starts_with(|c: char| c.is_ascii_digit())
    {
        label = rest;
    }
    label.to_owned()
}

/// Whether `text` looks like the start of a version: a digit, or a `v` in
/// front of one.
fn starts_a_version(text: &str) -> bool {
    let text = text.strip_prefix(['v', 'V']).unwrap_or(text);
    text.starts_with(|c: char| c.is_ascii_digit())
}

/// Whether `text` matches `pattern`, where `*` matches any run of characters
/// (`/` included), `?` matches exactly one, and every other character matches
/// itself. The whole tag must match — this is a filter, not a search.
fn glob_matches(pattern: &str, text: &str) -> bool {
    let p: Vec<char> = pattern.chars().collect();
    let t: Vec<char> = text.chars().collect();
    // Classic linear backtracking matcher: `star`/`mark` remember the last `*`
    // and the position it had consumed up to, so a failed suffix retries with
    // one more character eaten by that `*`.
    let (mut pi, mut ti) = (0usize, 0usize);
    let (mut star, mut mark) = (None, 0usize);
    while ti < t.len() {
        if pi < p.len() && (p[pi] == '?' || p[pi] == t[ti]) {
            pi += 1;
            ti += 1;
        } else if pi < p.len() && p[pi] == '*' {
            star = Some(pi);
            mark = ti;
            pi += 1;
        } else if let Some(s) = star {
            pi = s + 1;
            mark += 1;
            ti = mark;
        } else {
            return false;
        }
    }
    p[pi..].iter().all(|c| *c == '*')
}

/// The tags to import, oldest first.
///
/// Filtered by `pattern`, dropped when the tag is unusable in a URL or yields
/// no version label, ordered with the `package vcompare` port
/// ([`tcl_dialect::compare_versions`], never GitHub's listing order),
/// de-duplicated by label — two tags for one version would be a duplicate
/// snapshot — and finally cut to the **newest** `limit`, because recent
/// releases are what a spec is being written against.
///
/// The comparator is the dialect port rather than
/// `tcl_registry::version::compare` because a release tag routinely carries a
/// prerelease suffix: the registry comparator parses dotted integers only, so
/// `1.0a1`, `1.0b1`, and `1.0` all compare equal and the `limit` cut would
/// keep whichever the tag-name tiebreak happened to sort last. Duplicate
/// detection stays *textual*, so two differently-spelt labels for the same
/// release (`1.0` / `1.0.0`) both survive to the importer, which warns about
/// them by name.
fn select_tags(tags: &[String], pattern: Option<&str>, limit: Option<usize>) -> Vec<TagPick> {
    let mut picks: Vec<TagPick> = tags
        .iter()
        .filter(|tag| pattern.is_none_or(|p| glob_matches(p, tag)))
        .filter(|tag| tag.chars().all(is_tag_char) && !tag.split('/').any(|s| s == ".."))
        .filter_map(|tag| {
            let version = version_label(tag);
            (!version.is_empty()).then(|| TagPick {
                tag: tag.clone(),
                version,
            })
        })
        .collect();
    picks.sort_by(|a, b| {
        tcl_dialect::compare_versions(&a.version, &b.version).then_with(|| a.tag.cmp(&b.tag))
    });
    picks.dedup_by(|a, b| a.version == b.version);
    if let Some(limit) = limit
        && picks.len() > limit
    {
        picks.drain(..picks.len() - limit);
    }
    picks
}

/// `--list-tags`: say what would be fetched, and fetch nothing.
fn list_tags(
    repo: &GithubRepo,
    picks: &[TagPick],
    json: bool,
    out: Option<&Path>,
) -> anyhow::Result<u8> {
    let target = OutputTarget::from_arg(out);
    if json {
        let value = json!({
            "repository": repo.to_string(),
            "tags": picks.iter().map(|p| json!({
                "tag": p.tag,
                "version": p.version,
                "url": p.url(repo),
            })).collect::<Vec<_>>(),
        });
        write_text_output(&target, &format!("{value:#}\n"))?;
    } else {
        let text = picks.iter().fold(String::new(), |mut text, pick| {
            let _ = writeln!(text, "{}\t{}\t{}", pick.tag, pick.version, pick.url(repo));
            text
        });
        write_text_output(&target, &text)?;
    }
    eprint_status(
        warn_style(),
        format!(
            "{} release(s) would be fetched from {repo}; drop --list-tags to import them",
            picks.len()
        ),
    );
    Ok(0)
}

/// Fetch each picked tag's tarball and read its sources.
fn fetch_snapshots(
    repo: &GithubRepo,
    picks: &[TagPick],
    timeout: u64,
) -> anyhow::Result<Vec<VersionedSnapshot>> {
    let mut snapshots = Vec::with_capacity(picks.len());
    for pick in picks {
        let url = pick.url(repo);
        eprint_status(warn_style(), format!("fetching {} … {url}", pick.version));
        let scratch = ReleaseDir::new(&pick.version)?;
        tcl_pkg::fetchers::fetch_tarball(&url, scratch.path(), timeout)
            .map_err(|e| anyhow!("fetching {url}: {e}"))?;
        snapshots.push(load_snapshot(&pick.version, scratch.path())?);
    }
    Ok(snapshots)
}

/// A scratch directory for one fetched release, removed when it drops.
struct ReleaseDir(PathBuf);

impl ReleaseDir {
    fn new(version: &str) -> anyhow::Result<Self> {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
        // The label is sanitised, never used raw: a version derived from a tag
        // like `release/1.2` would otherwise dig a directory tree out of the
        // temp root. It is only there to make a stray directory identifiable.
        let label: String = version
            .chars()
            .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
            .take(24)
            .collect();
        let dir = std::env::temp_dir().join(format!(
            "tcl-spec-import-{}-{nanos}-{seq}-{label}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
        Ok(Self(dir))
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for ReleaseDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// The human account of the import, on stderr so it never contaminates a pack
/// written to stdout.
fn summarise(import: &SpecImport, fetched: bool) {
    let source = if fetched { "fetched" } else { "local" };
    eprint_status(
        warn_style(),
        format!(
            "{}: {} command(s) from {} {source} release(s) ({}); \
             {} introduced, {} retired",
            import.package,
            import.commands.len(),
            import.versions.len(),
            import.versions.join(", "),
            import.introduced_count(),
            import.retired_count(),
        ),
    );
    let gates: usize = import.commands.iter().map(|c| c.gates().len()).sum();
    if gates > 0 {
        eprint_status(
            warn_style(),
            format!("{gates} version fact(s) recorded as `version-gate:` notes in the pack header"),
        );
    }
    for warning in &import.warnings {
        eprint_status(warn_style(), format!("warning: {warning}"));
    }
    if !import.losses.is_empty() {
        eprint_status(
            warn_style(),
            format!(
                "{} field(s) could not be rendered into the pack; see its header",
                import.losses.len()
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_tag_maps_to_the_version_it_names() {
        assert_eq!(version_label("v1.2"), "1.2");
        assert_eq!(version_label("V2.0.1"), "2.0.1");
        assert_eq!(version_label("1.2"), "1.2");
        assert_eq!(version_label("tcllib-1.20"), "1.20");
        assert_eq!(version_label("release_2.0"), "2.0");
        assert_eq!(version_label("pkg-v3.1"), "3.1");
        // A head that carries a digit is part of the version, not a prefix.
        assert_eq!(version_label("v1.2-rc1"), "1.2-rc1");
        // An unrecognised scheme is passed through rather than mangled.
        assert_eq!(version_label("nightly"), "nightly");
        assert_eq!(version_label("version"), "version");
    }

    #[test]
    fn a_tag_pattern_anchors_to_the_whole_tag() {
        assert!(glob_matches("v*", "v1.2"));
        assert!(glob_matches("v1.*", "v1.20.3"));
        assert!(!glob_matches("v1.*", "v2.0"));
        assert!(glob_matches("v?.?", "v1.2"));
        assert!(!glob_matches("v?.?", "v1.20"));
        assert!(!glob_matches("1.2", "v1.2"), "the match is not a search");
        assert!(glob_matches("*", "anything"));
        assert!(glob_matches("release/*", "release/1.2"));
        assert!(glob_matches("v1.2", "v1.2"));
    }

    #[test]
    fn tags_are_selected_newest_last_and_deduplicated() {
        let tags: Vec<String> = ["v1.10", "v1.2", "v1.9", "1.10", "nightly", "v2.0"]
            .iter()
            .map(ToString::to_string)
            .collect();
        let picks = select_tags(&tags, Some("v*"), None);
        let versions: Vec<&str> = picks.iter().map(|p| p.version.as_str()).collect();
        // 1.10 sorts above 1.9 — version order, not lexicographic order — and
        // `1.10` is filtered out by the pattern before the dedupe can see it.
        assert_eq!(versions, ["1.2", "1.9", "1.10", "2.0"]);
    }

    #[test]
    fn a_limit_keeps_the_newest_releases() {
        let tags: Vec<String> = ["v1.0", "v1.1", "v2.0"]
            .iter()
            .map(ToString::to_string)
            .collect();
        let picks = select_tags(&tags, None, Some(2));
        let versions: Vec<&str> = picks.iter().map(|p| p.version.as_str()).collect();
        assert_eq!(versions, ["1.1", "2.0"]);
    }

    #[test]
    fn prerelease_tags_order_below_the_release_they_lead_up_to() {
        // `tcl_registry::version::compare` stops at the first non-numeric
        // segment, so it reads all three of these as `1.0` — the order would
        // then fall to the tag-name tiebreak (`v1.0` first) and `--limit 1`
        // would import a beta instead of the release. The `package vcompare`
        // port orders alpha < beta < release.
        let tags: Vec<String> = ["v1.0", "v1.0b1", "v1.0a1"]
            .iter()
            .map(ToString::to_string)
            .collect();
        let picks = select_tags(&tags, None, None);
        let versions: Vec<&str> = picks.iter().map(|p| p.version.as_str()).collect();
        assert_eq!(versions, ["1.0a1", "1.0b1", "1.0"]);
        // …so the newest-`limit` cut keeps the release, not the prerelease.
        let newest = select_tags(&tags, None, Some(1));
        assert_eq!(newest[0].version, "1.0");
    }

    #[test]
    fn two_tags_for_one_version_import_once() {
        let tags: Vec<String> = ["v1.0", "1.0"].iter().map(ToString::to_string).collect();
        assert_eq!(select_tags(&tags, None, None).len(), 1);
    }

    #[test]
    fn a_tag_that_could_escape_a_url_is_dropped() {
        let tags: Vec<String> = ["v1.0", "v1.1?x=1", "../../etc/passwd", "v1.2#frag"]
            .iter()
            .map(ToString::to_string)
            .collect();
        let picks = select_tags(&tags, None, None);
        let kept: Vec<&str> = picks.iter().map(|p| p.tag.as_str()).collect();
        assert_eq!(kept, ["v1.0"]);
    }

    #[test]
    fn a_repository_name_is_validated_before_it_reaches_a_url() {
        assert!(GithubRepo::parse("tcltk/tcllib").is_ok());
        assert!(GithubRepo::parse("tcltk/tcllib.git").is_ok());
        for bad in ["tcllib", "tcltk/", "/tcllib", "tcltk/../../x", "a b/c"] {
            assert!(GithubRepo::parse(bad).is_err(), "`{bad}` should be refused");
        }
    }

    #[test]
    fn the_codeload_url_is_the_tag_tarball() {
        let repo = GithubRepo::parse("tcltk/tcllib").expect("owner/repo");
        let pick = TagPick {
            tag: "v1.2".to_owned(),
            version: "1.2".to_owned(),
        };
        assert_eq!(
            pick.url(&repo),
            "https://codeload.github.com/tcltk/tcllib/tar.gz/refs/tags/v1.2"
        );
    }
}

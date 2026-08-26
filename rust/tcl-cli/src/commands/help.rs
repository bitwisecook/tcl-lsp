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

//! `help` (`docs`) verb: full-text search the bundled KCS help database.
//!
//! Runs the `search_help` / `list_features` query layer over the embedded
//! `kcs_help.db` (an `FTS5` index built at compile time by `build.rs` from
//! `docs/kcs/features/*.md`). The database is compiled into the binary
//! (`include_bytes!`) and opened read-only via `rusqlite`'s bundled `SQLite`
//! (which ships `FTS5`); to give `SQLite` a file path it is written to a unique
//! temp file for the duration of the queries.
//!
//! `--json` embeds the raw `BM25` `rank` float. Codes, content and ordering
//! match the captured golden output byte-for-byte, but the `rank` value is computed by whichever
//! `SQLite` version is linked (this crate uses rusqlite's bundled
//! one) and the two diverge in the low-order digits on some corpora — so it is
//! not a cross-environment-stable field (the json golden is captured
//! from this binary).

use std::io::Write as _;
use std::path::PathBuf;

use rusqlite::{Connection, OpenFlags};
use serde_json::{Map, Value, json};
use tcl_cli_support::{OutputTarget, ensure_ascii, write_text_output};
use tcl_dialect::DialectProfile;

/// The KCS help database, built at compile time by `build.rs` (from
/// `docs/kcs/features/*.md`) into `$OUT_DIR` and embedded here.
static KCS_DB: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/kcs_help.db"));

/// Per-dialect substring terms used to filter help entries — the
/// profile catalog's `help_terms` (dialect-profile model §5.4), so alias
/// spellings (`irules` → `f5-irules`) filter like the canonical name and
/// the term sets can never drift from the dialect model. `all` (and any
/// unknown dialect, via the permissive fallback's empty set) yields no
/// terms, meaning "no filtering".
fn dialect_terms(dialect: &str) -> &'static [&'static str] {
    match dialect {
        // The explicit "everything" selector — not a dialect name.
        "all" => &[],
        // The *analyser* ingress form, deliberately: `tk` takes the
        // permissive fallback's empty term set here, exactly as
        // `DialectProfile::by_name` gave it, so `tcl help --dialect tk`
        // keeps filtering nothing. Ledger C1 (post-P1-G): when the
        // profile retires, the term set becomes an environment fact and
        // `tk` gains its own.
        name => tcl_cli_support::environment::analyser_profile_for_dialect(name).help_terms,
    }
}

/// A KCS feature row. `rank`/`category` are only populated for search results;
/// `list_features` groups by category separately.
struct Feature {
    name: String,
    summary: String,
    category: String,
    how_to_use: String,
    file: String,
    rank: Option<f64>,
    applies_to: Vec<String>,
}

impl Feature {
    /// The lowercased searchable blob used by the dialect filter (`surface` is
    /// always empty for these rows).
    fn dialect_haystack(&self) -> String {
        format!(
            "{} {} {} {} {} {}",
            self.name, self.summary, "", self.category, self.how_to_use, self.file
        )
        .to_lowercase()
    }

    fn matches_dialect(&self, dialect: &str) -> bool {
        let terms = dialect_terms(dialect);
        if terms.is_empty() {
            return true;
        }
        let hay = self.dialect_haystack();
        terms.iter().any(|t| hay.contains(t))
    }

    fn matches_profile(&self, dialect: Option<&DialectProfile>) -> bool {
        dialect.is_none_or(|profile| {
            profile.help_terms.is_empty()
                || profile
                    .help_terms
                    .iter()
                    .any(|term| self.dialect_haystack().contains(term))
        })
    }

    /// JSON view for embedders (the MCP `help` tool). `rank` is included only
    /// for search results.
    fn to_json(&self) -> serde_json::Value {
        let mut v = serde_json::json!({
            "name": self.name,
            "summary": self.summary,
            "category": self.category,
            "how_to_use": self.how_to_use,
            "file": self.file,
            "applies_to": self.applies_to,
        });
        if let Some(rank) = self.rank {
            v.as_object_mut()
                .expect("json object")
                .insert("rank".to_owned(), serde_json::json!(rank));
        }
        v
    }
}

/// Structured KCS help lookup for embedders (the native MCP `help` tool).
///
/// Searches for `query` (empty → the full feature catalogue), filtered by
/// `dialect`, capped at `limit` matches. Returns a JSON value:
/// `{query, matches, count}` on a hit, `{error, available_sections}` when a
/// non-empty query matches nothing, or `{sections}` for the empty-query
/// catalogue.
#[must_use]
pub fn help_json(query: &str, dialect: &str, limit: usize) -> serde_json::Value {
    let (conn, path) = match load_db() {
        Ok(pair) => pair,
        Err(e) => return serde_json::json!({ "error": format!("KCS help DB: {e}") }),
    };
    let result = help_json_impl(&conn, query, dialect, limit);
    drop(conn);
    let _ = std::fs::remove_file(&path);
    result
}

fn help_json_impl(
    conn: &Connection,
    query: &str,
    dialect: &str,
    limit: usize,
) -> serde_json::Value {
    if query.trim().is_empty() {
        let grouped = match list_features(conn) {
            Ok(g) => g,
            Err(e) => return serde_json::json!({ "error": e.to_string() }),
        };
        let sections: Vec<serde_json::Value> = grouped
            .into_iter()
            .filter_map(|(category, feats)| {
                let feats: Vec<serde_json::Value> = feats
                    .iter()
                    .filter(|f| f.matches_dialect(dialect))
                    .map(Feature::to_json)
                    .collect();
                (!feats.is_empty())
                    .then(|| serde_json::json!({ "category": category, "features": feats }))
            })
            .collect();
        return serde_json::json!({ "sections": sections });
    }

    let feats = match search_help(conn, query, limit) {
        Ok(f) => f,
        Err(e) => return serde_json::json!({ "error": e.to_string() }),
    };
    let matches: Vec<serde_json::Value> = feats
        .iter()
        .filter(|f| f.matches_dialect(dialect))
        .map(Feature::to_json)
        .collect();
    if matches.is_empty() {
        let available: Vec<String> = list_features(conn)
            .map(|g| g.into_iter().map(|(category, _)| category).collect())
            .unwrap_or_default();
        return serde_json::json!({
            "error": format!("No features match '{query}'"),
            "available_sections": available,
        });
    }
    serde_json::json!({ "query": query, "matches": matches, "count": matches.len() })
}

/// Write the embedded DB to a unique temp file and open it read-only. Returns
/// the connection and the temp path (removed by the caller once queries are
/// done).
fn load_db() -> anyhow::Result<(Connection, PathBuf)> {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    let mut path = std::env::temp_dir();
    path.push(format!("tcl_kcs_help_{}_{nanos}.db", std::process::id()));
    std::fs::write(&path, KCS_DB)?;
    let conn = Connection::open_with_flags(
        &path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )?;
    Ok((conn, path))
}

/// Sorted `applies_to` tags for a feature file.
fn fetch_tags(conn: &Connection, file: &str) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT tag FROM feature_tags WHERE file = ?1 ORDER BY tag")?;
    let tags = stmt
        .query_map([file], |row| row.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(tags)
}

/// Full-text search across KCS features, ranked by relevance. Falls back to a
/// `LIKE` scan if the FTS `MATCH` query
/// errors.
fn search_help(conn: &Connection, query: &str, limit: usize) -> anyhow::Result<Vec<Feature>> {
    let safe = query.replace('"', "\"\"");
    let match_expr = format!("\"{safe}\" OR {safe}*");
    let limit_i = i64::try_from(limit).unwrap_or(i64::MAX);

    let fts = (|| -> rusqlite::Result<Vec<Feature>> {
        let mut stmt = conn.prepare(
            "SELECT name, summary, category, how_to_use, file, rank \
             FROM kcs_features WHERE kcs_features MATCH ?1 ORDER BY rank LIMIT ?2",
        )?;
        let rows = stmt
            .query_map(rusqlite::params![match_expr, limit_i], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, f64>(5)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        Ok(rows
            .into_iter()
            .map(
                |(name, summary, category, how_to_use, file, rank)| Feature {
                    name,
                    summary,
                    category,
                    how_to_use,
                    file,
                    rank: Some(rank),
                    applies_to: Vec::new(),
                },
            )
            .collect())
    })();

    let mut features = if let Ok(rows) = fts {
        rows
    } else {
        // Fallback: simple LIKE search (`except OperationalError`).
        {
            let like = format!("%{query}%");
            let mut stmt = conn.prepare(
                "SELECT name, summary, category, how_to_use, file, 0 as rank \
                 FROM kcs_features WHERE name LIKE ?1 OR summary LIKE ?1 OR how_to_use LIKE ?1 \
                 ORDER BY name LIMIT ?2",
            )?;
            let rows = stmt
                .query_map(rusqlite::params![like, limit_i], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, f64>(5)?,
                    ))
                })?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            rows.into_iter()
                .map(
                    |(name, summary, category, how_to_use, file, rank)| Feature {
                        name,
                        summary,
                        category,
                        how_to_use,
                        file,
                        rank: Some(rank),
                        applies_to: Vec::new(),
                    },
                )
                .collect()
        }
    };

    for f in &mut features {
        f.applies_to = fetch_tags(conn, &f.file)?;
    }
    Ok(features)
}

/// All features grouped by category, in `(category, name)` order.
/// Categories appear in first-seen order, which — because the
/// query is `ORDER BY category, name` — is sorted category order.
fn list_features(conn: &Connection) -> anyhow::Result<Vec<(String, Vec<Feature>)>> {
    let mut stmt = conn.prepare(
        "SELECT name, summary, category, how_to_use, file \
         FROM kcs_features ORDER BY category, name",
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;

    let mut catalogue: Vec<(String, Vec<Feature>)> = Vec::new();
    for (name, summary, category, how_to_use, file) in rows {
        let applies_to = fetch_tags(conn, &file)?;
        let feature = Feature {
            name,
            summary,
            category: category.clone(),
            how_to_use,
            file,
            rank: None,
            applies_to,
        };
        if let Some((_, features)) = catalogue.iter_mut().find(|(c, _)| *c == category) {
            features.push(feature);
        } else {
            catalogue.push((category, vec![feature]));
        }
    }
    Ok(catalogue)
}

/// JSON object for a catalogue feature (row dict:
/// `name`, `summary`, `how_to_use`, `file`, `applies_to` — category is the
/// grouping key, not a field).
fn catalogue_feature_json(feature: &Feature) -> Value {
    json!({
        "name": feature.name,
        "summary": feature.summary,
        "how_to_use": feature.how_to_use,
        "file": feature.file,
        "applies_to": feature.applies_to,
    })
}

/// JSON object for a search result (row dict, key
/// order: `name`, `summary`, `category`, `how_to_use`, `file`, `rank`,
/// `applies_to`).
fn search_result_json(feature: &Feature) -> Value {
    let mut obj = Map::new();
    obj.insert("name".to_owned(), json!(feature.name));
    obj.insert("summary".to_owned(), json!(feature.summary));
    obj.insert("category".to_owned(), json!(feature.category));
    obj.insert("how_to_use".to_owned(), json!(feature.how_to_use));
    obj.insert("file".to_owned(), json!(feature.file));
    obj.insert("rank".to_owned(), json!(feature.rank));
    obj.insert("applies_to".to_owned(), json!(feature.applies_to));
    Value::Object(obj)
}

/// Prefix a text rendering with one header line naming an active `--dialect`
/// filter, so a filtered listing is not read as the whole catalogue. `None`
/// (`--dialect all`) filters nothing and gets no header.
fn with_dialect_header(dialect: Option<&DialectProfile>, body: String) -> String {
    match dialect {
        Some(profile) => format!(
            "dialect filter: {} [{}]\n{body}",
            profile.display_name, profile.name
        ),
        None => body,
    }
}

/// Render the catalogue as text.
fn render_catalogue(catalogue: &[(String, Vec<Feature>)]) -> String {
    if catalogue.is_empty() {
        return "no help entries found".to_owned();
    }
    // Categories sorted case-insensitively; features sorted by name.
    let mut cats: Vec<&(String, Vec<Feature>)> = catalogue.iter().collect();
    cats.sort_by_key(|(c, _)| c.to_lowercase());
    let mut lines: Vec<String> = Vec::new();
    for (category, features) in cats {
        let mut sorted: Vec<&Feature> = features.iter().collect();
        sorted.sort_by_key(|f| f.name.to_lowercase());
        lines.push(format!("{category} ({}):", sorted.len()));
        for feature in sorted {
            let name = {
                let trimmed = feature.name.trim();
                if trimmed.is_empty() {
                    "<unnamed>"
                } else {
                    trimmed
                }
            };
            let summary = feature.summary.trim();
            if summary.is_empty() {
                lines.push(format!("  {name}"));
            } else {
                lines.push(format!("  {name}: {summary}"));
            }
        }
    }
    lines.join("\n")
}

/// Render search results as text.
fn render_search_results(results: &[Feature], query: &str) -> String {
    let match_word = if results.len() == 1 {
        "match"
    } else {
        "matches"
    };
    let mut lines: Vec<String> = vec![format!("{} {match_word} for '{query}':", results.len())];
    for result in results {
        let name = {
            let trimmed = result.name.trim();
            if trimmed.is_empty() {
                "<unnamed>".to_owned()
            } else {
                trimmed.to_owned()
            }
        };
        let category = result.category.trim();
        let summary = result.summary.trim();
        let file = result.file.trim();
        let heading = if category.is_empty() {
            name
        } else {
            format!("{name} [{category}]")
        };
        lines.push(format!("- {heading}"));
        if !summary.is_empty() {
            lines.push(format!("  {summary}"));
        }
        if !file.is_empty() {
            lines.push(format!("  file: {file}"));
        }
    }
    lines.join("\n")
}

/// `tcl help` (`docs`) — search the bundled KCS help database.
pub fn run_help(
    query: &[String],
    dialect: Option<&DialectProfile>,
    limit: usize,
    json: bool,
    output: Option<&std::path::Path>,
) -> anyhow::Result<u8> {
    if limit < 1 {
        anyhow::bail!("--limit must be >= 1");
    }

    let (conn, db_path) = load_db()?;
    let result = run_help_inner(&conn, query, dialect, limit, json, output);
    drop(conn);
    let _ = std::fs::remove_file(&db_path);
    result
}

fn run_help_inner(
    conn: &Connection,
    query: &[String],
    dialect: Option<&DialectProfile>,
    limit: usize,
    json: bool,
    output: Option<&std::path::Path>,
) -> anyhow::Result<u8> {
    let target = OutputTarget::from_arg(output);
    let query_str = query.join(" ");
    let query_str = query_str.trim();

    if query_str.is_empty() {
        let mut catalogue = list_features(conn)?;
        if dialect.is_some() {
            for (_, features) in &mut catalogue {
                features.retain(|f| f.matches_profile(dialect));
            }
            catalogue.retain(|(_, features)| !features.is_empty());
        }
        if json {
            write_text_output(&target, &catalogue_json(&catalogue))?;
        } else {
            write_text_output(
                &target,
                &with_dialect_header(dialect, render_catalogue(&catalogue)),
            )?;
        }
        return Ok(u8::from(catalogue.is_empty()));
    }

    let search_limit = if dialect.is_none() {
        limit
    } else {
        (limit * 5).max(limit)
    };
    let mut results = search_help(conn, query_str, search_limit)?;
    results.retain(|f| f.matches_profile(dialect));
    results.truncate(limit);

    if json {
        let arr: Vec<Value> = results.iter().map(search_result_json).collect();
        let text = ensure_ascii(&serde_json::to_string_pretty(&Value::Array(arr))?);
        write_text_output(&target, &text)?;
    } else if results.is_empty() {
        eprintln!("no KCS help matches for '{query_str}'");
    } else {
        write_text_output(
            &target,
            &with_dialect_header(dialect, render_search_results(&results, query_str)),
        )?;
    }

    if !results.is_empty() {
        return Ok(0);
    }

    if !json {
        let catalogue = list_features(conn)?;
        if !catalogue.is_empty() {
            let mut cats: Vec<&(String, Vec<Feature>)> = catalogue.iter().collect();
            cats.sort_by_key(|(c, _)| c.to_lowercase());
            let mut err = std::io::stderr().lock();
            let _ = writeln!(err, "available sections:");
            for (category, _) in cats {
                let _ = writeln!(err, "  {category}");
            }
        }
    }
    Ok(1)
}

/// JSON for the catalogue: `{category: [feature, ...]}`
/// (2-space-indented JSON), category order preserved.
fn catalogue_json(catalogue: &[(String, Vec<Feature>)]) -> String {
    let mut obj = Map::new();
    for (category, features) in catalogue {
        let arr: Vec<Value> = features.iter().map(catalogue_feature_json).collect();
        obj.insert(category.clone(), Value::Array(arr));
    }
    ensure_ascii(
        &serde_json::to_string_pretty(&Value::Object(obj)).unwrap_or_else(|_| "{}".to_owned()),
    )
}

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

//! `mro_overrides` — measures how often `TclOO` methods are **overridden**
//! across a class hierarchy, to size the value of a rename that spans
//! overrides.
//!
//! A method rename that touches only the class under the cursor silently
//! breaks the override relationship when the same method is (re)defined by
//! a super- or sub-class: after the rewrite the sibling definition no longer
//! overrides anything, and `$obj method` dispatch changes meaning.  This
//! harness reports, over a corpus:
//!
//! * total direct method definitions (a class defining a method in its own
//!   body — not inherited),
//! * how many of those are part of an **override family**: ≥2 classes in the
//!   same inheritance-connected component define the same method name, and
//! * the distribution of override-family sizes.
//!
//! Usage:
//!
//! ```text
//! cargo run -p tcl-compiler --example mro_overrides -- experiments/corpus
//! ```
//!
//! Nothing here touches shipping diagnostics.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use tcl_compiler::analyser::class_hierarchy::{ClassHierarchy, build_class_hierarchy};
use tcl_compiler::analyser::state::Analyser;
use tcl_compiler::analyser::types::ClassDef;

fn collect_tcl_files(root: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            if matches!(name, ".git" | "target" | "node_modules" | ".venv") {
                continue;
            }
            collect_tcl_files(&path, out);
        } else if path
            .extension()
            .and_then(|s| s.to_str())
            .is_some_and(|ext| ext == "tcl" || ext == "test")
        {
            out.push(path);
        }
    }
}

/// Analyse one file and return its class index (`None` on read error /
/// oversize / panic).
fn analyse_file(path: &Path) -> Option<HashMap<String, ClassDef>> {
    let src = std::fs::read_to_string(path).ok()?;
    if src.len() > 2_000_000 {
        return None;
    }
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut a = Analyser::new();
        a.analyse(&src, "tcl8.6").all_classes
    }))
    .ok()
}

/// `true` when `a` and `b` sit in the same inheritance-connected component
/// — one is a (transitive) subtype of the other.  This is the set a method
/// rename must stay consistent across: a polymorphic `$obj method` call can
/// reach any definition along the chain.
fn connected(h: &ClassHierarchy, a: &str, b: &str) -> bool {
    a == b || h.is_subtype(a, b) || h.is_subtype(b, a)
}

/// Lossless-enough `usize` → `f64` for count/ratio reporting: saturates at
/// `u32::MAX` so the cast never silently drops mantissa bits.
fn as_f64(n: usize) -> f64 {
    f64::from(u32::try_from(n).unwrap_or(u32::MAX))
}

/// Union-find root of `x` with path compression.
fn uf_find(parent: &mut [usize], x: usize) -> usize {
    let mut r = x;
    while parent[r] != r {
        r = parent[r];
    }
    let mut cur = x;
    while parent[cur] != r {
        let next = parent[cur];
        parent[cur] = r;
        cur = next;
    }
    r
}

/// Resolve corpus roots from CLI args, falling back to a built-in list of
/// well-known locations.  Exits the process when nothing usable is found.
fn resolve_roots() -> Vec<PathBuf> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let roots: Vec<PathBuf> = if args.is_empty() {
        [
            "experiments/corpus",
            "tmp/tcllib-2.0",
            "tmp/tcl8.6.18/tests",
            "tmp/tcl9.0.4/tests",
        ]
        .iter()
        .map(PathBuf::from)
        .filter(|p| p.exists())
        .collect()
    } else {
        args.iter().map(PathBuf::from).collect()
    };
    if roots.is_empty() {
        eprintln!("no corpus roots found; pass directories as arguments");
        std::process::exit(1);
    }
    roots
}

/// Walk every root, gather the Tcl files, and return them sorted.
fn discover_files(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for r in roots {
        collect_tcl_files(r, &mut files);
    }
    files.sort();
    println!("discovered {} Tcl files", files.len());
    files
}

/// Analyse every file, returning each file's class index and a corpus-wide
/// merged index (first definition wins on a name clash).
fn build_index(files: &[PathBuf]) -> (Vec<HashMap<String, ClassDef>>, HashMap<String, ClassDef>) {
    // Cache each file's class index so the within-file pass below reuses it
    // rather than re-analysing every file a second time.
    let mut per_file: Vec<HashMap<String, ClassDef>> = Vec::new();
    let mut merged: HashMap<String, ClassDef> = HashMap::new();
    for path in files {
        let Some(classes) = analyse_file(path) else {
            continue;
        };
        for (k, v) in &classes {
            merged.entry(k.clone()).or_insert_with(|| v.clone());
        }
        per_file.push(classes);
    }
    println!("merged class index: {} classes", merged.len());
    (per_file, merged)
}

/// Every direct method definition, indexed `method -> [class]`, plus the
/// total definition count.  Public + private + unexported all count — a
/// rename must catch them regardless of visibility.  Constructors/destructors
/// are excluded (not renameable).  Definer lists are sorted for run-to-run
/// deterministic grouping.
fn collect_definers(merged: &HashMap<String, ClassDef>) -> (HashMap<String, Vec<String>>, usize) {
    let mut definers: HashMap<String, Vec<String>> = HashMap::new();
    let mut total_defs = 0usize;
    for (qname, cd) in merged {
        for method in cd.methods.keys().chain(cd.class_methods.keys()) {
            definers
                .entry(method.clone())
                .or_default()
                .push(qname.clone());
            total_defs += 1;
        }
    }
    for classes in definers.values_mut() {
        classes.sort();
    }
    (definers, total_defs)
}

/// Corpus-wide override-family report.  `override_defs` uses an
/// order-independent pairwise existence test (deterministic); `family_sizes`
/// groups the definers into true connected components via union-find so the
/// histogram is stable across runs.
fn report_families(
    hierarchy: &ClassHierarchy,
    definers: &HashMap<String, Vec<String>>,
    total_defs: usize,
) {
    let mut override_defs = 0usize;
    let mut family_sizes: Vec<usize> = Vec::new();
    for classes in definers.values() {
        if classes.len() < 2 {
            continue;
        }
        // A definition is an override iff some *other* definer is connected.
        for i in 0..classes.len() {
            if (0..classes.len()).any(|j| j != i && connected(hierarchy, &classes[i], &classes[j]))
            {
                override_defs += 1;
            }
        }
        // Union-find over this method's definers for component sizes.
        let n = classes.len();
        let mut parent: Vec<usize> = (0..n).collect();
        for i in 0..n {
            for j in (i + 1)..n {
                if connected(hierarchy, &classes[i], &classes[j]) {
                    let (ri, rj) = (uf_find(&mut parent, i), uf_find(&mut parent, j));
                    if ri != rj {
                        parent[ri] = rj;
                    }
                }
            }
        }
        let mut sizes: HashMap<usize, usize> = HashMap::new();
        for i in 0..n {
            let r = uf_find(&mut parent, i);
            *sizes.entry(r).or_default() += 1;
        }
        for &s in sizes.values() {
            if s >= 2 {
                family_sizes.push(s);
            }
        }
    }

    let pct = |n: usize| {
        if total_defs == 0 {
            0.0
        } else {
            100.0 * as_f64(n) / as_f64(total_defs)
        }
    };
    println!("\n## results");
    println!("total direct method definitions: {total_defs}");
    println!(
        "definitions in an override family (≥2 connected classes define the same method): {} ({:.1}%)",
        override_defs,
        pct(override_defs),
    );
    println!("override families: {}", family_sizes.len());
    if !family_sizes.is_empty() {
        let max = family_sizes.iter().copied().max().unwrap_or(0);
        let sum: usize = family_sizes.iter().sum();
        let mut hist: HashMap<usize, usize> = HashMap::new();
        for &s in &family_sizes {
            *hist.entry(s).or_default() += 1;
        }
        let mut sizes: Vec<usize> = hist.keys().copied().collect();
        sizes.sort_unstable();
        println!(
            "family size: mean {:.1}, max {max}",
            as_f64(sum) / as_f64(family_sizes.len()),
        );
        println!("family-size histogram (classes-per-family: count):");
        for s in sizes {
            println!("  {s}: {}", hist[&s]);
        }
    }
}

/// Within-file portion: what a *single-document* rename can catch.
/// Re-analyse each file in isolation and count override families whose every
/// definer lives in that same file.
fn report_within_file(per_file: &[HashMap<String, ClassDef>]) {
    let mut single_doc_defs = 0usize;
    let mut single_doc_override_defs = 0usize;
    for classes in per_file {
        if classes.len() < 2 {
            // Still counts its method defs toward the denominator.
            for cd in classes.values() {
                single_doc_defs += cd.methods.len() + cd.class_methods.len();
            }
            continue;
        }
        let h = build_class_hierarchy(classes.clone());
        let mut defs_here: HashMap<String, Vec<String>> = HashMap::new();
        for (q, cd) in classes {
            for m in cd.methods.keys().chain(cd.class_methods.keys()) {
                defs_here.entry(m.clone()).or_default().push(q.clone());
                single_doc_defs += 1;
            }
        }
        let mut counted: std::collections::HashSet<(String, String)> =
            std::collections::HashSet::new();
        for (method, defms) in &defs_here {
            if defms.len() < 2 {
                continue;
            }
            for i in 0..defms.len() {
                for j in (i + 1)..defms.len() {
                    if connected(&h, &defms[i], &defms[j]) {
                        for c in [&defms[i], &defms[j]] {
                            if counted.insert((c.clone(), method.clone())) {
                                single_doc_override_defs += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    let sd_pct = |n: usize| {
        if single_doc_defs == 0 {
            0.0
        } else {
            100.0 * as_f64(n) / as_f64(single_doc_defs)
        }
    };
    println!("\n## within-file (what single-document rename catches)");
    println!("total direct method definitions (per-file): {single_doc_defs}");
    println!(
        "definitions in a *within-file* override family: {} ({:.1}%)",
        single_doc_override_defs,
        sd_pct(single_doc_override_defs),
    );
}

/// Print a few concrete example override families, sorted for determinism.
fn report_examples(hierarchy: &ClassHierarchy, definers: &HashMap<String, Vec<String>>) {
    let mut examples: Vec<(String, Vec<String>)> = Vec::new();
    for (method, classes) in definers {
        if classes.len() >= 2 {
            for i in 0..classes.len() {
                for j in (i + 1)..classes.len() {
                    if connected(hierarchy, &classes[i], &classes[j]) {
                        let mut fam = classes.clone();
                        fam.sort();
                        examples.push((method.clone(), fam));
                        break;
                    }
                }
            }
        }
    }
    examples.sort();
    examples.dedup();
    println!("\n## example override families (method: classes)");
    for (method, classes) in examples.iter().take(15) {
        println!("  {method}: {classes:?}");
    }
}

fn main() {
    let roots = resolve_roots();

    println!("# mro_overrides — method-override frequency across a corpus");
    println!(
        "corpus roots: {:?}",
        roots
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
    );

    let files = discover_files(&roots);
    let (per_file, merged) = build_index(&files);
    let hierarchy = build_class_hierarchy(merged.clone());
    let (definers, total_defs) = collect_definers(&merged);

    report_families(&hierarchy, &definers, total_defs);
    report_within_file(&per_file);
    report_examples(&hierarchy, &definers);
}

//! Generate the Zed tree-sitter highlight queries (`highlights.scm`) command
//! classification from the Rust command registry — so the coarse, offline
//! first-paint highlight layer stops drifting from `tcl-registry`, picks up new
//! commands automatically on `make generate`, and covers every editor language
//! (not just Tcl).
//!
//! ## What is and isn't generated
//!
//! A tree-sitter grammar is a *separate parser* from our lexer, so we cannot
//! make it track our parse tree. What we can do is stop hand-maintaining the
//! `(#any-of? …)` command lists: the registry already knows every command, its
//! dialect availability, and how it is classified (`CONTROL_FLOW`,
//! `LANGUAGE_KEYWORD`, otherwise a builtin), so we project those buckets into
//! one query per language — excluding the `tclUnixTest.c` debug-build
//! commands and `tcl::`-internal-namespace noise
//! ([`util::is_internal_or_test_harness_noise`]) that are real registry
//! entries but not "common" by any definition (see
//! [`crate::gen_tmlanguage_keywords`], which shares this filter).
//!
//! The structural rules (comments, strings, numbers, variables, operators,
//! punctuation) and the grammar's *dedicated-node* keywords (`if`, `while`,
//! `foreach`, `proc`, `namespace`, `try`, …) stay as a static template: those
//! are matched by grammar node type, not by command name, so the registry has
//! nothing to say about them.
//!
//! ## Dialect awareness — and its hard limit
//!
//! Each editor language directory maps to a [`DialectSet`], and we emit a query
//! scoped to it (Tcl+Tk, iRules, iApps, Expect). But a tree-sitter query is
//! *static per file type*: it cannot know whether a given `.tcl` file targets
//! 8.4 or 9.0, and Zed cannot swap queries by a runtime dialect setting. So the
//! base layer uses an over-approximating **union** per language (e.g. all Tcl
//! versions + Tk); a command added in 9.0 will paint as a builtin even in an
//! 8.4 file. That is intentional: precise per-version correctness is the LSP
//! **semantic-token** layer's job (the server knows the configured dialect and
//! its tokens paint over this one). Over-inclusion here is a harmless cosmetic
//! over-approximation, never a wrong diagnostic.
//!
//! Run `cargo xtask gen-zed-queries` to (re)write the files; `--check` verifies
//! the committed files match, exiting non-zero on drift.

use std::collections::BTreeSet;
use std::fs;
use std::process::ExitCode;

use anyhow::{Context, Result};
use tcl_dialect::{DialectProfile, DialectSet};
use tcl_registry::CommandRegistry;
use tcl_registry::traits::Traits;

use crate::util::{self, repo_root};

/// A language directory we emit a query for, and the dialect profile whose
/// static-grammar surface it covers.
struct Target {
    /// Directory under `editors/zed/languages/`.
    dir: &'static str,
    /// The dialect profile this language renders. The first-paint layer
    /// takes the profile's `grammar_union` — the deliberately coarse
    /// over-approximation for static grammars (dialect-profile-model.md
    /// §10) — and its `base_layers` command packs.
    profile: &'static DialectProfile,
}

/// Command words the tree-sitter-tcl grammar parses as dedicated node types
/// rather than generic `(command …)` nodes. They are highlighted structurally
/// by the static template, so we must not also list them in the generated
/// `#any-of?` buckets.
const GRAMMAR_NODE_WORDS: &[&str] = &[
    "if",
    "else",
    "elseif",
    "while",
    "foreach",
    "for",
    "proc",
    "namespace",
    "try",
    "catch",
    "finally",
    "set",
    "expr",
    "switch",
];

/// The profile's registry: `build_default` plus its on-demand command
/// packs, stamped with the profile so mask queries apply its subtractive
/// rules (§9 — the iRules disable list and operator-head exclusion).
fn registry_for(profile: &'static DialectProfile) -> &'static CommandRegistry {
    tcl_registry::registry_for_profile(profile)
}

/// The languages we generate queries for. iRules/iApps/Expect share the `tcl`
/// grammar and today ship *no* query files at all, so this also gives them a
/// first-paint highlight layer for the first time.
fn targets() -> Vec<Target> {
    vec![
        Target {
            // The PLAIN_TCL profile: grammar_union == ALL_TCL.  Tk is
            // filtered out by `required_package` (it needs `package
            // require Tk`), so the ambient Tcl core is just ALL_TCL here.
            dir: "tcl",
            profile: DialectProfile::plain_tcl(),
        },
        Target {
            // iRules are SUBTRACTIVE (dialect-profile-model.md §9): a Tcl
            // 8.4 base with a long list of disabled commands plus the F5
            // command surface — NOT the full Tcl-version union.  The
            // profile's grammar_union is deliberately the bare `IRULES`
            // bit (the shipped highlight fix): a command is projected iff
            // `spec_visible` holds under `IRULES` — i.e. it is
            // dialect-agnostic (`None`, e.g. `set`/`if`/`const`, which is
            // deliberately kept valid in iRules) or carries the `IRULES`
            // bit (the F5 surface), *minus* the profile's subtractive
            // rules: the disable list (`exec`, `file`, `socket`, …) and
            // the math-operator heads (`Traits::OPERATOR_COMMAND` —
            // operators live only inside `expr` in iRules).  Post-8.4
            // commands gate to `TCL85_PLUS` / `TCL86_PLUS` / `TCL90_PLUS`
            // (e.g. `dict`, `lassign`, `zipfs`), so they are excluded by
            // the mask itself — iRules is the Tcl 8.4 base, not 8.5+.
            dir: "irules",
            profile: DialectProfile::by_name("f5-irules"),
        },
        Target {
            dir: "iapps",
            profile: DialectProfile::by_name("f5-iapps"),
        },
        Target {
            dir: "expect",
            profile: DialectProfile::by_name("expect"),
        },
    ]
}

/// The three command buckets projected from the registry for one dialect set.
#[derive(Default)]
struct Buckets {
    control: BTreeSet<String>,
    keyword: BTreeSet<String>,
    builtin: BTreeSet<String>,
}

impl Buckets {
    fn total(&self) -> usize {
        self.control.len() + self.keyword.len() + self.builtin.len()
    }
}

/// Classify every command available in `dialects` into a highlight bucket.
///
/// Only the **ambient** command surface is projected — commands with a
/// `required_package` (all of Tk, tcllib, stdlib packages, itcl, ticklecharts)
/// are only available after a `package require`, which is per-file state the
/// static query cannot know. Those are deliberately left to the LSP semantic-
/// token layer, which sees the file's requires. iRules/iApps/Expect commands
/// carry no package (they are ambient in their runtimes), so they stay.
///
/// Namespaced ambient commands are kept: they are the *point* of the dialect
/// scoping (the F5 surface — `HTTP::uri`, `LB::server` — is entirely
/// namespaced), and the tcl grammar parses `ns::cmd` as one `simple_word`.
fn classify(reg: &CommandRegistry, dialects: DialectSet) -> Buckets {
    let mut b = Buckets::default();

    for name in reg.command_names() {
        if name == util::DISABLED_SENTINEL || GRAMMAR_NODE_WORDS.contains(&name) {
            continue;
        }
        if !util::is_safe_command_name(name) || util::is_internal_or_test_harness_noise(name) {
            continue;
        }
        let Some(spec) = reg.get(name) else { continue };
        if !reg.spec_visible(spec, dialects) {
            continue;
        }
        // Library commands (Tk / tcllib / stdlib packages) load dynamically via
        // `package require`; the LSP handles those. Keep only ambient
        // commands — including packages the profile ships ambiently (§7.1:
        // the F5 surfaces are the profile's own runtime, not a require).
        if let Some(pkg) = spec.required_package
            && !reg.profile().is_some_and(|p| p.is_ambient_package(pkg))
        {
            continue;
        }

        if spec.traits.contains(Traits::CONTROL_FLOW) {
            b.control.insert(name.to_owned());
        } else if spec.traits.contains(Traits::LANGUAGE_KEYWORD) {
            b.keyword.insert(name.to_owned());
        } else {
            b.builtin.insert(name.to_owned());
        }
    }
    b
}

/// Render a sorted set as `#any-of?` argument tokens, wrapped at 6 per line
/// with 4-space continuation indent.
fn render_any_of(names: &BTreeSet<String>) -> String {
    const PER_LINE: usize = 6;
    if names.is_empty() {
        // A `#any-of?` needs at least one argument; emit a sentinel that can
        // never match a real command word rather than an empty (invalid) list.
        return "    \"\\u0000never\"".to_owned();
    }
    names
        .iter()
        .map(|n| format!("\"{n}\""))
        .collect::<Vec<_>>()
        .chunks(PER_LINE)
        .map(|chunk| format!("    {}", chunk.join(" ")))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Build a full `highlights.scm` from the static template and the registry
/// buckets for one language's dialect set.
fn render(reg: &CommandRegistry, dialects: DialectSet) -> String {
    let b = classify(reg, dialects);
    format!(
        r#"; Tcl-family highlights for Zed
;
; DO NOT EDIT — the command-classification `#any-of?` lists below are generated
; from `tcl-registry` (scoped to this language's dialect) by
; `cargo xtask gen-zed-queries`. Structural rules are a static template in that
; generator. Run `make generate` after registry changes.

; Comments
(comment) @comment

; Strings and escapes
(quoted_word) @string
(escaped_character) @string.escape

; Numbers
(number) @number

((simple_word) @number
  (#match? @number "^[+-]?[0-9]+$"))

((simple_word) @boolean
  (#any-of? @boolean "true" "false"))

; Variables
(variable_substitution) @variable

(set (id) @variable)

(argument
  name: (_) @variable.parameter)

; Tcl built-in variables
((simple_word) @variable.builtin
  (#any-of? @variable.builtin
    "argc" "argv" "argv0" "auto_path" "env" "errorCode" "errorInfo"
    "tcl_interactive" "tcl_library" "tcl_nonwordchars" "tcl_patchLevel"
    "tcl_pkgPath" "tcl_platform" "tcl_precision" "tcl_rcFileName"
    "tcl_traceCompile" "tcl_traceExec" "tcl_wordchars" "tcl_version"))

; Procedure definitions
"proc" @keyword.function

(procedure
  name: (_) @function.definition)

; Control-flow keywords parsed as dedicated grammar nodes
["if" "else" "elseif"] @keyword.control
["while" "foreach"] @keyword.control
["catch" "try" "finally"] @keyword.control
["set" "namespace"] @keyword
"expr" @function.builtin

; --- generated from tcl-registry: control-flow commands ---
(command
  name: (simple_word) @keyword.control
  (#any-of? @keyword.control
{control}))

; --- generated from tcl-registry: language keywords ---
(command
  name: (simple_word) @keyword
  (#any-of? @keyword
{keyword}))

; --- generated from tcl-registry: built-in commands ---
(command
  name: (simple_word) @function.builtin
  (#any-of? @function.builtin
{builtin}))

; Highlight unset / variable arguments as variables
(command
  name: (simple_word) @_kw
  arguments: (word_list) @variable
  (#any-of? @_kw "unset" "variable"))

; Generic command calls
(command
  name: (simple_word) @function)

; Operators
(unpack) @operator

["**" "/" "*" "%" "+" "-"
 "<<" ">>"
 ">" "<" ">=" "<="
 "==" "!="
 "eq" "ne"
 "in" "ni"
 "&" "^" "|"
 "&&" "||"
] @operator

; Punctuation
["{{" "}}" "[" "]" ";"] @punctuation.bracket
"#,
        control = render_any_of(&b.control),
        keyword = render_any_of(&b.keyword),
        builtin = render_any_of(&b.builtin),
    )
}

/// The `(relative-path, content)` query file for each target language.
fn generated_files() -> Vec<(String, String)> {
    targets()
        .into_iter()
        .map(|t| {
            let reg = registry_for(t.profile);
            (
                format!("editors/zed/languages/{}/highlights.scm", t.dir),
                render(reg, t.profile.grammar_union),
            )
        })
        .collect()
}

/// Write (or, with `check`, verify) the per-language query files and print a
/// coverage report to stderr.
pub fn run(check: bool) -> Result<ExitCode> {
    let root = repo_root();

    for t in targets() {
        let reg = registry_for(t.profile);
        let b = classify(reg, t.profile.grammar_union);
        eprintln!(
            "  {:<7} {:>4} commands ({} control, {} keyword, {} builtin)",
            t.dir,
            b.total(),
            b.control.len(),
            b.keyword.len(),
            b.builtin.len(),
        );
    }

    let mut drift = Vec::new();
    for (rel, content) in generated_files() {
        let path = root.join(&rel);
        if check {
            let current =
                fs::read_to_string(&path).with_context(|| format!("reading {}", path.display()))?;
            if current != content {
                drift.push(rel);
            }
        } else {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).ok();
            }
            fs::write(&path, &content).with_context(|| format!("writing {}", path.display()))?;
            eprintln!("wrote {rel}");
        }
    }

    if check && !drift.is_empty() {
        eprintln!(
            "{} Zed query file(s) are stale — run `cargo xtask gen-zed-queries`:",
            drift.len()
        );
        for rel in &drift {
            eprintln!("  - {rel}");
        }
        return Ok(ExitCode::from(1));
    }
    if check {
        eprintln!("OK: all Zed query files are in sync with the registry.");
    }
    Ok(ExitCode::SUCCESS)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tcl_bucket_dwarfs_the_old_hand_list_and_is_disjoint() {
        // build_default already carries the Tk surface, so the plain-Tcl
        // profile's registry classifies the TK_AND_TCL union directly.
        let reg = registry_for(DialectProfile::plain_tcl());
        let b = classify(reg, DialectSet::TK_AND_TCL);
        assert!(
            b.builtin.len() > 100,
            "expected the registry builtin surface to dwarf the ~30-entry hand list, got {}",
            b.builtin.len()
        );
        assert!(b.control.is_disjoint(&b.keyword));
        assert!(b.control.is_disjoint(&b.builtin));
        assert!(b.keyword.is_disjoint(&b.builtin));
        // Grammar-node words are never emitted.
        for w in GRAMMAR_NODE_WORDS {
            assert!(!b.control.contains(*w) && !b.keyword.contains(*w) && !b.builtin.contains(*w));
        }
    }

    #[test]
    fn irules_surface_exceeds_plain_tcl() {
        // Loading the iRules dialect must add the F5 command surface on top of
        // core Tcl — the whole point of dialect scoping.
        let tcl_dialects = DialectSet::TK_AND_TCL;
        let irules_dialects = DialectSet::ALL_TCL.union(DialectSet::IRULES);
        let tcl = classify(registry_for(DialectProfile::plain_tcl()), tcl_dialects).total();
        let irules = classify(
            registry_for(DialectProfile::by_name("f5-irules")),
            irules_dialects,
        )
        .total();
        assert!(
            irules > tcl,
            "iRules ({irules}) should exceed plain Tcl ({tcl}) once F5 commands load"
        );
    }

    #[test]
    fn committed_queries_match_generated() {
        let root = repo_root();
        for (rel, content) in generated_files() {
            let path = root.join(&rel);
            let current = fs::read_to_string(&path)
                .unwrap_or_else(|e| panic!("reading {}: {e}", path.display()));
            assert_eq!(
                current, content,
                "{rel} is stale — run `cargo xtask gen-zed-queries`"
            );
        }
    }

    /// Extract the quoted command tokens inside the three generated `#any-of?`
    /// blocks of a committed `highlights.scm` — the actual data shipped to Zed,
    /// parsed independently of how the generator formats it.
    fn committed_generated_commands(src: &str) -> BTreeSet<String> {
        let start = src
            .find("--- generated from tcl-registry: control-flow")
            .expect("generated-block start marker present");
        let end = src
            .find("; Highlight unset")
            .expect("post-generated marker present");
        let mut out = BTreeSet::new();
        let mut rest = &src[start..end];
        while let Some(i) = rest.find('"') {
            rest = &rest[i + 1..];
            let Some(j) = rest.find('"') else { break };
            let tok = &rest[..j];
            // Capture names (`@function.builtin`) are unquoted, so only real
            // command tokens land here; skip the empty-bucket sentinel.
            if !tok.is_empty() && tok != "\\u0000never" {
                out.insert(tok.to_owned());
            }
            rest = &rest[j + 1..];
        }
        out
    }

    /// The strongest freshness guarantee: the command data actually present in
    /// each committed query file must equal what the *live registry* classifies
    /// for that language — parsed from the shipped file, so it holds regardless
    /// of how the generator renders. A registry change that isn't regenerated
    /// fails here even if someone hand-edited the file to look plausible.
    #[test]
    fn committed_command_data_equals_live_registry() {
        for Target { dir, profile } in targets() {
            let reg = registry_for(profile);
            let b = classify(reg, profile.grammar_union);
            let mut expected = BTreeSet::new();
            expected.extend(b.control.iter().cloned());
            expected.extend(b.keyword.iter().cloned());
            expected.extend(b.builtin.iter().cloned());

            let src = fs::read_to_string(
                repo_root().join(format!("editors/zed/languages/{dir}/highlights.scm")),
            )
            .unwrap();
            let committed = committed_generated_commands(&src);

            assert_eq!(
                committed, expected,
                "{dir}: committed tree-sitter command data has drifted from the \
                 registry — run `make generate`"
            );
        }
    }

    /// Ground-truth sentinels — hand-verified facts independent of the
    /// generator's own classification logic, so a `classify`/filter regression
    /// that still round-trips through `--check` is caught.
    #[test]
    fn ground_truth_sentinels() {
        let read = |dir: &str| {
            fs::read_to_string(
                repo_root().join(format!("editors/zed/languages/{dir}/highlights.scm")),
            )
            .unwrap()
        };
        let tcl = read("tcl");
        let irules = read("irules");
        let has = |s: &str, c: &str| s.contains(&format!("\"{c}\""));

        // Ambient core builtins must be present in the Tcl query.
        for c in ["puts", "lappend", "lindex", "incr", "dict", "regsub"] {
            assert!(has(&tcl, c), "tcl query is missing ambient builtin `{c}`");
        }
        // Package-gated library commands (Tk needs `package require Tk`) must be
        // absent — the LSP semantic-token layer colours those, not tree-sitter.
        for c in ["button", "canvas"] {
            assert!(
                !has(&tcl, c),
                "tcl query wrongly includes package-gated `{c}`"
            );
        }
        // Dialect scoping: the ambient F5 surface appears only in iRules.
        assert!(
            has(&irules, "HTTP::uri"),
            "irules query is missing F5 `HTTP::uri`"
        );
        assert!(
            !has(&tcl, "HTTP::uri"),
            "tcl query wrongly includes iRules `HTTP::uri`"
        );
        // Test-harness / internal-namespace noise (shared filter with
        // `gen_tmlanguage_keywords`, see `util::is_internal_or_test_harness_noise`,
        // whose own test module covers the substring-vs-prefix distinction in
        // detail) must not leak into the query the same way it used to.
        for c in ["testalarm", "testchannel", "tcl::process"] {
            assert!(!has(&tcl, c), "tcl query wrongly includes noise `{c}`");
        }
    }
}

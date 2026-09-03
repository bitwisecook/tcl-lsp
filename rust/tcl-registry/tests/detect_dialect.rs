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

//! Content-based dialect detection (`detect_dialect_from_source`).
//!
//! Detection priority: `# tcl-dialect:` directive (first 5 lines) > shebang
//! (first line) > `package require Tcl <x.y>` (first 30 lines). All assertions
//! are structural (string → dialect-name), so no tclsh proof applies.

use tcl_registry::detect_dialect_from_source as detect;

#[test]
fn shebang_tclsh84() {
    assert_eq!(detect("#!/usr/bin/tclsh8.4\nset x 1\n"), Some("tcl8.4"));
}

#[test]
fn shebang_tclsh85() {
    assert_eq!(detect("#!/usr/bin/tclsh8.5\nset x 1\n"), Some("tcl8.5"));
}

#[test]
fn shebang_env_tclsh86() {
    assert_eq!(detect("#!/usr/bin/env tclsh8.6\nset x 1\n"), Some("tcl8.6"));
}

#[test]
fn shebang_tclsh90() {
    assert_eq!(detect("#!/usr/bin/env tclsh9.0\nset x 1\n"), Some("tcl9.0"));
}

#[test]
fn shebang_expect() {
    assert_eq!(detect("#!/usr/bin/expect\nspawn ssh\n"), Some("expect"));
}

#[test]
fn shebang_env_expect() {
    assert_eq!(detect("#!/usr/bin/env expect\nspawn ssh\n"), Some("expect"));
}

#[test]
fn directive_tcl84() {
    assert_eq!(detect("# tcl-dialect: tcl8.4\nset x 1\n"), Some("tcl8.4"));
}

#[test]
fn directive_irules() {
    assert_eq!(
        detect("# tcl-dialect: f5-irules\nwhen HTTP_REQUEST {\n"),
        Some("f5-irules")
    );
}

#[test]
fn directive_on_later_line() {
    let src = "#!/usr/bin/tclsh\n# My script\n# tcl-dialect: tcl8.5\nset x 1\n";
    assert_eq!(detect(src), Some("tcl8.5"));
}

#[test]
fn directive_takes_priority_over_shebang() {
    let src = "#!/usr/bin/tclsh8.6\n# tcl-dialect: tcl8.4\nset x 1\n";
    assert_eq!(detect(src), Some("tcl8.4"));
}

#[test]
fn directive_case_insensitive() {
    assert_eq!(detect("# TCL-DIALECT: tcl9.0\nset x 1\n"), Some("tcl9.0"));
}

#[test]
fn directive_crlf_line_endings() {
    assert_eq!(
        detect("# tcl-dialect: tcl8.4\r\nset x 1\r\n"),
        Some("tcl8.4")
    );
}

#[test]
fn directive_unknown_dialect_ignored() {
    assert_eq!(detect("# tcl-dialect: unknown\nset x 1\n"), None);
}

/// E8 (#1631 §5.1): the directive resolves through the environment
/// registry, so an environment name that is not a catalogue dialect —
/// `tk` — resolves, and so does an alias, to its canonical id.
#[test]
fn directive_resolves_environment_names_and_aliases() {
    assert_eq!(detect("# tcl-dialect: tk\nwm title . hi\n"), Some("tk"));
    assert_eq!(detect("# tcl-dialect: wish\nwm title . hi\n"), Some("tk"));
    assert_eq!(
        detect("# tcl-dialect: irules\nwhen HTTP_REQUEST {\n"),
        Some("f5-irules")
    );
    assert_eq!(detect("# tcl-dialect: jimsh\nset x 1\n"), Some("jim"));
    // The answer is the canonical id, and the resolution is case-sensitive
    // on the name exactly as the settings ingress is.
    assert_eq!(detect("# tcl-dialect: TK\nset x 1\n"), None);
}

/// A pack-declared environment resolves through the directive once the
/// pack registers it — the same live registry the extension routing and
/// `setDialect` read — and stops resolving once it retires.
#[test]
fn directive_resolves_a_registered_environment() {
    use std::sync::Arc;
    use tcl_dialect::model::{
        BuildProfileId, CoreProfileSelector, DetectionFacts, EnvironmentDefinition, EnvironmentId,
        EnvironmentPolicy, Family, Provenance, Release, VersionAxisId, VersionSet, WorldPolicy,
    };
    use tcl_registry::model::{EnvironmentSource, sync_environment_sources};

    let source = "# tcl-dialect: directive-probe-shell\nset x 1\n";
    let alias = "# tcl-dialect: probe-shell\nset x 1\n";
    let definition = EnvironmentDefinition {
        id: EnvironmentId::new("directive-probe-shell"),
        aliases: vec![Arc::from("probe-shell")],
        display_name: Arc::from("Directive Probe"),
        editor_identity: None,
        core: Some(CoreProfileSelector {
            family: Family::Tcl,
            default_release: Release::TCL_8_6,
            build: BuildProfileId::Canonical,
        }),
        targets: VersionSet::from_requirements(VersionAxisId::core(Family::Tcl), &["8.6-"])
            .expect("targets"),
        expected_packages: Vec::new(),
        policy_defaults: EnvironmentPolicy {
            closed_world: WorldPolicy::Open,
            fixed_ensembles: false,
            strict_ascii: false,
            version_ceiling: None,
        },
        server_detection: DetectionFacts::default(),
        help_terms: Vec::new(),
        provenance: Provenance::User,
    };
    let outcome = sync_environment_sources(vec![EnvironmentSource {
        id: "test:directive-probe".to_owned(),
        definitions: vec![definition],
        extensions: Vec::new(),
    }]);
    assert!(outcome.rejected.is_empty(), "{:?}", outcome.rejected);
    assert_eq!(detect(source), Some("directive-probe-shell"));
    assert_eq!(detect(alias), Some("directive-probe-shell"));

    let _ = sync_environment_sources(Vec::new());
    assert_eq!(detect(source), None, "a retired environment abstains again");
}

#[test]
fn no_hint_returns_none() {
    assert_eq!(detect("set x 1\nputs $x\n"), None);
}

#[test]
fn shebang_plain_tclsh_no_version() {
    assert_eq!(detect("#!/usr/bin/tclsh\nset x 1\n"), None);
}

#[test]
fn directive_beyond_scan_window() {
    let mut s = "# line\n".repeat(10);
    s.push_str("# tcl-dialect: tcl8.4\nset x 1\n");
    assert_eq!(detect(&s), None);
}

// --- package-require path + shebang edges ---

#[test]
fn package_require_tcl_version() {
    assert_eq!(
        detect("# header\npackage require Tcl 8.6\n"),
        Some("tcl8.6")
    );
    assert_eq!(detect("package require -exact Tcl 9.0\n"), Some("tcl9.0"));
    // C Tcl 9 registers its core package under both `Tcl` and lower-case
    // `tcl`; its own bundled init.tcl uses the latter spelling.
    assert_eq!(detect("package require -exact tcl 9.0.3\n"), Some("tcl9.0"));
}

#[test]
fn package_require_non_tcl_ignored() {
    // Tcl 8 rejects lower-case `tcl`, and C Tcl does not accept arbitrary
    // case-folding (`TCL` remains a different package name).
    assert_eq!(detect("package require tcl 8.6\n"), None);
    assert_eq!(detect("package require TCL 9.0\n"), None);
    assert_eq!(detect("package require Tk 8.6\n"), None);
}

// --- tokenised `package vsatisfies [package require Tcl] <x.y>` guard ---
// (the idiomatic runtime minimum-version check — Tcl 9's own tclshrc uses it).

#[test]
fn package_vsatisfies_tcl_version() {
    assert_eq!(
        detect("[package vsatisfies [package require Tcl] 9.0]\n"),
        Some("tcl9.0")
    );
    assert_eq!(
        detect("[package vsatisfies [package require Tcl] 9.1]\n"),
        Some("tcl9.1")
    );
    assert_eq!(
        detect("[package vsatisfies [package require tcl] 9.0]\n"),
        Some("tcl9.0")
    );
    // Nested inside an `if` condition (the tclshrc shape) is still found.
    assert_eq!(
        detect("if {[package vsatisfies [package require Tcl] 9.0]} then {\n  setup\n}\n"),
        Some("tcl9.0")
    );
    // …and nested inside a proc body.
    assert_eq!(
        detect("proc init {} {\n  if {[package vsatisfies [package require Tcl] 9.0]} { x }\n}\n"),
        Some("tcl9.0")
    );
}

#[test]
fn package_vsatisfies_non_tcl_ignored() {
    // A `vsatisfies` over some other package must not pick a Tcl dialect.
    assert_eq!(
        detect("[package vsatisfies [package require Foo] 9.0]\n"),
        None
    );
}

#[test]
fn version_guard_in_comment_or_string_ignored() {
    // Tokenised detection (vs. plain string matching) does not match a
    // commented-out or string-literal `package require` / `vsatisfies`.
    assert_eq!(detect("# package require Tcl 8.4\nset x 1\n"), None);
    assert_eq!(detect("set msg \"package require Tcl 8.4\"\n"), None);
    assert_eq!(
        detect("# [package vsatisfies [package require Tcl] 9.0]\nset x 1\n"),
        None
    );
}

#[test]
fn version_guard_in_braced_data_ignored() {
    // A `package require` inside an *inert* braced word (data, not a script or
    // expression body) must not select a dialect: the scan recurses into braced
    // words only at script/expr positions, never arbitrary data arguments.
    assert_eq!(detect("set msg {package require Tcl 8.4}\n"), None);
    assert_eq!(detect("lappend cmds {package require Tcl 8.4}\n"), None);
    // The gate applies at every nesting level, so a data literal buried inside a
    // real script body stays inert too.
    assert_eq!(
        detect("if {$c} {\n  set msg {package require Tcl 8.4}\n}\n"),
        None
    );
    // …but a genuine `package require` in an executed script body IS found.
    assert_eq!(
        detect("if {$c} {\n  package require Tcl 9.0\n}\n"),
        Some("tcl9.0")
    );
}

// SpecTcl — `.tclspec` command packs (`docs/design/spec-packs.md`).

/// The extension is the registration `spec-packs.md` calls for: "the editor
/// extensions and the LSP register it as Tcl in the `SpecTcl` dialect, so a
/// pack file gets the full editor experience with no configuration."
#[test]
fn tclspec_extension_selects_the_spectcl_dialect() {
    use tcl_registry::dialects::{detect_dialect, dialect_from_extension};

    assert_eq!(dialect_from_extension("mylib.tclspec"), Some("spectcl"));
    // Case-folded and path-qualified, like every other extension rule.
    assert_eq!(
        dialect_from_extension("/pkg/.tcl-lsp/MyLib.TclSpec"),
        Some("spectcl")
    );
    // …and it reaches the shared detector, which is what the LSP and the CLI
    // both resolve a document through.
    assert_eq!(
        detect_dialect("command foo { arity 1 }\n", Some("mylib.tclspec"), "tcl9.0"),
        "spectcl"
    );
}

/// `speclib` is the DSL's one loader directive and its only possible
/// top-level word, so it is also a content signature — which is what
/// recognises a pack saved under a `.tcl` name, the case the extension tier
/// cannot reach.
#[test]
fn the_speclib_directive_is_a_content_signature() {
    use tcl_registry::dialects::detect_dialect;

    let pack = "speclib mylib 1.0 {\n    command with_var { arity 2 }\n}\n";
    assert_eq!(detect_dialect(pack, Some("mylib.tcl"), "tcl9.0"), "spectcl");
    // A comment merely mentioning it does not flip the dialect: full-line
    // comments are stripped before the signature scan.
    assert_eq!(
        detect_dialect("# see speclib for the format\nset x 1\n", None, "tcl9.0"),
        "tcl9.0"
    );
    // An explicit directive still outranks it, as does an explicit shebang.
    assert_eq!(
        detect_dialect(
            &format!("# tcl-dialect: tcl8.6\n{pack}"),
            Some("mylib.tclspec"),
            "tcl9.0"
        ),
        "tcl8.6"
    );
}

/// `spectcl` is a first-class catalogue dialect: it names a profile, parses
/// to its own surface, and round-trips through the catalogue.
#[test]
fn spectcl_is_a_catalogued_dialect() {
    use tcl_dialect::KNOWN_DIALECTS;
    use tcl_dialect::model::{Family, SurfaceLayer, SurfaceQuery};

    assert!(KNOWN_DIALECTS.contains(&"spectcl"));
    assert_eq!(
        tcl_dialect::DialectProfile::find("spectcl")
            .map(tcl_dialect::DialectProfile::surface_query),
        Some(SurfaceQuery::core(Family::Tcl, "9.0").with_packages(&["spectcl"]))
    );
    let profile = tcl_registry::model::ingress::resolve_environment("spectcl").analyser_profile();
    assert_eq!(profile.name, "spectcl");
    assert_eq!(profile.base_layers, &[SurfaceLayer::Package("spectcl")]);
    // The editor / MCP spellings canonicalise to it.
    for alias in ["tclspec", "tcl-spec"] {
        assert_eq!(
            tcl_registry::model::ingress::resolve_environment(alias)
                .analyser_profile()
                .name,
            "spectcl"
        );
    }
}

// SslicTcl — `.sslictcl` TLS declarations (issue #1543).

/// The extension registration: a `.sslictcl` document opens as Tcl in the
/// `SslicTcl` dialect with no configuration, exactly as a `.tclspec` does.
#[test]
fn sslictcl_extension_selects_the_sslictcl_dialect() {
    use tcl_registry::dialects::{detect_dialect, dialect_from_extension};

    assert_eq!(dialect_from_extension("site.sslictcl"), Some("sslictcl"));
    // Case-folded and path-qualified, like every other extension rule.
    assert_eq!(
        dialect_from_extension("/etc/tls/Site.SslicTcl"),
        Some("sslictcl")
    );
    // …and it reaches the shared detector, which is what the LSP and the CLI
    // both resolve a document through.
    assert_eq!(
        detect_dialect(
            "endpoint www { hostname www.example.com }\n",
            Some("site.sslictcl"),
            "tcl9.0"
        ),
        "sslictcl"
    );
}

/// The mandatory `sslictcl VERSION` header is a content signature — which is
/// what recognises a document saved under a `.tcl` name, the case the
/// extension tier cannot reach.
///
/// The signature is **structural**, not a bare word: `sslictcl` is ordinary
/// English, so it speaks for the dialect only where it is a command head
/// followed by an integer.
#[test]
fn the_sslictcl_header_is_a_content_signature() {
    use tcl_registry::dialects::detect_dialect;

    let doc = "sslictcl 1\n\nendpoint www {\n    hostname www.example.com\n}\n";
    assert_eq!(detect_dialect(doc, Some("site.tcl"), "tcl9.0"), "sslictcl");
    // Leading whitespace is fine — a header is still a command head.
    assert_eq!(
        detect_dialect("  sslictcl 1\n", Some("site.tcl"), "tcl9.0"),
        "sslictcl"
    );
    // …and so is a header that follows comments and blank lines, which is how
    // a real document opens.
    assert_eq!(
        detect_dialect(
            "# the production edge\n# generated 2026-09-02\n\nsslictcl 1\n",
            Some("site.tcl"),
            "tcl9.0"
        ),
        "sslictcl"
    );
    // An explicit directive still outranks it.
    assert_eq!(
        detect_dialect(
            &format!("# tcl-dialect: tcl8.6\n{doc}"),
            Some("site.sslictcl"),
            "tcl9.0"
        ),
        "tcl8.6"
    );
}

/// The word alone is not the signature: an ordinary Tcl script that merely
/// *mentions* `sslictcl` — as a value, in a string, or in a comment — stays
/// Tcl. A whole-word match would have routed every one of these.
#[test]
fn the_word_sslictcl_alone_does_not_route_a_tcl_script() {
    use tcl_registry::dialects::detect_dialect;

    for source in [
        "set format sslictcl\n",
        "puts \"sslictcl\"\n",
        "# see sslictcl for the format\nset x 1\n",
        "lappend formats sslictcl tclspec\n",
        // The head is right but the argument is not an integer, so this is
        // not a header — it is someone calling a command of their own.
        "sslictcl foo\n",
        // The head is right but there is no argument at all.
        "sslictcl\n",
    ] {
        assert_eq!(
            detect_dialect(source, Some("site.tcl"), "tcl9.0"),
            "tcl9.0",
            "{source:?} must stay Tcl"
        );
    }
    // The extension still routes regardless of content: a `.sslictcl` file is
    // one whatever it says.
    assert_eq!(
        detect_dialect("set format sslictcl\n", Some("site.sslictcl"), "tcl9.0"),
        "sslictcl"
    );
}

/// `sslictcl` is a first-class catalogue dialect: it names a profile, parses
/// to its own surface, and round-trips through the catalogue.
#[test]
fn sslictcl_is_a_catalogued_dialect() {
    use tcl_dialect::KNOWN_DIALECTS;
    use tcl_dialect::model::{Family, SurfaceLayer, SurfaceQuery};

    assert!(KNOWN_DIALECTS.contains(&"sslictcl"));
    assert_eq!(
        tcl_dialect::DialectProfile::find("sslictcl")
            .map(tcl_dialect::DialectProfile::surface_query),
        Some(SurfaceQuery::core(Family::Tcl, "9.0").with_packages(&["sslictcl"]))
    );
    let profile = tcl_registry::model::ingress::resolve_environment("sslictcl").analyser_profile();
    assert_eq!(profile.name, "sslictcl");
    assert_eq!(profile.base_layers, &[SurfaceLayer::Package("sslictcl")]);
    // The editor / MCP spellings canonicalise to it.
    for alias in ["sslic-tcl", "tls-sslictcl"] {
        assert_eq!(
            tcl_registry::model::ingress::resolve_environment(alias)
                .analyser_profile()
                .name,
            "sslictcl"
        );
    }
}

/// Extension→dialect routing derives from the `DialectProfile` catalog's
/// `file_extensions` axis — every owned extension routes to its owner, and
/// the newly catalogued routes (`.scf`, `.tmsh`, the iApp implementation
/// spellings) work exactly like the long-standing vendor ones.
#[test]
fn catalog_owned_extensions_route_to_their_dialect() {
    use tcl_registry::dialects::dialect_from_extension;

    for profile in tcl_dialect::DialectProfile::all() {
        for row in profile.file_extensions {
            assert_eq!(
                dialect_from_extension(&format!("design.{}", row.extension)),
                Some(profile.name),
                "{} should route to {}",
                row.extension,
                profile.name
            );
        }
    }
    // The routes the catalog move newly opened up, spelled concretely.
    assert_eq!(dialect_from_extension("bigip.scf"), Some("f5-bigip"));
    assert_eq!(dialect_from_extension("deploy.tmsh"), Some("f5-tmsh"));
    assert_eq!(dialect_from_extension("app.iappimpl"), Some("f5-iapps"));
    assert_eq!(dialect_from_extension("app.impl"), Some("f5-iapps"));
    assert_eq!(dialect_from_extension("lb.irules"), Some("f5-irules"));
    assert_eq!(dialect_from_extension("login.expect"), Some("expect"));
    // Deliberate non-mappings stay unmapped.
    assert_eq!(dialect_from_extension("rules.svrf"), None);
    assert_eq!(dialect_from_extension("main.tcl"), None);
}

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

use super::*;

struct Fixture(PathBuf);
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
fn root() -> Fixture {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let directory = std::env::temp_dir().join(format!(
        "source-owner-fixture-{}-{sequence}",
        std::process::id()
    ));
    std::fs::create_dir(&directory).unwrap();
    std::fs::create_dir_all(directory.join("rust/a/src/shared")).unwrap();
    for (file, contents) in [
        ("Cargo.toml", "[package]\nname = \"a\"\n"),
        ("src/lib.rs", "mod shared;"),
        ("src/shared.rs", "mod owner; mod caller;"),
        ("src/shared/owner.rs", "pub(super) fn selected() {}"),
        (
            "src/shared/caller.rs",
            "fn consume() { super::owner::selected(); }",
        ),
    ] {
        std::fs::write(directory.join("rust/a").join(file), contents).unwrap();
    }
    Fixture(directory)
}
fn manifest() -> String {
    format!(
        "{START}\n| Surface | Owner source | Crate | Owner module | Entry | Visibility | Consumer source | Consumer module | Consumer entry | Route |\n| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |\n| source | `rust/a/src/shared/owner.rs` | `a` | `crate::shared::owner` | `selected` | pub(super) | `rust/a/src/shared/caller.rs` | `crate::shared::caller` | `consume` | `static` |\n{END}"
    )
}

#[test]
fn actual_file_routes_keep_exact_source_crate_module_and_consumer_identity() {
    let fixture = root();
    let (coverage, problems) = validate(&fixture.0, &manifest());
    assert!(problems.is_empty(), "{problems:?}");
    assert_eq!(coverage.len(), 1);
    for changed in [
        manifest().replace("| `a` |", "| `other` |"),
        manifest().replace("crate::shared::owner", "crate::shared::other"),
        manifest().replace("rust/a/src/shared/owner.rs", "rust/a/src/shared/caller.rs"),
        manifest().replace("| `consume` |", "| `changed` |"),
        manifest().replace(
            "rust/a/src/shared/caller.rs",
            "rust/other/src/shared/caller.rs",
        ),
    ] {
        let (coverage, problems) = validate(&fixture.0, &changed);
        assert!(coverage.is_empty());
        assert_ne!(problems.len(), 0);
    }
}

#[test]
fn direct_self_module_routes_keep_the_existing_tree_bound_without_unrelated_loading() {
    let fixture = root();
    let source = "pub(super) struct Ledger; impl Ledger { pub(super) fn selected(&self) {} fn consume(&self) { self.selected(); } }";
    std::fs::write(fixture.0.join("rust/a/src/shared/owner.rs"), source).unwrap();
    let mut library = String::from("mod shared;\n");
    for index in 0..2050 {
        let module = format!("unrelated{index}");
        library.push_str("mod ");
        library.push_str(&module);
        library.push_str(";\n");
        std::fs::write(fixture.0.join(format!("rust/a/src/{module}.rs")), "").unwrap();
    }
    std::fs::write(fixture.0.join("rust/a/src/lib.rs"), library).unwrap();
    let mut rows = parse(&manifest()).unwrap();
    let row = rows.first_mut().unwrap();
    row.entry = "Ledger::selected".into();
    row.consumer_entry = "Ledger::consume".into();
    row.consumer_module = row.module.clone();
    row.consumer_source = row.source.clone();
    row.route = "self".into();
    let modules = CrateModules::load_for_route(&fixture.0, Path::new("rust/a"), row).unwrap();
    assert!(modules.check_row(row));
    assert!(modules.modules.len() < 2048);
    assert!(
        CrateModules::load(
            &fixture.0,
            Path::new("rust/a"),
            [&row.module, &row.consumer_module]
        )
        .is_err()
    );
    row.consumer_entry = "Other::consume".into();
    assert!(CrateModules::load_for_route(&fixture.0, Path::new("rust/a"), row).is_err());
}

#[test]
fn targeted_self_cache_cannot_lend_its_tree_to_full_receiver_routes() {
    let fixture = root();
    let mut library = String::from("mod shared;\n");
    for index in 0..2050 {
        let module = format!("unrelated{index}");
        library.push_str("mod ");
        library.push_str(&module);
        library.push_str(";\n");
        std::fs::write(fixture.0.join(format!("rust/a/src/{module}.rs")), "").unwrap();
    }
    std::fs::write(fixture.0.join("rust/a/src/lib.rs"), library).unwrap();
    std::fs::write(fixture.0.join("rust/a/src/shared/owner.rs"),
            "pub(super) struct Ledger; impl Ledger { pub(super) fn selected(&self) {} fn consume(&self) { self.selected(); } } fn borrow(ledger: &Ledger) { ledger.selected(); }").unwrap();
    let self_row = "| source | `rust/a/src/shared/owner.rs` | `a` | `crate::shared::owner` | `Ledger::selected` | pub(super) | `rust/a/src/shared/owner.rs` | `crate::shared::owner` | `Ledger::consume` | `self` |";
    let borrowed_row = self_row
        .replace("`Ledger::consume`", "`borrow`")
        .replace("`self`", "`parameter:ledger`");
    let text = format!("{START}\n{self_row}\n{borrowed_row}\n{END}");
    let (coverage, failures) = validate(&fixture.0, &text);
    assert_eq!(coverage.len(), 1);
    assert_eq!(failures.len(), 1);
    assert!(failures[0].contains("source module route exceeds bound"));

    // A separately loaded full mode keeps actual std shadow declarations.
    std::fs::write(fixture.0.join("rust/a/src/lib.rs"), "mod shared; mod std;").unwrap();
    std::fs::write(
        fixture.0.join("rust/a/src/std.rs"),
        "pub mod boxed { pub struct Box<T>(T); }",
    )
    .unwrap();
    std::fs::write(fixture.0.join("rust/a/src/shared/owner.rs"),
            "pub(super) struct Ledger; impl Ledger { pub(super) fn selected(&self) {} fn consume(&self) { self.selected(); } } fn borrow(ledger: &std::boxed::Box<Ledger>) { ledger.selected(); }").unwrap();
    let (coverage, failures) = validate(&fixture.0, &text);
    assert_eq!(coverage.len(), 1);
    assert_eq!(failures.len(), 1);
}

#[test]
fn explicit_binary_root_keeps_actual_child_modules_and_refuses_ambiguous_targets() {
    let fixture = root();
    std::fs::rename(
        fixture.0.join("rust/a/src/lib.rs"),
        fixture.0.join("rust/a/src/main.rs"),
    )
    .unwrap();
    let cargo = "[package]\nname = \"a\"\n[[bin]]\nname = \"a\"\npath = \"src/main.rs\"\n";
    std::fs::write(fixture.0.join("rust/a/Cargo.toml"), cargo).unwrap();
    let (coverage, failures) = validate(&fixture.0, &manifest());
    assert!(failures.is_empty(), "{failures:?}");
    assert_eq!(coverage.len(), 1);
    for changed in [
        cargo.replace("src/main.rs", "src/other.rs"),
        format!("{cargo}[[bin]]\nname = \"other\"\npath = \"src/main.rs\"\n"),
        format!("{cargo}[lib]\npath = \"src/custom.rs\"\n"),
    ] {
        std::fs::write(fixture.0.join("rust/a/Cargo.toml"), changed).unwrap();
        let (coverage, failures) = validate(&fixture.0, &manifest());
        assert!(coverage.is_empty());
        assert_ne!(failures.len(), 0);
    }
    std::fs::write(fixture.0.join("rust/a/Cargo.toml"), cargo).unwrap();
    std::fs::create_dir(fixture.0.join("rust/a/src/bin")).unwrap();
    assert_ne!(validate(&fixture.0, &manifest()).1.len(), 0);
}

#[test]
fn file_routes_refuse_path_overrides_conditionals_and_duplicate_modules() {
    for declaration in [
        "#[path = \"alternate.rs\"] mod owner; mod caller;",
        "#[cfg(test)] mod owner; mod caller;",
        "mod owner; mod owner; mod caller;",
        "mod owner { pub(super) fn selected() {} } mod caller;",
    ] {
        let fixture = root();
        std::fs::write(fixture.0.join("rust/a/src/shared.rs"), declaration).unwrap();
        let (coverage, problems) = validate(&fixture.0, &manifest());
        assert!(coverage.is_empty(), "{declaration}");
        assert!(!problems.is_empty(), "{declaration}");
    }
}

#[test]
fn registry_public_facade_private_kernel_keeps_real_self_and_static_bounds() {
    let fixture = root();
    std::fs::write(
            fixture.0.join("rust/a/src/shared/owner.rs"),
            "pub(super) struct Registry; impl Registry { pub(super) fn resolve(&self) { self.resolve_provider(); } fn resolve_provider(&self) { super::caller::build(); } }",
        ).unwrap();
    std::fs::write(
        fixture.0.join("rust/a/src/shared/caller.rs"),
        "pub(super) fn build() {}",
    )
    .unwrap();
    let mut library = String::from("mod shared;\n");
    for index in 0..2050 {
        let module = format!("unrelated{index}");
        library.push_str("mod ");
        library.push_str(&module);
        library.push_str(";\n");
        std::fs::write(fixture.0.join(format!("rust/a/src/{module}.rs")), "").unwrap();
    }
    std::fs::write(fixture.0.join("rust/a/src/lib.rs"), library).unwrap();
    let private_kernel = "| source | `rust/a/src/shared/owner.rs` | `a` | `crate::shared::owner` | `Registry::resolve_provider` | private | `rust/a/src/shared/owner.rs` | `crate::shared::owner` | `Registry::resolve` | `self` |";
    let internal_constructor = "| internal dependency | `rust/a/src/shared/caller.rs` | `a` | `crate::shared::caller` | `build` | pub(super) | `rust/a/src/shared/owner.rs` | `crate::shared::owner` | `Registry::resolve_provider` | `static` |";
    let text = format!("{START}\n{private_kernel}\n{internal_constructor}\n{END}");
    let (coverage, failures) = validate(&fixture.0, &text);
    assert_eq!(coverage.len(), 1);
    assert!(coverage.contains(&(
        "source".into(),
        "rust/a/src/shared/owner.rs".into(),
        "Registry::resolve_provider".into(),
    )));
    assert_eq!(failures.len(), 1);
    assert!(failures[0].contains("source module route exceeds bound"));
    assert!(failures[0].contains("internal dependency"));

    for changed in [
        private_kernel.replace("| private |", "| pub |"),
        private_kernel.replace("`Registry::resolve`", "`Other::resolve`"),
        private_kernel.replace("`Registry::resolve_provider`", "`Registry::absent`"),
    ] {
        let (coverage, failures) = validate(&fixture.0, &format!("{START}\n{changed}\n{END}"));
        assert!(coverage.is_empty());
        assert_ne!(failures.len(), 0);
    }
}

fn two_crates(
    owner_lib: &str,
    owner_files: &[(&str, &str)],
    consumer: &str,
    consumer_cargo: &str,
) -> Fixture {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let sequence = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let directory = std::env::temp_dir().join(format!(
        "source-owner-foreign-{}-{sequence}",
        std::process::id()
    ));
    std::fs::create_dir_all(directory.join("rust/b/src")).unwrap();
    std::fs::create_dir_all(directory.join("rust/a/src")).unwrap();
    std::fs::write(
        directory.join("rust/b/Cargo.toml"),
        "[package]\nname = \"b\"\n",
    )
    .unwrap();
    std::fs::write(directory.join("rust/b/src/lib.rs"), owner_lib).unwrap();
    for (file, contents) in owner_files {
        std::fs::write(directory.join("rust/b/src").join(file), contents).unwrap();
    }
    std::fs::write(directory.join("rust/a/Cargo.toml"), consumer_cargo).unwrap();
    std::fs::write(directory.join("rust/a/src/lib.rs"), "mod user;").unwrap();
    std::fs::write(directory.join("rust/a/src/user.rs"), consumer).unwrap();
    Fixture(directory)
}

const DEPENDS: &str = "[package]\nname = \"a\"\n\n[dependencies]\nb = { path = \"../b\" }\n";

fn foreign_manifest(
    owner_source: &str,
    owner_module: &str,
    visibility: &str,
    route: &str,
) -> String {
    format!(
        "{START}\n| Surface | Owner source | Crate | Owner module | Entry | Visibility | Consumer source | Consumer module | Consumer entry | Route |\n| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |\n| source | `{owner_source}` | `b` | `{owner_module}` | `tool` | {visibility} | `rust/a/src/user.rs` | `crate::user` | `consume` | `{route}` |\n{END}"
    )
}

#[test]
#[allow(clippy::too_many_lines)] // one table of accepted and rejected routes
fn foreign_static_routes_resolve_through_public_paths_only() {
    let direct = foreign_manifest("rust/b/src/util.rs", "crate::util", "pub", "static");
    let reexported = foreign_manifest("rust/b/src/inner.rs", "crate::inner", "pub", "static");
    let accepted = [
        (
            two_crates(
                "pub mod util;",
                &[("util.rs", "pub fn tool() {}")],
                "fn consume() { b::util::tool(); }",
                DEPENDS,
            ),
            &direct,
        ),
        (
            two_crates(
                "pub mod util;",
                &[("util.rs", "pub fn tool() {}")],
                "use b::util::tool; fn consume() { tool(); }",
                DEPENDS,
            ),
            &direct,
        ),
        (
            two_crates(
                "mod inner; pub use inner::tool;",
                &[("inner.rs", "pub fn tool() {}")],
                "fn consume() { b::tool(); }",
                DEPENDS,
            ),
            &reexported,
        ),
    ];
    for (fixture, manifest) in &accepted {
        let (coverage, problems) = validate(&fixture.0, manifest);
        assert!(problems.is_empty(), "{problems:?}");
        assert_eq!(coverage.len(), 1);
    }
    let crate_only = foreign_manifest("rust/b/src/util.rs", "crate::util", "pub(crate)", "static");
    let rejected = [
        (
            two_crates(
                "pub mod util;",
                &[("util.rs", "pub(crate) fn tool() {}")],
                "fn consume() { b::util::tool(); }",
                DEPENDS,
            ),
            &crate_only,
        ),
        (
            two_crates(
                "pub(crate) mod util;",
                &[("util.rs", "pub fn tool() {}")],
                "fn consume() { b::util::tool(); }",
                DEPENDS,
            ),
            &direct,
        ),
        (
            two_crates(
                "mod inner; pub(crate) use inner::tool;",
                &[("inner.rs", "pub fn tool() {}")],
                "fn consume() { b::tool(); }",
                DEPENDS,
            ),
            &reexported,
        ),
        (
            two_crates(
                "pub mod util;",
                &[("util.rs", "pub fn tool() {}")],
                "fn consume() { b::util::tool(); }",
                "[package]\nname = \"a\"\n",
            ),
            &direct,
        ),
        (
            two_crates(
                "pub mod util;",
                &[("util.rs", "pub fn tool() {}")],
                "fn consume() { b::util::tool(); }",
                "[package]\nname = \"a\"\n\n[dependencies]\nb = { path = \"../b\", package = \"c\" }\n",
            ),
            &direct,
        ),
        (
            two_crates(
                "pub mod util;",
                &[("util.rs", "pub fn tool() {}")],
                "fn consume() { b::util::other(); }",
                DEPENDS,
            ),
            &direct,
        ),
    ];
    for (fixture, manifest) in &rejected {
        let (coverage, problems) = validate(&fixture.0, manifest);
        assert!(coverage.is_empty(), "{manifest}");
        assert_ne!(problems.len(), 0);
    }
    let typed = foreign_manifest("rust/b/src/util.rs", "crate::util", "pub", "type");
    let fixture = two_crates(
        "pub mod util;",
        &[("util.rs", "pub fn tool() {}")],
        "fn consume() { b::util::tool(); }",
        DEPENDS,
    );
    let (coverage, problems) = validate(&fixture.0, &typed);
    assert!(coverage.is_empty());
    assert_ne!(problems.len(), 0);
}

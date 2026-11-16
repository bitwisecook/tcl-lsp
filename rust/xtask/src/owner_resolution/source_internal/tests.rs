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

fn fixture(owner: &str, consumer: &str, entry: &str, route: &str) -> (CrateModules, Row) {
    let modules = CrateModules {
        name: "fixture".into(),
        modules: [
            ("crate", ""),
            ("crate::shared", "mod owner; mod caller;"),
            ("crate::shared::owner", owner),
            ("crate::shared::caller", consumer),
        ]
        .into_iter()
        .map(|(module, source)| {
            (
                module.into(),
                SourceModule {
                    access_scope: module
                        .rsplit_once("::")
                        .map_or("crate", |(parent, _)| parent)
                        .into(),
                    public: false,
                    source: PathBuf::from(format!(
                        "rust/fixture/src/{}.rs",
                        module.replace("::", "/")
                    )),
                    file: syn::parse_file(source).unwrap(),
                },
            )
        })
        .collect(),
        externs: BTreeMap::new(),
    };
    let row = Row {
        surface: "source".into(),
        source: "rust/fixture/src/shared/owner.rs".into(),
        crate_name: "fixture".into(),
        module: "crate::shared::owner".into(),
        entry: entry.into(),
        visibility: "pub(super)".into(),
        consumer_source: "rust/fixture/src/shared/caller.rs".into(),
        consumer_module: "crate::shared::caller".into(),
        consumer_entry: "consume".into(),
        route: route.into(),
    };
    (modules, row)
}

#[test]
fn source_type_routes_follow_explicit_local_annotations_only() {
    let owner = "pub(super) struct Ledger;";
    for consumer in [
        "use super::owner::Ledger; fn consume() { let value: Ledger = make(); }",
        "fn consume() { let value: Option<super::owner::Ledger> = make(); }",
    ] {
        let (modules, row) = fixture(owner, consumer, "Ledger", "type");
        assert!(modules.check_row(&row), "{consumer}");
    }
    for consumer in [
        "struct Other; fn consume() { let value: Other = make(); }",
        "use super::owner::Ledger; fn consume() { let value = make(); }",
        "use super::owner::Ledger; fn consume() { #[cfg(test)] let value: Ledger = make(); }",
    ] {
        let (modules, row) = fixture(owner, consumer, "Ledger", "type");
        assert!(!modules.check_row(&row), "{consumer}");
    }
}

#[test]
fn source_routes_follow_current_nested_fields_and_standard_pointer_types() {
    let owner = "pub(super) struct Ledger; impl Ledger { pub(super) fn inspect(&self) {} }";
    for pointer in ["std::sync::Arc", "std::rc::Rc", "Box"] {
        let caller = format!(
            "use super::owner::Ledger; struct Module {{ ledger: Option<{pointer}<Ledger>> }} struct Unit {{ module: Module }} fn consume(unit: &Unit) -> Option<()> {{ let module = &unit.module; let ledger = module.ledger.as_deref()?; ledger.inspect(); Some(()) }}"
        );
        let (modules, row) = fixture(
            owner,
            &caller,
            "Ledger::inspect",
            "parameter-field:unit:module.ledger",
        );
        assert!(modules.check_row(&row), "{pointer}");
    }
    let caller = "use super::owner::Ledger; use std::sync::Arc as Shared; struct Module { ledger: Option<Shared<Ledger>> } struct Unit { module: Module } fn consume(unit: &Unit) -> Option<()> { let module = &unit.module; let ledger = module.ledger.as_deref()?; ledger.inspect(); Some(()) }";
    let (modules, row) = fixture(
        owner,
        caller,
        "Ledger::inspect",
        "parameter-field:unit:module.ledger",
    );
    assert!(modules.check_row(&row));
}

#[test]
fn source_routes_refuse_shadowed_pointers_and_foreign_nested_field_origins() {
    let owner = "pub(super) struct Ledger; impl Ledger { pub(super) fn inspect(&self) {} }";
    for caller in [
        "use super::owner::Ledger; struct Arc<T>(T); struct Module { ledger: Option<Arc<Ledger>> } struct Unit { module: Module } fn consume(unit: &Unit) -> Option<()> { let ledger = unit.module.ledger.as_deref()?; ledger.inspect(); Some(()) }",
        "use super::owner::Ledger; type Shared<T> = std::sync::Arc<T>; struct Module { ledger: Option<Shared<Ledger>> } struct Unit { module: Module } fn consume(unit: &Unit) -> Option<()> { let ledger = unit.module.ledger.as_deref()?; ledger.inspect(); Some(()) }",
        "use super::owner::Ledger; struct Module { #[cfg(any())] ledger: Option<std::sync::Arc<Ledger>> } struct Unit { module: Module } fn consume(unit: &Unit) -> Option<()> { let ledger = unit.module.ledger.as_deref()?; ledger.inspect(); Some(()) }",
        "use super::owner::Ledger; struct Module { ledger: Option<std::sync::Arc<Ledger>> } struct Unit { module: Module } fn consume(unit: &Unit, other: &Unit) -> Option<()> { let unit = other; let ledger = unit.module.ledger.as_deref()?; ledger.inspect(); Some(()) }",
    ] {
        let (modules, row) = fixture(
            owner,
            caller,
            "Ledger::inspect",
            "parameter-field:unit:module.ledger",
        );
        assert!(!modules.check_row(&row), "{caller}");
    }
}

#[test]
fn source_routes_bind_only_actual_optional_constructor_results() {
    let owner = "pub(super) struct Ledger; impl Ledger { pub(super) fn inspect(&self) {} } pub(super) fn issue() -> Option<Ledger> { None }";
    for caller in [
        "fn consume() { if let Some(current) = super::owner::issue() { current.inspect(); } }",
        "fn consume() { let Some(current) = super::owner::issue() else { return; }; current.inspect(); }",
    ] {
        let (modules, row) = fixture(
            owner,
            caller,
            "Ledger::inspect",
            "producer:crate::shared::owner::issue",
        );
        assert!(modules.check_row(&row), "{caller}");
    }
    for caller in [
        "struct Some<T>(T); fn consume() { if let Some(current) = super::owner::issue() { current.inspect(); } }",
        "fn consume() { if let Some(current) = super::owner::issue() { let current = (); current.inspect(); } }",
        "fn consume() { let Some(current) = super::owner::issue() else { donate!(); return; }; current.inspect(); }",
        "fn consume() { if let Some(current) = super::owner::issue() {} current.inspect(); }",
    ] {
        let (modules, row) = fixture(
            owner,
            caller,
            "Ledger::inspect",
            "producer:crate::shared::owner::issue",
        );
        assert!(!modules.check_row(&row), "{caller}");
    }
}

#[test]
fn source_enum_receivers_follow_actual_iterator_item_and_tuple_variant_fields() {
    let owner = "pub(super) struct Ledger; impl Ledger { pub(super) fn inspect(&self) {} } pub(super) enum Entry { Declared(std::sync::Arc<Ledger>), Empty } #[derive(Clone)] pub(super) struct Row { pub(super) entry: std::sync::Arc<Entry> } pub(super) fn rows() -> Option<impl Iterator<Item = Row> + Clone> { None::<std::vec::IntoIter<Row>> }";
    let caller = "fn consume() { let Some(rows) = super::owner::rows() else { return; }; for row in rows { match row.entry.as_ref() { super::owner::Entry::Declared(current) => current.inspect(), _ => {} } } }";
    let (modules, row) = fixture(
        owner,
        caller,
        "Ledger::inspect",
        "producer-field:crate::shared::owner::rows:entry:variant:Declared:0",
    );
    assert!(modules.check_row(&row));
}

#[test]
fn source_enum_receivers_refuse_other_variants_shadows_and_unavailable_shapes() {
    let owner = "pub(super) struct Ledger; impl Ledger { pub(super) fn inspect(&self) {} } pub(super) enum Entry { Declared(std::sync::Arc<Ledger>), Other(std::sync::Arc<Ledger>) } #[derive(Clone)] pub(super) struct Row { pub(super) entry: std::sync::Arc<Entry> } pub(super) fn rows() -> Option<impl Iterator<Item = Row> + Clone> { None::<std::vec::IntoIter<Row>> }";
    for caller in [
        "fn consume() { let Some(rows) = super::owner::rows() else { return; }; for row in rows { match row.entry.as_ref() { super::owner::Entry::Other(current) => current.inspect(), _ => {} } } }",
        "fn consume() { let Some(rows) = super::owner::rows() else { return; }; for row in rows { match row.entry.as_ref() { super::owner::Entry::Declared(current) => { let current = (); current.inspect(); }, _ => {} } } }",
        "fn consume() { let Some(rows) = super::owner::rows() else { return; }; for row in rows { match row.entry.as_ref() { super::owner::Entry::Declared(current) => {}, _ => {} } current.inspect(); } }",
    ] {
        let (modules, row) = fixture(
            owner,
            caller,
            "Ledger::inspect",
            "producer-field:crate::shared::owner::rows:entry:variant:Declared:0",
        );
        assert!(!modules.check_row(&row), "{caller}");
    }
    for changed in [
        owner.replace(
            "Declared(std::sync::Arc<Ledger>)",
            "#[cfg(any())] Declared(std::sync::Arc<Ledger>)",
        ),
        owner.replace("Iterator<Item = Row>", "Iterator<Item = ()>"),
        owner.replace("Iterator<Item = Row>", "?Iterator<Item = Row>"),
        owner.replace("pub(super) entry:", "entry:"),
    ] {
        let caller = "fn consume() { let Some(rows) = super::owner::rows() else { return; }; for row in rows { match row.entry.as_ref() { super::owner::Entry::Declared(current) => current.inspect(), _ => {} } } }";
        let (modules, row) = fixture(
            &changed,
            caller,
            "Ledger::inspect",
            "producer-field:crate::shared::owner::rows:entry:variant:Declared:0",
        );
        assert!(!modules.check_row(&row));
    }
}

#[test]
fn source_struct_type_rows_require_actual_accessible_field_occurrences() {
    for caller in [
        "struct Selected { value: super::owner::Ledger }",
        "struct Selected(std::sync::Arc<super::owner::Ledger>);",
        "use super::owner::Ledger as Current; struct Selected { values: std::collections::BTreeMap<u32, Option<Current>> }",
        "struct Selected<'a> { value: &'a super::owner::Ledger }",
    ] {
        let (modules, mut row) = fixture("pub(super) struct Ledger;", caller, "Ledger", "type");
        row.consumer_entry = "Selected".into();
        assert!(modules.check_row(&row), "{caller}");
    }
    for caller in [
        "struct Selected;",
        "struct Selected { value: u32 }",
        "struct Selected<Ledger> { value: Ledger }",
        "struct Selected<const N: usize> { value: super::owner::Ledger }",
        "#[cfg(any())] struct Selected { value: super::owner::Ledger }",
        "#[custom] struct Selected { value: super::owner::Ledger }",
        "struct Selected { #[cfg(any())] value: super::owner::Ledger }",
        "struct Selected { #[custom] value: super::owner::Ledger }",
        "struct Selected(#[cfg(any())] super::owner::Ledger);",
        "type Alias = super::owner::Ledger; struct Selected { value: Alias }",
        "struct Selected { value: super::owner::Ledger<u8> }",
        "struct Selected { value: <super::owner::Ledger as Unknown>::Output }",
        "struct Selected { value: super::owner::Ledger, hidden: select_type!() }",
        "struct Selected { value: super::owner::Ledger } struct Selected { value: super::owner::Ledger }",
        "struct Selected { value: super::owner::Ledger } enum Selected { Empty }",
    ] {
        let (modules, mut row) = fixture("pub(super) struct Ledger;", caller, "Ledger", "type");
        row.consumer_entry = "Selected".into();
        assert!(!modules.check_row(&row), "{caller}");
    }
    let (modules, mut row) = fixture(
        "struct Ledger;",
        "struct Selected { value: super::owner::Ledger }",
        "Ledger",
        "type",
    );
    row.consumer_entry = "Selected".into();
    row.visibility = "private".into();
    assert!(!modules.check_row(&row));
}

#[test]
fn source_enum_type_rows_require_actual_accessible_field_occurrences() {
    for caller in [
        "enum Selected { Owned(Box<super::owner::Ledger>) }",
        "use super::owner::Ledger as Current; enum Selected { Owned { current: std::sync::Arc<Current> } }",
        "enum Selected<'a> { Owned(&'a super::owner::Ledger) }",
    ] {
        let (modules, mut row) = fixture("pub(super) struct Ledger;", caller, "Ledger", "type");
        row.consumer_entry = "Selected".into();
        assert!(modules.check_row(&row), "{caller}");
    }
    for caller in [
        "enum Selected { Empty }",
        "struct Selected { value: u32 }",
        "enum Selected<Ledger> { Owned(Ledger) }",
        "enum Selected<const N: usize> { Owned(super::owner::Ledger) }",
        "#[cfg(any())] enum Selected { Owned(super::owner::Ledger) }",
        "#[custom] enum Selected { Owned(super::owner::Ledger) }",
        "enum Selected { #[cfg(any())] Owned(super::owner::Ledger) }",
        "enum Selected { Owned(#[cfg(any())] super::owner::Ledger) }",
        "enum Selected { Owned(#[custom] super::owner::Ledger) }",
        "type Alias = super::owner::Ledger; enum Selected { Owned(Alias) }",
        "enum Selected { Owned(super::owner::Ledger<u8>) }",
        "enum Selected { Owned(<super::owner::Ledger as Unknown>::Output) }",
        "enum Selected { Owned(super::owner::Ledger), Unknown(select_type!()) }",
        "enum Selected { Empty = 1, Owned(super::owner::Ledger) }",
        "enum Selected { Owned(super::owner::Ledger) } enum Selected { Owned(super::owner::Ledger) }",
    ] {
        let (modules, mut row) = fixture("pub(super) struct Ledger;", caller, "Ledger", "type");
        row.consumer_entry = "Selected".into();
        assert!(!modules.check_row(&row), "{caller}");
    }
    let (modules, mut row) = fixture(
        "struct Ledger;",
        "enum Selected { Owned(super::owner::Ledger) }",
        "Ledger",
        "type",
    );
    row.consumer_entry = "Selected".into();
    row.visibility = "private".into();
    assert!(!modules.check_row(&row));
    let (modules, mut row) = fixture(
        "pub(super) struct Ledger;",
        "enum Selected { Owned(super::owner::Ledger) }",
        "Ledger",
        "static",
    );
    row.consumer_entry = "Selected".into();
    assert!(!modules.check_row(&row));
}

#[test]
fn source_standard_wrappers_refuse_local_union_shadows() {
    for caller in [
        "union Box<T> { value: std::mem::ManuallyDrop<T> } trait Audit { fn inspect(&self); } impl<T> Audit for Box<T> { fn inspect(&self) {} } fn consume(current: &Box<super::owner::Ledger>) { current.inspect(); }",
        "union Option<T> { value: std::mem::ManuallyDrop<T> } impl<T> Option<T> { fn as_ref(&self) -> std::option::Option<&Self> { None } } trait Audit { fn inspect(&self); } impl<T> Audit for Option<T> { fn inspect(&self) {} } fn consume(current: &Option<super::owner::Ledger>) -> std::option::Option<()> { current.as_ref()?.inspect(); Some(()) }",
    ] {
        let (modules, row) = fixture(
            "pub(super) struct Ledger; impl Ledger { pub(super) fn inspect(&self) {} }",
            caller,
            "Ledger::inspect",
            "parameter:current",
        );
        assert!(!modules.check_row(&row), "{caller}");
    }
}

#[test]
fn source_dot_routes_require_the_actual_owner_receiver() {
    let owner = "pub(super) struct Ledger; impl Ledger { pub(super) fn inspect() {} } pub(super) trait Audit { fn inspect(&self); } impl Audit for Ledger { fn inspect(&self) {} }";
    let (modules, row) = fixture(
        owner,
        "use super::owner::Audit; fn consume(ledger: &super::owner::Ledger) { ledger.inspect(); }",
        "Ledger::inspect",
        "parameter:ledger",
    );
    assert!(!modules.check_row(&row));
    let (modules, row) = fixture(
        owner,
        "fn consume() { super::owner::Ledger::inspect(); }",
        "Ledger::inspect",
        "static",
    );
    assert!(modules.check_row(&row));
}

#[test]
fn source_routes_accept_actual_nested_static_and_typed_parameter_calls() {
    let (modules, row) = fixture(
        "pub(super) struct Ledger; impl Ledger { pub(super) fn capture() {} pub(super) fn revision(&self, depth: usize) {} }",
        "fn consume() { super::owner::Ledger::capture(); }",
        "Ledger::capture",
        "static",
    );
    assert!(modules.check_row(&row));
    let (modules, row) = fixture(
        "pub(super) struct Ledger; impl Ledger { pub(super) fn revision(&self, depth: usize) {} }",
        "use super::owner::Ledger; fn consume(ledger: &Ledger) { ledger.revision(4); }",
        "Ledger::revision",
        "parameter:ledger",
    );
    assert!(modules.check_row(&row));
}

#[test]
fn source_routes_follow_only_actual_standard_option_map_arguments() {
    let owner = "pub(super) struct Ledger; impl Ledger { pub(super) fn revision(&self) {} }";
    for caller in [
        "use super::owner::Ledger; fn consume(current: Option<&Ledger>) { current.map(|ledger| ledger.revision()); }",
        "fn consume(current: std::option::Option<&super::owner::Ledger>) { current.map(|ledger| ledger.revision()); }",
        "use std::option::Option as Maybe; fn consume(current: Maybe<&super::owner::Ledger>) { current.map(|ledger| ledger.revision()); }",
        "use super::owner::Ledger; fn consume(current: Option<&Ledger>) { current.map(|ledger| { { let ledger = (); } ledger.revision(); }); }",
    ] {
        let (modules, row) = fixture(owner, caller, "Ledger::revision", "parameter:current");
        assert!(modules.check_row(&row), "{caller}");
    }
    for caller in [
        "struct Other; impl Other { fn revision(&self) {} } fn consume(current: Option<&Other>) { current.map(|ledger| ledger.revision()); }",
        "struct Maybe<T>(T); impl<T> Maybe<T> { fn map<F>(&self, f: F) {} } fn consume(current: Maybe<&super::owner::Ledger>) { current.map(|ledger| ledger.revision()); }",
        "struct Option<T>(T); fn consume(current: Option<&super::owner::Ledger>) { current.map(|ledger| ledger.revision()); }",
        "type Maybe<T> = std::option::Option<T>; fn consume(current: Maybe<&super::owner::Ledger>) { current.map(|ledger| ledger.revision()); }",
        "use super::owner::Ledger; fn consume(current: Option<&Ledger>) { use fake::Ledger; current.map(|ledger| ledger.revision()); }",
        "use super::owner::Ledger; fn consume(current: Option<&Ledger>) { current.map(|ledger: &Ledger| ledger.revision()); }",
        "use super::owner::Ledger; fn consume(current: Option<&Ledger>) { current.map(|ledger, extra| ledger.revision()); }",
        "use super::owner::Ledger; fn consume(current: Option<&Ledger>) { current.map::<()>(|ledger| ledger.revision()); }",
        "use super::owner::Ledger; fn consume(current: Option<&Ledger>) { current.map(|ledger| { let ledger = (); ledger.revision(); }); }",
        "use super::owner::Ledger; fn consume(current: Option<&Ledger>) { current.map(#[cfg(any())] |ledger| ledger.revision()); }",
        "use super::owner::Ledger; fn consume(current: Option<&Ledger>) { current.map(|ledger| { inject!(); ledger.revision(); }); }",
    ] {
        let (modules, row) = fixture(owner, caller, "Ledger::revision", "parameter:current");
        assert!(!modules.check_row(&row), "{caller}");
    }
}

#[test]
fn source_routes_accept_only_actual_fields_and_constructor_results() {
    let owner = "pub(super) struct Ledger; impl Ledger { pub(super) fn capture() -> Option<Vec<Self>> { None } pub(super) fn revision(&self) {} }";
    let (modules, row) = fixture(
        owner,
        "use super::owner::Ledger; fn consume() -> Option<()> { let values = Ledger::capture()?; values.iter().map(|value| value.revision()).count(); Some(()) }",
        "Ledger::revision",
        "producer:crate::shared::owner::Ledger::capture",
    );
    assert!(modules.check_row(&row));
    let (modules, mut row) = fixture(
        owner,
        "struct Reader { values: Vec<super::owner::Ledger> } impl Reader { fn consume(&self) { self.values.iter().zip(&[()]).any(|(value, other)| { value.revision(); false }); } }",
        "Ledger::revision",
        "field:self.values",
    );
    row.consumer_entry = "Reader::consume".into();
    assert!(modules.check_row(&row));
    let (modules, row) = fixture(
        owner,
        "struct Fake; fn consume(ledger: &Fake) { ledger.revision(); }",
        "Ledger::revision",
        "parameter:ledger",
    );
    assert!(!modules.check_row(&row));
}

#[test]
fn source_routes_verify_actual_parameter_and_producer_fields() {
    let owner = "pub(super) struct Ledger; impl Ledger { pub(super) fn revision(&self) {} }";
    let (modules, row) = fixture(
        owner,
        "struct Tokens { binding: Option<super::owner::Ledger> } fn consume(parent: &Tokens) -> Option<()> { let binding = parent.binding.as_ref()?; binding.revision(); Some(()) }",
        "Ledger::revision",
        "parameter-field:parent:binding",
    );
    assert!(modules.check_row(&row));
    let (modules, mut row) = fixture(
        owner,
        "struct Tokens { binding: Option<super::owner::Ledger> } struct Reader; impl Reader { fn tokens(&self) -> Tokens { todo!() } fn consume(&self) -> Option<()> { let tokens = self.tokens(); tokens.binding.as_ref()?.revision(); Some(()) } }",
        "Ledger::revision",
        "producer-field:Reader::tokens:binding",
    );
    row.consumer_entry = "Reader::consume".into();
    assert!(modules.check_row(&row));
    let (modules, row) = fixture(
        owner,
        "struct Fake; impl Fake { fn revision(&self) {} } struct Tokens { binding: Option<Fake> } fn consume(parent: &Tokens) -> Option<()> { let binding = parent.binding.as_ref()?; binding.revision(); Some(()) }",
        "Ledger::revision",
        "parameter-field:parent:binding",
    );
    assert!(!modules.check_row(&row));
}

#[test]
fn source_routes_keep_plain_local_imports_scoped_and_preserve_target_shadows() {
    let owner = "pub(super) struct Ledger; impl Ledger { pub(super) fn capture() {} }";
    for caller in [
        "fn consume() { use std::collections::HashSet; super::owner::Ledger::capture(); }",
        "use super::owner::Ledger; fn consume() { { use other::Ledger; } Ledger::capture(); }",
    ] {
        let (modules, row) = fixture(owner, caller, "Ledger::capture", "static");
        assert!(modules.check_row(&row), "{caller}");
    }
    for caller in [
        "use super::owner::Ledger; fn consume() { Ledger::capture(); use other::Ledger; }",
        "use super::owner::Ledger; fn consume() { use other::Other as Ledger; Ledger::capture(); }",
        "fn consume() { use other::*; super::owner::Ledger::capture(); }",
        "fn consume() { #[cfg(test)] use other::Other; super::owner::Ledger::capture(); }",
    ] {
        let (modules, row) = fixture(owner, caller, "Ledger::capture", "static");
        assert!(!modules.check_row(&row), "{caller}");
    }
}

#[test]
fn parameter_origins_refuse_body_type_shadows_and_restore_enclosing_scope() {
    let owner = "pub(super) struct Ledger; impl Ledger { pub(super) fn revision(&self) {} }";
    for caller in [
        "use super::owner::Ledger; fn consume(ledger: &Ledger) { ledger.revision(); use fake::Ledger; }",
        "use super::owner::Ledger; fn consume(ledger: &Ledger) { use fake::Other as Ledger; let alias = ledger; alias.revision(); }",
        "use super::owner::Ledger; fn consume(ledger: &Ledger) { let alias = ledger; { use fake::Ledger; alias.revision(); } }",
        "use super::owner::Ledger; fn consume(ledger: &Option<Ledger>) { use fake::Ledger; let Some(current) = ledger.as_ref() else { return; }; current.revision(); }",
    ] {
        let (modules, row) = fixture(owner, caller, "Ledger::revision", "parameter:ledger");
        assert!(!modules.check_row(&row), "{caller}");
    }
    for caller in [
        "fn consume(ledger: &super::owner::Ledger) { use fake::Ledger; ledger.revision(); }",
        "use super::owner::Ledger; fn consume(ledger: &Ledger) { use fake::Other; ledger.revision(); }",
        "use super::owner::Ledger; fn consume(ledger: &Ledger) { { use fake::Ledger; } ledger.revision(); }",
    ] {
        let (modules, row) = fixture(owner, caller, "Ledger::revision", "parameter:ledger");
        assert!(modules.check_row(&row), "{caller}");
    }
}

#[test]
fn source_self_fields_require_one_exact_transparent_boxed_state_hop() {
    let owner = "pub(super) struct Ledger; impl Ledger { pub(super) fn revision(&mut self) {} }";
    let state = "struct State { ledger: super::owner::Ledger } struct Reader { state: Box<State> }";
    let immutable = "impl std::ops::Deref for Reader { type Target = State; fn deref(&self) -> &State { &self.state } }";
    let mutable = "impl std::ops::DerefMut for Reader { fn deref_mut(&mut self) -> &mut State { &mut self.state } }";
    let consumer = "impl Reader { fn consume(&mut self) { self.ledger.revision(); } }";
    let check = |caller: String| {
        let (modules, mut row) = fixture(owner, &caller, "Ledger::revision", "field:self.ledger");
        row.consumer_entry = "Reader::consume".into();
        modules.check_row(&row)
    };
    assert!(check(format!("{state} {immutable} {mutable} {consumer}")));
    for caller in [
        format!("{state} {immutable} {consumer}"),
        format!("{state} {mutable} {consumer}"),
        format!("{state} #[cfg(any())] {immutable} {mutable} {consumer}"),
        format!("{state} {immutable} {immutable} {mutable} {consumer}"),
        format!(
            "{state} {} {mutable} {consumer}",
            immutable.replace("&self.state", "read_state(self)")
        ),
        format!(
            "{state} {immutable} {} {consumer}",
            mutable.replace("self.state", "self.other")
        ),
        format!(
            "{state} {} {mutable} {consumer}",
            immutable.replace("type Target = State", "type Target = Other")
        ),
        format!(
            "{} {immutable} {mutable} {consumer}",
            state.replace("Box<State>", "Other<State>")
        ),
        format!("mod std {{}} {state} {immutable} {mutable} {consumer}"),
        format!("struct Box<T>(T); {state} {immutable} {mutable} {consumer}"),
        format!(
            "{state} {} {mutable} {consumer}",
            immutable.replace("fn deref", "#[cfg(any())] fn deref")
        ),
        format!(
            "{state} {} {mutable} {consumer}",
            immutable.replace("{ &self.state }", "{ let state = &self.state; state }")
        ),
        format!(
            "{} {immutable} {mutable} {consumer}",
            state.replace(
                "struct Reader {",
                "struct Fake; struct Reader { ledger: Fake,"
            )
        ),
    ] {
        assert!(!check(caller.clone()), "{caller}");
    }
    let mut source = format!("{state} {immutable} {mutable} {consumer}");
    source.push_str(" struct Outer { inner: Box<Reader> } impl std::ops::Deref for Outer { type Target = Reader; fn deref(&self) -> &Reader { &self.inner } } impl std::ops::DerefMut for Outer { fn deref_mut(&mut self) -> &mut Reader { &mut self.inner } } impl Outer { fn consume(&mut self) { self.ledger.revision(); } }");
    let (modules, mut row) = fixture(owner, &source, "Ledger::revision", "field:self.ledger");
    row.consumer_entry = "Outer::consume".into();
    assert!(!modules.check_row(&row));
}

#[test]
fn source_type_declarations_keep_passive_lints_and_refuse_availability_attributes() {
    let caller = "use super::owner::Ledger; fn consume(ledger: &Ledger) { ledger.revision(); }";
    for attribute in [
        "#[allow(dead_code)]",
        "#[warn(clippy::all)]",
        "#[deny(unused)]",
        "#[forbid(unsafe_code)]",
        "#[expect(dead_code)]",
    ] {
        let owner = format!(
            "{attribute} pub(super) struct Ledger; impl Ledger {{ pub(super) fn revision(&self) {{}} }}"
        );
        let (modules, row) = fixture(&owner, caller, "Ledger::revision", "parameter:ledger");
        assert!(modules.check_row(&row), "{attribute}");
    }
    for attribute in [
        "#[cfg(any())]",
        "#[cfg_attr(any(), derive(Clone))]",
        "#[unknown]",
    ] {
        let owner = format!(
            "{attribute} pub(super) struct Ledger; impl Ledger {{ pub(super) fn revision(&self) {{}} }}"
        );
        let (modules, row) = fixture(&owner, caller, "Ledger::revision", "parameter:ledger");
        assert!(!modules.check_row(&row), "{attribute}");
    }
}

#[test]
fn signature_type_routes_count_only_explicit_supported_declaration_types() {
    let owner = "pub(super) enum Ledger { Fixed }";
    let caller =
        "use super::owner::Ledger; fn consume(selected: Ledger) { diagnostic!(selected); }";
    let (modules, mut row) = fixture(owner, caller, "Ledger", "signature-type");
    assert!(modules.check_row(&row));
    row.route = "type".into();
    assert!(!modules.check_row(&row));
    for caller in [
        "fn consume() { let selected = super::owner::Ledger::Fixed; }",
        "use super::owner::Ledger; fn consume<T>(selected: Ledger) {}",
        "use super::owner::Ledger; fn consume(selected: Ledger<Other>) {}",
        "use super::owner::Ledger; #[cfg(any())] fn consume(selected: Ledger) {}",
        "use super::owner::Ledger; fn consume(#[cfg(any())] selected: Ledger) {}",
        "use super::owner::Ledger; fn consume(selected: Ledger, other: hidden!()) {}",
        "type Alias = super::owner::Ledger; fn consume(selected: Alias) {}",
    ] {
        let (modules, row) = fixture(owner, caller, "Ledger", "signature-type");
        assert!(!modules.check_row(&row), "{caller}");
    }
}

#[test]
fn source_type_routes_require_explicit_current_struct_constructors() {
    let owner = "pub(super) struct Ledger { pub(super) current: bool }";
    let (modules, row) = fixture(
        owner,
        "fn consume() { let current = super::owner::Ledger { current: true }; }",
        "Ledger",
        "type",
    );
    assert!(modules.check_row(&row));
    for caller in [
        "type Donated = super::owner::Ledger; fn consume() { let current = Donated { current: true }; }",
        "use super::owner::Ledger; fn consume() { let current = Ledger { current: true }; use other::Ledger; }",
        "fn consume() { #[cfg(test)] let current = super::owner::Ledger { current: true }; }",
        "fn consume() { make!(super::owner::Ledger { current: true }); }",
        "fn consume() { let current = super::owner::Ledger::<Other> { current: true }; }",
    ] {
        let (modules, row) = fixture(owner, caller, "Ledger", "type");
        assert!(!modules.check_row(&row), "{caller}");
    }
}

#[test]
fn source_routes_follow_only_actual_standard_mutable_option_fields() {
    let owner = "pub(super) struct Ledger; impl Ledger { pub(super) fn revision(&mut self) {} }";
    let (modules, row) = fixture(
        owner,
        "struct Tokens { binding: Option<super::owner::Ledger> } fn consume(parent: &mut Tokens) { if let Some(binding) = parent.binding.as_mut() { binding.revision(); } }",
        "Ledger::revision",
        "parameter-field:parent:binding",
    );
    assert!(modules.check_row(&row));
    for caller in [
        "struct Fake; impl Fake { fn revision(&mut self) {} } struct Tokens { binding: Option<Fake> } fn consume(parent: &mut Tokens) { if let Some(binding) = parent.binding.as_mut() { binding.revision(); } }",
        "struct Other<T>(T); impl<T> Other<T> { fn as_mut(&mut self) -> Option<&mut T> { Some(&mut self.0) } } struct Tokens { binding: Other<super::owner::Ledger> } fn consume(parent: &mut Tokens) { if let Some(binding) = parent.binding.as_mut() { binding.revision(); } }",
        "struct Tokens { binding: Option<super::owner::Ledger> } fn consume(parent: &mut Tokens) { if let Some(binding) = parent.binding.as_mut::<Other>() { binding.revision(); } }",
    ] {
        let (modules, row) = fixture(
            owner,
            caller,
            "Ledger::revision",
            "parameter-field:parent:binding",
        );
        assert!(!modules.check_row(&row), "{caller}");
    }
}

#[test]
fn source_routes_reject_shadow_macro_alias_and_conditional_calls() {
    let owner = "pub(super) struct Ledger; impl Ledger { pub(super) fn capture() {} }";
    for consumer in [
        "use super::owner::Ledger; fn consume() { let Ledger = fake; Ledger::capture(); }",
        "type Ledger = super::owner::Ledger; fn consume() { Ledger::capture(); }",
        "fn consume() { invoke!(super::owner::Ledger::capture()); }",
        "fn consume() { #[cfg(test)] super::owner::Ledger::capture(); }",
        "#[cfg(test)] fn consume() { super::owner::Ledger::capture(); }",
        "fn consume() { super::owner::Ledger::other(); }",
    ] {
        let (modules, row) = fixture(owner, consumer, "Ledger::capture", "static");
        assert!(!modules.check_row(&row), "{consumer}");
    }
}

#[test]
fn source_routes_clear_every_shadowing_pattern_and_refuse_binding_macros() {
    let owner = "pub(super) struct Ledger; impl Ledger { pub(super) fn revision(&self) {} }";
    for body in [
        "items.map(|ledger| ledger.revision());",
        "for ledger in items { ledger.revision(); }",
        "match fake { Some(ledger) => ledger.revision(), _ => () }",
        "match fake { Some(ledger) if { ledger.revision(); true } => (), _ => () }",
        "match fake { (Some(ledger), _) if { ledger.revision(); true } => (), _ => () }",
        "match fake { Some(other) if { bind!(ledger); ledger.revision(); true } => (), _ => () }",
        "if let Some(ledger) = fake { ledger.revision(); }",
        "while let Some(ledger) = fake { ledger.revision(); }",
        "let (ledger, other) = fake; ledger.revision();",
        "let Some(ledger) = fake else { return; }; ledger.revision();",
        "bind!(ledger); ledger.revision();",
        "macro_rules! bind { () => {} } ledger.revision();",
        "#[cfg(test)] for other in items { ledger.revision(); }",
    ] {
        let consumer =
            format!("use super::owner::Ledger; fn consume(ledger: &Ledger) {{ {body} }}");
        let (modules, row) = fixture(owner, &consumer, "Ledger::revision", "parameter:ledger");
        assert!(!modules.check_row(&row), "{body}");
    }
    let (modules, row) = fixture(
        owner,
        "use super::owner::Ledger; fn consume(ledger: &Ledger) { for unrelated in items {} ledger.revision(); }",
        "Ledger::revision",
        "parameter:ledger",
    );
    assert!(modules.check_row(&row));
}

#[test]
fn source_routes_refuse_enclosing_generic_receiver_name_shadows() {
    let (modules, mut row) = fixture(
        "pub(super) struct Ledger; impl Ledger { pub(super) fn revision(&self) {} }",
        "use super::owner::Ledger; struct Reader<T>(T); impl<Ledger> Reader<Ledger> { fn consume(&self, ledger: &Ledger) { ledger.revision(); } }",
        "Ledger::revision",
        "parameter:ledger",
    );
    row.consumer_entry = "Reader::consume".into();
    assert!(!modules.check_row(&row));
}

#[test]
fn source_routes_require_direct_module_visibility_and_preserve_real_reexports() {
    let (mut modules, mut row) = fixture(
        "pub(crate) struct Ledger; impl Ledger { pub(crate) fn capture() {} }",
        "fn consume() { crate::shared::owner::Ledger::capture(); }",
        "Ledger::capture",
        "static",
    );
    row.visibility = "pub(crate)".into();
    let caller = modules.modules.remove("crate::shared::caller").unwrap();
    modules.modules.insert(
        "crate::outside".into(),
        SourceModule {
            access_scope: "crate".into(),
            ..caller
        },
    );
    row.consumer_module = "crate::outside".into();
    assert!(!modules.check_row(&row));
    modules.modules.get_mut("crate::shared").unwrap().file =
        syn::parse_file("mod owner; pub(crate) use owner::Ledger;").unwrap();
    modules.modules.get_mut("crate::outside").unwrap().file =
        syn::parse_file("fn consume() { crate::shared::Ledger::capture(); }").unwrap();
    assert!(modules.check_row(&row));
}

#[test]
fn source_visibility_requires_a_real_ancestor_scope() {
    for (written, expected) in [
        ("pub(in crate::shared)", "crate::shared"),
        ("pub(in super)", "crate::shared"),
        ("pub(in super::super)", "crate"),
        ("pub(in self)", "crate::shared::owner"),
        ("pub(crate)", "crate"),
    ] {
        let visibility = syn::parse_str::<syn::Visibility>(written).unwrap();
        assert_eq!(
            visibility_scope(&visibility, "crate::shared::owner"),
            Some(expected.into())
        );
        assert_eq!(visibility_name(&visibility), written);
    }
    for written in [
        "pub(in crate::outside)",
        "pub(in self::child)",
        "pub(in super::super::super)",
        "pub(in alias)",
    ] {
        let visibility = syn::parse_str::<syn::Visibility>(written).unwrap();
        assert!(
            visibility_scope(&visibility, "crate::shared::owner").is_none(),
            "{written}"
        );
    }
    let visibility = syn::parse_str::<syn::Visibility>("pub(in crate::shared)").unwrap();
    assert!(accessible(
        &visibility,
        "crate::shared::owner",
        "crate::shared::caller"
    ));
    assert!(!accessible(
        &visibility,
        "crate::shared::owner",
        "crate::shared_extra::caller"
    ));
    assert!(!accessible(
        &visibility,
        "crate::shared::owner",
        "crate::outside"
    ));
    let mut generic = visibility.clone();
    let syn::Visibility::Restricted(scope) = &mut generic else {
        panic!();
    };
    *scope.path = syn::parse_str::<syn::Path>("crate::shared::<Other>").unwrap();
    assert!(visibility_scope(&generic, "crate::shared::owner").is_none());
}

#[test]
fn source_file_module_loader_keeps_ancestor_and_parent_visibility() {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let root = std::env::temp_dir().join(format!(
        "tcl-owner-ancestor-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir(&root).unwrap();
    let crate_root = PathBuf::from("rust/fixture");
    std::fs::create_dir_all(root.join(&crate_root).join("src/shared")).unwrap();
    for (file, contents) in [
        ("Cargo.toml", "[package]\nname = \"fixture\"\n"),
        ("src/lib.rs", "pub(crate) mod shared; mod outside;"),
        (
            "src/shared/mod.rs",
            "pub(in crate::shared) mod owner; pub(crate) mod caller;",
        ),
        (
            "src/shared/owner.rs",
            "pub(crate) struct Ledger; impl Ledger { pub(crate) fn capture() {} }",
        ),
        (
            "src/shared/caller.rs",
            "fn consume() { super::owner::Ledger::capture(); }",
        ),
        (
            "src/outside.rs",
            "fn consume() { crate::shared::owner::Ledger::capture(); }",
        ),
    ] {
        std::fs::write(root.join(&crate_root).join(file), contents).unwrap();
    }
    let (_, mut row) = fixture("", "", "Ledger::capture", "static");
    row.visibility = "pub(crate)".into();
    let modules =
        CrateModules::load(&root, &crate_root, [&row.module, &row.consumer_module]).unwrap();
    assert!(modules.check_row(&row));
    row.consumer_module = "crate::outside".into();
    row.consumer_source = "rust/fixture/src/outside.rs".into();
    assert!(!modules.check_row(&row));
    assert!(!modules.module_accessible("crate::shared::owner", "crate::outside"));
    std::fs::write(
        root.join(&crate_root).join("src/shared/mod.rs"),
        "pub(in crate::outside) mod owner; mod caller;",
    )
    .unwrap();
    assert!(CrateModules::load(&root, &crate_root, [&row.module, &row.consumer_module]).is_err());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn source_file_module_loader_keeps_expansion_budget_and_route_refusals() {
    static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let root = std::env::temp_dir().join(format!(
        "tcl-owner-budget-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let crate_root = PathBuf::from("rust/fixture");
    std::fs::create_dir_all(root.join(&crate_root).join("src/shared")).unwrap();
    for (file, contents) in [
        ("Cargo.toml", "[package]\nname = \"fixture\"\n"),
        ("src/shared/mod.rs", "mod owner; mod caller;"),
        (
            "src/shared/owner.rs",
            "pub(super) struct Ledger; impl Ledger { pub(super) fn capture() {} }",
        ),
        (
            "src/shared/caller.rs",
            "fn consume() { super::owner::Ledger::capture(); }",
        ),
    ] {
        std::fs::write(root.join(&crate_root).join(file), contents).unwrap();
    }
    let (_, row) = fixture("", "", "Ledger::capture", "static");
    for attribute in [
        r#"#![recursion_limit = "256"]"#,
        r#"#![cfg_attr(test, recursion_limit = "512")]"#,
    ] {
        std::fs::write(
            root.join(&crate_root).join("src/lib.rs"),
            format!("{attribute}\nmod shared;"),
        )
        .unwrap();
        let modules = CrateModules::load_for_route(&root, &crate_root, &row).unwrap();
        assert!(modules.check_row(&row), "{attribute}");
    }
    for attribute in [
        "#![cfg(any())]",
        "#![rewrite_route]",
        r#"#![cfg_attr(test, recursion_limit = "256", cfg(test))]"#,
    ] {
        std::fs::write(
            root.join(&crate_root).join("src/lib.rs"),
            format!("{attribute}\nmod shared;"),
        )
        .unwrap();
        assert!(
            CrateModules::load_for_route(&root, &crate_root, &row)
                .is_err_and(|error| error == "conditional required module `crate`"),
            "{attribute}"
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn source_routes_refuse_function_generic_wrappers_and_nested_item_scopes() {
    let owner = "pub(super) struct Ledger; impl Ledger { pub(super) fn capture() -> Option<Vec<Self>> { None } pub(super) fn revision(&self) {} }";
    for consumer in [
        "use super::owner::Ledger; fn consume<Vec>(ledger: &Ledger) { ledger.revision(); }",
        "use super::owner::Ledger; fn consume<const N: usize>(ledger: &Ledger) { ledger.revision(); }",
        "use super::owner::Ledger; fn consume(ledger: &Ledger) { fn nested() { ledger.revision(); } }",
        "use super::owner::Ledger; fn consume(ledger: &Ledger) { use fake::Ledger; ledger.revision(); }",
        "use super::owner::Ledger; fn consume(ledger: &Ledger) { struct Hidden; ledger.revision(); }",
    ] {
        let (modules, row) = fixture(owner, consumer, "Ledger::revision", "parameter:ledger");
        assert!(!modules.check_row(&row), "{consumer}");
    }
    let (modules, row) = fixture(
        "pub(super) struct Ledger; impl Ledger { pub(super) fn capture<Vec>() -> Option<Vec<Self>> { None } pub(super) fn revision(&self) {} }",
        "use super::owner::Ledger; fn consume() { let values = Ledger::capture()?; values.iter().map(|value| value.revision()); }",
        "Ledger::revision",
        "producer:crate::shared::owner::Ledger::capture",
    );
    assert!(!modules.check_row(&row));
}

#[test]
fn source_declaration_metadata_keeps_passive_lints_and_refuses_availability_changes() {
    let (modules, row) = fixture(
        "pub(super) struct Ledger; impl Ledger { #[must_use] #[inline] pub(super) fn revision(&self) -> bool { true } }",
        "use super::owner::Ledger; #[allow(clippy::too_many_lines)] fn consume(ledger: &Ledger) { ledger.revision(); }",
        "Ledger::revision",
        "parameter:ledger",
    );
    assert!(modules.check_row(&row));
    for attribute in [
        "#[cfg(test)]",
        "#[cfg_attr(test, allow(dead_code))]",
        "#[rewrite]",
        "#[path = \"other.rs\"]",
    ] {
        let consumer = format!(
            "use super::owner::Ledger; {attribute} fn consume(ledger: &Ledger) {{ ledger.revision(); }}"
        );
        let (modules, row) = fixture(
            "pub(super) struct Ledger; impl Ledger { pub(super) fn revision(&self) {} }",
            &consumer,
            "Ledger::revision",
            "parameter:ledger",
        );
        assert!(!modules.check_row(&row), "{attribute}");
    }
}

#[test]
fn source_routes_refuse_generic_field_and_nominal_type_argument_erasure() {
    let owner = "pub(super) struct Ledger; impl Ledger { pub(super) fn revision(&self) {} }";
    let (modules, row) = fixture(
        owner,
        "use super::owner::Ledger; struct Tokens<Ledger> { binding: Ledger } struct Fake; fn consume(parent: &Tokens<Fake>) { parent.binding.revision(); }",
        "Ledger::revision",
        "parameter-field:parent:binding",
    );
    assert!(!modules.check_row(&row));
    let (modules, row) = fixture(
        "pub(super) struct Ledger<T>(T); impl Ledger<Fake> { pub(super) fn revision(&self) {} }",
        "use super::owner::Ledger; fn consume(ledger: &Ledger<Other>) { ledger.revision(); }",
        "Ledger::revision",
        "parameter:ledger",
    );
    assert!(!modules.check_row(&row));
    let (modules, mut row) = fixture(
        owner,
        "struct Reader<'a> { ledger: &'a super::owner::Ledger } impl<'a> Reader<'a> { fn consume(&self) { self.ledger.revision(); } }",
        "Ledger::revision",
        "field:self.ledger",
    );
    row.consumer_entry = "Reader::consume".into();
    assert!(modules.check_row(&row));
}

#[test]
fn source_routes_reject_duplicate_declarations_and_wrong_visibility() {
    for owner in [
        "pub(super) struct Ledger; impl Ledger { fn capture() {} }",
        "pub(super) struct Ledger; impl Ledger { pub(super) fn capture() {} pub(super) fn capture() {} }",
        "#[cfg(test)] pub(super) struct Ledger; impl Ledger { pub(super) fn capture() {} }",
        "pub(super) struct Ledger; #[cfg(test)] impl Ledger { pub(super) fn capture() {} }",
    ] {
        let (modules, row) = fixture(
            owner,
            "fn consume() { super::owner::Ledger::capture(); }",
            "Ledger::capture",
            "static",
        );
        assert!(!modules.check_row(&row), "{owner}");
    }
    let (modules, mut row) = fixture(
        "pub(super) struct Ledger; impl Ledger { pub(super) fn capture() {} }",
        "fn consume() { super::owner::Ledger::capture(); }",
        "Ledger::capture",
        "static",
    );
    row.consumer_module = "crate::outside".into();
    assert!(!modules.check_row(&row));
}

#[test]
fn source_routes_resolve_actual_reexports_and_reject_private_alias_routes() {
    let (mut modules, row) = fixture(
        "pub(crate) struct Ledger; impl Ledger { pub(super) fn capture() {} }",
        "fn consume() { crate::shared::Ledger::capture(); }",
        "Ledger::capture",
        "static",
    );
    modules.modules.get_mut("crate::shared").unwrap().file =
        syn::parse_file("mod owner; pub(crate) use owner::Ledger;").unwrap();
    assert!(modules.check_row(&row));
    modules.modules.get_mut("crate::shared").unwrap().file =
        syn::parse_file("mod owner; #[cfg(test)] pub(crate) use owner::Ledger;").unwrap();
    assert!(!modules.check_row(&row));
}

#[test]
fn source_manifest_requires_exact_markers_scope_and_repository_paths() {
    assert!(parse(START).is_err());
    assert!(parse(END).is_err());
    assert!(parse(&format!("{START}\n{END}")).is_err());
    for source in [
        "/tmp/lib.rs",
        "rust/a/../a/src/lib.rs",
        "src/lib.rs",
        "rust/a/lib.rs",
    ] {
        assert!(crate_root(source).is_err(), "{source}");
    }
    assert_eq!(
        crate_root("rust/a/src/nested/owner.rs").unwrap(),
        Path::new("rust/a")
    );
}
#[test]
fn static_source_routes_keep_concrete_targets_under_bounded_function_generics() {
    let owner = "pub(super) struct Ledger; impl Ledger { pub(super) fn capture() {} }";
    for caller in [
        "fn consume<O: ObjectPointer>() { super::owner::Ledger::capture(); }",
        "struct Reader; impl Reader { fn consume<O: ObjectPointer>(&self) { super::owner::Ledger::capture(); } }",
    ] {
        let (modules, mut row) = fixture(owner, caller, "Ledger::capture", "static");
        if caller.starts_with("struct") {
            row.consumer_entry = "Reader::consume".into();
        }
        assert!(modules.check_row(&row), "{caller}");
    }
    for caller in [
        "use super::owner::Ledger; fn consume<Ledger: ObjectPointer>() { Ledger::capture(); }",
        "use super::owner::Ledger; fn consume<O: ObjectPointer>() { O::capture(); }",
        "fn consume<const N: usize>() { super::owner::Ledger::capture(); }",
        "fn consume<O>() { super::owner::Ledger::capture(); }",
        "fn consume<O>() where O: ObjectPointer { super::owner::Ledger::capture(); }",
        "#[cfg(any())] fn consume<O: ObjectPointer>() { super::owner::Ledger::capture(); }",
        "fn consume<O: ObjectPointer>() { #[cfg(any())] super::owner::Ledger::capture(); }",
    ] {
        let (modules, row) = fixture(owner, caller, "Ledger::capture", "static");
        assert!(!modules.check_row(&row), "{caller}");
    }
}

#[test]
fn local_source_declarations_shadow_only_module_globs() {
    let owner = "use super::*; pub(super) fn selected() {}";
    let (modules, row) = fixture(
        owner,
        "use super::*; fn consume() { super::owner::selected(); }",
        "selected",
        "static",
    );
    assert!(modules.check_row(&row));
    for owner in [
        "use super::*; use fake::selected; pub(super) fn selected() {}",
        "use super::{*, other::selected}; pub(super) fn selected() {}",
        "use super::*; #[cfg(any())] pub(super) fn selected() {}",
        "use super::*; pub(super) fn selected() {} pub(super) fn selected() {}",
    ] {
        let (modules, row) = fixture(
            owner,
            "fn consume() { super::owner::selected(); }",
            "selected",
            "static",
        );
        assert!(!modules.check_row(&row), "{owner}");
    }
    let (modules, row) = fixture(
        "pub(super) struct Ledger; impl Ledger { pub(super) fn capture() {} }",
        "use super::*; use super::owner::Ledger; fn consume() { Ledger::capture(); }",
        "Ledger::capture",
        "static",
    );
    assert!(
        !modules.check_row(&row),
        "a glob cannot manufacture an imported target identity"
    );
}
#[test]
fn literal_local_constants_preserve_independent_receiver_routes_and_block_scope() {
    let owner = "pub(super) struct Ledger; impl Ledger { pub(super) fn revision(&self) {} }";
    for body in [
        "const NAMES: &[&[u8]] = &[b\"which\".as_slice(), b\"code\"]; for _ in NAMES { ledger.revision(); }",
        "{ const ledger: u8 = 0; } ledger.revision();",
        "#[doc = \"literal data\"] const VALUE: (u8, bool) = (1, true); ledger.revision();",
    ] {
        let caller = format!("use super::owner::Ledger; fn consume(ledger: &Ledger) {{ {body} }}");
        let (modules, row) = fixture(owner, &caller, "Ledger::revision", "parameter:ledger");
        assert!(modules.check_row(&row), "{body}");
    }
}

#[test]
fn local_constants_cannot_supply_calls_or_bypass_shadow_and_availability_checks() {
    let owner = "pub(super) struct Ledger; impl Ledger { pub(super) fn revision(&self) {} pub(super) const fn capture() {} }";
    for body in [
        "ledger.revision(); const ledger: u8 = 0;",
        "const ledger: u8 = 0; ledger.revision();",
        "#[cfg(test)] const VALUE: u8 = 0; ledger.revision();",
        "const VALUE: () = { Ledger::capture() }; ledger.revision();",
        "const VALUE: u8 = opaque!(); ledger.revision();",
        "const VALUE: &[u8] = b\"x\".as_mut_slice(); ledger.revision();",
        "const VALUE: &[u8] = b\"x\".as_slice(1); ledger.revision();",
        "const VALUE: &[u8] = b\"x\".as_slice::<u8>(); ledger.revision();",
        "const VALUE: &[u8] = make().as_slice(); ledger.revision();",
        "const VALUE: &[u8] = \"x\".as_slice(); ledger.revision();",
    ] {
        let caller = format!("use super::owner::Ledger; fn consume(ledger: &Ledger) {{ {body} }}");
        let (modules, row) = fixture(owner, &caller, "Ledger::revision", "parameter:ledger");
        assert!(!modules.check_row(&row), "{body}");
    }
    let (modules, row) = fixture(
        owner,
        "use super::owner::Ledger; fn consume() { Ledger::capture(); const Ledger: u8 = 0; }",
        "Ledger::capture",
        "static",
    );
    assert!(!modules.check_row(&row));
    let (modules, row) = fixture(
        owner,
        "use super::owner::Ledger; fn consume() { const VALUE: () = { Ledger::capture() }; }",
        "Ledger::capture",
        "static",
    );
    assert!(!modules.check_row(&row));
}

#[test]
fn self_source_routes_keep_actual_plain_trait_consumer_bodies() {
    let owner = "pub(super) struct Ledger; impl Ledger { pub(super) fn revision(&self) {} }";
    let consumer = "use super::owner::Ledger; trait Reader { fn consume(&self); } impl Reader for Ledger { fn consume(&self) { self.revision(); } }";
    let (modules, mut row) = fixture(owner, consumer, "Ledger::revision", "self");
    row.consumer_entry = "Ledger::consume".into();
    assert!(modules.check_row(&row));
    for changed in [
        consumer.replace(
            "impl Reader for Ledger",
            "#[rewrite] impl Reader for Ledger",
        ),
        consumer.replace("fn consume(&self) {", "#[cfg(test)] fn consume(&self) {"),
        consumer.replace("self.revision();", "other.revision();"),
        consumer.replace("impl Reader for Ledger", "impl !Reader for Ledger"),
        consumer.replace("impl Reader for Ledger", "unsafe impl Reader for Ledger"),
        format!("{consumer} impl Ledger {{ fn consume(&self) {{ self.revision(); }} }}"),
    ] {
        let (modules, mut row) = fixture(owner, &changed, "Ledger::revision", "self");
        row.consumer_entry = "Ledger::consume".into();
        assert!(!modules.check_row(&row), "{changed}");
    }
    let (modules, mut row) = fixture(
        "pub(super) struct Ledger; pub(super) trait Reader { fn consume(&self); } impl Reader for Ledger { fn consume(&self) {} }",
        "use super::owner::{Ledger, Reader}; fn consume(ledger: &Ledger) { ledger.consume(); }",
        "Ledger::consume",
        "parameter:ledger",
    );
    row.visibility = "private".into();
    assert!(
        !modules.check_row(&row),
        "a trait consumer cannot become an inherent owner"
    );
}

#[test]
fn self_source_routes_keep_unbounded_result_parameters_without_result_donation() {
    let owner = "pub(super) struct Ledger; impl Ledger { pub(super) fn selected<T>(&self, mut publish: impl FnMut() -> Option<T>) -> Option<T> { publish() } }";
    let consumer = "use super::owner::Ledger; impl Ledger { fn consume(&self) { self.selected(|| Some(1)); } }";
    let (modules, mut row) = fixture(owner, consumer, "Ledger::selected", "self");
    row.consumer_entry = "Ledger::consume".into();
    assert!(modules.check_row(&row));
    let file = syn::parse_file(owner).unwrap();
    assert!(
        declaration(&file, "Ledger::selected").is_none(),
        "generic return inference keeps the original strict barrier"
    );
    for changed in [
        owner.replace("selected<T>", "selected<Ledger>"),
        owner.replace("selected<T>", "selected<const N: usize>"),
        owner.replace("selected<T>", "selected<T: Fake>"),
        owner.replace("&self,", "self: std::sync::Arc<Self>,"),
        owner.replace(
            "pub(super) fn selected",
            "#[rewrite] pub(super) fn selected",
        ),
    ] {
        let (modules, mut row) = fixture(&changed, consumer, "Ledger::selected", "self");
        row.consumer_entry = "Ledger::consume".into();
        assert!(!modules.check_row(&row), "{changed}");
    }
}

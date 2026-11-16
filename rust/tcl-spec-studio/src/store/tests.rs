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

const PACK: &str = r"# A hand-written banner the studio must never eat.

speclib demo 1.0 {

# Why greet exists, in the author's own words.
command greet {
    arity 1..2
    hover {
        summary {Say hello.}
    }
}

# And a second one, with its own comment.
command farewell {
    arity 1
}

}
";

#[test]
fn the_document_is_the_store_and_drafts_are_derived_from_it() {
    let store = PackStore::from_source(PACK);
    assert_eq!(store.name(), "demo");
    assert_eq!(store.dsl_version(), "1.0");
    let names: Vec<&str> = store.commands().iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, vec!["greet", "farewell"]);
    assert_eq!(store.source(), PACK, "the source is kept verbatim");
    assert_eq!(
        store.draft("greet").and_then(|d| d.get("name")),
        Some(&json!("greet"))
    );
}

#[test]
fn the_views_carry_what_the_pack_calls_itself_and_its_files() {
    let source = "speclib upf 1.1 {\ndisplay_name {IEEE 1801 UPF}\n\
file_extension upf -name {Unified Power Format} -dialect synopsys-eda-tcl\n\
command add_parameter {\narity 1..\n}\n}\n";
    let store = PackStore::from_source(source);
    let resolution = Resolution::new(Builtins::for_dialect("synopsys-eda-tcl"), &store);

    let view = resolution.store_view();
    assert_eq!(view["display_name"], json!("IEEE 1801 UPF"));
    assert_eq!(
        view["file_extensions"],
        json!([{
            "extension": "upf",
            "display_name": "Unified Power Format",
            "dialect": "synopsys-eda-tcl",
        }])
    );

    let command = resolution
        .view("add_parameter")
        .expect("the pack's command");
    assert_eq!(command["pack_display_name"], json!("IEEE 1801 UPF"));
    assert_eq!(command["pack_file_extensions"], view["file_extensions"]);
}

#[test]
fn a_pack_that_names_neither_reports_neither() {
    let store = PackStore::from_source(PACK);
    let view = Resolution::new(Builtins::for_dialect("tcl9.0"), &store).store_view();
    assert_eq!(view["display_name"], Value::Null);
    assert_eq!(view["file_extensions"], json!([]));
}

#[test]
fn a_command_span_covers_exactly_its_statement() {
    let span = command_span(PACK, "farewell").expect("farewell is declared");
    assert_eq!(&PACK[span], "command farewell {\n    arity 1\n}");
}

#[test]
fn the_store_index_matches_the_public_source_queries() {
    let store = PackStore::from_source(PACK);
    assert_eq!(store.source_index().pack_body, pack_body(PACK));
    for (name, _) in store.commands() {
        let indexed = store
            .source_index()
            .command(name)
            .map(|command| command.statement.clone());
        assert_eq!(indexed, command_span(PACK, name), "{name}");
    }
    assert!(store.source_index().command("not-declared").is_none());
}

#[test]
fn the_pack_body_span_is_the_contents_of_the_speclib_braces() {
    let body = pack_body(PACK).expect("a speclib body");
    let text = &PACK[body];
    assert!(
        text.trim_start().starts_with("# Why greet exists"),
        "{text}"
    );
    assert!(text.trim_end().ends_with('}'), "{text}");
    assert!(!text.contains("speclib demo"));
}

#[test]
fn a_form_edit_splices_and_leaves_every_other_byte_alone() {
    let mut store = PackStore::from_source(PACK);
    let mut edited = store.draft("greet").expect("greet").clone();
    edited.insert("return_type".to_owned(), json!("String"));

    assert_eq!(
        store.set_command("greet", &edited, false).how,
        WriteBack::Spliced
    );

    let text = store.source();
    assert!(
        text.contains("# A hand-written banner the studio must never eat."),
        "{text}"
    );
    assert!(
        text.contains("# And a second one, with its own comment."),
        "{text}"
    );
    assert!(
        text.contains("command farewell {\n    arity 1\n}"),
        "{text}"
    );
    assert!(text.contains("return_type"), "{text}");
    assert_eq!(
        store.draft("greet").and_then(|d| d.get("return_type")),
        Some(&json!("String"))
    );
    // Untouched neighbours keep their drafts byte for byte.
    assert_eq!(
        store.draft("farewell").and_then(|d| d.get("arity")),
        PackStore::from_source(PACK)
            .draft("farewell")
            .and_then(|d| d.get("arity"))
    );
}

#[test]
fn the_comment_inside_the_edited_command_is_the_documented_casualty() {
    // The banner above the command survives (it is not part of the
    // statement); a comment *inside* the braces does not. That is the
    // limitation this module's header states, pinned as a test so it
    // cannot regress into a surprise.
    let source = "speclib demo 1.0 {\ncommand greet {\n    # inside\n    arity 1\n}\n}\n";
    let mut store = PackStore::from_source(source);
    let mut edited = store.draft("greet").expect("greet").clone();
    edited.insert("return_type".to_owned(), json!("String"));
    store.set_command("greet", &edited, false);
    assert!(!store.source().contains("# inside"), "{}", store.source());
}

/// The bug this exists to stop: a form edit deleting the author's Tcl.
///
/// A hook body loads into a function pointer, so the draft the form holds
/// records only "set, expression not recovered" — render that back and the
/// body is gone. The write-back has to carry the original statement across
/// verbatim.
#[test]
fn a_form_edit_does_not_delete_a_hook_body_it_cannot_re_render() {
    let source = "speclib demo 1.0 {\n\
                      command greet {\n\
                      \x20   arity 1..2\n\
                      \x20   arg_role_resolver {words ctx} {\n\
                      \x20       role 1 Body\n\
                      \x20   }\n\
                      }\n}\n";
    let mut store = PackStore::from_source(source);
    let mut edited = store.draft("greet").expect("greet").clone();
    edited.insert("return_type".to_owned(), json!("String"));

    let write = store.set_command("greet", &edited, false);
    assert_eq!(write.how, WriteBack::Spliced);
    assert_eq!(write.dropped, Vec::<String>::new(), "{}", store.source());
    assert!(
        store.source().contains("arg_role_resolver {words ctx} {"),
        "the hook body must survive a form edit:\n{}",
        store.source()
    );
    assert!(
        store.source().contains("role 1 Body"),
        "the hook body's own text, verbatim:\n{}",
        store.source()
    );
    assert!(store.source().contains("return_type"), "{}", store.source());
    // And it is still a *live* hook after the round trip, not just text.
    assert!(
        store
            .draft("greet")
            .and_then(|d| d.get(draft::UNRENDERABLE_KEY))
            .and_then(Value::as_array)
            .is_some_and(|lost| lost.iter().any(|k| k == "arg_role_resolver")),
        "the reloaded command should carry the resolver again"
    );
}

/// A `state_transitions` resolver body is the author's Tcl like any other
/// hook body: a form edit carries the whole block forward verbatim, and
/// the reloaded command carries the descriptor again.
#[test]
fn a_state_transitions_resolver_body_survives_a_form_edit() {
    let source = "speclib demo 2.1 {\n\
                      command link {\n\
                      \x20   arity 2..\n\
                      \x20   state_transitions {\n\
                      \x20       argument_shape Positional\n\
                      \x20       resolver {words ctx} {\n\
                      \x20           alias 1 0\n\
                      \x20       }\n\
                      \x20       commit MayCommitBeforeAbruptCompletion\n\
                      \x20   }\n\
                      }\n}\n";
    let mut store = PackStore::from_source(source);
    let mut edited = store.draft("link").expect("link").clone();
    edited.insert("return_type".to_owned(), json!("String"));

    let write = store.set_command("link", &edited, false);
    assert_eq!(write.dropped, Vec::<String>::new(), "{}", store.source());
    assert!(
        store.source().contains("alias 1 0"),
        "the resolver body survives verbatim:\n{}",
        store.source()
    );
    assert!(
        store
            .source()
            .contains("commit MayCommitBeforeAbruptCompletion"),
        "{}",
        store.source()
    );
    assert!(store.source().contains("return_type"), "{}", store.source());
    assert!(
        store
            .draft("link")
            .and_then(|d| d.get(draft::UNRENDERABLE_KEY))
            .and_then(Value::as_array)
            .is_some_and(|lost| lost.iter().any(|k| k == "state_transitions")),
        "the reloaded command carries the descriptor again"
    );
}

/// A hook hanging off a **subcommand** survives a form edit to the
/// command around it: carry-forward reaches one level down now, so the
/// author's own `const_fold` bytes are spliced back in rather than
/// degrading to a `-native` placeholder and being reported as dropped.
#[test]
fn a_subcommands_hook_body_survives_a_form_edit() {
    let source = "speclib demo 1.0 {\n\
                      command greet {\n\
                      \x20   subcommand hi {\n\
                      \x20       arity 1\n\
                      \x20       const_fold {words ctx} {\n\
                      \x20           result hello\n\
                      \x20       }\n\
                      \x20   }\n\
                      }\n}\n";
    let mut store = PackStore::from_source(source);
    let mut edited = store.draft("greet").expect("greet").clone();
    edited.insert("return_type".to_owned(), json!("String"));

    let write = store.set_command("greet", &edited, false);
    assert_eq!(
        write.dropped,
        Vec::<String>::new(),
        "the subcommand's body is carried forward, not dropped:\n{}",
        store.source()
    );
    assert!(
        store.source().contains("result hello"),
        "the author's own const_fold body survives verbatim:\n{}",
        store.source()
    );
    assert!(
        !store.source().contains("-native"),
        "no placeholder was left behind:\n{}",
        store.source()
    );
    // And it is still a *live* hook after the round trip, not just text.
    let sub_lost = store
        .draft("greet")
        .and_then(|d| d["subcommands"][0].get(draft::UNRENDERABLE_KEY).cloned());
    assert!(
        sub_lost
            .as_ref()
            .and_then(Value::as_array)
            .is_some_and(|lost| lost.iter().any(|k| k == "const_fold")),
        "the reloaded subcommand should carry the folder again: {sub_lost:?}"
    );
}

#[test]
fn renaming_a_command_edits_it_where_it_stands() {
    let mut store = PackStore::from_source(PACK);
    let mut edited = store.draft("greet").expect("greet").clone();
    edited.insert("name".to_owned(), json!("salute"));

    assert_eq!(
        store.set_command("greet", &edited, false).how,
        WriteBack::Spliced
    );
    let names: Vec<&str> = store.commands().iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(
        names,
        vec!["salute", "farewell"],
        "a rename must not move the command to the end of the file"
    );
    assert!(
        store.source().contains("# Why greet exists"),
        "{}",
        store.source()
    );
}

#[test]
fn a_new_command_is_appended_to_the_pack_body() {
    let mut store = PackStore::from_source(PACK);
    let mut fresh = draft::default_command_draft();
    fresh.insert("name".to_owned(), json!("wave"));
    fresh.insert("arity".to_owned(), json!({ "min": 1, "max": 1 }));

    store.set_command("wave", &fresh, false);
    let names: Vec<&str> = store.commands().iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, vec!["greet", "farewell", "wave"]);
    assert!(
        store.source().contains("# A hand-written banner"),
        "{}",
        store.source()
    );
}

#[test]
fn removing_a_command_takes_its_statement_and_nothing_else() {
    let mut store = PackStore::from_source(PACK);
    assert!(store.remove_command("greet"));
    let names: Vec<&str> = store.commands().iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, vec!["farewell"]);
    assert!(store.source().contains("# A hand-written banner"));
    assert!(store.source().contains("# And a second one"));
    assert!(!store.remove_command("greet"), "already gone");
}

#[test]
fn an_empty_store_is_valid_dsl_that_loads_back_as_itself() {
    let store = PackStore::empty("mylib");
    assert_eq!(store.name(), "mylib");
    assert!(store.commands().is_empty());
    assert!(store.source().contains(&format!(
        "speclib mylib {} {{",
        crate::render_spectcl::DSL_VERSION
    )));
}

#[test]
fn selecting_a_shipped_command_and_canonicalising_preserves_the_pack_version() {
    // Selecting `doctools::search` seeds it into a fresh
    // 1.1 pack; a naive Pack DSL → Re-render canonically step would rewrite the
    // header to the renderer's newest (1.2) vocabulary. Selection itself
    // is a normal `set_command` write, so exercise the shared store rather
    // than teaching the browser a version rule of its own.
    let mut store = PackStore::from_source("speclib mylib 1.1 {\n}\n");
    let selected = Builtins::for_dialect("tcl9.0")
        .draft("doctools::search")
        .expect("doctools::search is a browsable shipped command");
    store.set_command("doctools::search", &selected, true);
    assert!(store.source().starts_with("speclib mylib 1.1 {"));

    // The browser hydrates every tab from this one stored draft.  Keep
    // the DSL, generated Rust, and Tcl stub outputs coupled so selecting
    // a registry command cannot leave either generated tab blank.
    let hydrated = store
        .draft("doctools::search")
        .expect("selected command is retained in the pack");
    let rust = crate::render_rs::render(hydrated);
    let stub = crate::render_stub::render(
        std::slice::from_ref(hydrated),
        crate::render_stub::Mode::Inline,
        "tcl9.0",
    );
    assert!(
        store
            .source()
            .contains("command doctools::search -override"),
        "DSL tab must contain the selected command:\n{}",
        store.source()
    );
    assert!(
        !rust.trim().is_empty() && rust.contains("doctools::search"),
        "Rust tab must render the selected command:\n{rust}"
    );
    assert!(
        !stub.trim().is_empty() && stub.contains("doctools::search"),
        "Tcl stub tab must render the selected command:\n{stub}"
    );

    let canonical = store.canonical();
    assert!(
        canonical.contains("\nspeclib mylib 1.1 {\n"),
        "canonical rendering must preserve the loaded header:\n{canonical}"
    );
    let round_tripped = PackStore::from_source(canonical);
    assert_eq!(round_tripped.dsl_version(), "1.1");
    assert!(round_tripped.draft("doctools::search").is_some());
    assert!(round_tripped.overrides_shipped("doctools::search"));
}

#[test]
fn adding_newer_registry_data_upgrades_the_header_before_the_first_render() {
    let mut store = PackStore::from_source("speclib mylib 1.1 {\n}\n");
    let selected = Builtins::for_dialect("tcl9.0")
        .draft("ttk::treeview")
        .expect("ttk::treeview is a browsable shipped command");
    let write = store.set_command("ttk::treeview", &selected, true);

    assert_eq!(write.how, WriteBack::VocabularyUpgraded);
    assert_eq!(write.upgraded_from.as_deref(), Some("1.1"));
    assert_eq!(store.dsl_version(), render_spectcl::DSL_VERSION);
    assert!(store.source().contains("command ttk::treeview -override"));
    assert!(
        store.notices().iter().all(|notice| !notice
            .message
            .contains("vocabulary, but this pack declares vocabulary")),
        "an upgraded document must not contain newer words under an older header: {:?}",
        store.notices()
    );
    assert_eq!(
        PackStore::from_source(store.canonical()).dsl_version(),
        render_spectcl::DSL_VERSION,
        "canonical re-render is stable after the edit-time upgrade"
    );
}

#[test]
fn override_survives_a_write_back() {
    let source = "speclib demo 1.0 {\ncommand lsort -override {\n    arity 1\n}\n}\n";
    let mut store = PackStore::from_source(source);
    assert!(store.overrides_shipped("lsort"));
    let mut edited = store.draft("lsort").expect("lsort").clone();
    edited.insert("return_type".to_owned(), json!("List"));
    store.set_command("lsort", &edited, true);
    assert!(
        store.overrides_shipped("lsort"),
        "-override must survive a form edit:\n{}",
        store.source()
    );
}

#[test]
fn the_resolution_facade_applies_the_shipped_collision_policy() {
    let source = "speclib demo 1.0 {\n\
                      command lsort {\n    arity 1\n}\n\
                      command uplift -override {\n    arity 1\n}\n\
                      command mything {\n    arity 1\n}\n}\n";
    let store = PackStore::from_source(source);
    let builtins = Builtins::for_dialect("tcl9.0");
    let merged = Resolution::new(builtins, &store);

    assert_eq!(merged.origin("lsort"), Some(Origin::Shadowed));
    assert_eq!(merged.origin("mything"), Some(Origin::Pack));
    assert_eq!(merged.origin("lappend"), Some(Origin::Builtin));
    assert_eq!(merged.origin("not::a::command"), None);
    // `uplift` is not a shipped Tcl command, so `-override` is harmless.
    assert_eq!(merged.origin("uplift"), Some(Origin::Pack));

    // Shadowed means the shipped spec is what the editor would use.
    let view = merged.view("lsort").expect("a view for lsort");
    assert_eq!(view["origin"], json!("shadowed"));
    assert_eq!(view["effective"], view["builtin"]);
    assert_ne!(view["pack"], Value::Null);

    let view = merged.view("mything").expect("a view for mything");
    assert_eq!(view["effective"], view["pack"]);
    assert_eq!(view["builtin"], Value::Null);
}

#[test]
fn the_store_view_counts_what_a_sidebar_shows() {
    let store = PackStore::from_source(PACK);
    let merged = Resolution::new(Builtins::for_dialect("tcl9.0"), &store);
    let view = merged.store_view();
    assert_eq!(view["pack"], json!("demo"));
    assert_eq!(view["summary"]["commands"], json!(2));
    let rows = view["commands"].as_array().expect("rows");
    assert_eq!(rows[0]["name"], json!("greet"));
    assert_eq!(rows[0]["summary"], json!("Say hello."));
    assert!(
        rows[0]["fields_set"].as_u64().is_some_and(|n| n >= 2),
        "{:?}",
        rows[0]
    );
}

#[test]
fn validation_reports_what_the_loader_dropped() {
    let source = "speclib demo 1.0 {\ncommand greet {\n    arity 1\n    nonsense yes\n}\n}\n";
    let store = PackStore::from_source(source);
    let merged = Resolution::new(Builtins::for_dialect("tcl9.0"), &store);
    let report = merged.validate();
    let notices = report["notices"].as_array().expect("notices");
    assert_eq!(notices.len(), 1, "{report}");
    assert_eq!(notices[0]["context"], json!("command greet"));
    assert!(
        notices[0]["reason"]
            .as_str()
            .is_some_and(|r| r.contains("nonsense")),
        "{report}"
    );
    assert_eq!(report["summary"]["notices"], json!(1));
    assert_eq!(report["commands"][0]["name"], json!("greet"));
}

#[test]
fn a_document_that_is_not_a_pack_loads_as_an_empty_store_with_a_notice() {
    let store = PackStore::from_source("this is not a pack\n");
    assert!(store.commands().is_empty());
    assert!(!store.notices().is_empty());
    // And it can still be written to — the write-back falls through to the
    // re-render floor, which produces a real pack.
    let mut store = store;
    let mut fresh = draft::default_command_draft();
    fresh.insert("name".to_owned(), json!("wave"));
    assert_eq!(
        store.set_command("wave", &fresh, false).how,
        WriteBack::Rerendered
    );
    assert_eq!(store.commands().len(), 1);
}

#[test]
fn canonical_rendering_round_trips_the_declared_commands() {
    let store = PackStore::from_source(PACK);
    let canonical = PackStore::from_source(store.canonical());
    let before: Vec<&str> = store.commands().iter().map(|(n, _)| n.as_str()).collect();
    let after: Vec<&str> = canonical
        .commands()
        .iter()
        .map(|(n, _)| n.as_str())
        .collect();
    assert_eq!(before, after);
    for (name, d) in store.commands() {
        assert_eq!(canonical.draft(name), Some(d), "{name} changed meaning");
    }
}

// ── The evaluation loader underneath (design E §15.2) ─────────────

/// A canonical document is observably what it always was: the store
/// browses the same commands, in the same order, with the same drafts.
#[test]
fn a_canonical_document_is_unchanged_by_evaluation() {
    let store = PackStore::from_source(PACK);
    assert_eq!(store.name(), "demo");
    let names: Vec<&str> = store.commands().iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(names, vec!["greet", "farewell"]);
    assert!(!store.target_dependent());
    assert_eq!(store.untrusted_tier_refusal(), None);
    // Every declaration is a statement in this document, so none is an
    // expansion.
    for (name, _) in store.commands() {
        let site = store.declaration_site(name).expect("a declaration site");
        assert!(!site.expanded, "{name}: {site:?}");
        assert!(site.line > 0, "{name}: {site:?}");
        assert_eq!(site.file, None);
    }
}

/// A *programmed* document browses its snapshot: the loop's output is
/// browsable, and each derived command is marked as an expansion with
/// the line inside the program that registered it.
#[test]
fn a_programmed_document_browses_its_expansion_with_provenance() {
    let store = PackStore::from_source(
        "speclib fleet 2.0 {\n    \
             proc fleet-command {name} {\n        \
             command fleet::$name {\n            arity 1\n        }\n    }\n    \
             foreach name {alpha beta} { fleet-command $name }\n}\n",
    );
    let names: Vec<&str> = store.commands().iter().map(|(n, _)| n.as_str()).collect();
    assert_eq!(
        names,
        vec!["fleet::alpha", "fleet::beta"],
        "{:?}",
        store.notices()
    );
    for name in names {
        let site = store.declaration_site(name).expect("a declaration site");
        assert!(site.expanded, "{name}: {site:?}");
        // The line points *into the program* — the registering statement,
        // not a `command` statement in the file, because there is none.
        assert!(site.line > 1, "{name}: {site:?}");
        assert!(
            crate::store::command_span(store.source(), name).is_none(),
            "{name}: an expanded command has no statement of its own"
        );
    }
}

/// `available?` makes the browsed surface one target's answer, and the
/// store says so.
#[test]
fn a_target_dependent_document_is_flagged() {
    let store = PackStore::from_source(
        "speclib trap 2.0 {\n    default available {tcl 8.6-}\n    \
             command base { arity 1 }\n    if {[available? {tcl 8.6-}]} {\n        \
             command extra { arity 1 }\n    }\n}\n",
    );
    assert!(store.target_dependent());
    assert_eq!(store.commands().len(), 2);
}

// ── E-R12: patch-pack editing ──────────────────────────────────────

/// The programmed document every patch test edits: one data table, one
/// helper `proc`, one `foreach`, and nothing a splice could reach.
const PROGRAMMED: &str = "speclib fleet 2.0 {\n    \
         proc fleet-command {name arity} {\n        \
         command fleet::$name {\n            arity $arity\n        }\n    }\n    \
         foreach {name arity} {alpha 1 beta 2} { fleet-command $name $arity }\n}\n";

/// A canonical pack is canonical, and the studio keeps editing it in
/// place — the property the round-trip suite rests on.
#[test]
fn a_canonical_document_is_not_programmed() {
    let store = PackStore::from_source(PACK);
    assert_eq!(store.programmed(), None);
    assert_eq!(store.patch_source(), None);
    assert!(store.standing_overrides().is_empty());
}

/// A pack whose declarations are all literal but which wraps them in a
/// program is still a program: no declaration is an expansion, so only
/// the straight-line check catches it — and it must, because the
/// re-render floor would delete the `proc`.
#[test]
fn a_literal_pack_around_a_program_is_still_programmed() {
    let store = PackStore::from_source(
        "speclib demo 2.0 {\n    set width 3\n    \
             command greet {\n        arity $width\n    }\n}\n",
    );
    assert_eq!(
        store.programmed(),
        Some(Programmed::NonCanonicalStatement),
        "{:#?}",
        store.notices()
    );
}

/// A form edit against a programmed pack leaves the source alone and
/// lands as a canonical patch pack in the `StudioOverride` tier.
#[test]
fn an_edit_to_a_programmed_pack_becomes_a_studio_override_patch() {
    let mut store = PackStore::from_source(PROGRAMMED);
    assert_eq!(store.programmed(), Some(Programmed::Expanded));
    let before = store.source().to_owned();

    let mut edited = store.draft("fleet::alpha").expect("a draft").clone();
    edited["arity"] = json!({ "min": 2, "max": 4 });
    let write = store.set_command("fleet::alpha", &edited, true);

    assert_eq!(write.how, WriteBack::Patched);
    assert!(write.dropped.is_empty(), "{:?}", write.dropped);
    assert_eq!(store.source(), before, "the program was rewritten");

    let patch = store.patch_source().expect("a patch pack");
    assert!(
        patch.starts_with("speclib fleet-studio-overrides 2.0 {"),
        "{patch}"
    );
    assert!(
        patch.contains("command fleet::alpha -override {"),
        "{patch}"
    );
    // Canonical by construction: it reloads as itself.
    assert_eq!(
        PackStore::from_source(patch).source(),
        patch,
        "the patch is not canonical"
    );
    // The base declaration is untouched, and the patch is what is live.
    assert_eq!(
        store
            .draft("fleet::alpha")
            .and_then(|d| d["arity"].get("min")),
        Some(&json!(1))
    );
    assert_eq!(
        store
            .effective_draft("fleet::alpha")
            .and_then(|d| d["arity"].get("min")),
        Some(&json!(2))
    );
}

/// The patch layers over the base by the **shipped** collision policy:
/// second pack in the set, `-override` on every declaration, so the
/// registry the Test tab queries answers with the patched spec.
#[test]
fn the_patch_layers_over_the_base_under_the_collision_policy() {
    let mut store = PackStore::from_source(PROGRAMMED);
    let mut edited = store.draft("fleet::alpha").expect("a draft").clone();
    edited["arity"] = json!({ "min": 2, "max": 4 });
    store.set_command("fleet::alpha", &edited, true);

    let set = store.pack_set();
    assert_eq!(set.packs.len(), 2);
    assert_eq!(set.packs[0].tier, Tier::Workspace);
    assert_eq!(set.packs[1].tier, Tier::StudioOverride);
    assert_eq!(set.packs[1].name, "fleet-studio-overrides");
    assert!(
        set.packs[1].commands.iter().all(|c| c.overrides_shipped),
        "a patch declaration that does not say -override cannot win"
    );

    let registry = Resolution::new(Builtins::for_dialect("tcl"), &store).registry();
    let spec = registry.get("fleet::alpha").expect("the patched command");
    assert_eq!(spec.arity.min, 2);
    // And the command the patch does not mention still comes from the base.
    assert_eq!(
        registry
            .get("fleet::beta")
            .expect("the base command")
            .arity
            .min,
        2
    );
}

/// The standing-overrides report names the patch, the base, and where the
/// base declaration came from — the data a UI labels a patched row with.
#[test]
fn the_standing_overrides_report_lists_the_patch() {
    let mut store = PackStore::from_source(PROGRAMMED);
    let edited = store.draft("fleet::beta").expect("a draft").clone();
    store.set_command("fleet::beta", &edited, true);

    let standing = store.standing_overrides();
    assert_eq!(standing.len(), 1);
    assert_eq!(standing[0].command, "fleet::beta");
    assert_eq!(standing[0].patch_pack, "fleet-studio-overrides");
    assert_eq!(standing[0].base_pack, "fleet");
    assert!(standing[0].base_expanded);
    assert!(standing[0].base_line.is_some_and(|line| line > 1));

    let view = Resolution::new(Builtins::for_dialect("tcl"), &store).store_view();
    assert_eq!(view["standing_overrides"][0]["command"], "fleet::beta");
    assert_eq!(view["programmed"]["why"], "expanded");
    assert_eq!(view["patch"]["pack"], "fleet-studio-overrides");
    assert_eq!(view["summary"]["standing_overrides"], 1);
}

/// Removing the patch restores the base declaration — the promise that
/// makes layering safe to try.
#[test]
fn removing_the_patch_restores_the_base() {
    let mut store = PackStore::from_source(PROGRAMMED);
    let mut edited = store.draft("fleet::alpha").expect("a draft").clone();
    edited["arity"] = json!({ "min": 2, "max": 4 });
    store.set_command("fleet::alpha", &edited, true);
    let key_with_patch = store.overlay_key();

    assert!(store.remove_override("fleet::alpha"));
    assert_eq!(store.patch_source(), None);
    assert!(store.standing_overrides().is_empty());
    assert_eq!(store.pack_set().packs.len(), 1);
    assert_ne!(store.overlay_key(), key_with_patch);
    assert_eq!(
        store
            .effective_draft("fleet::alpha")
            .and_then(|d| d["arity"].get("min")),
        Some(&json!(1))
    );
    assert!(!store.remove_override("fleet::alpha"), "already gone");
}

/// Two edits share one patch pack, and a patch survives a round trip
/// through its own text — which is how a browser reload restores it.
#[test]
fn a_patch_accumulates_and_round_trips_through_its_own_text() {
    let mut store = PackStore::from_source(PROGRAMMED);
    for name in ["fleet::alpha", "fleet::beta"] {
        let draft = store.draft(name).expect("a draft").clone();
        store.set_command(name, &draft, true);
    }
    assert_eq!(store.standing_overrides().len(), 2);
    let patch = store.patch_source().expect("a patch").to_owned();

    let restored = PackStore::from_source_with_patch(PROGRAMMED, &patch);
    assert_eq!(restored.patch_source(), Some(patch.as_str()));
    assert_eq!(restored.standing_overrides().len(), 2);
    assert_eq!(restored.overlay_key(), store.overlay_key());
}

/// The patch carries the base pack's `default` rows, so a patched command
/// keeps the availability of the one it replaces rather than silently
/// widening it — the "minimal available context" a patch pack needs.
#[test]
fn the_patch_carries_the_base_packs_available_context() {
    let mut store = PackStore::from_source(
        "speclib fleet 2.0 {\n    default available {tcl 8.6-}\n    \
             proc fleet-command {name} {\n        \
             command fleet::$name {\n            arity 1\n        }\n    }\n    \
             foreach name {alpha beta} { fleet-command $name }\n}\n",
    );
    let base = store
        .draft("fleet::alpha")
        .expect("a draft")
        .get("available")
        .cloned();
    store.set_command(
        "fleet::alpha",
        &store.draft("fleet::alpha").expect("a draft").clone(),
        true,
    );
    let patch = store.patch_source().expect("a patch pack");
    assert!(patch.contains("default available"), "{patch}");
    assert_eq!(
        store
            .patch_draft("fleet::alpha")
            .expect("the patched draft")
            .get("available")
            .cloned(),
        base,
        "the patch changed more than the author edited"
    );
}

/// A patch that claims a compiled name loads for its author and would be
/// refused by a real `StudioOverride` load — reported, not applied, the
/// same posture the document itself gets.
#[test]
fn a_patch_over_a_compiled_name_reports_its_override_tier_refusal() {
    let mut store = PackStore::from_source(
        "speclib trap 2.0 {\n    set names lsort\n    \
             foreach n $names { command $n -override {\n        arity 1..\n    } }\n}\n",
    );
    assert!(store.programmed().is_some());
    let draft = store.draft("lsort").expect("a draft").clone();
    assert_eq!(
        store.set_command("lsort", &draft, true).how,
        WriteBack::Patched
    );
    let (_, why) = store
        .patch_untrusted_tier_refusal()
        .expect("an override-tier refusal");
    assert!(why.contains("design E-R2"), "{why}");
}

/// The authoring buffer is trusted — a studio author may write an
/// `-override` on a compiled name — and the refusal a workspace load
/// would raise is reported instead of applied.
#[test]
fn an_override_is_authorable_and_its_workspace_refusal_is_reported() {
    let store = PackStore::from_source(
        "speclib demo 2.0 {\n    command lsort -override {\n        arity 1..\n    }\n}\n",
    );
    assert_eq!(store.tier(), Tier::Bundled);
    assert_eq!(store.commands().len(), 1, "{:#?}", store.notices());
    assert!(store.overrides_shipped("lsort"));
    let (line, why) = store
        .untrusted_tier_refusal()
        .expect("a workspace tier would refuse this");
    assert_eq!(line, 2);
    assert!(why.contains("design E-R2"), "{why}");

    // A *workspace* pack is `Provenance::WorkspaceTrusted` (redesign
    // §6.4 keys the untrusted class on the editor's Workspace Trust
    // state, not on where the file was found), so it still loads and
    // still overrides — the report above is the warning about the day
    // it does not.
    let workspace = PackStore::from_source_at_tier(store.source(), Tier::Workspace);
    assert_eq!(workspace.commands().len(), 1);

    // The live Spec Studio override tier *is* untrusted, and refuses it,
    // which is what makes the report meaningful rather than theoretical.
    let override_tier = PackStore::from_source_at_tier(store.source(), Tier::StudioOverride);
    assert!(override_tier.commands().is_empty());
}

/// A codegen-axis stamp is authorable — the document and its draft keep
/// the row — but the installed world drops it, as a workspace load does,
/// and the report says so on the command's row. Only the stamp goes.
#[test]
fn a_stamp_is_kept_in_the_document_and_dropped_from_the_installed_world() {
    let store = PackStore::from_source(
        "speclib vendor 2.0 {\n    command vendor::unpack {\n        arity 2..\n        \
             alias_of lassign\n        codegen_hook -native Lassign\n    }\n}\n",
    );
    let draft = store.draft("vendor::unpack").expect("a draft");
    assert!(
        draft
            .get("codegen_hook")
            .is_some_and(|value| !value.is_null()),
        "the draft keeps the row as written: {draft:?}"
    );
    assert_eq!(
        store.stamp_refusals(),
        vec![(
            2,
            "`codegen_hook Lassign` refused for `vendor::unpack`: a trusted workspace pack \
                 may not name a codegen catalogue member; only a bundled pack may carry \
                 `alias_of lassign`'s own stamp"
                .to_owned()
        )]
    );
    assert!(store.patch_stamp_refusals().is_empty());

    let registry =
        tcl_spectcl::install::registry_for_dialect_with_packs("tcl8.6", &store.pack_set());
    let installed = registry.get("vendor::unpack").expect("installed");
    assert_eq!(installed.codegen_hook, None, "the installed world drops it");
    assert_eq!(installed.arity.min, 2, "and keeps every other fact");
    assert_eq!(installed.alias_of, Some("lassign"));
}

/// A stamp in a window is a stamp: the draft keeps the row, the report
/// names it, and the installed world drops it from the windows.
#[test]
fn a_windowed_stamp_is_kept_in_the_document_and_dropped_from_the_installed_world() {
    let store = PackStore::from_source(
        "speclib vendor 2.2 {\n    command vendor::unpack {\n        arity 2..\n        \
             alias_of lassign\n        codegen_hook -native Lassign -introduced 9.0\n    }\n}\n",
    );
    let draft = store.draft("vendor::unpack").expect("a draft");
    assert_eq!(
        draft["codegen_hook_windows"][0]["value"],
        serde_json::json!("Lassign"),
        "the draft keeps the window as written: {draft:?}"
    );
    assert_eq!(
        store.stamp_refusals(),
        vec![(
            2,
            "`codegen_hook Lassign` refused for `vendor::unpack`: a trusted workspace pack \
                 may not name a codegen catalogue member; only a bundled pack may carry \
                 `alias_of lassign`'s own stamp"
                .to_owned()
        )]
    );

    let registry =
        tcl_spectcl::install::registry_for_dialect_with_packs("tcl8.6", &store.pack_set());
    let installed = registry.get("vendor::unpack").expect("installed");
    assert!(
        installed.codegen_hook_windows.is_empty(),
        "no window survives"
    );
    assert_eq!(installed.arity.min, 2, "and every other fact does");
    assert_eq!(installed.alias_of, Some("lassign"));
}

#[test]
fn original_pack_store_keeps_file_bom_and_nested_splice_geometry() {
    // naming.source.original-configured-script-completeness
    // docs/design/analysis/name-resolution-proofs/source-original-configured-script-completeness.md
    let source = "\u{feff}speclib demo 1.0 {\ncommand visible {arity 0}\n\u{feff}command hidden {arity 0}\n}\n";
    let store = PackStore::from_source(source);
    assert_eq!(store.source(), source);
    assert_eq!(store.name(), "demo");
    assert_eq!(store.dsl_version(), "1.0");
    assert!(store.draft("visible").is_some());
    assert!(store.draft("hidden").is_none());
    let body = pack_body(source).expect("same genuine file body");
    let visible = command_span(source, "visible").expect("same original command extent");
    assert_eq!(&source[visible.clone()], "command visible {arity 0}");
    assert!(body.start <= visible.start && visible.end <= body.end);
    assert!(command_span(source, "hidden").is_none());
    let root = segments(source, SourceSegmentationPurpose::WholeInput);
    assert_eq!(root[0].words[0], "speclib");
    assert_eq!(root[0].span.start, '\u{feff}'.len_utf8());
    let block = "\u{feff}command generated {arity 0}";
    let nested = segments(block, SourceSegmentationPurpose::NestedScript);
    assert_eq!(nested[0].words[0], "\u{feff}command");
    assert_eq!(nested[0].span.start, 0);
    assert_eq!(
        segments(&source[body], SourceSegmentationPurpose::NestedScript)[1].words[0],
        "\u{feff}command"
    );
}

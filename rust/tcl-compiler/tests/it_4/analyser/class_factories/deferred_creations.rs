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

//! Creations deferred until their metaclass is proved: replay, define stubs, conflicting creations and the incremental path.

use super::*;

#[test]
fn a_post_pass_metaclass_classifies_a_creation_in_its_own_file() {
    // TP — `::T::D::class` is proved a metaclass only by the
    // parameterised-class join, which runs *after* the walk, so a
    // creation call naming it in the same document had nothing to be
    // classified against and the class it makes went unrecorded: no
    // factory, no members, no completion.
    //
    // tclsh 8.6.16 / 9.0.4 run the whole chain: `::T::W` exists, is a
    // class, `info object class ::T::W` is `::T::D::class`, its methods
    // are `go`, and `[[::T::W new] go]` answers `went`.
    let src = format!(
        "{COMPUTED_METACLASS}::T::D::class create ::T::W {{ method go {{}} {{ return went }} }}\n"
    );
    let result = analysis(&src, "tcl9.0");
    let w = result
        .all_classes
        .get("::T::W")
        .expect("the class the post-pass metaclass makes must be recorded");
    assert!(
        w.methods.contains_key("go"),
        "its members come from the creation body; got {:?}",
        w.methods.keys().collect::<Vec<_>>()
    );
    assert!(
        w.metaclass.ends_with("T::D::class"),
        "the recorded metaclass is the head the source wrote; got {:?}",
        w.metaclass
    );
    // `::T::W` declares no superclass, so it is an ordinary class rather
    // than another factory — the replay must not manufacture one.
    assert!(
        !result.class_factories().contains_key("::T::W"),
        "a plain class must publish no factory; got {:?}",
        result.class_factories().keys().collect::<Vec<_>>()
    );
}

#[test]
fn a_replayed_creation_can_itself_prove_the_next_one() {
    // The replay runs in document order, so a chain settles in one pass:
    // `::T::Sub` is created by the post-pass metaclass *and* declares it
    // as its superclass, which makes `::T::Sub` a factory in turn, which
    // is what classifies `::T::Leaf`.
    //
    // tclsh 8.6.16 / 9.0.4: `info object class ::T::Sub` is
    // `::T::D::class`, `info object class ::T::Leaf` is `::T::Sub`, and
    // `[[::T::Leaf new] go]` answers `leafy`.
    let src = format!(
        "{COMPUTED_METACLASS}\
             ::T::D::class create ::T::Sub {{ superclass ::T::D::class }}\n\
             ::T::Sub create ::T::Leaf {{ method go {{}} {{ return leafy }} }}\n"
    );
    let result = analysis(&src, "tcl9.0");
    assert!(
        result.class_factories().contains_key("::T::Sub"),
        "a replayed class that inherits the metaclass is a factory too; got {:?}",
        result.class_factories().keys().collect::<Vec<_>>()
    );
    let leaf = result
        .all_classes
        .get("::T::Leaf")
        .expect("the second link of the chain must be recorded");
    assert!(
        leaf.methods.contains_key("go"),
        "got {:?}",
        leaf.methods.keys().collect::<Vec<_>>()
    );
}

#[test]
fn an_unknown_head_that_never_becomes_a_factory_records_nothing() {
    // FP guard: deferring a call is not a claim about it. A head that is
    // still unknown when the join has settled — no proc, no class, no
    // registry command — must record exactly what it did before this
    // pass existed, which is nothing at all.
    let src = concat!(
        "namespace eval ::T {}\n",
        "::T::NoSuchMeta create ::T::Q { method m {} {} }\n",
    );
    let result = analysis(src, "tcl9.0");
    assert!(
        !result.all_classes.contains_key("::T::Q"),
        "an unproved head must manufacture nothing; got {:?}",
        result.all_classes.keys().collect::<Vec<_>>()
    );
}

#[test]
fn the_incremental_path_records_the_same_classes_as_a_full_analysis() {
    // The deferred creation is *pending verdict* state, so it has to
    // survive a snapshot/restore exactly as the pending-arity buffers
    // do. The LSP's incremental path
    // restores a snapshot covering the clean prefix and re-walks only the
    // dirty chunk: if the buffer is not in the snapshot, a creation call
    // written in that prefix is gone, while the evidence that proves its
    // metaclass — the placeholder and the load-time call — is restored
    // with the result. The join then proves `::T::D::class` and nothing
    // recreates `::T::W`, so the same document answers differently
    // depending on which entry point analysed it.
    let src = format!(
        "{COMPUTED_METACLASS}\
             ::T::D::class create ::T::W {{ method go {{}} {{ return went }} }}\n\
             set unrelated 1\n"
    );
    let full = analysis(&src, "tcl9.0");
    assert!(
        full.all_classes.contains_key("::T::W"),
        "precondition: the full analysis records the class"
    );

    let commands = tcl_source::segment::segment_commands(&src);
    let split = commands.len() - 1;
    let (prefix, dirty) = commands.split_at(split);
    let mut analyser = Analyser::new();
    let (_, snapshots) =
        analyser.analyse_chunked(&src, vec![prefix.to_vec(), dirty.to_vec()], "tcl9.0");
    // Re-analyse the dirty chunk from the clean prefix's snapshot, which
    // is what an edit to the last line does.
    let mut incremental = Analyser::new();
    incremental.restore(snapshots[0].clone());
    let restored = incremental.analyse_commands(&src, dirty, "tcl9.0", true);

    let mut full_classes: Vec<&String> = full.all_classes.keys().collect();
    let mut restored_classes: Vec<&String> = restored.all_classes.keys().collect();
    full_classes.sort();
    restored_classes.sort();
    assert_eq!(
        restored_classes, full_classes,
        "incremental analysis must record the same classes as a full one"
    );
}

#[test]
fn a_deferred_creation_never_leaks_into_the_next_document() {
    // The other half of owning per-run state: an analyser is reused, and
    // a walk that never runs the post-walk tail (`finalise: false`, the
    // partial-snapshot path) leaves the buffer full. Those calls belong
    // to a document that is over — their tokens index its source — so
    // carrying them forward would let the *next* document, which happens
    // to prove the same metaclass, record a class it never wrote.
    let mut analyser = Analyser::new();
    let first =
        format!("{COMPUTED_METACLASS}::T::D::class create ::T::W {{ method go {{}} {{}} }}\n");
    let first_commands = tcl_source::segment::segment_commands(&first);
    let _ = analyser.analyse_commands(&first, &first_commands, "tcl9.0", false);

    let second = COMPUTED_METACLASS.to_owned();
    let second_commands = tcl_source::segment::segment_commands(&second);
    let result = analyser.analyse_commands(&second, &second_commands, "tcl9.0", true);
    assert!(
        !result.all_classes.contains_key("::T::W"),
        "the second document writes no creation call; got {:?}",
        result.all_classes.keys().collect::<Vec<_>>()
    );
}

#[test]
fn a_creation_written_before_its_metaclass_is_proved_stays_unrecorded() {
    // The limit this deliberately keeps. Written in this order the
    // program does not run at all: tclsh 8.6.16 / 9.0.4 both raise
    // `invalid command name "::U::D::class"`, because the metaclass does
    // not exist until `::U::mk` is called on the next line. The walk's
    // own reachability proof stops at the unknown head, so nothing is
    // claimed — the correct answer for a program with no successful run.
    let src = concat!(
        "namespace eval ::U {}\n",
        "oo::class create ::U::Mother { superclass oo::class }\n",
        "proc ::U::mk {name} { ::U::Mother create ${name}::class { superclass ::U::Mother } }\n",
        "namespace eval ::U::D {}\n",
        "::U::D::class create ::U::W { method go {} {} }\n",
        "::U::mk ::U::D\n",
    );
    let result = analysis(src, "tcl9.0");
    assert!(
        !result.all_classes.contains_key("::U::W"),
        "a creation that cannot have run must not be recorded; got {:?}",
        result.all_classes.keys().collect::<Vec<_>>()
    );
}

#[test]
fn a_replayed_creation_keeps_the_members_its_define_stub_already_held() {
    // The regression the replay would otherwise cause. While the creation
    // was invisible, `oo::define ::T::W` had nothing to extend, so the
    // walk made a *stub* and put `extra` in it. Recording the creation
    // afterwards under the same name must not throw that away — the class
    // really has both members (tclsh 8.6.16 / 9.0.4: `info class methods
    // ::T::W` is `go extra`).
    let src = format!(
        "{COMPUTED_METACLASS}\
             ::T::D::class create ::T::W {{ method go {{}} {{ return went }} }}\n\
             ::oo::define ::T::W {{ method extra {{}} {{}} }}\n"
    );
    let result = analysis(&src, "tcl9.0");
    let w = result
        .all_classes
        .get("::T::W")
        .expect("the created class is recorded");
    assert!(
        w.methods.contains_key("go") && w.methods.contains_key("extra"),
        "both the creation body's member and the stub's must survive; got {:?}",
        w.methods.keys().collect::<Vec<_>>()
    );
    assert!(
        !w.via_define,
        "this file creates the class, so the record is not an extension stub"
    );
    assert!(
        w.metaclass.ends_with("T::D::class"),
        "got {:?}",
        w.metaclass
    );
}

#[test]
fn a_replayed_creation_with_a_relative_name_homes_where_it_was_written() {
    // A replayed creation goes back through the handler the walk uses, so
    // its name homes by the same rule: `create W` written inside
    // `namespace eval ::T` is `::T::W`, not a global `::W`. Recording it
    // anywhere else would put a class in the index no interpreter has.
    //
    // tclsh 8.6.16 / 9.0.4: `info commands ::T::W` finds it and
    // `info commands ::W` does not.
    let src = format!(
        "{COMPUTED_METACLASS}namespace eval ::T {{\n    ::T::D::class create W {{ method go {{}} {{}} }}\n}}\n"
    );
    let result = analysis(&src, "tcl9.0");
    assert!(
        result.all_classes.contains_key("::T::W"),
        "the relative name homes to the enclosing namespace; got {:?}",
        result.all_classes.keys().collect::<Vec<_>>()
    );
    assert!(
        !result.all_classes.contains_key("::W"),
        "and nowhere else; got {:?}",
        result.all_classes.keys().collect::<Vec<_>>()
    );
}

#[test]
fn a_head_the_walk_knew_only_as_a_define_stub_is_still_deferred() {
    // "Known" is not "judged". The `oo::define` here records `::T::D::class`
    // during the walk — as a stub with no factory, because the creation
    // that proves it is still a computed name — so a creation call naming
    // it looks like a call to a *recorded class*. Its verdict is no more
    // final than an unknown head's: the post-pass join is what gives that
    // stub its factory, and only then can this call be classified.
    //
    // tclsh 8.6.16 / 9.0.4 run it: `::T::D::class` has method `extra`,
    // `info object class ::T::W` is `::T::D::class`, and `[[::T::W new] go]`
    // answers `went`.
    let src = format!(
        "{COMPUTED_METACLASS}\
             ::oo::define ::T::D::class {{ method extra {{}} {{ return e }} }}\n\
             ::T::D::class create ::T::W {{ method go {{}} {{ return went }} }}\n"
    );
    let result = analysis(&src, "tcl9.0");
    let w = result
        .all_classes
        .get("::T::W")
        .expect("a stub-known metaclass must still classify its creation");
    assert!(
        w.methods.contains_key("go"),
        "got {:?}",
        w.methods.keys().collect::<Vec<_>>()
    );
    assert!(
        result
            .all_classes
            .get("::T::D::class")
            .is_some_and(|meta| meta.methods.contains_key("extra")),
        "and the metaclass keeps the stub's own member"
    );
}

#[test]
fn a_later_define_replaces_the_replayed_creation_superclass() {
    // The creation body and the later `oo::define` declare different
    // superclasses. They run in order, so the define replaces the
    // superclass and both bodies' members stay. tclsh 8.6.18 / 9.0.4 /
    // 9.1.0: `info class superclasses ::T::W` is `::T::Other`, `info class
    // methods ::T::W` is `extra go`, and `info object class ::T::W` is
    // `::T::D::class`. `::T::Other` is no metaclass, so `::T::W` ends a
    // plain class and publishes no factory.
    let src = format!(
        "{COMPUTED_METACLASS}\
             oo::class create ::T::Other {{}}\n\
             ::T::D::class create ::T::W {{ superclass ::T::Mother\n method go {{}} {{}} }}\n\
             ::oo::define ::T::W {{ superclass ::T::Other\n method extra {{}} {{}} }}\n"
    );
    let result = analysis(&src, "tcl9.0");
    let w = result
        .all_classes
        .get("::T::W")
        .expect("the class is recorded");
    assert!(
        !w.inheritance_unknown && w.superclasses == ["::T::Other"],
        "the later define replaces the superclass; got {:?} (unknown {})",
        w.superclasses,
        w.inheritance_unknown
    );
    assert!(
        w.methods.contains_key("go") && w.methods.contains_key("extra"),
        "both bodies' members stay; got {:?}",
        w.methods.keys().collect::<Vec<_>>()
    );
    assert!(
        w.metaclass.ends_with("T::D::class"),
        "the creating metaclass is unchanged; got {:?}",
        w.metaclass
    );
    assert!(
        !result.class_factories().contains_key("::T::W"),
        "a class whose superclass is no metaclass publishes no factory"
    );
}

#[test]
fn full_tcllib_clay_dialect_metaclass_resolves_when_the_corpus_is_available() {
    // Exact tcllib 2.0 corpus oracle. Developer/bootstrap environments
    // fetch this source under tmp; a source-only distribution may omit it.
    //
    // Corpus-gated: a checkout without the corpus must not
    // read as a silent pass — announce the skip on stderr, matching the
    // convention `tcl-lsp-db/tests/compiler_check_corpus.rs` already uses
    // for the same tcllib-2.0 corpus ("skip: {path} not present"), so
    // `cargo test -- --nocapture` (or a failure elsewhere in the run)
    // makes it obvious this test asserted nothing.
    //
    // # What this pins, and why not more
    //
    // `::clay::dialect::MotherOfAllMetaClasses` is written literally
    // (`::oo::class create … { superclass ::oo::class }`), so it is a
    // straight true positive: the corpus must publish its factory, and
    // `::clay::class` must be recorded as a class.
    //
    // What is deliberately *not* asserted is that `::clay::class`
    // publishes a factory of its own. That class is manufactured inside
    // `::clay::dialect::create` under the computed name
    // `${NSPACE}::class`, so proving the name needs the whole dominating
    // prefix to be statically executable — from the file's first command
    // to the `::clay::dialect::create ::clay` call site, and from the
    // procedure's first statement to the creation. Four constructs stand
    // in that prefix. Two were *over-broad* abstentions and are fixed
    // alongside this test (see
    // `a_collection_loop_in_the_dominating_prefix_preserves_provenance`
    // and `a_declaration_body_the_walk_cannot_read_is_not_a_blocker`).
    // The other two are genuine limits of the static-execution walk, both
    // inside the `::clay::PROC` helper the file calls at load time:
    //
    //  * `if {[info commands $name] ne {}} return` — a conditional
    //    `return` selected by the live command table. The walk treats "an
    //    arm may not reach the next statement" as a reason to abstain
    //    rather than distinguishing an early *normal* completion from an
    //    abnormal one;
    //  * `eval $ninja` — the fall-through proof carries no argument
    //    binding, so it cannot see that this call site leaves `ninja` at
    //    its `{}` default.
    //
    // Asserting the *absence* of a `::clay::class` factory would ossify
    // those two limits: lifting either is a strict improvement and must
    // not be reported here as a failure. So this test asserts only what
    // is proved, and the prose above records the boundary instead.
    let corpus = std::env::var_os("TCLLIB_2_0_DIR").map_or_else(
        || std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tmp/tcllib-2.0"),
        std::path::PathBuf::from,
    );
    let path = corpus.join("modules/clay/clay.tcl");
    let Ok(src) = std::fs::read_to_string(&path) else {
        eprintln!(
            "skip: {} not present (fetch tcllib 2.0 under tmp/, or set TCLLIB_2_0_DIR)",
            path.display()
        );
        return;
    };
    let result = analysis(&src, "tcl9.0");
    assert!(
        result.all_classes.contains_key("::clay::class"),
        "the exact tcllib 2.0 corpus must record ::clay::class; got {:?}",
        result.all_classes.keys().collect::<Vec<_>>()
    );
    assert_eq!(
        result
            .all_classes
            .get("::clay::dialect::MotherOfAllMetaClasses")
            .map(|class| class.superclasses.as_slice()),
        Some(&["::oo::class".to_owned()][..]),
        "the literally-written dialect root must keep its superclass"
    );
    assert!(
        result
            .class_factories()
            .contains_key("::clay::dialect::MotherOfAllMetaClasses"),
        "the literally-written dialect root must publish its factory; factories={:?}",
        result.class_factories().keys().collect::<Vec<_>>()
    );
}

#[test]
fn the_unresolvable_record_is_retracted_once_a_real_name_is_proved() {
    // FP guard — a class literally named `::T::${ns}::class`
    // exists in no interpreter, so once the real name is proved the
    // phantom must not remain beside it in the class index.
    let result = analysis(COMPUTED_METACLASS, "tcl9.0");
    assert!(
        !result.all_classes.contains_key("::T::${ns}::class"),
        "the unresolved placeholder must be retracted; got {:?}",
        result.all_classes.keys().collect::<Vec<_>>()
    );
}

#[test]
fn a_later_define_stub_does_not_cost_the_metaclass_its_factory() {
    // TP — an `oo::define` on the computed-name class creates a
    // stub whose `metaclass` was never written, so it holds
    // `ClassDef::default()`'s `"oo::class"`. That default is not an
    // observation: joining it against the proved `::T::Mother` must not
    // read as two walks disagreeing — treating it that way would
    // abstain the record down to `factory: None` with no superclasses.
    let src = format!("{COMPUTED_METACLASS}::oo::define ::T::D::class {{ method m {{}} {{}} }}\n");
    let result = analysis(&src, "tcl9.0");
    let class = result
        .all_classes
        .get("::T::D::class")
        .expect("the resolved metaclass must still be recorded");
    assert_eq!(
        class.superclasses.as_slice(),
        &["::T::Mother".to_owned()][..],
        "the proved superclass must survive the stub join"
    );
    assert!(
        result.class_factories().contains_key("::T::D::class"),
        "the resolved metaclass must keep its factory; factories={:?}",
        result.class_factories().keys().collect::<Vec<_>>()
    );
    assert!(
        class.metaclass.ends_with("T::Mother"),
        "the joined record must name the metaclass the source wrote, \
             not the default stand-in; got {:?}",
        class.metaclass
    );
    assert!(
        class.methods.contains_key("m"),
        "the stub's own member must still land; methods={:?}",
        class.methods.keys().collect::<Vec<_>>()
    );
    assert!(
        !class.via_define,
        "the class is created in this file, so it is not an extension stub"
    );
}

#[test]
fn a_define_stub_replaces_a_member_the_creation_body_also_declared() {
    // The two records the join unions are ordered whenever one
    // of them is an `oo::define`: the class has to exist before it can
    // be extended, so the stub ran second and its `method m` replaces.
    // tclsh 8.6.16 on the same shape answers `{a b} …` for
    // `info class definition ::T::D::class m`.
    let src = format!(
        "{}::oo::define ::T::D::class {{ method m {{a b}} {{}} }}\n",
        COMPUTED_METACLASS.replace(
            "    ::T::Mother create ${NSPACE}::class { superclass ::T::Mother }\n",
            "    ::T::Mother create ${NSPACE}::class {\n\
                 \x20       superclass ::T::Mother\n\
                 \x20       method m {a} {}\n\
                 \x20   }\n",
        )
    );
    let result = analysis(&src, "tcl9.0");
    let method = result
        .all_classes
        .get("::T::D::class")
        .and_then(|class| class.methods.get("m"))
        .expect("the contested member is recorded once");
    assert_eq!(
        method.params.len(),
        2,
        "the `oo::define` body's declaration is the live one; got {:?}",
        method.params
    );
}

#[test]
fn a_second_creation_of_an_existing_class_fails_and_keeps_the_first() {
    // The second creation names a class that already exists, so it raises
    // and the first creation's record stands. tclsh 8.6.18 / 9.0.4 /
    // 9.1.0: `::T::mk ::T::D` -> `can't create object "::T::D::class":
    // command already exists with that name`, and `info object class
    // ::T::D::class` stays `::T::MotherA`.
    let src = concat!(
        "namespace eval ::T {}\n",
        "oo::class create ::T::MotherA { superclass oo::class }\n",
        "oo::class create ::T::MotherB { superclass oo::class }\n",
        "::T::MotherA create ::T::D::class { method m {} {} }\n",
        "proc ::T::mk {name} {\n",
        "    ::T::MotherB create ${name}::class { method m {} {} }\n",
        "}\n",
        "::T::mk ::T::D\n",
    );
    let result = analysis(src, "tcl9.0");
    let class = result
        .all_classes
        .get("::T::D::class")
        .expect("the first creation's class is recorded");
    assert!(
        class.superclasses.is_empty() && !class.inheritance_unknown,
        "the failing second creation changes nothing; got {:?} (unknown {})",
        class.superclasses,
        class.inheritance_unknown
    );
    assert!(
        class.metaclass.ends_with("T::MotherA")
            && class.metaclass_provenance == MetaclassProvenance::Observed,
        "the metaclass is the first creation's; got {:?}",
        class.metaclass
    );
    assert!(
        !result.class_factories().contains_key("::T::D::class"),
        "a class with no metaclass superclass publishes no factory"
    );
}

#[test]
fn a_define_stub_that_wins_the_join_still_sheds_its_stub_provenance() {
    // The mirror of the case above: the stub declares the
    // superclass and the creation body does not, so the *stub* is the
    // more complete observation and wins the join. It is still not a
    // cross-file extension record — this file creates the class — and
    // the creation observation's own facts must survive underneath it.
    let src = format!(
        "{}::oo::define ::T::D::class {{ superclass ::T::Mother }}\n",
        COMPUTED_METACLASS.replace(
            "    ::T::Mother create ${NSPACE}::class { superclass ::T::Mother }\n",
            "    ::T::Mother create ${NSPACE}::class { method m {} {} }\n",
        )
    );
    let result = analysis(&src, "tcl9.0");
    let class = result
        .all_classes
        .get("::T::D::class")
        .expect("the resolved metaclass must still be recorded");
    assert_eq!(
        class.superclasses.as_slice(),
        &["::T::Mother".to_owned()][..],
        "the stub's superclass must survive"
    );
    assert!(
        class.methods.contains_key("m"),
        "the creation body's member must survive the stub winning; methods={:?}",
        class.methods.keys().collect::<Vec<_>>()
    );
    assert!(
        class.metaclass.ends_with("T::Mother")
            && class.metaclass_provenance == MetaclassProvenance::Observed,
        "the winning stub never read a metaclass, so it must adopt the \
             one the creation observed; got {:?}",
        class.metaclass
    );
    assert!(
        !class.via_define,
        "a record one of whose observations is the creation is not a stub"
    );
}

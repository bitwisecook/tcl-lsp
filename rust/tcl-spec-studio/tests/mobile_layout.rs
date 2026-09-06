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

//! Pins the front-end rules the studio's phone layout depends on.
//!
//! Reported from an iPhone 15 Pro Max: the page rendered far wider than the
//! screen, and typing a command name into the filter box appeared to do
//! nothing. Both were real.
//!
//! The width was not a missing breakpoint — the breakpoint fired and the grid
//! did collapse to one column. A grid item's automatic minimum size is its
//! *min-content* width, and one long unwrapped command summary in the browser
//! list gave `.cmdlist` a min-content of 764px, so the lone `1fr` track
//! resolved to 806px inside a 430px viewport and scrolled the whole page
//! sideways. `min-width: 0` on the grid children is what caps it.
//!
//! The second was a discoverability failure: the box only ever filtered the
//! list, and on a phone the list sits below the fold, so a typed name looked
//! ignored. It now loads by name via an explicit button, the Enter key, and a
//! native `<datalist>`.
//!
//! These are source-level assertions, not a rendering test — a browser check
//! needs Playwright, which does not run in this suite. They exist so the
//! specific rules that fix a 430px viewport cannot be dropped silently by an
//! unrelated style edit. The measured verification (six viewports × five
//! tabs, zero horizontal overflow) is recorded in the commit that added them.

const CSS: &str = include_str!("../web/src/studio.css");
const HTML: &str = include_str!("../web/studio.html");
const STUDIO_TS: &str = include_str!("../web/src/studio.ts");

/// Strip whitespace so an assertion survives reformatting.
fn squashed(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace()).collect()
}

#[test]
fn grid_children_cannot_be_widened_by_their_own_content() {
    // The single rule that stops a long command summary dragging the page
    // wider than the phone. Without it the track sizes to min-content.
    assert!(
        squashed(CSS).contains(".split>*{min-width:0;}"),
        "`.split > * {{ min-width: 0 }}` is missing — a grid item's automatic \
         minimum is its min-content width, so the browser list will size the \
         whole track to its longest summary and the page will scroll sideways \
         on a phone"
    );
}

#[test]
fn the_command_list_summary_stays_clipped() {
    // `white-space: nowrap` + ellipsis only clips when the box is actually
    // constrained; without `min-width: 0` the box grows instead of clipping.
    let css = squashed(CSS);
    assert!(
        css.contains("white-space:nowrap;min-width:0;"),
        "the `.cmdlist .sm` summary must carry `min-width: 0` alongside \
         `white-space: nowrap`, or it grows its container instead of ellipsing"
    );
}

#[test]
fn a_phone_breakpoint_exists_below_the_tablet_one() {
    // 62rem collapses the two-column split; phones need a second, narrower
    // breakpoint that also unwraps the toolbar and scrolls the tab strip.
    assert!(
        squashed(CSS).contains("@media(max-width:34rem)"),
        "the phone breakpoint (max-width: 34rem) is missing"
    );
    assert!(
        CSS.contains("@media (pointer: coarse)"),
        "the coarse-pointer block that lifts tap targets to 44px is missing"
    );
}

#[test]
fn a_typed_command_name_can_be_loaded_without_the_list() {
    // The reported bug: filtering was the *only* way in, and the list is below
    // the fold on a phone. All three entry points must survive.
    assert!(
        HTML.contains(r#"id="loadCmd""#),
        "the Load button next to the filter box is missing"
    );
    assert!(
        HTML.contains(r#"id="cmdOptions""#) && HTML.contains(r#"list="cmdOptions""#),
        "the filter box must be wired to a <datalist> for native autocomplete"
    );
    assert!(
        STUDIO_TS.contains("loadTypedCommand"),
        "the load-by-name path is missing"
    );
    for wiring in [r#"byId("loadCmd").addEventListener"#, r#""Enter""#] {
        assert!(
            STUDIO_TS.contains(wiring),
            "load-by-name is not wired up: {wiring} not found"
        );
    }
}

#[test]
fn an_ambiguous_or_unknown_name_is_reported_rather_than_guessed() {
    // Loading the wrong command silently is worse than saying nothing matched,
    // so both branches must stay present.
    assert!(
        STUDIO_TS.contains("no command matches"),
        "an unmatched name must be reported"
    );
    assert!(
        STUDIO_TS.contains("pick one from the list"),
        "an ambiguous name must be reported rather than resolved arbitrarily"
    );
}

#[test]
fn autocomplete_suggests_name_matches_only() {
    // The list matches summaries too, but an autocomplete answering "lin" with
    // a command whose *summary* says "linear" is noise. Suggestions are name
    // matches, prefix first.
    assert!(
        STUDIO_TS.contains("startsWith(query)"),
        "datalist suggestions must rank prefix matches first"
    );
}

/* The live documentation dock.
 *
 * The dock is the surface that documents whatever the author is touching
 * *without moving it*, so its whole value rests on layout rules: it is a
 * column of its own where there is room, a bottom bar where there is not, and
 * a one-line strip on a phone. The rules below are the ones that stop it
 * covering the control being edited — the failure that would make it worse
 * than the inline panel it was built to supplement. */

#[test]
fn the_dock_is_its_own_column_only_where_there_is_room_for_one() {
    let css = squashed(CSS);
    assert!(
        css.contains("@media(min-width:75rem)"),
        "the dock's sidebar breakpoint (min-width: 75rem ≈ 1200px) is missing — \
         below it the dock has to be a bottom bar, not a third grid track"
    );
    assert!(
        css.contains("grid-template-columns:20remminmax(0,1fr)23rem"),
        "the wide layout must give the dock its own track between 22 and 26rem, \
         with the workbench on a `minmax(0, 1fr)` so a long line cannot widen it"
    );
}

#[test]
fn the_bottom_bar_reserves_its_own_room_at_the_end_of_the_page() {
    // A fixed bar covers whatever the page ends with unless the page keeps
    // that much padding — and on a phone the expanded dock is half the
    // viewport, which is most of the form.
    let css = squashed(CSS);
    assert!(
        css.contains("--dock-h"),
        "`--dock-h` is missing — the page and the fields both size their \
         clearance from how much room the dock is taking"
    );
    assert!(
        css.contains("padding-bottom:calc(var(--dock-h)+2rem)"),
        "the page must reserve the bottom bar's height, or its last field \
         cannot be scrolled out from under it"
    );
    assert!(
        css.contains("scroll-margin-bottom:calc(var(--dock-h)+1rem)"),
        "a field must carry the bar's height as scroll margin, or a deep link \
         scrolls it to exactly where the bar covers it"
    );
    assert!(
        css.contains(r#":root[data-dock="collapsed"]"#),
        "a collapsed dock must reserve less room than an expanded one"
    );
}

#[test]
fn the_dock_collapses_to_a_strip_that_still_names_its_subject() {
    assert!(
        HTML.contains(r#"id="dockToggle""#) && HTML.contains(r#"id="dockSubject""#),
        "the collapsed dock is a strip carrying the current subject's name and \
         the control that reopens it"
    );
    assert!(
        HTML.contains(r#"aria-controls="dockBody""#) && HTML.contains(r#"aria-expanded="true""#),
        "the collapse control must report what it controls and its state"
    );
    assert!(
        STUDIO_TS.contains("max-width: 34rem)\").matches"),
        "a phone must start on the summary line rather than covering the form"
    );
}

#[test]
fn the_dock_is_a_labelled_region_whose_changing_half_is_announced() {
    assert!(
        HTML.contains(r#"<aside class="dock""#) && HTML.contains(r#"aria-labelledby="dockLabel""#),
        "the dock must be a labelled complementary region"
    );
    assert!(
        HTML.contains(r#"id="dockBody" aria-live="polite""#),
        "the part of the dock that changes as focus moves must be a polite live region"
    );
}

#[test]
fn a_deep_link_lands_on_a_stable_anchor_and_says_where_it_landed() {
    assert!(
        STUDIO_TS.contains("fieldAnchorId(field.key)") && STUDIO_TS.contains("revealField"),
        "every field control needs the stable anchor id a related-setting link \
         navigates to"
    );
    assert!(
        squashed(CSS).contains(".field.dock-target{animation:dock-flash"),
        "the landing flash is what tells one row of a 137-setting form from its \
         neighbours"
    );
    assert!(
        CSS.contains("@media (prefers-reduced-motion: reduce)")
            && squashed(CSS).contains("animation:none;outline:2pxsolidvar(--accent)"),
        "reduced motion must hold a static outline rather than dropping the \
         answer to where the link landed"
    );
}

#[test]
fn the_dock_follows_focus_and_never_the_pointer() {
    for wiring in [
        "\"focusin\"",
        "\"change\"",
        "retargetFromForm",
        "retargetFromBrowser",
    ] {
        assert!(
            STUDIO_TS.contains(wiring),
            "the dock follows focus and deliberate choices: {wiring} not found"
        );
    }
    for churn in ["mouseover", "mouseenter", "pointerover"] {
        assert!(
            !STUDIO_TS.contains(churn),
            "the dock must not re-target on {churn} — a pointer crossing the \
             form would churn the panel"
        );
    }
}

/* The open-command tab strip.
 *
 * Several commands open at once is what makes the studio an IDE for the DSL
 * rather than a form with one slot — and the strip that carries them is the
 * one new thing on the page that can push the workbench around. A strip that
 * wraps grows a row every few tabs and moves the form under it; a strip whose
 * tabs shrink to fit is twelve tabs nobody can read. Both are pinned here. */

#[test]
fn the_open_command_strip_scrolls_sideways_rather_than_wrapping() {
    let css = squashed(CSS);
    assert!(
        css.contains(".opentabs{display:flex;flex-wrap:nowrap;"),
        "the open-command strip must never wrap — a second row of tabs changes \
         the strip's height and moves the form underneath it"
    );
    assert!(
        css.contains("overflow-x:auto;overflow-y:hidden;"),
        "the open-command strip must scroll sideways when it overflows, the \
         way the workbench tab strip already does on a phone"
    );
    assert!(
        css.contains(".opentab{flex:00auto;"),
        "an open-command tab must hold its size: tabs squeezed to fit are tabs \
         that no longer name the command they carry"
    );
}

#[test]
fn an_open_command_tab_is_a_real_tab_that_can_be_closed_from_the_keyboard() {
    assert!(
        HTML.contains(r#"id="openTabs" role="tablist" aria-label="Open commands""#),
        "the strip must be a labelled tablist, not a row of buttons"
    );
    assert!(
        STUDIO_TS.contains(r#"role: "tab","#)
            && STUDIO_TS.contains(r#""aria-controls": "pane-editor""#),
        "each open-command tab must own the editor panel it switches"
    );
    for wiring in [
        "focusTabButton",
        "\"ArrowRight\"",
        "\"Delete\"",
        "closeActiveOpenTab",
    ] {
        assert!(
            STUDIO_TS.contains(wiring),
            "the strip must be navigable and closable from the keyboard: {wiring} not found"
        );
    }
    assert!(
        STUDIO_TS.contains("auxclick"),
        "middle-click must close a tab, as it does on every other tab strip"
    );
}

#[test]
fn an_open_command_tab_is_a_view_and_never_a_second_store() {
    // The property the whole feature rests on: the `.tclspec` document stays
    // the one model, and `writeBackOpenCommand` stays the one path into it.
    assert!(
        STUDIO_TS.contains("state.tabs = ") && !STUDIO_TS.contains("tab.draft"),
        "a tab must carry its command's name and its view, never a draft"
    );
    assert!(
        STUDIO_TS.contains("function flushEdits"),
        "moving between commands has to commit the pending write-back first, or \
         a keystroke inside the settle window is lost when the form is rebuilt"
    );
    assert!(
        STUDIO_TS.contains("retainTabs(state.tabs, names)"),
        "a tab is a view of a declaration: deleting the declaration must take \
         the tab with it"
    );
}

#[test]
fn the_open_command_strip_has_room_for_a_finger_and_a_narrow_screen() {
    let css = squashed(CSS);
    assert!(
        css.contains(".opentab>[role=tab]{max-width:8.5rem;}"),
        "the phone breakpoint must shorten the tab names so more than one tab \
         is reachable without dragging the strip"
    );
    assert!(
        css.contains(".opentab>[role=tab],.opentab.tabclose{min-height:44px;}"),
        "a coarse pointer must get 44px on both the tab and its close control"
    );
}

/* The command palette's provenance.
 *
 * The registry browser has said what it is viewing since packs became its top
 * level; the palette searched more surfaces and said less about any of them. */

#[test]
fn the_palette_says_what_it_searched_and_where_each_hit_came_from() {
    assert!(
        HTML.contains(r#"id="paletteScope""#),
        "the palette needs a line naming the surfaces it searched, the way the \
         browser's count line names the dialect and its packs"
    );
    for wiring in [
        "paletteSummary",
        "surfaceLabel",
        "markedText",
        "searchPalette",
    ] {
        assert!(
            STUDIO_TS.contains(wiring),
            "the palette must label and rank its hits: {wiring} not found"
        );
    }
    assert!(
        STUDIO_TS.contains("candidate.pack ? packChip(candidate.pack) : null"),
        "a hit's pack chip is the only thing that says which shipped pack \
         declares it, and the row already has the data"
    );
    assert!(
        squashed(CSS)
            .contains(".palette.sm{color:var(--muted);font-size:.78rem;flex:11auto;min-width:0;"),
        "the summary must take the slack and give it back, or a long one pushes \
         the provenance chips off the end of the row"
    );
    assert!(
        squashed(CSS).contains(".palettemark{"),
        "the matched run has to be visibly marked, in both themes"
    );
}

/* The pack export.
 *
 * The Export tab replaced two per-command output panes with one pack-level
 * reader: a list of every file the pack produces beside the one it is showing.
 * That is a second two-column split on a page that already has one, and the
 * failure mode is the same — a long rendered path is min-content wide, and an
 * unconstrained grid item sizes its track to it. */

#[test]
fn the_export_split_collapses_and_cannot_be_widened_by_a_rendered_path() {
    let css = squashed(CSS);
    assert!(
        css.contains(".exportsplit{display:grid;grid-template-columns:17remminmax(0,1fr);"),
        "the export pane must put the file list beside the file on a wide \
         screen, with the file on a `minmax(0, 1fr)` so a long line cannot \
         widen the track"
    );
    assert!(
        css.contains(".exportsplit>*{min-width:0;}"),
        "`.exportsplit > * {{ min-width: 0 }}` is missing — \
         `rust/tcl-registry/src/commands/mylib/greet.rs` is one unbroken word, \
         so without it the list sizes its track to that path and the pane \
         scrolls sideways on a phone"
    );
    assert!(
        css.contains("@media(max-width:62rem){.exportsplit{grid-template-columns:1fr;}}"),
        "below the tablet breakpoint the export pane must stack, the way the \
         browser/workbench split and the Test tab's own split already do"
    );
}

#[test]
fn the_export_file_list_stays_a_chooser_on_a_phone() {
    let css = squashed(CSS);
    assert!(
        css.contains(".exportlist{max-height:14rem;}"),
        "stacked, the file list sits above the file it chose — uncapped, a \
         nine-file pack pushes the editor entirely off the screen"
    );
    assert!(
        css.contains(".editorhost.output{height:22rem;}"),
        "the phone breakpoint must shorten the output editor, or the list and \
         the file together are three screens of scrolling"
    );
    assert!(
        css.contains(".exportfile{min-height:44px;}"),
        "a coarse pointer must get 44px on a file row, as it does on every \
         other list row in the studio"
    );
}

#[test]
fn the_export_file_list_is_a_list_that_can_be_driven_from_the_keyboard() {
    assert!(
        HTML.contains(
            r#"<ul class="exportlist" id="exportList" role="listbox" tabindex="0" aria-label="Files this pack produces">"#
        ),
        "the file list must be a focusable, labelled listbox rather than a pile \
         of clickable rows"
    );
    for wiring in [
        "moveExportSelection",
        "\"ArrowDown\"",
        "\"Home\"",
        "aria-activedescendant",
    ] {
        assert!(
            STUDIO_TS.contains(wiring),
            "the file list must be navigable from the keyboard: {wiring} not found"
        );
    }
    assert!(
        STUDIO_TS.contains(r#"byId("exportView").setAttribute("aria-labelledby""#),
        "the editor shows one file at a time, so it must be labelled by the row \
         that chose it rather than by a heading that never changes"
    );
}

#[test]
fn the_inline_help_panels_remain_the_narrow_viewport_surface() {
    // The dock is an additional surface over the same schema text, not a
    // replacement: on a phone the inline panel is still the primary one.
    assert!(
        STUDIO_TS.contains("helpButton(help, field.label)"),
        "the field's inline ? button must survive the dock"
    );
    assert!(
        STUDIO_TS.contains("helpWithExample(field.help, field.example)"),
        "the inline panel and the dock must render the same schema help"
    );
}

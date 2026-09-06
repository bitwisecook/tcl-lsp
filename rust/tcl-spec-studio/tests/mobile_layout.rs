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

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

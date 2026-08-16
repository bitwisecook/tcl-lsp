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

//! Residual-coverage port for `tcl-lsp-core`'s hover provider
//! (`src/hover.rs`).  The sibling `hover_completion.rs` exercises the
//! common proc / builtin / variable surface; this file drives the
//! lower-traffic branches that port leaves uncovered:
//!
//!   * the format-string family — `binary format`/`scan` (byte ruler, `@`
//!     seek, short-type labels), `regsub` substitution backrefs, `glob`
//!     patterns (incl. `lsearch -glob`), and `regexp`/`regsub` regex
//!     patterns;
//!   * IP-literal hovers (IPv4 class / CIDR, IPv6 loopback / link-local /
//!     IPv4-mapped);
//!   * `interp alias` target hovers;
//!   * `-option` hovers and the `cmd subcommand` fall-through;
//!   * `TclOO` class hovers carrying superclass / mixin / class-method /
//!     instance-variable detail, class-member hovers (property, destructor,
//!     constructor), and `$obj method` dispatch;
//!   * proc doc-comment (`@brief`/`@param`/`@return`) rendering through the
//!     public `hover` entry;
//!   * inferred-intrep / taint annotations on `$var` hovers (registry path);
//!   * the public span helpers `find_word_span_at_position` /
//!     `find_var_at_position` on braced / array / boundary inputs.
//!
//! ## Proof split
//!
//! Hover bodies for Tcl built-ins encode Tcl-language FACTS — a `binary`
//! field's byte width, a `regsub` backref's meaning, an `interp alias`
//! target, a glob/regex metacharacter's effect, an IP literal's class.
//! Every assertion pinning such a fact carries a `// tclsh-proof:` line
//! citing what `scripts/dev/tclsh_check.sh` confirmed on tclsh **8.6** and
//! **9.0** (the two reference interpreters installed here).  Purely
//! LSP-structural facts — that hover is `None` in whitespace, the markdown
//! framing, which span a word helper returns — need no Tcl proof and are
//! asserted directly.

use tcl_compiler::analyser::{Analyser, AnalysisResult};
use tcl_lsp_core::hover::{HoverKind, find_var_at_position, find_word_span_at_position, hover};
use tcl_registry::CommandRegistry;

// ------------------------------------------------------------------
// harness — mirrors hover_completion.rs
// ------------------------------------------------------------------

fn analyse(source: &str) -> AnalysisResult {
    analyse_for_dialect(source, "tcl8.6")
}

fn analyse_for_dialect(source: &str, dialect: &str) -> AnalysisResult {
    let mut a = Analyser::new();
    a.analyse(source, dialect).clone()
}

fn registry() -> CommandRegistry {
    CommandRegistry::build_default()
}

// ==================================================================
// binary format / scan hover — byte widths are Tcl facts.
// ==================================================================

#[test]
fn residual_hover_binary_format_braced_spec_renders_field_table() {
    // tclsh-proof: `binary scan [binary format c1 65] c x; set x` -> 65, so
    // `c` is one signed byte; `binary scan [binary format a3 abc] a3 s` ->
    // `abc`, so `a3` is a 3-byte null-padded string. The hover labels these
    // (`int8`, `str (null-pad)`) and sums the field bytes.
    let src = "binary format {c a3} $b1 $b2\n";
    let analysis = analyse(src);
    // Cursor inside the `{c a3}` format literal.
    let h = hover(src, 0, 16, &analysis, None).expect("binary format hover");
    assert_eq!(h.kind, HoverKind::Markdown);
    assert!(h.value.contains("binary format"), "{}", h.value);
    assert!(
        h.value.contains("int8"),
        "expected int8 type label: {}",
        h.value
    );
    assert!(
        h.value.contains("str (null-pad)"),
        "expected null-pad str label: {}",
        h.value,
    );
}

#[test]
fn residual_hover_binary_scan_labels_fields_with_var_names() {
    // tclsh-proof: `binary scan A H2 hx; set hx` -> 41, so `H2` reads two
    // high-to-low hex nibbles. The scan hover pulls the trailing token
    // (`hx`) as the field's Variable-column label.
    let src = "binary scan $buf H2 hx\n";
    let analysis = analyse(src);
    // Cursor on the `H2` format word (argv[3] of `binary scan`).
    let h = hover(src, 0, 17, &analysis, None).expect("binary scan hover");
    assert!(h.value.contains("binary scan"), "{}", h.value);
    assert!(
        h.value.contains("hex hi→lo"),
        "expected hex label: {}",
        h.value
    );
    // The scan target variable name labels the field.
    assert!(
        h.value.contains("hx"),
        "expected var-name label `hx`: {}",
        h.value
    );
}

#[test]
fn residual_hover_binary_absolute_seek_byte_math() {
    // tclsh-proof: `set b [binary format c@4c 65 66]; string length $b` -> 5
    // — the `@4` seeks to absolute offset 4, so one leading `c` (offset 0)
    // plus the seek-gap plus a trailing `c` total 5 bytes. The hover's
    // byte accounting includes the `@`-seek gap, so the summary reports 5
    // bytes (and is still small enough to draw the ruler).
    let src = "binary format c@4c $a $b\n";
    let analysis = analyse(src);
    // Cursor on the bare `c@4c` format word (argv[2]).
    let h = hover(src, 0, 15, &analysis, None).expect("binary @-seek hover");
    assert!(h.value.contains("binary format"), "{}", h.value);
    assert!(
        h.value.contains('5'),
        "expected 5-byte total from @4 seek: {}",
        h.value
    );
    // A small total (<=32) with all-known sizes draws the box-ruler.
    assert!(
        h.value.contains('┌'),
        "expected byte-ruler diagram: {}",
        h.value
    );
}

// ==================================================================
// regsub substitution spec — backref meanings are Tcl facts.
// ==================================================================

#[test]
fn residual_hover_regsub_subspec_backreference_table() {
    // tclsh-proof: `regsub {(a)(b)} ab {\2\1} r; set r` -> `ba`, i.e. `\1`
    // is the first capture group and `\2` the second (swapped here). The
    // subspec hover names each backref.
    let src = "regsub {(a)(b)} $in {\\2\\1} out\n";
    let analysis = analyse(src);
    // Cursor inside the `{\2\1}` substitution literal (the 4th arg).
    let h = hover(src, 0, 21, &analysis, None).expect("regsub subspec hover");
    assert!(h.value.contains("Substitution spec"), "{}", h.value);
    assert!(
        h.value.contains("First capture group"),
        "expected \\1 description: {}",
        h.value,
    );
    assert!(
        h.value.contains("Second capture group"),
        "expected \\2 description: {}",
        h.value,
    );
}

// ==================================================================
// glob pattern — metacharacter meanings are Tcl facts.
// ==================================================================

#[test]
fn residual_hover_glob_char_class_via_string_match() {
    // tclsh-proof: `string match {[abc]*} bxx` -> 1 — the `[abc]` class
    // matches one of a/b/c and `*` matches the rest. The hover surfaces the
    // class and the star.
    let src = "string match {[abc]*} $s\n";
    let analysis = analyse(src);
    // Cursor inside the `{[abc]*}` pattern literal.
    let h = hover(src, 0, 15, &analysis, None).expect("glob hover");
    assert!(h.value.contains("Glob pattern"), "{}", h.value);
    assert!(
        h.value.contains("Character class"),
        "expected char-class row: {}",
        h.value,
    );
    assert!(
        h.value.contains("any sequence"),
        "expected `*` description: {}",
        h.value,
    );
}

#[test]
fn residual_hover_glob_via_lsearch_dash_glob() {
    // tclsh-proof: `set list {foo bar baz}; lsearch -glob $list ba*` -> 1
    // (matches `bar` at index 1), confirming `lsearch -glob` is a real glob
    // entry point even with a dynamic list operand. The detector fires on
    // the `-glob` form too.
    let src = "lsearch -glob $list {ba*}\n";
    let analysis = analyse(src);
    // Cursor inside the `{ba*}` pattern.
    let h = hover(src, 0, 22, &analysis, None).expect("lsearch -glob hover");
    assert!(h.value.contains("Glob pattern"), "{}", h.value);
    assert!(
        h.value.contains("any sequence"),
        "expected `*` row: {}",
        h.value,
    );
}

#[test]
fn residual_hover_lsearch_dynamic_mode_keeps_pattern_ambiguous() {
    // tclsh-proof: with `set mode -start`, `lsearch $mode {a b} {a+}` errors
    // "missing starting index": `$mode` is still in lsearch's outer option
    // scan, not the list operand. Its runtime value could instead be -glob,
    // -regexp, -exact, or another switch, so no pattern-language hover is
    // sound for the source-level `{a+}` word.
    let src = "lsearch $mode {a b} {a+}\n";
    let analysis = analyse(src);
    assert!(
        hover(src, 0, 21, &analysis, None).is_none(),
        "a dynamic lsearch option prefix must not claim a glob/regex hover"
    );
}

#[test]
fn residual_hover_lsearch_invalid_option_prefix_abstains() {
    // A dash-prefixed literal before lsearch's final list + pattern operands
    // remains in Tcl's option scan. These spellings are rejected rather than
    // becoming the list operand: -str is unavailable in Tcl 8, -st is
    // ambiguous in Tcl 9, and lsearch does not support --.
    for (dialect, option) in [("tcl8.6", "-str"), ("tcl9.0", "-st"), ("tcl9.0", "--")] {
        let src = format!("lsearch -regexp {option} {{a+}} {{a b}} x\n");
        let analysis = analyse_for_dialect(&src, dialect);
        let plus = u32::try_from(src.find('+').expect("pattern plus") + 1)
            .expect("source position fits LSP coordinate");
        assert!(
            hover(&src, 0, plus, &analysis, None).is_none(),
            "{dialect} {option} is invalid inside lsearch's option scan and must not claim a regex hover"
        );
    }
}

#[test]
fn residual_hover_lsearch_final_option_shaped_operands_stay_positional() {
    // With exactly two arguments, both are mandatory operands. `-regexp` is
    // the list, not a switch, so the final `{-x*}` keeps lsearch's default
    // glob language even though the list operand looks option-shaped.
    let src = "lsearch -regexp {-x*}\n";
    let analysis = analyse(src);
    let h = hover(src, 0, 20, &analysis, None).expect("positional glob hover");
    assert!(h.value.contains("Glob pattern"), "{}", h.value);
    assert!(h.value.contains("any sequence"), "{}", h.value);
}

// ==================================================================
// regex pattern — anchor / class meanings are Tcl facts.
// ==================================================================

#[test]
fn residual_hover_regexp_pattern_anchors_and_class() {
    // tclsh-proof: `regexp {^[a-z]+$} abc` -> 1 — `^`/`$` anchor the whole
    // string and `[a-z]+` matches one-or-more lowercase letters. The hover
    // names the anchors, the class, and the `+` quantifier.
    let src = "regexp {^[a-z]+$} $line\n";
    let analysis = analyse(src);
    // Cursor inside the `{^[a-z]+$}` pattern literal.
    let h = hover(src, 0, 12, &analysis, None).expect("regexp hover");
    assert!(h.value.contains("Regex pattern"), "{}", h.value);
    assert!(
        h.value.contains("anchor"),
        "expected anchor desc: {}",
        h.value
    );
    assert!(
        h.value.contains("Character class"),
        "expected char-class desc: {}",
        h.value,
    );
    assert!(
        h.value.contains("One or more"),
        "expected `+` quantifier desc: {}",
        h.value,
    );
}

#[test]
fn residual_hover_regexp_literal_pattern_notes_no_metacharacters() {
    // An explicit `regexp` pattern argument hovers even with no
    // metacharacters; the renderer emits the "Literal string" note. (Pure
    // LSP presentation — the detector fires because the command is `regexp`,
    // not because of any Tcl matching fact.)
    let src = "regexp {plainword} $line\n";
    let analysis = analyse(src);
    let h = hover(src, 0, 12, &analysis, None).expect("regexp literal hover");
    assert!(h.value.contains("Regex pattern"), "{}", h.value);
    assert!(
        h.value.contains("Literal string"),
        "expected literal-string note: {}",
        h.value,
    );
}

// ==================================================================
// IP-address literal hovers — RFC classification is a network fact,
// asserted against Rust's std::net parsing (the provider's source of
// truth). These are not Tcl-language facts.
// ==================================================================

#[test]
fn residual_hover_ipv4_with_cidr_prefix() {
    // `192.168.1.0/24` is RFC-1918 private; the `/24` suffix is rendered as
    // a CIDR network line. (std::net classification, not a Tcl fact.)
    let src = "set net 192.168.1.0/24\n";
    let analysis = analyse(src);
    // Cursor on the IPv4/CIDR word.
    let h = hover(src, 0, 12, &analysis, None).expect("ipv4 cidr hover");
    assert!(h.value.contains("IPv4 address"), "{}", h.value);
    assert!(h.value.contains("Private (RFC 1918)"), "{}", h.value);
    assert!(
        h.value.contains("CIDR network: `192.168.1.0/24`"),
        "expected CIDR line: {}",
        h.value,
    );
}

#[test]
fn residual_hover_ipv4_link_local_and_multicast() {
    // Link-local `169.254.0.1` and multicast `224.0.0.1` classify distinctly.
    let src = "set a 169.254.0.1\nset b 224.0.0.1\n";
    let analysis = analyse(src);
    let ll = hover(src, 0, 9, &analysis, None).expect("link-local hover");
    assert!(ll.value.contains("Link-local"), "{}", ll.value);
    let mc = hover(src, 1, 9, &analysis, None).expect("multicast hover");
    assert!(mc.value.contains("Multicast"), "{}", mc.value);
}

#[test]
fn residual_hover_ipv6_link_local_word() {
    // `fe80::1` is IPv6 link-local (segment 0 high bits == fe80). The word
    // contains `:` so the IP detector engages.
    let src = "set ll fe80::1\n";
    let analysis = analyse(src);
    let h = hover(src, 0, 9, &analysis, None).expect("ipv6 link-local hover");
    assert!(h.value.contains("IPv6 address"), "{}", h.value);
    assert!(h.value.contains("Link-local"), "{}", h.value);
}

#[test]
fn residual_hover_ipv6_mapped_shows_ipv4_form() {
    // `::ffff:203.0.113.5` is an IPv4-mapped IPv6 address; the hover both
    // classifies it and surfaces the embedded IPv4 form.
    let src = "set m ::ffff:203.0.113.5\n";
    let analysis = analyse(src);
    let h = hover(src, 0, 10, &analysis, None).expect("ipv4-mapped hover");
    assert!(h.value.contains("IPv6 address"), "{}", h.value);
    assert!(h.value.contains("IPv4-mapped"), "{}", h.value);
    assert!(
        h.value.contains("203.0.113.5"),
        "expected embedded IPv4 form: {}",
        h.value,
    );
}

// ==================================================================
// interp alias hover — the resolved target is a Tcl fact.
// ==================================================================

#[test]
fn residual_hover_interp_alias_shows_target() {
    // tclsh-proof: `interp alias {} ll {} list; ll 1 2 3` -> `1 2 3`, so the
    // alias `ll` really resolves to `list`. Hovering the alias name surfaces
    // the resolved target.
    let src = "interp alias {} ll {} list\nll 1 2 3\n";
    let analysis = analyse(src);
    // Cursor on the `ll` use on line 1.
    let h = hover(src, 1, 0, &analysis, None).expect("alias hover");
    assert!(h.value.contains("Alias"), "{}", h.value);
    assert!(
        h.value.contains("list"),
        "expected target `list`: {}",
        h.value
    );
}

#[test]
fn residual_hover_interp_alias_with_prefix_args_in_target() {
    // tclsh-proof: `interp alias {} greet {} puts Hello; greet` -> `Hello`,
    // so a curried prefix arg (`Hello`) rides along with the target. The
    // hover renders `puts Hello` (target plus its baked-in extra arg).
    let src = "interp alias {} greet {} puts Hello\ngreet\n";
    let analysis = analyse(src);
    let h = hover(src, 1, 2, &analysis, None).expect("alias-with-extras hover");
    assert!(h.value.contains("Alias"), "{}", h.value);
    assert!(
        h.value.contains("puts Hello"),
        "expected target + curried arg: {}",
        h.value,
    );
}

// ==================================================================
// option hover + `cmd subcommand` fall-through.
// ==================================================================

#[test]
fn residual_hover_option_word_of_command() {
    // tclsh-proof: `lsearch -exact {a b c} b` -> 1 — `-exact` is a real
    // `lsearch` switch. Hovering the option word surfaces its detail as an
    // option of `lsearch`.
    let src = "lsearch -exact $list $needle\n";
    let analysis = analyse(src);
    let reg = registry();
    // Cursor on the `-exact` option word.
    let h = hover(src, 0, 11, &analysis, Some(&reg)).expect("option hover");
    assert!(
        h.value.contains("option of `lsearch`"),
        "expected option framing: {}",
        h.value,
    );
    assert!(h.value.contains("-exact"), "{}", h.value);
}

#[test]
fn residual_hover_cursor_on_command_word_is_not_subcommand() {
    // With the cursor on the command word itself (`string`), the subcommand
    // path declines (cmd == cursor word) and the builtin-command hover wins.
    // tclsh-proof: `string length hello` -> 5, so `length` is a real
    // subcommand listed under the bare-`string` hover.
    let src = "string length $name\n";
    let analysis = analyse(src);
    let reg = registry();
    // Cursor on the `string` command word (col 2), NOT on `length`.
    let h = hover(src, 0, 2, &analysis, Some(&reg)).expect("command hover");
    assert!(h.value.contains("built-in command"), "{}", h.value);
    assert!(
        h.value.contains("length"),
        "expected `length` in subcommand list: {}",
        h.value,
    );
}

// ==================================================================
// TclOO class hover — superclass / mixin / class-method / variable detail.
// ==================================================================

#[test]
fn residual_hover_class_lists_superclass_and_instance_variables() {
    // tclsh-proof: `oo::class create Animal {}; oo::class create Dog {
    //   superclass Animal }; info class superclasses Dog` -> `::Animal`, so
    // the superclass relationship is real. The class hover renders the
    // metaclass `create` line with the superclass and lists declared
    // instance variables.
    let src = concat!(
        "oo::class create Animal {}\n",
        "oo::class create Dog {\n",
        "    superclass Animal\n",
        "    variable name\n",
        "    method speak {} {}\n",
        "}\n",
    );
    let analysis = analyse(src);
    // Cursor on the `Dog` class name (line 1, col 18).
    let h = hover(src, 1, 18, &analysis, None).expect("class hover");
    assert!(h.value.contains("oo::class create"), "{}", h.value);
    assert!(h.value.contains("Dog"), "{}", h.value);
    assert!(
        h.value.contains("superclass"),
        "expected superclass annotation: {}",
        h.value,
    );
    assert!(h.value.contains("Animal"), "{}", h.value);
    assert!(
        h.value.contains("Instance variables"),
        "expected instance-variable list: {}",
        h.value,
    );
    assert!(
        h.value.contains("name"),
        "expected variable `name`: {}",
        h.value
    );
}

#[test]
fn residual_hover_class_lists_class_methods() {
    // The `classmethod NAME` define-subcommand records a class-level method;
    // the hover surfaces a **Class methods** section listing it (distinct
    // from the per-instance **Methods** section). Class-membership is an
    // analyser-structural fact, not a runtime value, so it is asserted
    // structurally.
    // tclsh-proof: `oo::class create C { classmethod build {} {} }` is
    // accepted by tclsh **9.0** (`-> ok`); on 8.6 `classmethod` is not a
    // core command (`-> ERR invalid command name`), so this fixture pins the
    // analyser's recognition of the 9.0 form, not cross-version runtime
    // behaviour.
    let src = concat!(
        "oo::class create Factory {\n",
        "    classmethod make {} {}\n",
        "    method use {} {}\n",
        "}\n",
    );
    let analysis = analyse(src);
    // Cursor on the `Factory` class name (line 0, col 18).
    let h = hover(src, 0, 18, &analysis, None).expect("class hover");
    assert!(
        h.value.contains("Class methods"),
        "expected class-methods section: {}",
        h.value,
    );
    assert!(
        h.value.contains("make"),
        "expected class method `make`: {}",
        h.value
    );
}

// ==================================================================
// class-member hover (inside a class body) + $obj method dispatch.
// ==================================================================

#[test]
fn residual_hover_class_member_property_inside_body() {
    // A `property colour` inside an `oo::configurable` body is recorded as a
    // class property; hovering its name elsewhere in the body renders a
    // **property** summary. (Structural — the analyser records the member;
    // no Tcl runtime fact is asserted beyond the declaration.)
    let src = concat!(
        "oo::configurable create Widget {\n",
        "    property colour\n",
        "    method paint {} { my colour }\n",
        "}\n",
    );
    let analysis = analyse(src);
    // Cursor on the `colour` token in the `my colour` call (line 2).
    let h = hover(src, 2, 25, &analysis, None);
    if let Some(h) = h {
        assert!(
            h.value.contains("property") || h.value.contains("colour"),
            "expected property/member summary: {}",
            h.value,
        );
    }
}

#[test]
fn residual_hover_class_with_mixin_and_doc_comment() {
    // tclsh-proof: `oo::class create M {}; oo::class create C { mixin M };
    // info class mixins C` -> `::M`, so the mixin relationship is real. The
    // class hover annotates the signature with the mixin and appends the
    // harvested class doc-comment.
    let src = concat!(
        "oo::class create M {}\n",
        "# A widget mixed with M.\n",
        "oo::class create C {\n",
        "    mixin M\n",
        "    method draw {} {}\n",
        "}\n",
    );
    let analysis = analyse(src);
    // Cursor on the `C` class name (line 2, col 18).
    let h = hover(src, 2, 18, &analysis, None).expect("class+mixin hover");
    assert!(
        h.value.contains("mixin"),
        "expected mixin annotation: {}",
        h.value
    );
    assert!(h.value.contains('M'), "{}", h.value);
    assert!(
        h.value.contains("A widget mixed with M"),
        "expected class doc-comment: {}",
        h.value,
    );
}

#[test]
fn residual_hover_ipv4_unspecified_broadcast_documentation() {
    // Three RFC-special IPv4 literals classify distinctly: `0.0.0.0`
    // (unspecified), `255.255.255.255` (broadcast), `192.0.2.1`
    // (documentation, RFC 5737). (std::net classification, not a Tcl fact.)
    let src = "set a 0.0.0.0\nset b 255.255.255.255\nset c 192.0.2.1\n";
    let analysis = analyse(src);
    let a = hover(src, 0, 9, &analysis, None).expect("unspecified hover");
    assert!(a.value.contains("Unspecified"), "{}", a.value);
    let b = hover(src, 1, 12, &analysis, None).expect("broadcast hover");
    assert!(b.value.contains("Broadcast"), "{}", b.value);
    let c = hover(src, 2, 9, &analysis, None).expect("documentation hover");
    assert!(c.value.contains("Documentation"), "{}", c.value);
}

#[test]
fn residual_hover_class_member_constructor_arity() {
    // The `constructor` keyword inside a class body that defines one renders
    // a **constructor** summary with the parameter count. tclsh-proof:
    // `oo::class create P { constructor {a b} {} }; P new 1 2` succeeds, so
    // the constructor really takes two parameters.
    let src = concat!(
        "oo::class create P {\n",
        "    constructor {a b} {}\n",
        "    method touch {} { constructor }\n",
        "}\n",
    );
    let analysis = analyse(src);
    // Cursor on the `constructor` word in the method body (line 2).
    let h = hover(src, 2, 25, &analysis, None).expect("constructor member hover");
    assert!(h.value.contains("constructor"), "{}", h.value);
    assert!(h.value.contains("::P"), "expected class name: {}", h.value);
    assert!(
        h.value.contains("2 param"),
        "expected 2-param constructor arity: {}",
        h.value,
    );
}

#[test]
fn residual_hover_class_member_destructor_keyword() {
    // The `destructor` keyword inside a class body that defines one renders
    // a **destructor** summary naming the class.
    let src = concat!(
        "oo::class create C {\n",
        "    destructor {}\n",
        "    method touch {} { destructor }\n",
        "}\n",
    );
    let analysis = analyse(src);
    // Cursor on the `destructor` word in the method body (line 2).
    let h = hover(src, 2, 25, &analysis, None).expect("destructor member hover");
    assert!(h.value.contains("destructor"), "{}", h.value);
    assert!(h.value.contains("::C"), "expected class name: {}", h.value);
}

#[test]
fn residual_hover_obj_method_dispatch_to_instance_method() {
    // `$obj method` where the instance's class is known dispatches to the
    // method summary (the `obj_method_hover_text` path, searched before a
    // same-named proc). tclsh-proof: `oo::class create Dog { method bark {}
    // {return woof} }; set d [Dog new]; $d bark` -> `woof`, confirming the
    // instance-method call is real.
    let src = concat!(
        "oo::class create Dog {\n",
        "    method bark {} {}\n",
        "    method run {} {}\n",
        "}\n",
        "set d [Dog new]\n",
        "$d bark\n",
    );
    let analysis = analyse(src);
    // Line 5 `$d bark` — cursor on the `bark` method word.
    let h = hover(src, 5, 4, &analysis, None).expect("obj method hover");
    assert!(h.value.contains("**method**"), "{}", h.value);
    assert!(
        h.value.contains("Dog::bark"),
        "expected receiver-qualified method: {}",
        h.value,
    );
}

// ==================================================================
// proc doc-comment rendering through the public hover entry.
// ==================================================================

#[test]
fn residual_hover_proc_renders_doc_comment_tags() {
    // A Doxygen-style doc-comment block above a proc is harvested and
    // rendered as structured markdown (brief / Parameters / Returns) by the
    // hover. (Presentation — the proc params themselves are the Tcl fact;
    // tclsh: `proc greet {name} {}; info args greet` -> `name`.)
    let src = concat!(
        "# @brief Greets a person.\n",
        "# @param name the person to greet\n",
        "# @return the greeting string\n",
        "proc greet {name} { return \"hi $name\" }\n",
        "greet bob\n",
    );
    let analysis = analyse(src);
    // Cursor on the `greet` proc name (line 3, col 6).
    let h = hover(src, 3, 6, &analysis, None).expect("proc-with-doc hover");
    assert!(h.value.contains("proc ::greet"), "{}", h.value);
    assert!(
        h.value.contains("Greets a person"),
        "expected brief: {}",
        h.value
    );
    assert!(
        h.value.contains("**Parameters:**"),
        "expected parameters block: {}",
        h.value,
    );
    assert!(
        h.value.contains("the person to greet"),
        "expected param desc: {}",
        h.value,
    );
    assert!(
        h.value.contains("**Returns:**"),
        "expected returns block: {}",
        h.value,
    );
}

// ==================================================================
// inferred-intrep / taint on $var hovers (registry path).
// ==================================================================

#[test]
fn residual_hover_var_intrep_string_with_registry() {
    // A variable bound to a string literal infers a `string` intrep through
    // the compiler pipeline; the registry-bearing hover surfaces it.
    // tclsh-proof: `set s hello; string length $s` -> 5 — `$s` holds the
    // 5-char string `hello`. (The *label* `string` is the compiler's
    // intrep classification, surfaced as presentation; the assertion is
    // structural — that an inferred-intrep line appears.)
    let src = "set s hello\nputs $s\n";
    let analysis = analyse(src);
    let reg = registry();
    // Cursor on the `$s` reference (line 1, col 6).
    let h = hover(src, 1, 6, &analysis, Some(&reg)).expect("var hover");
    assert!(h.value.contains("**Variable** `s`"), "{}", h.value);
    assert!(
        h.value.contains("**Inferred intrep**"),
        "expected an inferred-intrep line: {}",
        h.value,
    );
}

#[test]
fn residual_hover_var_without_registry_omits_intrep() {
    // Without a registry the compiler pipeline isn't built, so the var hover
    // carries only the reference count — no intrep / taint lines. (Pure
    // LSP-structural: the registry gate, not a Tcl fact.)
    let src = "set s hello\nputs $s\n";
    let analysis = analyse(src);
    let h = hover(src, 1, 6, &analysis, None).expect("var hover no-reg");
    assert!(h.value.contains("**Variable** `s`"), "{}", h.value);
    assert!(
        !h.value.contains("**Inferred intrep**"),
        "intrep must be gated on a registry: {}",
        h.value,
    );
    assert!(!h.value.contains("**Taint**"), "{}", h.value);
}

// ==================================================================
// public span helpers — find_word_span / find_var on tricky inputs.
// ==================================================================

#[test]
fn residual_find_var_at_position_braced_array_index() {
    // `${arr(idx)}` resolves to the base array variable `arr` for a cursor
    // anywhere inside the braces — including on the index `idx`. (Matches
    // the analyser's array-name normalisation; LSP-structural.)
    let src = "set x ${arr(idx)}\n";
    // Cursor on `i` of the index (inside the parens).
    assert_eq!(
        find_var_at_position(src, 0, 12),
        Some("arr".to_owned()),
        "braced array index should resolve to base `arr`",
    );
}

#[test]
fn residual_find_var_at_position_none_in_plain_word() {
    // A cursor on a bare (non-`$`) word is not a variable reference.
    let src = "puts hello\n";
    assert!(find_var_at_position(src, 0, 6).is_none());
    // Out-of-range line: no panic, no match.
    assert!(find_var_at_position(src, 9, 0).is_none());
}

#[test]
fn residual_find_word_span_namespace_qualified_token() {
    // A `::`-qualified bareword is one word (the `:` is not a delimiter), so
    // the span helper returns the whole `::foo::bar` run with correct
    // columns. (LSP-structural.)
    let src = "set x ::foo::bar\n";
    let (word, start, end) = find_word_span_at_position(src, 0, 10).expect("word span");
    assert_eq!(word, "::foo::bar");
    assert_eq!(start, 6);
    assert_eq!(end, 16);
}

// ==================================================================
// None / edge paths through the public hover entry.
// ==================================================================

#[test]
fn residual_hover_in_comment_line_is_none() {
    // A cursor inside a `#`-comment word resolves no proc / class / var /
    // builtin, so hover returns None (no registry, no match). LSP-structural.
    let src = "# this is a comment about widgets\nputs hi\n";
    let analysis = analyse(src);
    // Cursor on `widgets` inside the comment (no registry to match a word).
    assert!(hover(src, 0, 28, &analysis, None).is_none());
}

#[test]
fn residual_hover_on_operator_char_is_none() {
    // tclsh-proof: `expr {1 + 2}` -> 3 — `+` is an operator, not an
    // identifier. A cursor on the bare `+` resolves no word/var, so hover is
    // None.
    let src = "set y [expr {1 + 2}]\n";
    let analysis = analyse(src);
    // Cursor on the `+` operator (col 15).
    assert!(hover(src, 0, 15, &analysis, None).is_none());
}

#[test]
fn residual_hover_unknown_command_with_registry_is_none() {
    // tclsh-proof: `info commands totallymadeupcmd` -> "" (no such command),
    // and no proc/class/var/IP matches, so even WITH a registry the hover
    // returns None.
    let src = "totallymadeupcmd arg\n";
    let analysis = analyse(src);
    let reg = registry();
    assert!(hover(src, 0, 4, &analysis, Some(&reg)).is_none());
}

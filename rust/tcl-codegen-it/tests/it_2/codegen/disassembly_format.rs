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

//! The disassembly text: sections, offsets, labels and literal escaping.

use super::*;

// Formatting

#[test]
fn format_header() {
    crate::install_surfaces();
    let text = format_function_asm(&top_asm("set x 1"));
    assert!(text.starts_with("ByteCode ::top"));
    assert!(text.contains("instructions"));
    assert!(text.contains("literals"));
    assert!(text.contains("variables"));
}

#[test]
fn format_literals_section() {
    crate::install_surfaces();
    let text = format_function_asm(&top_asm("set x hello"));
    assert!(text.contains("Literals:"));
    assert!(text.contains("\"hello\""));
}

#[test]
fn format_variables_section() {
    crate::install_surfaces();
    // Top-level scripts use stack-based var access → no LVT.
    let text = format_function_asm(&top_asm("set x 1"));
    assert!(text.contains("0 variables"));
    // Proc bodies still use the LVT.
    let ma = asm_for("proc foo {x} { set x 1 }");
    let proc_text = format_function_asm(
        ma.procedures
            .get(&fixture_id(&ma.procedures, "::foo"))
            .expect("authored fixture body"),
    );
    assert!(proc_text.contains("Local variables:"));
    assert!(proc_text.contains("%v0: \"x\""));
}

#[test]
fn format_instruction_offsets() {
    crate::install_surfaces();
    let text = format_function_asm(&top_asm("set x 1"));
    assert!(text.contains("(0) push1"));
}

#[test]
fn format_block_labels() {
    crate::install_surfaces();
    let text = format_function_asm(&top_asm("set x 1"));
    assert!(text.contains("# "), "block label comment expected");
}

#[test]
fn format_module() {
    crate::install_surfaces();
    let ma = asm_for("proc foo {x} { return $x }\nfoo 1");
    let text = format_module_asm(&ma);
    assert!(text.contains("ByteCode ::top"));
    assert!(text.contains("ByteCode ::foo"));
}

#[test]
fn format_instruction_offsets_monotonic() {
    crate::install_surfaces();
    let fa = top_asm("set x 1; set y 2; set z [expr {$x + $y}]");
    let offsets: Vec<i32> = fa
        .instructions
        .iter()
        .map(|i| i.offset)
        .filter(|&o| o >= 0)
        .collect();
    for w in offsets.windows(2) {
        assert!(w[1] >= w[0], "offsets must be monotonic: {offsets:?}");
    }
}

#[test]
fn format_proc_has_local_variables() {
    crate::install_surfaces();
    let ma = asm_for("proc foo {a b} { set c [expr {$a + $b}]; return $c }");
    let text = format_function_asm(
        ma.procedures
            .get(&fixture_id(&ma.procedures, "::foo"))
            .expect("authored fixture body"),
    );
    assert!(text.contains("Local variables:"));
    assert!(text.contains("\"a\""));
    assert!(text.contains("\"b\""));
}

#[test]
fn format_done_always_at_end() {
    crate::install_surfaces();
    let ma =
        asm_for("proc add {a b} { expr {$a + $b} }\nproc sub {a b} { expr {$a - $b} }\nset x 1");
    assert_eq!(ma.top_level.instructions.last().unwrap().op, Op::DONE);
    for (name, fa) in &ma.procedures {
        assert_eq!(
            fa.instructions.last().unwrap().op,
            Op::DONE,
            "proc {name:?} missing trailing done"
        );
    }
}

// Disassembly escaping
//
// `esc` is BYTE-WISE over UTF-8 (matches C-Tcl's disassembler), not
// codepoint-wise. The named/NUL/C0/DEL/quote/ASCII cases are unaffected; the
// high-BMP / supplementary cases assert the byte-wise expectation (see file
// header note 3).

#[test]
fn esc_named_controls_use_short_forms() {
    crate::install_surfaces();
    assert_eq!(esc("a\nb\tc\rd\x0be\x0cf", 40), "a\\nb\\tc\\rd\\ve\\ff");
}

#[test]
fn esc_nul_escaped_as_u_xxxx() {
    crate::install_surfaces();
    assert_eq!(esc("a\x00b", 40), "a\\u0000b");
}

#[test]
fn esc_stx_and_other_c0_controls_escaped() {
    crate::install_surfaces();
    // STX (0x02) is the byte that leaked in tcllib 2.0 installer.tcl.
    assert_eq!(esc("\x01\x02\x03\x1f", 40), "\\u0001\\u0002\\u0003\\u001f");
}

#[test]
fn esc_del_and_high_bmp_escaped_bytewise() {
    crate::install_surfaces();
    assert_eq!(esc("\x7f", 40), "\\u007f");
    // Byte-wise: U+00FF is the two UTF-8 bytes C3 BF → Ã¿.
    assert_eq!(esc("\u{ff}", 40), "\\u00c3\\u00bf");
}

#[test]
fn esc_supplementary_plane_is_bytewise_four_bytes() {
    crate::install_surfaces();
    // Byte-wise: U+1F600 is the four UTF-8 bytes F0 9F 98 80.
    assert_eq!(esc("\u{1f600}", 40), "\\u00f0\\u009f\\u0098\\u0080");
}

#[test]
fn esc_printable_ascii_unchanged() {
    crate::install_surfaces();
    assert_eq!(esc("hello world!", 40), "hello world!");
}

#[test]
fn esc_quote_escaped() {
    crate::install_surfaces();
    assert_eq!(esc("a\"b", 40), "a\\\"b");
}

#[test]
fn literal_with_embedded_stx_appears_escaped_in_disassembly() {
    crate::install_surfaces();
    // End-to-end: a literal interned with STX renders escaped in the
    // Literals: section — no raw control byte reaches the formatter output.
    let mut cfg = CfgFunction::new("::top", "entry_0");
    let entry = cfg.entry;
    cfg.blocks
        .get_mut(&entry)
        .unwrap()
        .statements
        .push(Statement::AssignConst {
            span: sp(),
            name: "x".into(),
            name_braced: false,
            value: "\x00\x02-".into(),
            value_span: None,
        });
    cfg.blocks.get_mut(&entry).unwrap().terminator = Some(Terminator::Return {
        expr_base: None,
        tokens: None,
        value: None,
        value_word: None,
        span: None,
        expr: None,
        braced: false,
    });
    let fa = codegen_function(&cfg, &[], false, &registry());
    let text = format_function_asm(&fa);
    assert!(!text.contains('\x02'));
    assert!(text.contains("\\u0000\\u0002-"));
}

// Codegen output format validation (labels)

#[test]
fn labels_point_to_valid_offsets() {
    crate::install_surfaces();
    let fa = top_asm("if {$x} {set a 1} else {set a 2}");
    // Labels resolve to non-negative byte offsets (layout-resolved).
    for (label, &off) in &fa.labels {
        // `usize` offset is inherently >= 0; assert it is within the byte span
        // and the label is non-empty so the table is meaningful.
        assert_ne!(label.as_str(), "");
        let total_bytes: usize = fa
            .instructions
            .iter()
            .map(|i| usize::from(i.op.size()))
            .sum();
        assert!(
            off <= total_bytes + 16,
            "label {label} offset {off} absurdly large"
        );
    }
    assert!(fa.labels.len() >= 2, "if/else needs at least 2 labels");
}

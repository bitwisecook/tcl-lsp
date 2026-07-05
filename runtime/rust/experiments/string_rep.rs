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

//! EXPERIMENT (throwaway): how to do char-indexed access on a UTF-8 Tcl string,
//! and how to append, without an O(n^2) cliff (the low-O() tenet).
//!
//! `string index`/`length`/`range` are CHARACTER-indexed; UTF-8 makes naive
//! char access O(n). But most Tcl strings are ASCII (byte index == char index).
//! Candidates for a char-indexed traversal (the O(n^2) shape):
//!   S  scan UTF-8 to the i-th char each time
//!   I  build a Vec<usize> of char byte-offsets once, then O(1)
//!   A  ASCII fast path: if all-ASCII, byte index == char index (O(1)); else I
//! And append (building a string by repeated concat):
//!   R  reallocate an exact buffer each append (today's set_owned_string)
//!   V  push onto a Vec with amortized capacity
//!
//! Built to wasm32-wasip1, run under wasmtime (the real target).
//!   rustc -O --edition 2021 --target wasm32-wasip1 string_rep.rs -o /tmp/s.wasm && wasmtime /tmp/s.wasm

use std::time::Instant;

fn ascii_string(nchars: usize) -> Vec<u8> {
    (0..nchars).map(|i| b'a' + (i % 26) as u8).collect()
}
fn utf8_string(nchars: usize) -> Vec<u8> {
    // 2-byte chars (U+00E9 'é' = 0xC3 0xA9)
    let mut v = Vec::with_capacity(nchars * 2);
    for _ in 0..nchars {
        v.push(0xC3);
        v.push(0xA9);
    }
    v
}

// char byte-offset of the i-th char by scanning UTF-8 from the start.
fn scan_to(s: &[u8], i: usize) -> usize {
    let mut idx = 0;
    let mut ci = 0;
    while idx < s.len() && ci < i {
        idx += utf8_len(s[idx]);
        ci += 1;
    }
    idx
}
#[inline]
fn utf8_len(b: u8) -> usize {
    if b < 0x80 {
        1
    } else if b < 0xE0 {
        2
    } else if b < 0xF0 {
        3
    } else {
        4
    }
}
fn offsets(s: &[u8]) -> Vec<usize> {
    let mut v = Vec::new();
    let mut idx = 0;
    while idx < s.len() {
        v.push(idx);
        idx += utf8_len(s[idx]);
    }
    v
}

fn bench_index(label: &str, s: &[u8]) {
    let nchars = offsets(s).len();
    // Scan is O(n^2)/pass; scale passes by n^2 so total scan work is bounded.
    let passes = (200_000_000 / (nchars * nchars).max(1)).max(20);

    // S: scan each char (the O(n^2) traversal)
    let t = Instant::now();
    let mut acc = 0usize;
    for _ in 0..passes {
        for i in 0..nchars {
            acc = acc.wrapping_add(scan_to(s, i));
        }
    }
    let scan = t.elapsed().as_nanos() as f64 / passes as f64 / 1000.0;

    // I: build offsets once, then O(1)
    let t = Instant::now();
    let mut acc2 = 0usize;
    for _ in 0..passes {
        let off = offsets(s);
        for i in 0..nchars {
            acc2 = acc2.wrapping_add(off[i]);
        }
    }
    let index = t.elapsed().as_nanos() as f64 / passes as f64 / 1000.0;

    // A: ASCII fast path
    let is_ascii = s.iter().all(|&b| b < 0x80);
    let t = Instant::now();
    let mut acc3 = 0usize;
    for _ in 0..passes {
        if is_ascii {
            for i in 0..nchars {
                acc3 = acc3.wrapping_add(i); // byte index == char index
            }
        } else {
            let off = offsets(s);
            for i in 0..nchars {
                acc3 = acc3.wrapping_add(off[i]);
            }
        }
    }
    let ascii = t.elapsed().as_nanos() as f64 / passes as f64 / 1000.0;

    println!(
        "{label:>16} nchars={nchars:>6}  S(scan)={scan:>9.1}us  I(index)={index:>8.1}us  A(ascii-fast)={ascii:>8.1}us/pass  (acc={acc} {acc2} {acc3})"
    );
}

fn bench_append(n: usize) {
    let piece = b"chunk-12";
    // R: realloc exact each append
    let t = Instant::now();
    let mut buf: Vec<u8> = Vec::new();
    for _ in 0..n {
        let mut nb = Vec::with_capacity(buf.len() + piece.len() + 1);
        nb.extend_from_slice(&buf);
        nb.extend_from_slice(piece);
        nb.push(0);
        nb.pop();
        buf = nb;
    }
    let realloc = t.elapsed().as_micros();

    // V: amortized Vec
    let t = Instant::now();
    let mut v: Vec<u8> = Vec::new();
    for _ in 0..n {
        v.extend_from_slice(piece);
    }
    let veccap = t.elapsed().as_micros();

    println!("        append n={n:>6}  R(realloc-each)={realloc:>10}us  V(vec-capacity)={veccap:>8}us  (len {} {})", buf.len(), v.len());
}

fn main() {
    // Char sizes kept modest: strategy S is O(n^2) per pass, so a few thousand
    // chars already shows the cliff decisively without a multi-second run.
    println!("== char-indexed traversal ==");
    for &n in &[64usize, 1000, 4000] {
        bench_index("ascii", &ascii_string(n));
        bench_index("utf8(2-byte)", &utf8_string(n));
    }
    println!("\n== append (string build loop) ==");
    for &n in &[1000usize, 20_000, 200_000] {
        bench_append(n);
    }
}

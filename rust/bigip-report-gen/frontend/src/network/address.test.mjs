// tcl-lsp — a language server and toolchain for Tcl
// SPDX-License-Identifier: AGPL-3.0-or-later

import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const { instance } = await WebAssembly.instantiate(readFileSync(new URL("address.wasm", import.meta.url)));
const api = instance.exports;
const input = new Uint8Array(api.memory.buffer, api.address_input(), 256);
const encoder = new TextEncoder();

function invoke(text, action) {
  const bytes = encoder.encode(text);
  input.set(bytes);
  return action(bytes.length);
}

function describe(text) {
  return invoke(text, api.describe_address) >>> 0;
}

function matches(host, network, overlap = false) {
  return Boolean(invoke(host + "\0" + network, (length) =>
    api.match_addresses(length, Number(overlap))));
}

test("preserves explicit domains and validates the full address", () => {
  assert.equal(describe("2001:db8::53%0/64"), (1 << 16) | (64 << 8) | 6);
  assert.equal(describe("::ffff:192.0.2.53%65534"), ((65535 << 16) | (128 << 8) | 6) >>> 0);
  assert.equal(describe("192.0.2.53%65535"), 0);
  assert.equal(describe("12:30:45"), 0);
});

test("applies family and network checks even for a zero-width prefix", () => {
  assert(matches("::ffff:192.0.2.53", "::ffff:192.0.2.0/120"));
  assert(!matches("bad:address", "::/0"));
  assert(matches("2001:db8::53", "::/0"));
  assert(!matches("192.0.2.53", "::/0"));
});

test("search overlaps networks while listener lookup uses a source host", () => {
  assert(!matches("192.0.3.53/16", "192.0.2.0/24"));
  assert(matches("192.0.3.53/16", "192.0.2.0/24", true));
});

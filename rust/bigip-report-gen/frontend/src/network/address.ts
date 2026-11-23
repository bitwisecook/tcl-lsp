// tcl-lsp — a language server and toolchain for Tcl
// SPDX-License-Identifier: AGPL-3.0-or-later

import wasm from "./address.wasm";

interface AddressExports {
  memory: WebAssembly.Memory;
  address_input(): number;
  describe_address(length: number): number;
  match_addresses(length: number, overlap: number): number;
}

interface Network {
  text: string;
  family: number;
  prefix: number;
  routeDomain: number | null;
}

let engine: AddressExports | undefined;
const capacity = 256;
const encoder = new TextEncoder();

function withInput<T>(
  text: string,
  operation: (engine: AddressExports, length: number) => T,
): T | undefined {
  const bytes = encoder.encode(text);
  if (bytes.length > capacity) return undefined;
  engine ??= new WebAssembly.Instance(new WebAssembly.Module(wasm))
    .exports as unknown as AddressExports;
  const input = new Uint8Array(engine.memory.buffer, engine.address_input(), capacity);
  input.set(bytes);
  return operation(engine, bytes.length);
}

export function readNetwork(text: string): Network | null {
  const description = withInput(text, (engine, length) => engine.describe_address(length));
  if (!description) return null;
  const domain = description >>> 16;
  return {
    text,
    family: description & 255,
    prefix: (description >>> 8) & 255,
    routeDomain: domain ? domain - 1 : null,
  };
}

export function networkMatches(host: Network, network: string, overlap = false): boolean {
  return Boolean(withInput(host.text + "\0" + network, (engine, length) =>
    engine.match_addresses(length, Number(overlap))));
}

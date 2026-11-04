// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

// Capture only synthetic-fixture definition and hover replies from the existing
// Node LSP drivers. Request bodies and other protocol messages are not saved.
'use strict';
const fs = require('node:fs');
const parse = JSON.parse;
const stringify = JSON.stringify;
const wanted = new Map();
const methods = new Set(['textDocument/definition', 'textDocument/hover']);
JSON.stringify = function (value, ...args) {
    if (value && value.jsonrpc === '2.0' && value.id !== undefined && methods.has(value.method)) {
        wanted.set(value.id, value.method);
    }
    return Reflect.apply(stringify, this, [value, ...args]);
};
JSON.parse = function (text, ...args) {
    const value = Reflect.apply(parse, this, [text, ...args]);
    if (value && value.jsonrpc === '2.0' && value.method === undefined && wanted.has(value.id)
        && (Object.hasOwn(value, 'result') || Object.hasOwn(value, 'error'))) {
        const target = process.env.LSP_RPC_RESPONSE_LOG;
        if (target) {
            const record = stringify({method: wanted.get(value.id), response: value});
            fs.appendFileSync(target, `${record}\n`, {mode: 0o600});
        }
        wanted.delete(value.id);
    }
    return value;
};

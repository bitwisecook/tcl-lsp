// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

// Capture only synthetic-fixture definition and hover replies from the existing
// Node LSP drivers. Request bodies and other protocol messages are not saved.
'use strict';
const fs = require('node:fs');
const parse = JSON.parse;
const stringify = JSON.stringify;

// Capture only Wasmtime child diagnostics for the synthetic WASI failure rerun.
// Keep a global byte budget, report truncation, and never alter protocol stdout.
const childProcess = require('node:child_process');
const {errorMonitor} = require('node:events');
const path = require('node:path');
const spawn = childProcess.spawn;
const stderrTarget = process.env.LSP_WASI_CHILD_STDERR_LOG;
if (stderrTarget) {
    let nextSession = 0;
    let remaining = 256 * 1024;
    let truncated = false;
    let stderrRows = 0;
    let sessionOverflow = false;
    const record = (value) => {
        try { fs.appendFileSync(stderrTarget, `${stringify(value)}\n`, {mode: 0o600}); }
        catch { /* Diagnostic recording must not change the workload. */ }
    };
    childProcess.spawn = function (command, ...args) {
        const child = Reflect.apply(spawn, this, [command, ...args]);
        if (typeof command !== 'string' || path.basename(command) !== 'wasmtime') return child;
        if (nextSession >= 32) {
            if (!sessionOverflow) {
                sessionOverflow = true;
                record({event: 'session_limit_reached', limit_sessions: 32});
            }
            return child;
        }
        const session = ++nextSession;
        const identity = {session, pid: child.pid ?? null, command: 'wasmtime'};
        record({...identity, event: 'spawn'});
        child.stderr?.on('data', (data) => {
            const bytes = Buffer.isBuffer(data) ? data : Buffer.from(data);
            let take = 0;
            const limit = Math.min(bytes.length, remaining);
            while (take < limit && stderrRows < 128) {
                const end = Math.min(take + 4096, limit);
                record({...identity, event: 'stderr', data_base64: bytes.subarray(take, end).toString('base64')});
                stderrRows += 1;
                take = end;
            }
            remaining -= take;
            if (take < bytes.length && !truncated) {
                truncated = true;
                record({...identity, event: 'stderr_truncated', limit_bytes: 256 * 1024, limit_rows: 128});
            }
        });
        child.once(errorMonitor, (error) => record({...identity, event: 'error', code: error.code ?? null}));
        child.once('exit', (code, signal) => record({...identity, event: 'exit', code, signal}));
        child.once('close', (code, signal) => record({...identity, event: 'close', code, signal}));
        return child;
    };
    // The WASI driver imports spawn as an ESM named binding.
    require('node:module').syncBuiltinESMExports();
}
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

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

/*
 * Drive the WASI language server through scripted LSP sessions under wasmtime.
 *
 * The sibling of `rust/tcl-lsp-server-wasm/test/e2e.mjs`, and deliberately the
 * same shape — but where that harness calls a wasm-bindgen object in-process,
 * this one is a real LSP *client*: it spawns `wasmtime run`, writes
 * Content-Length-framed JSON-RPC to the child's stdin, and reads it back off
 * stdout. A pass here means an editor's stdio client sees the same answers.
 *
 * Four of the scenarios exist to prove the driver in `src/driver.rs` rather
 * than the server behind it, because a single-threaded WASI host is where a
 * transport's liveness properties actually get tested:
 *
 *   (1) round-trip        `workspace/configuration` is issued by the server
 *                         during `initialized` and answered by the client on
 *                         stdin, without the driver deadlocking on its own
 *                         handler.
 *   (2) starvation        a `didOpen` produces `publishDiagnostics` with the
 *                         client sending NOTHING further. The browser host
 *                         gets this from the JS event loop; the WASI driver has
 *                         to produce it from its own wait.
 *   (3) idle timers       the 10 s `workspace/configuration` deadline expires,
 *                         and says so, on a session that is otherwise silent —
 *                         the runtime is neither starved nor aborted while it
 *                         waits.
 *   (4) shutdown/exit     `shutdown` then `exit` ends the process with 0;
 *                         `exit` alone ends it with 1.
 *
 * The fifth is the filesystem: a multi-file session where the sourced sibling
 * exists only inside a `--dir` preopen, so `vfs::NativeStore` has to read it.
 *
 * The sixth is the filesystem the other way round — a session that WRITES.
 * Every other scenario leaves the compiled-pack cache unreachable, which hid a
 * wasip1 abort for the whole life of this file; see `cacheWritableSession`.
 *
 * Run with `make lsp-server-wasi-test`, or
 * `node test/e2e.mjs [path/to/module.wasm]` against an already-built `dist/`.
 */

import { spawn } from "node:child_process";
import { cp, mkdtemp, readFile, rm, stat } from "node:fs/promises";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";
import { dirname, join, resolve } from "node:path";

const here = dirname(fileURLToPath(import.meta.url));
const defaultModule = join(here, "..", "dist", "tcl-lsp-server-wasi.wasm");
const modulePath = resolve(process.argv[2] ?? defaultModule);
const fixture = join(here, "fixture");

/** The guest path the fixture directory is preopened at. */
const ROOT = "/w";
const APP_URI = `file://${ROOT}/app.tcl`;
const LIB_URI = `file://${ROOT}/lib.tcl`;
const BROKEN_URI = `file://${ROOT}/broken.tcl`;

/*
 * Deliberately wrong, in a way the analyser reports: `greet` takes one
 * argument and is called with two. Never written to disk — it only ever exists
 * as a `didOpen`, so the diagnostics it produces can only come from analysis
 * the server ran after the notification returned.
 */
const BROKEN_SOURCE = 'proc greet {name} {\n    puts "hi $name"\n}\n\ngreet world extra\n';

/*
 * wasmtime compiles a 19 MiB module from scratch on a cold cache, which on a
 * CI runner is seconds rather than milliseconds. Every wait here is therefore a
 * poll with a generous ceiling rather than a fixed sleep: the assertions are
 * about what eventually arrives, never about how fast.
 */
const STARTUP_BUDGET_MS = 120_000;
const REPLY_BUDGET_MS = 60_000;

let failures = 0;
const results = [];

function check(label, ok, detail) {
    results.push({ label, ok, detail });
    if (!ok) failures += 1;
    console.log(`${ok ? "ok  " : "FAIL"} ${label}${detail ? ` — ${detail}` : ""}`);
}

const sleep = (ms) => new Promise((resolve_) => setTimeout(resolve_, ms));

/** Poll until `probe` returns something truthy, or give up. */
async function until(probe, budgetMs, every = 50) {
    const deadline = Date.now() + budgetMs;
    for (;;) {
        const value = probe();
        if (value) return value;
        if (Date.now() > deadline) return null;
        await sleep(every);
    }
}

/**
 * An LSP client speaking the base protocol to a wasmtime child.
 *
 * `answerConfiguration: false` withholds the reply to
 * `workspace/configuration` — the whole of scenario (3).
 */
class Session {
    constructor({ answerConfiguration = true, preopen = fixture, env = {} } = {}) {
        this.answerConfiguration = answerConfiguration;
        this.nextId = 1;
        this.pending = new Map();
        this.diagnostics = new Map();
        this.logs = [];
        this.serverRequests = [];
        this.exitCode = null;
        this.exited = new Promise((resolve_) => {
            this.resolveExit = resolve_;
        });
        this.buffer = Buffer.alloc(0);
        this.stderr = "";

        const envArgs = Object.entries(env).flatMap(([key, value]) => [
            "--env",
            `${key}=${value}`,
        ]);
        this.child = spawn(
            "wasmtime",
            ["run", "--dir", `${preopen}::${ROOT}`, ...envArgs, modulePath],
            { stdio: ["pipe", "pipe", "pipe"] },
        );
        this.child.stdout.on("data", (chunk) => this.receive(chunk));
        this.child.stderr.on("data", (chunk) => {
            this.stderr += chunk.toString("utf8");
        });
        this.child.on("exit", (code) => {
            this.exitCode = code;
            this.resolveExit(code);
        });
    }

    receive(chunk) {
        this.buffer = Buffer.concat([this.buffer, chunk]);
        for (;;) {
            const at = this.buffer.indexOf("\r\n\r\n");
            if (at < 0) return;
            const header = this.buffer.slice(0, at).toString("utf8");
            const match = /content-length:\s*(\d+)/i.exec(header);
            if (!match) return;
            const length = Number(match[1]);
            if (this.buffer.length < at + 4 + length) return;
            const body = this.buffer.slice(at + 4, at + 4 + length).toString("utf8");
            this.buffer = this.buffer.slice(at + 4 + length);
            this.dispatch(JSON.parse(body));
        }
    }

    dispatch(message) {
        if (message.method === undefined) {
            const resolve_ = this.pending.get(message.id);
            if (resolve_) {
                this.pending.delete(message.id);
                resolve_(message);
            }
            return;
        }
        if (message.method === "textDocument/publishDiagnostics") {
            this.diagnostics.set(message.params.uri, message.params.diagnostics);
        } else if (message.method === "window/logMessage") {
            this.logs.push(message.params.message);
        }
        if (message.id === undefined) return;
        this.serverRequests.push(message.method);
        if (message.method === "workspace/configuration") {
            if (!this.answerConfiguration) return;
            this.post({
                jsonrpc: "2.0",
                id: message.id,
                result: message.params.items.map(() => ({})),
            });
            return;
        }
        this.post({ jsonrpc: "2.0", id: message.id, result: null });
    }

    post(message) {
        const body = JSON.stringify(message);
        this.child.stdin.write(
            `Content-Length: ${Buffer.byteLength(body, "utf8")}\r\n\r\n${body}`,
        );
    }

    notify(method, params) {
        this.post({ jsonrpc: "2.0", method, params });
    }

    request(method, params, budgetMs = REPLY_BUDGET_MS) {
        const id = this.nextId++;
        const reply = new Promise((resolve_) => this.pending.set(id, resolve_));
        this.post({ jsonrpc: "2.0", id, method, params });
        return Promise.race([reply, sleep(budgetMs).then(() => null)]);
    }

    kill() {
        this.child.kill("SIGKILL");
    }
}

const CLIENT_CAPABILITIES = {
    general: { positionEncodings: ["utf-16"] },
    workspace: { configuration: true, workspaceFolders: true },
    textDocument: {
        synchronization: { dynamicRegistration: false },
        hover: { contentFormat: ["markdown", "plaintext"] },
        definition: { linkSupport: false },
        documentSymbol: { hierarchicalDocumentSymbolSupport: false },
        semanticTokens: {
            requests: { full: true },
            tokenTypes: [],
            tokenModifiers: [],
            formats: ["relative"],
        },
        publishDiagnostics: {},
    },
};

function initializeParams() {
    return {
        processId: null,
        rootUri: `file://${ROOT}`,
        workspaceFolders: [{ uri: `file://${ROOT}`, name: "fixture" }],
        capabilities: CLIENT_CAPABILITIES,
    };
}

/** Every `Location` in a definition result, whatever shape it came back in. */
function locations(result) {
    if (!result) return [];
    if (Array.isArray(result)) {
        return result.map((entry) => ({
            uri: entry.uri ?? entry.targetUri,
            range: entry.range ?? entry.targetSelectionRange ?? entry.targetRange,
        }));
    }
    if (result.uri) return [{ uri: result.uri, range: result.range }];
    return [];
}

function describeHover(hover) {
    const contents = hover?.contents;
    if (contents === undefined || contents === null) return "";
    if (typeof contents === "string") return contents;
    if (typeof contents.value === "string") return contents.value;
    if (Array.isArray(contents)) {
        return contents.map((c) => (typeof c === "string" ? c : (c.value ?? ""))).join(" ");
    }
    return JSON.stringify(contents);
}

/*
 * The main session: initialisation, the configuration round-trip, the
 * starvation test, the multi-file filesystem scenario, and a clean exit.
 */
async function mainSession() {
    const session = new Session();
    const appSource = await readFile(join(fixture, "app.tcl"), "utf8");

    const initialize = await session.request(
        "initialize",
        initializeParams(),
        STARTUP_BUDGET_MS,
    );
    const caps = initialize?.result?.capabilities;
    check("initialize returns capabilities", Boolean(caps));
    check(
        "the semantic-tokens provider is advertised",
        Boolean(caps?.semanticTokensProvider),
        `hoverProvider=${caps?.hoverProvider} definitionProvider=${caps?.definitionProvider}`,
    );
    check(
        "the typeHierarchyProvider shim is applied",
        caps?.typeHierarchyProvider === true,
        "the initialize-response shim from tcl_lsp_server::service",
    );

    // ---- (1) the server-initiated round-trip ------------------------------
    session.notify("initialized", {});
    const configured = await until(
        () => session.serverRequests.includes("workspace/configuration"),
        REPLY_BUDGET_MS,
    );
    check(
        "the server issues workspace/configuration during initialized",
        Boolean(configured),
        `server requests: ${[...new Set(session.serverRequests)].join(", ") || "none"}`,
    );
    // The reply went back over stdin. If the driver had stopped reading while
    // its own handler was in flight, nothing after this point would happen at
    // all — so every later check is also evidence the round-trip completed.
    const initialised = await until(
        () => session.logs.some((line) => line.includes("initialised")),
        REPLY_BUDGET_MS,
    );
    check(
        "the configuration reply is routed back and initialisation completes",
        Boolean(initialised),
        "the reply arrives on stdin while the initialized handler awaits it",
    );

    // ---- (2) the starvation test ------------------------------------------
    // Everything below the didOpen happens with the client silent. Nothing is
    // sent, nothing is polled at the protocol level: the diagnostics have to be
    // produced by detached work that the driver keeps running while it waits on
    // an empty stdin, and delivered through a 50 ms debounce timer that only
    // fires if the runtime is still being driven.
    session.notify("textDocument/didOpen", {
        textDocument: { uri: APP_URI, languageId: "tcl", version: 1, text: appSource },
    });
    const published = await until(() => session.diagnostics.get(APP_URI), REPLY_BUDGET_MS);
    check(
        "publishDiagnostics arrives with the client sending nothing further",
        Array.isArray(published),
        published ? `${published.length} diagnostics, unprompted` : "none",
    );

    // The same property again, but with something to *say*: a document the
    // analyser has a real complaint about. A published empty list proves the
    // notification path survived; a published finding proves the analysis
    // behind it ran to completion, unprompted, on a driver waiting on nothing.
    session.notify("textDocument/didOpen", {
        textDocument: {
            uri: BROKEN_URI,
            languageId: "tcl",
            version: 1,
            text: BROKEN_SOURCE,
        },
    });
    const complaints = await until(() => {
        const diags = session.diagnostics.get(BROKEN_URI);
        return diags && diags.length > 0 ? diags : null;
    }, REPLY_BUDGET_MS);
    // The exact code matters: E003 is "too many arguments", which is the
    // mistake the fixture makes. Asserting only "some diagnostic arrived"
    // would pass on a parse error from a mangled document too, and then the
    // check would no longer be evidence that analysis actually ran.
    const arityComplaint = (complaints ?? []).find((d) => d.code === "E003");
    check(
        "a broken document's diagnostics are computed and delivered unprompted",
        arityComplaint !== undefined,
        complaints
            ? `${complaints.length}: ${complaints.map((d) => d.code ?? "?").join(",")} — ${complaints[0].message}`
            : "none",
    );

    // ---- (5) the filesystem, over a preopen -------------------------------
    // `lib.tcl` is never sent to the server. Reaching it means NativeStore read
    // the preopened directory.
    const helperLine = appSource.split("\n").findIndex((line) => line.includes("[helper "));
    const definition = await session.request("textDocument/definition", {
        textDocument: { uri: APP_URI },
        position: { line: helperLine, character: appSource.split("\n")[helperLine].indexOf("helper") + 2 },
    });
    const found = locations(definition?.result);
    // Assert the position, not just the file. Landing anywhere in lib.tcl
    // would satisfy a URI-only check even if the definition were resolved to
    // the wrong proc — and `unused_helper` sits a few lines below `helper`,
    // so that is a real way to be wrong. The expected line is derived from the
    // fixture rather than hard-coded, so editing lib.tcl cannot silently
    // invalidate the check.
    const libSource = await readFile(join(fixture, "lib.tcl"), "utf8");
    const expectedHelperLine = libSource
        .split("\n")
        .findIndex((line) => line.startsWith("proc helper "));
    const atHelper = found.find(
        (loc) => loc.uri === LIB_URI && loc.range?.start?.line === expectedHelperLine,
    );
    check(
        "go-to-definition follows `source` into a file that exists only on the preopened disk",
        atHelper !== undefined,
        found.length > 0
            ? `${found.map((l) => `${l.uri}:${l.range?.start?.line}`).join(" ")} (expected line ${expectedHelperLine})`
            : "no locations",
    );

    const hover = await session.request("textDocument/hover", {
        textDocument: { uri: APP_URI },
        position: { line: helperLine, character: appSource.split("\n")[helperLine].indexOf("helper") + 2 },
    });
    const hoverText = describeHover(hover?.result);
    check(
        "hover on the cross-file call answers from the sibling on disk",
        hoverText.includes("helper"),
        JSON.stringify(hoverText.slice(0, 80)),
    );

    const symbols = await session.request("textDocument/documentSymbol", {
        textDocument: { uri: APP_URI },
    });
    check(
        "documentSymbol answers for the open document",
        Array.isArray(symbols?.result),
        `${symbols?.result?.length ?? 0} symbols`,
    );

    const tokens = await session.request("textDocument/semanticTokens/full", {
        textDocument: { uri: APP_URI },
    });
    const data = tokens?.result?.data ?? [];
    check(
        "semanticTokens/full returns tokens",
        data.length > 0 && data.length % 5 === 0,
        `${data.length / 5} tokens`,
    );

    // ---- (4) clean shutdown and exit --------------------------------------
    const shutdown = await session.request("shutdown", null);
    check("shutdown answers", Boolean(shutdown) && shutdown.error === undefined);
    session.notify("exit", null);
    const code = await Promise.race([session.exited, sleep(30_000).then(() => "timeout")]);
    check(
        "exit after shutdown ends the process with status 0",
        code === 0,
        `exit status ${code}`,
    );
    if (code === "timeout") session.kill();
    return session;
}

/*
 * (3) The idle-timer session.
 *
 * The client answers nothing. `initialized` pulls configuration, waits out the
 * server's own 10 s deadline, and logs that it did — a timer that can only fire
 * if the driver keeps the runtime turning while it sits on a silent stdin. It
 * is simultaneously the proof that a wasip1 runtime parked for ten seconds with
 * nothing to do does not abort with "condvar wait not supported".
 */
async function idleTimerSession() {
    const session = new Session({ answerConfiguration: false });
    const initialize = await session.request(
        "initialize",
        initializeParams(),
        STARTUP_BUDGET_MS,
    );
    check("initialize answers on the idle-timer session", Boolean(initialize?.result));

    session.notify("initialized", {});
    const askedAt = Date.now();
    const asked = await until(
        () => session.serverRequests.includes("workspace/configuration"),
        REPLY_BUDGET_MS,
    );
    check("the idle session is asked for configuration", Boolean(asked));

    // From here the client is completely silent for over ten seconds.
    const warned = await until(
        () =>
            session.logs.find((line) =>
                line.includes("did not answer workspace/configuration"),
            ),
        30_000,
        250,
    );
    const elapsedMs = Date.now() - askedAt;
    const elapsed = (elapsedMs / 1000).toFixed(1);
    // Assert the lower bound as well as the fact. A warning that arrived
    // early would mean the deadline is not the 10s one the server sets —
    // the whole point of this session is that a *real* ten-second timer
    // survived on a driver with nothing else to do. No upper bound: the
    // module compiles cold on CI, and this check is about the timer firing
    // at all, not about scheduling precision.
    check(
        "the 10s configuration deadline fires on an otherwise idle driver",
        Boolean(warned) && elapsedMs >= 10_000,
        warned
            ? `after ${elapsed}s (must be >= 10.0s): ${warned.slice(0, 50)}…`
            : `nothing after ${elapsed}s`,
    );
    check(
        "the runtime survived ten idle seconds without aborting",
        session.exitCode === null && !session.stderr.includes("condvar"),
        session.stderr ? `stderr: ${session.stderr.slice(0, 120)}` : "no stderr",
    );

    // ---- (4b) exit without shutdown --------------------------------------
    session.notify("exit", null);
    const code = await Promise.race([session.exited, sleep(30_000).then(() => "timeout")]);
    check(
        "exit without a preceding shutdown ends the process with status 1",
        code === 1,
        `exit status ${code}`,
    );
    if (code === "timeout") session.kill();
    return session;
}

/*
 * (4c) The editor went away.
 *
 * A closed stdin is how a crashed or force-quit editor ends a session, and the
 * driver has to notice: `poll_oneoff` reports the fd readable, the read that
 * follows returns zero bytes, and the process ends normally rather than
 * spinning on a stream that will never speak again. The native transport
 * behaves the same way — `Server::serve` returns when its stream ends.
 */
async function closedStdinSession() {
    const session = new Session();
    const initialize = await session.request(
        "initialize",
        initializeParams(),
        STARTUP_BUDGET_MS,
    );
    check("initialize answers on the closed-stdin session", Boolean(initialize?.result));
    session.child.stdin.end();
    const code = await Promise.race([session.exited, sleep(30_000).then(() => "timeout")]);
    check(
        "a closed stdin ends the session instead of spinning",
        code === 0,
        `exit status ${code}`,
    );
    if (code === "timeout") session.kill();
    return session;
}

/*
 * (4d) `exit` and the close arrive together.
 *
 * The regression guard for a real bug: the driver reads stdin and *then*
 * reports end-of-file, so a single pass can decode messages and see EOF at
 * once. Returning "end of input" straight from that pass stranded whatever it
 * had just decoded — and when an `exit` was among it, a session that should end
 * with status 1 ended with 0 instead.
 *
 * Forcing the window from outside is not deterministic: it needs the readiness
 * probe that follows a read to report the fd readable *before* its zero-length
 * clock subscription fires, and which of the two wasmtime reports first is a
 * genuine race. So this runs the scenario repeatedly and requires every run to
 * be right. Pre-fix the failure reproduced in roughly one run in six; post-fix
 * the loop is correct on both sides of the race, so any 0 here is a real
 * regression rather than a flake.
 */
const EOF_RACE_RUNS = 12;

async function exitClosingStdinSession() {
    // Enough work to keep the driver away from the poll for tens of
    // milliseconds, so `exit` and the close land while it is busy and are
    // taken in one pass when it returns.
    const busy = `proc p {a} {return $a}\n${"p 1\n".repeat(4000)}`;
    const codes = [];
    for (let run = 0; run < EOF_RACE_RUNS; run += 1) {
        const session = new Session();
        const initialize = await session.request(
            "initialize",
            initializeParams(),
            STARTUP_BUDGET_MS,
        );
        if (!initialize?.result) {
            codes.push("no-initialize");
            session.kill();
            continue;
        }
        session.notify("textDocument/didOpen", {
            textDocument: {
                uri: `file://${ROOT}/busy.tcl`,
                languageId: "tcl",
                version: 1,
                text: busy,
            },
        });
        await sleep(60);
        // One tick: the frame and the FIN go into the pipe together.
        session.notify("exit", null);
        session.child.stdin.end();
        const code = await Promise.race([session.exited, sleep(30_000).then(() => "timeout")]);
        codes.push(code);
        if (code === "timeout") session.kill();
    }
    const wrong = codes.filter((code) => code !== 1);
    check(
        `\`exit\` arriving with the stdin close still exits 1, across ${EOF_RACE_RUNS} runs`,
        wrong.length === 0,
        wrong.length === 0
            ? `all ${EOF_RACE_RUNS} runs exited 1`
            : `${wrong.length}/${EOF_RACE_RUNS} wrong: ${codes.join(",")}`,
    );
}

/*
 * (6) A cache directory the guest can actually write to.
 *
 * Every other scenario preopens the tracked `fixture/` directory with no
 * `XDG_CACHE_HOME`, so `tcl_userdirs::cache_dir()` resolves outside every
 * preopen and the compiled-pack cache is simply uncreatable — which meant the
 * whole spec-pack *write* path was structurally unreachable from this harness,
 * and stayed green through a defect that killed the module in the field.
 *
 * `write_atomically` named its temp file with `std::process::id()`. On
 * wasm32-wasip1 that is `unsupported::process::id`, which panics, and this
 * module is `panic = "abort"` — so `wasmtime run --dir <project>` (the
 * invocation the install docs recommend) aborted with status 134 seconds after
 * `initialized`, right after creating `spectcl/`. The eight embedded packs are
 * enough to trigger it; no user pack, no editor feature, nothing optional.
 *
 * The trigger is exactly "the cache directory resolves inside a preopen", so
 * that is what this scenario arranges: a scratch copy of the fixture, mounted
 * writable, with `XDG_CACHE_HOME` pointing inside the mount. The assertion that
 * the cache directory really appeared is load-bearing — without it a future
 * change to the preopen layout could quietly make this vacuous again, which is
 * the precise way the original gap opened.
 */
async function cacheWritableSession() {
    // A scratch copy: the tracked fixture must never grow a `.cache/`.
    const scratch = await mkdtemp(join(tmpdir(), "tcl-lsp-wasi-cache-"));
    let session;
    try {
        await cp(fixture, scratch, { recursive: true });
        session = new Session({
            preopen: scratch,
            env: { XDG_CACHE_HOME: `${ROOT}/.cache` },
        });
        const appSource = await readFile(join(fixture, "app.tcl"), "utf8");

        const initialize = await session.request(
            "initialize",
            initializeParams(),
            STARTUP_BUDGET_MS,
        );
        check(
            "initialize answers with a writable cache directory in reach",
            Boolean(initialize?.result),
        );

        // `initialized` is what loads the spec packs, and the load is what
        // writes the cache. Pre-fix the process was gone before the next check.
        session.notify("initialized", {});
        const initialised = await until(
            () => session.logs.some((line) => line.includes("initialised")),
            REPLY_BUDGET_MS,
        );
        check(
            "the session survives spec-pack load when the cache is writable",
            Boolean(initialised) && session.exitCode === null,
            session.exitCode === null
                ? "still running"
                : `died with status ${session.exitCode}: ${session.stderr.split("\n")[1] ?? session.stderr.slice(0, 100)}`,
        );

        // Naming the abort, not just "it died": a panic message in stderr is
        // the signature of an unsupported-on-wasip1 std call, and this harness
        // should say which one rather than reporting a bare non-zero exit.
        check(
            "no unsupported-platform panic reaches stderr",
            !session.stderr.includes("panicked") && !session.stderr.includes("no pids"),
            session.stderr ? `stderr: ${session.stderr.slice(0, 160)}` : "no stderr",
        );

        // The write path really ran. If this directory is absent the scenario
        // proved nothing — the cache was unreachable again, exactly as in every
        // other scenario here.
        const cacheDir = join(scratch, ".cache", "tcl-lsp", "spectcl");
        const wrote = await stat(cacheDir).then(
            (entry) => entry.isDirectory(),
            () => false,
        );
        check(
            "the compiled-pack cache was actually written inside the preopen",
            wrote,
            wrote ? cacheDir : `${cacheDir} was never created — this scenario is vacuous`,
        );

        // And the server still works afterwards: a load that aborted halfway
        // through could leave the registry incomplete rather than kill anyone.
        session.notify("textDocument/didOpen", {
            textDocument: { uri: APP_URI, languageId: "tcl", version: 1, text: appSource },
        });
        const published = await until(() => session.diagnostics.get(APP_URI), REPLY_BUDGET_MS);
        check(
            "analysis still answers after the cache write",
            Array.isArray(published),
            published ? `${published.length} diagnostics` : "none",
        );

        session.notify("exit", null);
        await Promise.race([session.exited, sleep(30_000).then(() => "timeout")]);
        if (session.exitCode === null) session.kill();
        return session;
    } finally {
        await rm(scratch, { recursive: true, force: true });
    }
}

async function main() {
    console.log(`module: ${modulePath}`);
    const first = await mainSession();
    console.log("");
    const second = await idleTimerSession();
    console.log("");
    const third = await closedStdinSession();
    console.log("");
    await exitClosingStdinSession();
    console.log("");
    const fourth = await cacheWritableSession();

    console.log("");
    console.log(`${results.length - failures}/${results.length} checks passed`);
    for (const session of [first, second, third, fourth]) {
        if (session.stderr.trim()) {
            console.log("server stderr:");
            for (const line of session.stderr.trim().split("\n").slice(0, 10)) {
                console.log(`   | ${line}`);
            }
        }
    }
    process.exit(failures === 0 ? 0 : 1);
}

main().catch((err) => {
    console.error("e2e harness failed:", err);
    process.exit(1);
});

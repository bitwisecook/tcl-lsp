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

// The front-end's DOM-free logic, under node's own test runner.
//
// The GitHub half is covered entirely by stubs. That is not a convenience: the
// container this is developed in cannot reach `api.github.com` at all, and a
// test that only passes where the network does is not a test of the code. Every
// behaviour that matters — pagination, the rate-limit message and its reset
// time, the tag→version derivation, the failure text a user is shown — is
// driven by a scripted `Http` here.
//
// Run with `npm test` (which bundles this file first — it imports the same
// TypeScript sources the browser bundle does).

import assert from "node:assert/strict";
import { describe, it } from "node:test";

import {
  failureMessage,
  fetchZipball,
  listTags,
  nextPageUrl,
  parseRepoRef,
  versionFromTag,
  zipballUrl,
  type Http,
  type HttpResponse,
} from "../src/github.js";
import {
  lifecycleOf,
  snapshotsJson,
  validateReleases,
  versionFromFileName,
  type Release,
} from "../src/releases.js";
import { mapSelectionThroughFormat } from "../src/textSelection.js";

describe("mapSelectionThroughFormat", () => {
  it("keeps a caret after its text when formatting inserts indentation", () => {
    const source = "command foo {\nsummary bar\n}";
    const formatted = "command foo {\n    summary bar\n}";
    const caret = source.indexOf("bar") + "bar".length;

    assert.deepEqual(mapSelectionThroughFormat(source, formatted, { start: caret, end: caret }), {
      start: formatted.indexOf("bar") + "bar".length,
      end: formatted.indexOf("bar") + "bar".length,
    });
  });

  it("keeps the caret at the end of indentation before the next token", () => {
    const source = "command foo {\n summary bar\n}";
    const formatted = "command foo {\n    summary bar\n}";
    const caret = source.indexOf("summary");

    assert.deepEqual(mapSelectionThroughFormat(source, formatted, { start: caret, end: caret }), {
      start: formatted.indexOf("summary"),
      end: formatted.indexOf("summary"),
    });
  });

  it("maps both ends of a selection through whitespace-only edits", () => {
    const source = "command foo {\nsummary bar\n}";
    const formatted = "command foo {\n    summary bar\n}";

    assert.deepEqual(
      mapSelectionThroughFormat(source, formatted, {
        start: source.indexOf("summary"),
        end: source.indexOf("bar") + "bar".length,
      }),
      {
        start: formatted.indexOf("summary"),
        end: formatted.indexOf("bar") + "bar".length,
      },
    );
  });
});

/** A scripted reply. */
interface Scripted {
  status?: number;
  headers?: Record<string, string>;
  body?: unknown;
  bytes?: Uint8Array;
}

/** Build an `Http` that answers each URL in turn from `script`, recording calls. */
function stubHttp(script: Scripted[]): { http: Http; calls: string[] } {
  const calls: string[] = [];
  let at = 0;
  const http: Http = (url: string) => {
    calls.push(url);
    const step = script[at] ?? {};
    at += 1;
    const status = step.status ?? 200;
    const headers = step.headers ?? {};
    const response: HttpResponse = {
      ok: status >= 200 && status < 300,
      status,
      header: (name) => headers[name.toLowerCase()] ?? null,
      json: () => Promise.resolve(step.body),
      bytes: () => Promise.resolve(step.bytes ?? new Uint8Array()),
    };
    return Promise.resolve(response);
  };
  return { http, calls };
}

/** Tag entries as the GitHub tags endpoint returns them. */
function tagsBody(...names: string[]): unknown {
  return names.map((name) => ({ name, commit: { sha: "0".repeat(40) } }));
}

describe("parseRepoRef", () => {
  it("accepts the shapes a repository actually reaches a clipboard in", () => {
    for (const input of [
      "bitwisecook/tcl-lsp",
      "  bitwisecook/tcl-lsp  ",
      "https://github.com/bitwisecook/tcl-lsp",
      "https://www.github.com/bitwisecook/tcl-lsp",
      "http://github.com/bitwisecook/tcl-lsp.git",
      "git@github.com:bitwisecook/tcl-lsp.git",
      "https://github.com/bitwisecook/tcl-lsp/releases/tag/v2.1.0",
    ]) {
      assert.deepEqual(parseRepoRef(input), { owner: "bitwisecook", repo: "tcl-lsp" }, input);
    }
  });

  it("refuses what is not a repository reference", () => {
    for (const input of ["", "   ", "tcl-lsp", "https://example.com/a/b", "a/b/c/d e"]) {
      assert.equal(parseRepoRef(input), null, input);
    }
  });
});

describe("versionFromTag", () => {
  it("reads the version out of the usual tag spellings", () => {
    assert.equal(versionFromTag("v1.2.0"), "1.2.0");
    assert.equal(versionFromTag("1.2.0"), "1.2.0");
    assert.equal(versionFromTag("mylib-1.2"), "1.2");
    assert.equal(versionFromTag("release-2.0.0rc1"), "2.0.0rc1");
  });

  it("hands back a tag it cannot read rather than dropping it", () => {
    assert.equal(versionFromTag("stable"), "stable");
    assert.equal(versionFromTag("  nightly  "), "nightly");
  });
});

describe("versionFromFileName", () => {
  it("suggests a label from the archive names GitHub produces", () => {
    assert.equal(versionFromFileName("mylib-1.2.0.zip"), "1.2.0");
    assert.equal(versionFromFileName("v1.2.0.zip"), "1.2.0");
    assert.equal(versionFromFileName("mylib-1.2.0-src.zip"), "1.2.0");
    assert.equal(versionFromFileName("downloads/tcllib-2.0.zip"), "2.0");
    assert.equal(versionFromFileName("pkg-1.0.0b2.zip"), "1.0.0b2");
  });

  it("suggests nothing rather than something wrong", () => {
    // A branch archive has no version in it, and `utf8` is not version 8.
    assert.equal(versionFromFileName("myrepo-main.zip"), "");
    assert.equal(versionFromFileName("mylib-utf8.zip"), "");
    assert.equal(versionFromFileName("sources.zip"), "");
  });
});

describe("nextPageUrl", () => {
  it("finds the next link among several relations", () => {
    const link =
      '<https://api.github.com/repositories/1/tags?page=2>; rel="next", ' +
      '<https://api.github.com/repositories/1/tags?page=9>; rel="last"';
    assert.equal(nextPageUrl(link), "https://api.github.com/repositories/1/tags?page=2");
  });

  it("reports no next page on the last one, and on no header at all", () => {
    assert.equal(nextPageUrl('<https://api.github.com/x?page=1>; rel="prev"'), null);
    assert.equal(nextPageUrl(null), null);
    assert.equal(nextPageUrl(""), null);
  });
});

describe("failureMessage", () => {
  const reply = (status: number, headers: Record<string, string>): HttpResponse => ({
    ok: false,
    status,
    header: (name) => headers[name.toLowerCase()] ?? null,
    json: () => Promise.resolve(null),
    bytes: () => Promise.resolve(new Uint8Array()),
  });

  it("names the reset time when the rate limit is what failed", () => {
    const reset = Math.floor(Date.UTC(2026, 7, 14, 18, 30, 0) / 1000);
    const message = failureMessage(
      reply(403, { "x-ratelimit-remaining": "0", "x-ratelimit-reset": String(reset) }),
      "could not list tags",
    );
    assert.match(message, /rate limit is used up/);
    assert.match(message, /It resets at /);
    // The reader's local clock, not a raw epoch second.
    assert.doesNotMatch(message, new RegExp(String(reset)));
  });

  it("treats 429 with no remaining quota as a rate limit too", () => {
    const message = failureMessage(
      reply(429, { "x-ratelimit-remaining": "0" }),
      "could not list tags",
    );
    assert.match(message, /rate limit is used up/);
    assert.match(message, /an unknown time/);
  });

  it("does not blame the rate limit for a 403 that still has quota", () => {
    const message = failureMessage(
      reply(403, { "x-ratelimit-remaining": "42" }),
      "could not list tags",
    );
    assert.equal(message, "could not list tags: GitHub replied HTTP 403.");
  });

  it("explains a 404 in terms a user can act on", () => {
    const message = failureMessage(reply(404, {}), "could not list tags");
    assert.match(message, /no such repository/);
    assert.match(message, /private repository/);
  });
});

describe("listTags", () => {
  const ref = { owner: "bitwisecook", repo: "tcl-lsp" };

  it("returns tags in GitHub's order with a version derived for each", async () => {
    const { http, calls } = stubHttp([{ body: tagsBody("v2.1.0", "v2.0.0", "v1.9.0") }]);
    const tags = await listTags(http, ref, 20);
    assert.deepEqual(tags, [
      { tag: "v2.1.0", version: "2.1.0" },
      { tag: "v2.0.0", version: "2.0.0" },
      { tag: "v1.9.0", version: "1.9.0" },
    ]);
    assert.equal(calls.length, 1);
    assert.match(calls[0], /^https:\/\/api\.github\.com\/repos\/bitwisecook\/tcl-lsp\/tags\?/);
  });

  it("follows the Link header until it has enough, then stops", async () => {
    const { http, calls } = stubHttp([
      {
        body: tagsBody("v3.0", "v2.0"),
        headers: { link: '<https://api.github.com/x?page=2>; rel="next"' },
      },
      { body: tagsBody("v1.0", "v0.9") },
    ]);
    const tags = await listTags(http, ref, 3);
    assert.deepEqual(
      tags.map((t) => t.tag),
      ["v3.0", "v2.0", "v1.0"],
    );
    assert.equal(calls.length, 2);
    assert.equal(calls[1], "https://api.github.com/x?page=2");
  });

  it("stops at the last page rather than asking forever", async () => {
    const { http, calls } = stubHttp([{ body: tagsBody("v1.0") }]);
    const tags = await listTags(http, ref, 100);
    assert.equal(tags.length, 1);
    assert.equal(calls.length, 1);
  });

  it("turns a failed page into the message the panel shows", async () => {
    const { http } = stubHttp([{ status: 404 }]);
    await assert.rejects(
      () => listTags(http, ref, 20),
      (e: Error) => {
        assert.match(e.message, /could not list the tags of bitwisecook\/tcl-lsp/);
        assert.match(e.message, /no such repository/);
        return true;
      },
    );
  });

  it("ignores entries a page carries that are not named tags", async () => {
    const { http } = stubHttp([{ body: [{ name: "v1.0" }, { nope: true }, { name: "" }, 7] }]);
    const tags = await listTags(http, ref, 20);
    assert.deepEqual(
      tags.map((t) => t.tag),
      ["v1.0"],
    );
  });
});

describe("zipballUrl and fetchZipball", () => {
  const ref = { owner: "bitwisecook", repo: "tcl-lsp" };

  it("addresses the tag through refs/tags so a same-named branch cannot win", () => {
    assert.equal(
      zipballUrl(ref, "v1.2.0"),
      "https://codeload.github.com/bitwisecook/tcl-lsp/zip/refs/tags/v1.2.0",
    );
  });

  it("escapes each path segment of a slashed tag without escaping the slash", () => {
    assert.equal(
      zipballUrl(ref, "release/1.0 final"),
      "https://codeload.github.com/bitwisecook/tcl-lsp/zip/refs/tags/release/1.0%20final",
    );
  });

  it("returns the archive bytes on success", async () => {
    const payload = new Uint8Array([0x50, 0x4b, 0x03, 0x04]);
    const { http, calls } = stubHttp([{ bytes: payload }]);
    assert.deepEqual(await fetchZipball(http, ref, "v1.2.0"), payload);
    assert.equal(calls[0], zipballUrl(ref, "v1.2.0"));
  });

  it("names the release in the failure so a partial fetch says which one stopped", async () => {
    const { http } = stubHttp([{ status: 500 }]);
    await assert.rejects(
      () => fetchZipball(http, ref, "v1.2.0"),
      (e: Error) => {
        assert.match(e.message, /could not download bitwisecook\/tcl-lsp v1\.2\.0/);
        assert.match(e.message, /HTTP 500/);
        return true;
      },
    );
  });
});

describe("validateReleases", () => {
  const release = (version: string, origin = `${version}.zip`): Release => ({
    id: origin,
    origin,
    version,
    entries: [{ name: "a.tcl", text: "proc a {} {}" }],
    skipped: [],
  });

  it("passes a well-formed set silently", () => {
    assert.deepEqual(validateReleases([release("1.0"), release("1.1")]), []);
  });

  it("says a single release cannot witness a range", () => {
    const problems = validateReleases([release("1.0")]);
    assert.equal(problems.length, 1);
    assert.match(problems[0], /cannot witness a range/);
  });

  it("names every unlabelled archive rather than only the first", () => {
    const problems = validateReleases([release(""), release(" ", "b.zip"), release("2.0")]);
    assert.equal(problems.filter((p) => p.includes("no version label")).length, 2);
  });

  it("refuses duplicate labels, which would collapse two releases into one", () => {
    const problems = validateReleases([release("1.0", "a.zip"), release("1.0", "b.zip")]);
    assert.ok(problems.some((p) => /Version 1\.0 is used by 2 archives/.test(p)));
  });

  it("reports an archive that yielded no Tcl at all", () => {
    const empty: Release = { ...release("1.0"), entries: [] };
    assert.ok(validateReleases([empty, release("1.1")]).some((p) => /no Tcl sources/.test(p)));
  });
});

describe("snapshotsJson", () => {
  it("emits exactly the shape the wasm export parses, with labels trimmed", () => {
    const json = snapshotsJson([
      {
        id: "a",
        origin: "a.zip",
        version: " 1.0 ",
        entries: [{ name: "pkg/a.tcl", text: "proc a {} {}" }],
        skipped: ["big.tcl: too big"],
      },
    ]);
    assert.deepEqual(JSON.parse(json), [
      { version: "1.0", files: [{ name: "pkg/a.tcl", text: "proc a {} {}" }] },
    ]);
  });
});

describe("lifecycleOf", () => {
  it("reads the bounds off the draft that is about to be written", () => {
    assert.deepEqual(lifecycleOf({ introduced_version: "1.2", retired_version: "3.0" }), {
      introduced: "1.2",
      retired: "3.0",
      deprecated: null,
    });
  });

  it("treats an absent, null, or empty field as no claim", () => {
    assert.deepEqual(lifecycleOf({ introduced_version: "", retired_version: null }), {
      introduced: null,
      retired: null,
      deprecated: null,
    });
  });
});

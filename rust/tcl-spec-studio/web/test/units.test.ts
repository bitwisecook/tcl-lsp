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
import {
  alsoInSentence,
  browserCountLine,
  groupByPack,
  packCountLabel,
  packIndex,
  packSections,
} from "../src/packs.js";
import {
  describeSubject,
  fieldAnchorId,
  formatHash,
  historyMode,
  labelIndex,
  ownerField,
  parseHash,
  relatedGroups,
  restoreFor,
  routeSubject,
  type DockSources,
  type Route,
  type Restore,
} from "../src/docsDock.js";
import { mapSelectionThroughFormat } from "../src/textSelection.js";
import type { CodeExample, FieldSchema, IndexEntry, PackRow, Schema } from "../src/types.js";

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

/* The registry browser's pack navigator -------------------------------- */

/** One row of the command index, with only the fields the browser reads set. */
function entry(name: string, pack: string, summary = "", alsoIn: string[] = []): IndexEntry {
  return {
    name,
    summary,
    synopsis: "",
    subcommands: 0,
    options: 0,
    deprecated: false,
    pack,
    also_in: alsoIn,
  };
}

/** One catalogue row; only the id and the counts change between them. */
function pack(id: string, commands: number): PackRow {
  return {
    id,
    label: id.toUpperCase(),
    blurb: `the ${id} pack`,
    commands,
    path: `rust/tcl-registry/src/commands/${id}`,
  };
}

describe("groupByPack", () => {
  it("files each command under the pack that declares it, names ascending", () => {
    const grouped = groupByPack([
      entry("lsort", "tcl"),
      entry("grid", "tk"),
      entry("append", "tcl"),
      entry("bind", "tk"),
    ]);
    assert.deepEqual(
      [...grouped.keys()],
      ["tcl", "tk"],
      "a pack appears where its first command does",
    );
    assert.deepEqual(
      grouped.get("tcl")?.map((e) => e.name),
      ["append", "lsort"],
    );
    assert.deepEqual(
      grouped.get("tk")?.map((e) => e.name),
      ["bind", "grid"],
    );
  });

  it("has no pack at all for an empty index", () => {
    assert.equal(groupByPack([]).size, 0);
  });
});

describe("packSections", () => {
  const index = [
    entry("append", "tcl"),
    entry("lsort", "tcl", "sort a list"),
    entry("grid", "tk", "the grid geometry manager"),
    entry("spawn", "expect"),
  ];
  const catalogue = [pack("tcl", 2), pack("tk", 1), pack("expect", 1), pack("itcl", 0)];

  it("keeps the catalogue's order and drops the packs this dialect never reaches", () => {
    const sections = packSections(catalogue, groupByPack(index), "");
    assert.deepEqual(
      sections.map((s) => s.pack.id),
      ["tcl", "tk", "expect"],
    );
    assert.deepEqual(
      sections.map((s) => s.matches.length),
      [2, 1, 1],
      "an empty filter matches everything",
    );
  });

  it("keeps only the packs a filter leaves something in", () => {
    const sections = packSections(catalogue, groupByPack(index), "sort");
    assert.deepEqual(
      sections.map((s) => s.pack.id),
      ["tcl"],
    );
    assert.deepEqual(
      sections[0].matches.map((e) => e.name),
      ["lsort"],
      "a summary match counts as much as a name match",
    );
    assert.equal(sections[0].commands.length, 2, "the section still knows its real size");
  });

  it("still browses a dialect the catalogue does not describe", () => {
    // An unknown dialect has an empty pack list; showing nothing would be
    // worse than showing the commands under a bare heading.
    const sections = packSections([], groupByPack(index), "");
    assert.deepEqual(
      sections.map((s) => s.pack.id),
      ["expect", "tcl", "tk"],
    );
    assert.equal(sections[0].pack.blurb, "", "an invented pack claims no documentation");
  });

  it("files a command with no declared pack rather than losing it", () => {
    const sections = packSections(
      [pack("tcl", 1)],
      groupByPack([entry("append", "tcl"), entry("mystery", "")]),
      "",
    );
    assert.deepEqual(
      sections.map((s) => s.pack.label),
      ["TCL", "Other commands"],
    );
  });
});

describe("packIndex", () => {
  it("answers a chip's question: what is this pack id called, and where is it", () => {
    const byId = packIndex([pack("tcl", 2), pack("tk", 1)]);
    assert.equal(byId.get("tk")?.path, "rust/tcl-registry/src/commands/tk");
    assert.equal(byId.get("nosuch"), undefined);
  });
});

describe("packCountLabel", () => {
  it("names the pack's size when nothing is filtered out", () => {
    assert.equal(packCountLabel(199, 199), "199 commands");
    assert.equal(packCountLabel(1, 1), "1 command");
  });

  it("gives the share of the pack a filter left", () => {
    assert.equal(packCountLabel(3, 199), "3 of 199");
    assert.equal(packCountLabel(0, 199), "0 of 199");
  });
});

describe("browserCountLine", () => {
  it("says what is being viewed and where it lives", () => {
    assert.equal(browserCountLine("Tcl 9.0", 187, 187, 4), "187 Tcl 9.0 commands in 4 packs");
    assert.equal(browserCountLine("Tcl 9.0", 1, 1, 1), "1 Tcl 9.0 command in 1 pack");
  });

  it("names the packs a filter matched as well as the commands", () => {
    assert.equal(browserCountLine("Tcl 9.0", 187, 12, 3), "12 of 187 Tcl 9.0 commands, in 3 packs");
  });

  it("says so plainly when there is nothing to show", () => {
    assert.equal(browserCountLine("Tcl 9.0", 187, 0, 0), "no match in 187 Tcl 9.0 commands");
    assert.equal(browserCountLine("nosuch", 0, 0, 0), "no commands in nosuch");
  });
});

describe("alsoInSentence", () => {
  it("names the other packs that declare the same command", () => {
    assert.equal(
      alsoInSentence("close", ["expect", "irules"]),
      "close is also declared in expect, irules.",
    );
  });

  it("says nothing about a name only one pack declares", () => {
    assert.equal(alsoInSentence("lsort", []), "");
    assert.equal(alsoInSentence("lsort", [""]), "");
  });
});

/* The live documentation dock ------------------------------------------- */

/** The smallest example the annotated renderer will take. */
function example(code: string): CodeExample {
  return { code, annotations: [] };
}

function field(key: string, over: Partial<FieldSchema> = {}): FieldSchema {
  return {
    key,
    label: key.replace(/_/g, " "),
    doc: `what ${key} means`,
    group: "Behaviour",
    help: `the long form of ${key}`,
    example: example(`# ${key}`),
    kind: { tag: "bool" },
    ...over,
  };
}

/** A schema with just enough in it to resolve every kind of subject. */
function schema(): Schema {
  return {
    groups: ["Identity", "Behaviour"],
    groupHelp: { Behaviour: "what the command does when it runs" },
    groupExamples: { Behaviour: example("# behaviour") },
    catalogues: {
      taintColour: [
        { key: "Http", doc: "from the request", example: example("# http") },
        { key: "Session", doc: "from the session", group: "Sources", example: example("# ssn") },
      ],
    },
    catalogueHelp: {
      taintColour: {
        title: "Taint colours",
        intro: "where a value came from",
        example: example(""),
      },
    },
    nestedFields: [
      {
        key: "variable_scope",
        label: "Variable scope",
        doc: "which scope an option writes",
        owner: "OptionArg",
        field: "options",
        group: "Behaviour",
        help: "the long form of variable_scope",
        example: example("# scope"),
      },
      {
        key: "orphan_property",
        label: "Orphan property",
        doc: "a property from a wasm that predates the owning-row key",
        owner: "OptionArg",
        group: "Behaviour",
        help: "the long form of orphan_property",
        example: example("# orphan"),
      },
    ],
    command: [
      field("pure", {
        related: [
          {
            name: "Purity",
            why: "a pure command cannot also declare side effects.",
            keys: ["pure", "side_effects", "nosuch_key"],
          },
        ],
      }),
      field("side_effects", { label: "Side effects" }),
      field("options", { label: "Options", group: "Behaviour" }),
    ],
    subcommand: [field("arity", { group: "Identity" })],
  };
}

function sources(): DockSources {
  return {
    schema: schema(),
    packs: new Map([["tk", pack("tk", 12)]]),
  };
}

describe("describeSubject", () => {
  it("documents a setting with its schema help and its example", () => {
    const content = describeSubject(sources(), { kind: "field", key: "pure" });
    assert.equal(content?.title, "pure");
    assert.equal(content?.code, "pure");
    assert.equal(content?.doc, "what pure means");
    assert.equal(content?.help, "the long form of pure");
    assert.equal(content?.example?.code, "# pure");
    assert.equal(content?.kindLabel, "Setting · Behaviour");
  });

  it("falls back to the subcommand table, then to nested properties", () => {
    assert.equal(describeSubject(sources(), { kind: "field", key: "arity" })?.title, "arity");
    const nested = describeSubject(sources(), { kind: "field", key: "variable_scope" });
    assert.equal(nested?.title, "Variable scope");
    // Named by the row it is a property of, not by the Rust type carrying it:
    // the row is the place the author edits it.
    assert.equal(nested?.kindLabel, "Setting · inside Options");
    assert.equal(
      describeSubject(sources(), { kind: "field", key: "orphan_property" })?.kindLabel,
      "Setting · inside OptionArg",
    );
  });

  it("documents a group, a catalogue, one of its values, and a pack", () => {
    const group = describeSubject(sources(), { kind: "group", name: "Behaviour" });
    assert.equal(group?.help, "what the command does when it runs");
    assert.equal(group?.doc, "3 settings in this group.");

    const catalogue = describeSubject(sources(), { kind: "catalogue", id: "taintColour" });
    assert.equal(catalogue?.title, "Taint colours");
    assert.equal(catalogue?.doc, "2 values to pick from.");

    const value = describeSubject(sources(), {
      kind: "value",
      catalogue: "taintColour",
      key: "Session",
    });
    assert.equal(value?.title, "Session");
    assert.equal(value?.kindLabel, "Value of Taint colours");
    assert.equal(value?.doc, "from the session");

    const declaring = describeSubject(sources(), { kind: "pack", id: "tk" });
    assert.equal(declaring?.title, "TK");
    assert.equal(declaring?.kindLabel, "Pack · 12 commands");
  });

  it("resolves nothing it cannot document, so the dock keeps what it had", () => {
    assert.equal(describeSubject(sources(), { kind: "field", key: "nosuch" }), null);
    assert.equal(describeSubject(sources(), { kind: "group", name: "Nosuch" }), null);
    assert.equal(describeSubject(sources(), { kind: "catalogue", id: "nosuch" }), null);
    assert.equal(
      describeSubject(sources(), { kind: "value", catalogue: "taintColour", key: "nosuch" }),
      null,
    );
    assert.equal(describeSubject(sources(), { kind: "pack", id: "nosuch" }), null);
  });
});

describe("relatedGroups", () => {
  it("labels every key, marks the field itself, and flags keys the schema lacks", () => {
    const clusters = describeSubject(sources(), { kind: "field", key: "pure" })?.related ?? [];
    assert.equal(clusters.length, 1);
    assert.equal(clusters[0]?.name, "Purity");
    assert.equal(clusters[0]?.why, "a pure command cannot also declare side effects.");
    assert.deepEqual(clusters[0]?.links, [
      { key: "pure", label: "pure", self: true, known: true, target: "pure" },
      {
        key: "side_effects",
        label: "Side effects",
        self: false,
        known: true,
        target: "side_effects",
      },
      { key: "nosuch_key", label: "nosuch_key", self: false, known: false, target: null },
    ]);
  });

  it("sends a nested key to the row it is edited in, and offers no other", () => {
    // A link that goes nowhere is worse than no link: a property with a row
    // targets that row, and one the schema cannot place targets nothing, so
    // the dock draws it inert.
    const clusters = relatedGroups(
      field("pure", {
        related: [
          {
            name: "Scope",
            why: "an option's scope is read with the command's purity.",
            keys: ["variable_scope", "orphan_property"],
          },
        ],
      }),
      schema(),
    );
    assert.deepEqual(clusters[0]?.links, [
      {
        key: "variable_scope",
        label: "Variable scope",
        self: false,
        known: true,
        target: "options",
      },
      {
        key: "orphan_property",
        label: "Orphan property",
        self: false,
        known: true,
        target: null,
      },
    ]);
  });

  it("says nothing when a studio build predates the key", () => {
    // `related` is optional in the wire contract: a wasm module built before
    // it existed simply sends no clusters, and the dock ends at the example.
    assert.deepEqual(relatedGroups(field("pure"), schema()), []);
    assert.deepEqual(
      describeSubject(sources(), { kind: "field", key: "side_effects" })?.related,
      [],
    );
  });
});

describe("ownerField", () => {
  it("answers with the row a key is edited in, and nothing for a key it lacks", () => {
    const model = schema();
    assert.equal(ownerField(model, "pure"), "pure");
    assert.equal(ownerField(model, "arity"), "arity", "a subcommand key is a row of its own");
    assert.equal(ownerField(model, "variable_scope"), "options");
    assert.equal(ownerField(model, "orphan_property"), null);
    assert.equal(ownerField(model, "nosuch"), null);
  });

  it("indexes every documented key, nested properties included", () => {
    const labels = labelIndex(schema());
    assert.equal(labels.get("side_effects"), "Side effects");
    assert.equal(labels.get("variable_scope"), "Variable scope");
    assert.equal(labels.has("nosuch"), false);
  });
});

describe("fieldAnchorId", () => {
  it("derives one stable id per key", () => {
    assert.equal(fieldAnchorId("arity_windows"), "field-arity_windows");
    assert.equal(fieldAnchorId("options.arity_hook"), "field-options-arity_hook");
  });
});

describe("parseHash and formatHash", () => {
  it("round-trips a command, a focused setting, and a reference entry", () => {
    const cases: Route[] = [
      { view: "command", dialect: "tcl9.0", command: "lsort", field: null },
      { view: "command", dialect: "tcl9.0", command: "lsort", field: "arity_windows" },
      { view: "reference", catalogue: "taintColour", variant: null },
      { view: "reference", catalogue: "taintColour", variant: "Http" },
    ];
    for (const route of cases) {
      assert.deepEqual(parseHash(formatHash(route)), route, formatHash(route));
    }
  });

  it("encodes a command name that a path would otherwise split", () => {
    const route: Route = {
      view: "command",
      dialect: "spectcl",
      command: "::tcl::mathfunc::ceil",
      field: null,
    };
    assert.equal(formatHash(route), "#/c/spectcl/%3A%3Atcl%3A%3Amathfunc%3A%3Aceil");
    assert.deepEqual(parseHash(formatHash(route)), route);
  });

  it("reads a fragment that names no view of the studio as no route", () => {
    for (const hash of ["", "#", "#/", "#/c", "#/c/tcl9.0", "#/ref", "#/nosuch/thing"]) {
      assert.equal(parseHash(hash), null, hash);
    }
  });

  it("survives a hand-edited fragment rather than throwing during boot", () => {
    assert.deepEqual(parseHash("#/c/tcl9.0/%E0%A4%A"), {
      view: "command",
      dialect: "tcl9.0",
      command: "%E0%A4%A",
      field: null,
    });
  });
});

describe("historyMode", () => {
  it("replaces within one command so Back moves between commands", () => {
    const lsort: Route = { view: "command", dialect: "tcl9.0", command: "lsort", field: null };
    const focused: Route = { ...lsort, field: "arity" };
    assert.equal(historyMode(lsort, focused), "replace");
    assert.equal(historyMode(focused, lsort), "replace");
  });

  it("pushes for another command, another dialect, or another kind of view", () => {
    const lsort: Route = { view: "command", dialect: "tcl9.0", command: "lsort", field: null };
    assert.equal(historyMode(lsort, { ...lsort, command: "lindex" }), "push");
    assert.equal(historyMode(lsort, { ...lsort, dialect: "spectcl" }), "push");
    assert.equal(
      historyMode(lsort, { view: "reference", catalogue: "taintColour", variant: null }),
      "push",
    );
    assert.equal(historyMode(null, lsort), "push");
  });

  it("replaces when a route is re-written over itself, as restoring one does", () => {
    const entry: Route = { view: "reference", catalogue: "taintColour", variant: "Http" };
    assert.equal(historyMode(entry, { ...entry }), "replace");
    assert.equal(historyMode(entry, { ...entry, variant: "Session" }), "push");
    assert.equal(historyMode({ ...entry, variant: null }, { ...entry, variant: null }), "replace");
  });
});

describe("restoreFor", () => {
  const lsort: Route = { view: "command", dialect: "tcl9.0", command: "lsort", field: null };
  const reference: Route = { view: "reference", catalogue: "taintColour", variant: "Http" };

  it("restores a Reference entry as the Reference view, not as the visit it was opened from", () => {
    // The entry is tagged with whichever visit was open when it was written,
    // so a Reference entry carries the command it was opened from; the
    // fragment is what says which view the entry is *for*.
    assert.deepEqual(restoreFor(reference, 0, "lsort"), { kind: "route", route: reference });
  });

  it("opens the visit an entry names when the fragment agrees it is that command", () => {
    assert.deepEqual(restoreFor(lsort, 3, "lsort"), { kind: "visit", visit: 3, field: null });
    assert.deepEqual(restoreFor({ ...lsort, field: "arity" }, 3, "lsort"), {
      kind: "visit",
      visit: 3,
      field: "arity",
    });
  });

  it("follows the fragment when the visit stack disagrees or has lost the visit", () => {
    assert.deepEqual(restoreFor(lsort, 3, "lindex"), { kind: "route", route: lsort });
    assert.deepEqual(restoreFor(lsort, 3, null), { kind: "route", route: lsort });
    assert.deepEqual(restoreFor(lsort, null, null), { kind: "route", route: lsort });
  });

  it("falls back to the entry's visit only where there is no view to restore", () => {
    assert.deepEqual(restoreFor(null, 2, "lsort"), { kind: "visit", visit: 2, field: null });
    assert.deepEqual(restoreFor(null, null, null), { kind: "nothing" });
  });

  it("takes X → Reference → Y back to the Reference entry and then to X", () => {
    // The sequence the single-history design has to get right: Back off Y
    // lands on the Reference entry, and the next Back on X's own entry.
    const y: Route = { ...lsort, command: "lindex" };
    const back: Restore[] = [restoreFor(reference, 0, "lsort"), restoreFor(lsort, 0, "lsort")];
    assert.deepEqual(back, [
      { kind: "route", route: reference },
      { kind: "visit", visit: 0, field: null },
    ]);
    assert.deepEqual(restoreFor(y, 1, "lindex"), { kind: "visit", visit: 1, field: null });
  });
});

describe("routeSubject", () => {
  it("documents the setting a command route focuses, and nothing when it names none", () => {
    assert.deepEqual(
      routeSubject({ view: "command", dialect: "tcl9.0", command: "lsort", field: "pure" }),
      { kind: "field", key: "pure" },
    );
    assert.equal(
      routeSubject({ view: "command", dialect: "tcl9.0", command: "lsort", field: null }),
      null,
    );
  });

  it("documents a reference route as its catalogue, or as the one value it names", () => {
    assert.deepEqual(routeSubject({ view: "reference", catalogue: "taint", variant: null }), {
      kind: "catalogue",
      id: "taint",
    });
    assert.deepEqual(routeSubject({ view: "reference", catalogue: "taint", variant: "Http" }), {
      kind: "value",
      catalogue: "taint",
      key: "Http",
    });
  });
});

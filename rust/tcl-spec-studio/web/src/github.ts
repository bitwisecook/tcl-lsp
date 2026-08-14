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

// The studio's one network feature, in one file.
//
// Everything else in this page is offline by construction. This module fetches
// a repository's tags from `api.github.com` and its release archives from
// `codeload.github.com`, unauthenticated, straight from the browser — so it is
// kept apart, named in the page's privacy notice, and reached only when the
// author asks for it by filling the panel in.
//
// Every request goes through the injected {@link Http} rather than calling
// `fetch` directly. That is not ceremony: the container this is developed in
// cannot reach GitHub at all, so the behaviour that matters — pagination,
// rate-limit reporting, the tag→version derivation, the failure messages —
// is exercised against stubs in `test/units.test.ts` and never against the
// live API.

/** One HTTP reply, narrowed to what this module reads. */
export interface HttpResponse {
  readonly ok: boolean;
  readonly status: number;
  /** A response header, case-insensitively, or `null` when absent. */
  header(name: string): string | null;
  json(): Promise<unknown>;
  bytes(): Promise<Uint8Array>;
}

/** The one way this module reaches the network. */
export type Http = (url: string) => Promise<HttpResponse>;

/** A repository, as the two path segments GitHub addresses it by. */
export interface RepoRef {
  owner: string;
  repo: string;
}

/** One tag, as the panel lists it. */
export interface TagRef {
  /** The git tag, verbatim (`v1.2.0`). */
  tag: string;
  /** The version label derived from it (`1.2.0`) — editable by the author. */
  version: string;
}

/** How many tag pages are ever walked, whatever the caller asks for. */
const MAX_PAGES = 10;

/** GitHub's maximum page size for the tags endpoint. */
const PAGE_SIZE = 100;

/** `fetch`, wrapped in the {@link Http} shape the module is written against. */
export const browserHttp: Http = async (url: string): Promise<HttpResponse> => {
  const response = await fetch(url, { credentials: "omit", referrerPolicy: "no-referrer" });
  return {
    ok: response.ok,
    status: response.status,
    header: (name: string) => response.headers.get(name),
    json: () => response.json() as Promise<unknown>,
    bytes: async () => new Uint8Array(await response.arrayBuffer()),
  };
};

/**
 * Read `owner/repo` out of whatever the author pasted.
 *
 * A repository reaches a clipboard in half a dozen shapes — the browser URL,
 * the clone URL, the SSH remote, or just the two names. Accepting all of them
 * costs one regular expression and removes the most likely reason for the
 * panel's first attempt to fail.
 */
export function parseRepoRef(input: string): RepoRef | null {
  const trimmed = input.trim().replace(/\.git$/, "");
  if (!trimmed) return null;
  // Two shapes, not one with an optional prefix: a trailing path is meaningful
  // after a github.com URL (`…/releases/tag/v1`) and meaningless without one,
  // so `a/b/c/d` must not quietly become `a/b`.
  const url =
    /^(?:https?:\/\/(?:www\.)?github\.com\/|git@github\.com:)([\w.-]+)\/([\w.-]+)(?:[/#?].*)?$/.exec(
      trimmed,
    );
  const bare = /^([\w.-]+)\/([\w.-]+)$/.exec(trimmed);
  const match = url ?? bare;
  if (!match) return null;
  const [, owner, repo] = match;
  if (!owner || !repo || owner === "." || repo === ".") return null;
  return { owner, repo };
}

/**
 * The version label a tag names.
 *
 * `v1.2.0` and `mylib-1.2.0` are both the release `1.2.0` as far as
 * `package require` is concerned, and the version comparison the multi-release
 * importer runs is defined over that form. A tag with no recognisable version
 * in it is handed back unchanged rather than dropped — the author can correct
 * it, and refusing to guess is better than guessing wrong.
 */
export function versionFromTag(tag: string): string {
  const match = /(\d+(?:\.\d+)*(?:[.-]?(?:a|b|rc|alpha|beta)\d*)?)$/i.exec(tag.trim());
  return match?.[1] ?? tag.trim();
}

/**
 * The `rel="next"` URL out of a `Link` header, or `null` at the last page.
 *
 * GitHub paginates by header rather than by a field in the body, so this is
 * the only place the walk below learns whether there is more.
 */
export function nextPageUrl(link: string | null): string | null {
  if (!link) return null;
  for (const part of link.split(",")) {
    const match = /^\s*<([^>]+)>\s*;\s*rel="next"\s*$/.exec(part);
    if (match) return match[1];
  }
  return null;
}

/**
 * The message for a reply that failed, including the rate-limit reset time.
 *
 * An unauthenticated caller gets 60 requests an hour from a shared pool, so
 * "403" on its own is the least useful thing the page could say. When the
 * headers name the reset, the message says when it is in the reader's own
 * local time — that is the only number that answers "so when can I retry?".
 */
export function failureMessage(response: HttpResponse, what: string): string {
  const remaining = response.header("x-ratelimit-remaining");
  const reset = response.header("x-ratelimit-reset");
  const rateLimited =
    (response.status === 403 || response.status === 429) && remaining !== null && remaining === "0";
  if (rateLimited) {
    const at = Number.parseInt(reset ?? "", 10);
    const when = Number.isFinite(at) ? new Date(at * 1000).toLocaleTimeString() : "an unknown time";
    return (
      `GitHub's unauthenticated rate limit is used up (60 requests an hour, shared by everyone ` +
      `on your address). It resets at ${when}.`
    );
  }
  if (response.status === 404) {
    return `${what}: GitHub says there is no such repository (404) — check the owner and name, and note that a private repository cannot be read without a token.`;
  }
  return `${what}: GitHub replied HTTP ${response.status}.`;
}

/** One tags-endpoint page, as far as this module reads it. */
function tagsOfPage(body: unknown): string[] {
  if (!Array.isArray(body)) return [];
  return body
    .map((entry) =>
      entry && typeof entry === "object" && "name" in entry
        ? (entry as { name: unknown }).name
        : null,
    )
    .filter((name): name is string => typeof name === "string" && name.length > 0);
}

/**
 * List a repository's tags, newest first, walking pagination until `limit`.
 *
 * GitHub returns tags in its own order (newest first for the common case of
 * an ascending version scheme), and that order is preserved rather than
 * re-sorted here — the multi-release importer sorts by version itself, and
 * pretending to know better about a tag naming scheme we have not seen would
 * only move the guess earlier.
 */
export async function listTags(http: Http, ref: RepoRef, limit: number): Promise<TagRef[]> {
  let url = `https://api.github.com/repos/${encodeURIComponent(ref.owner)}/${encodeURIComponent(
    ref.repo,
  )}/tags?per_page=${PAGE_SIZE}`;
  const tags: TagRef[] = [];
  for (let page = 0; page < MAX_PAGES && tags.length < limit; page += 1) {
    const response = await http(url);
    if (!response.ok) {
      throw new Error(
        failureMessage(response, `could not list the tags of ${ref.owner}/${ref.repo}`),
      );
    }
    const names = tagsOfPage(await response.json());
    for (const name of names) {
      tags.push({ tag: name, version: versionFromTag(name) });
    }
    const next = nextPageUrl(response.header("link"));
    if (!next || names.length === 0) break;
    url = next;
  }
  return tags.slice(0, limit);
}

/** The codeload URL a tag's source zip lives at. */
export function zipballUrl(ref: RepoRef, tag: string): string {
  // `refs/tags/<tag>` rather than the bare tag: a tag and a branch may share a
  // name, and the unqualified form resolves to whichever GitHub prefers.
  return `https://codeload.github.com/${encodeURIComponent(ref.owner)}/${encodeURIComponent(
    ref.repo,
  )}/zip/refs/tags/${tag.split("/").map(encodeURIComponent).join("/")}`;
}

/** Fetch one tag's source archive. */
export async function fetchZipball(http: Http, ref: RepoRef, tag: string): Promise<Uint8Array> {
  const response = await http(zipballUrl(ref, tag));
  if (!response.ok) {
    throw new Error(failureMessage(response, `could not download ${ref.owner}/${ref.repo} ${tag}`));
  }
  return response.bytes();
}

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

import * as assert from "assert";
import {
  buildIruleEventSkeleton,
  buildIruleEventSkeletonFor,
  COMMON_IRULE_EVENTS,
  describeSkeletonRejections,
  iruleEventDeclaredAt,
} from "../iruleSkeleton";

suite("iRule Event Skeleton", () => {
  test("auto-includes CLIENT_ACCEPTED for HTTP handlers", () => {
    const skeleton = buildIruleEventSkeleton(["HTTP_REQUEST"]);
    const clientAcceptedIndex = skeleton.indexOf("when CLIENT_ACCEPTED {");
    const requestIndex = skeleton.indexOf("when HTTP_REQUEST {");

    assert.ok(clientAcceptedIndex >= 0, "CLIENT_ACCEPTED block should be auto-included");
    assert.ok(
      requestIndex > clientAcceptedIndex,
      "HTTP_REQUEST should appear after CLIENT_ACCEPTED",
    );
    assert.ok(skeleton.includes("set debug 0"), "CLIENT_ACCEPTED should set debug flag");
    assert.ok(skeleton.includes("if {$debug}"), "HTTP_REQUEST scaffold should guard with $debug");
    assert.strictEqual(
      skeleton.indexOf("when RULE_INIT {"),
      -1,
      "RULE_INIT should not be auto-included",
    );
  });

  test("request payload scaffold includes collect/release-safe structure", () => {
    const skeleton = buildIruleEventSkeleton(["HTTP_REQUEST_DATA"]);

    assert.ok(skeleton.includes("when HTTP_REQUEST_DATA {"));
    assert.ok(skeleton.includes("HTTP::release"), "payload handler should include HTTP::release");
    assert.ok(skeleton.includes("set payload [HTTP::payload]"));
  });

  test("deduplicates events and keeps canonical ordering", () => {
    const skeleton = buildIruleEventSkeleton([
      "HTTP_RESPONSE",
      "RULE_INIT",
      "HTTP_REQUEST",
      "http_response",
    ]);

    const ruleInitIndex = skeleton.indexOf("when RULE_INIT {");
    const clientAcceptedIndex = skeleton.indexOf("when CLIENT_ACCEPTED {");
    const requestIndex = skeleton.indexOf("when HTTP_REQUEST {");
    const responseIndex = skeleton.indexOf("when HTTP_RESPONSE {");

    assert.ok(ruleInitIndex >= 0);
    assert.ok(clientAcceptedIndex > ruleInitIndex, "CLIENT_ACCEPTED after RULE_INIT");
    assert.ok(requestIndex > clientAcceptedIndex, "HTTP_REQUEST after CLIENT_ACCEPTED");
    assert.ok(responseIndex > requestIndex);
    assert.strictEqual(
      skeleton.match(/when HTTP_RESPONSE \{/g)?.length ?? 0,
      1,
      "HTTP_RESPONSE should appear once",
    );
  });

  test("returns empty string for empty event selection", () => {
    assert.strictEqual(buildIruleEventSkeleton([]), "");
  });

  test("writes only registry catalogue event names, never the request text", () => {
    const result = buildIruleEventSkeletonFor([
      "HTTP_REQUEST",
      "NOT_AN_EVENT",
      "HTTP_REQUEST {}\nwhen RULE_INIT",
    ]);
    assert.ok(result.source.includes("when HTTP_REQUEST {"));
    assert.ok(!result.source.includes("NOT_AN_EVENT"));
    assert.ok(!result.source.includes("when RULE_INIT"), "request text is never interpolated");
    assert.deepStrictEqual(result.rejected, [
      { name: "NOT_AN_EVENT", reason: "unknown" },
      { name: "HTTP_REQUEST {}\nwhen RULE_INIT", reason: "unknown" },
    ]);
    assert.match(describeSkeletonRejections(result.rejected), /NOT_AN_EVENT \(not an iRules event/);
  });

  test("a configured BIG-IP release leaves out events its lifecycle does not declare", () => {
    assert.ok(iruleEventDeclaredAt("XML_BEGIN_DOCUMENT", "9.4.0"));
    assert.ok(!iruleEventDeclaredAt("XML_BEGIN_DOCUMENT", "16.1.0"));
    assert.ok(!iruleEventDeclaredAt("HTTP_REQUEST", "8.4.0"));
    assert.ok(iruleEventDeclaredAt("HTTP_REQUEST", "16.1.0"));

    const current = buildIruleEventSkeletonFor(["HTTP_REQUEST", "XML_BEGIN_DOCUMENT"], {
      bigipVersion: "16.1.0",
    });
    assert.ok(current.source.includes("when HTTP_REQUEST {"));
    assert.ok(!current.source.includes("XML_BEGIN_DOCUMENT"));
    assert.deepStrictEqual(current.rejected, [
      { name: "XML_BEGIN_DOCUMENT", reason: "notAvailable", bigipVersion: "16.1.0" },
    ]);
    const unconfigured = buildIruleEventSkeletonFor(["XML_BEGIN_DOCUMENT"], { bigipVersion: "" });
    assert.ok(unconfigured.source.includes("when XML_BEGIN_DOCUMENT {"));
    assert.deepStrictEqual(unconfigured.rejected, []);
  });

  test("event catalog exposes common HTTP events", () => {
    const eventNames = new Set(COMMON_IRULE_EVENTS.map((entry) => entry.name));
    assert.ok(eventNames.has("HTTP_REQUEST"));
    assert.ok(eventNames.has("HTTP_REQUEST_DATA"));
    assert.ok(eventNames.has("LB_FAILED"));
  });
});

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
import { findGenerateDocstringAction } from "../codeActionKinds";

suite("Code action kinds", () => {
  test("selects the docstring action by kind regardless of display title", () => {
    const translated = {
      title: "Créer une documentation",
      kind: { value: "source.generateDocstring" },
    };
    const misleading = {
      title: "Generate docstring for another feature",
      kind: { value: "quickfix" },
    };

    assert.strictEqual(findGenerateDocstringAction([misleading, translated]), translated);
  });

  test("does not select a missing, generic, or unrelated kind by title", () => {
    const titleOnly = { title: "Generate docstring" };
    const genericSource = { title: "Generate docstring", kind: { value: "source" } };
    const unrelated = {
      title: "Generate docstring",
      kind: { value: "source.organiseImports" },
    };

    assert.strictEqual(
      findGenerateDocstringAction([titleOnly, genericSource, unrelated]),
      undefined,
    );
  });

  test("does not return disabled docstring actions for execution", () => {
    const disabled = {
      title: "Documentation indisponible",
      kind: { value: "source.generateDocstring" },
      disabled: { reason: "No procedure at cursor" },
    };

    assert.strictEqual(findGenerateDocstringAction([disabled]), undefined);
  });
});

#!/usr/bin/env node
// Filter README.md for a single-editor distribution (Node port of the former
// filter_readme.py — the VSIX build is Python-free).
//
// Produces a per-editor README by keeping only sections relevant to the target
// editor and stripping references to other editors. A line-by-line markdown
// parser that understands headings and fenced code blocks (so `#` inside code
// is never mistaken for a heading).
//
// Heading conventions (the source README must follow these):
//   1. Editor subsections under `## Install` (or the older `## Editor support`)
//      are named `### EditorName` (e.g. `### VS Code`). Only the target
//      editor's subsection is kept.
//   2. Sections tagged with `(EditorName ...)` at the end of the heading are
//      kept only for the matching editor, with the parenthetical removed.
//   3. A `### All editors` section (multi-editor only, e.g. the comparison
//      table) is stripped for all single-editor builds. It must sit under the
//      `## Install` heading for that rule to fire.
//   4. Lines containing `<!-- editors:Name1,Name2 -->` are kept only when the
//      target editor appears in the comma-separated list.
//
// Usage:
//   node scripts/install/filter-readme.mjs README.md --editor "VS Code" -o build/README.md
//   node scripts/install/filter-readme.mjs README.md --editor "VS Code"   # stdout

import { readFileSync, writeFileSync } from "node:fs";

const KNOWN_EDITORS = ["VS Code", "Neovim", "Zed", "Emacs", "Helix", "Sublime Text", "JetBrains"];

const HEADING_RE = /^(#{2,4})\s+(.+)$/;
const FENCE_RE = /^(`{3,}|~{3,})/;
const escapeRe = (s) => s.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
const PAREN_EDITOR_RE = new RegExp("\\((" + KNOWN_EDITORS.map(escapeRe).join("|") + ")[^)]*\\)\\s*$");
const INLINE_EDITOR_RE = /<!--\s*editors:\s*([^>]+?)\s*-->/;
const INLINE_EDITOR_RE_G = /<!--\s*editors:\s*([^>]+?)\s*-->/g;

function filterReadme(text, editor) {
  const lines = text.split("\n");
  const out = [];

  let skipUntilLevel = null; // skip until a heading at this level or higher
  let inEditorSupport = false; // inside `## Editor support`
  let inFence = false; // inside a fenced code block
  let fenceMarker = ""; // the opening fence string (to match the closer)

  for (let line of lines) {
    // Track fenced code blocks.
    const fenceMatch = FENCE_RE.exec(line);
    if (fenceMatch) {
      const matched = fenceMatch[1];
      if (!inFence) {
        inFence = true;
        fenceMarker = matched;
      } else if (matched[0] === fenceMarker[0] && matched.length >= fenceMarker.length) {
        inFence = false;
        fenceMarker = "";
      }
    }

    // Inside a fenced code block, skip heading/section logic but still respect
    // inline editor markers and skip state.
    if (inFence && !fenceMatch) {
      if (skipUntilLevel !== null) continue;
      const inline = INLINE_EDITOR_RE.exec(line);
      if (inline) {
        const allowed = inline[1].split(",").map((e) => e.trim());
        if (!allowed.includes(editor)) continue;
        line = line.replace(INLINE_EDITOR_RE_G, "").replace(/\s+$/, "");
      }
      out.push(line);
      continue;
    }

    // Handle headings (only outside code blocks).
    const heading = !inFence ? HEADING_RE.exec(line) : null;
    if (heading) {
      const level = heading[1].length;
      const title = heading[2].trim();

      if (skipUntilLevel !== null) {
        if (level <= skipUntilLevel) {
          skipUntilLevel = null;
        } else {
          continue; // still inside skipped section
        }
      }

      // The section holding the per-editor subsections. Named "Install"
      // since the README restructure; "Editor support" is the older name.
      if (level === 2) inEditorSupport = title === "Install" || title === "Editor support";

      // Rule 3: `### All editors` under `## Editor support` — strip entirely.
      if (inEditorSupport && level === 3 && title === "All editors") {
        skipUntilLevel = level;
        continue;
      }

      // Rule 1: editor subsections under `## Editor support`.
      if (inEditorSupport && level === 3 && KNOWN_EDITORS.includes(title)) {
        if (title !== editor) {
          skipUntilLevel = level;
          continue;
        }
        out.push(line);
        continue;
      }

      // Rule 2: parenthetical editor qualifiers.
      const paren = PAREN_EDITOR_RE.exec(title);
      if (paren) {
        if (paren[1] !== editor) {
          skipUntilLevel = level;
          continue;
        }
        const cleanTitle = title.slice(0, paren.index).replace(/\s+$/, "");
        out.push(`${heading[1]} ${cleanTitle}`);
        continue;
      }

      out.push(line);
      continue;
    }

    // Non-heading lines.
    if (skipUntilLevel !== null) continue;

    // Rule 4: inline editor markers.
    const inline = INLINE_EDITOR_RE.exec(line);
    if (inline) {
      const allowed = inline[1].split(",").map((e) => e.trim());
      if (!allowed.includes(editor)) continue;
      line = line.replace(INLINE_EDITOR_RE_G, "").replace(/\s+$/, "");
    }

    out.push(line);
  }

  // Collapse runs of 3+ blank lines to 2 (single visual gap).
  return out.join("\n").replace(/\n{3,}/g, "\n\n");
}

function main() {
  const argv = process.argv.slice(2);
  let input;
  let editor = "VS Code";
  let output;
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === "--editor") {
      editor = argv[++i];
    } else if (a === "-o" || a === "--output") {
      output = argv[++i];
    } else if (!input) {
      input = a;
    }
  }
  if (!input) {
    process.stderr.write("usage: filter-readme.mjs <input> --editor <name> [-o <output>]\n");
    process.exit(2);
  }
  if (!KNOWN_EDITORS.includes(editor)) {
    process.stderr.write(`unknown editor '${editor}'. valid: ${KNOWN_EDITORS.join(", ")}\n`);
    process.exit(2);
  }
  const result = filterReadme(readFileSync(input, "utf8"), editor);
  if (output) {
    writeFileSync(output, result);
  } else {
    process.stdout.write(result);
  }
}

main();

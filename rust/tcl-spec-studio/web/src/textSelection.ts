// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

/** A textarea selection expressed as UTF-16 offsets. */
export interface TextSelection {
  start: number;
  end: number;
}

const isWhitespace = (character: string): boolean => /\s/u.test(character);

/** The document with formatter-controlled whitespace removed. */
function significantText(text: string): string {
  return text.replace(/\s/gu, "");
}

/** Offset immediately after the first `count` non-whitespace code units. */
function afterSignificant(text: string, count: number): number {
  if (count === 0) return 0;
  let seen = 0;
  for (let at = 0; at < text.length; at += 1) {
    if (!isWhitespace(text[at])) {
      seen += 1;
      if (seen === count) return at + 1;
    }
  }
  return text.length;
}

/** Offset immediately before the non-whitespace code unit after `count`. */
function beforeNextSignificant(text: string, count: number): number {
  let seen = 0;
  for (let at = 0; at < text.length; at += 1) {
    if (isWhitespace(text[at])) continue;
    if (seen === count) return at;
    seen += 1;
  }
  return text.length;
}

/**
 * Keep an offset attached to the same content while a formatter changes whitespace.
 *
 * Formatting a line can insert indentation before the caret. Numeric offsets do
 * not describe that intent, so map through the surrounding pair of significant
 * characters instead. Within a whitespace run, retain affinity to the nearer
 * edge so both "after the token" and "before the next token" remain natural.
 */
function throughWhitespace(source: string, formatted: string, offset: number): number {
  const bounded = Math.max(0, Math.min(offset, source.length));
  let significantBefore = 0;
  for (let at = 0; at < bounded; at += 1) {
    if (!isWhitespace(source[at])) significantBefore += 1;
  }

  const sourceStart = afterSignificant(source, significantBefore);
  const sourceEnd = beforeNextSignificant(source, significantBefore);
  const targetStart = afterSignificant(formatted, significantBefore);
  const targetEnd = beforeNextSignificant(formatted, significantBefore);

  if (bounded <= sourceStart) return targetStart;
  if (bounded >= sourceEnd) return targetEnd;

  const fromStart = bounded - sourceStart;
  const fromEnd = sourceEnd - bounded;
  return fromStart <= fromEnd
    ? Math.min(targetStart + fromStart, targetEnd)
    : Math.max(targetEnd - fromEnd, targetStart);
}

/** Conservative mapping for an unexpected formatter change beyond whitespace. */
function throughChangedText(source: string, formatted: string, offset: number): number {
  const bounded = Math.max(0, Math.min(offset, source.length));
  let prefix = 0;
  while (
    prefix < source.length &&
    prefix < formatted.length &&
    source[prefix] === formatted[prefix]
  ) {
    prefix += 1;
  }

  let suffix = 0;
  while (
    suffix < source.length - prefix &&
    suffix < formatted.length - prefix &&
    source[source.length - suffix - 1] === formatted[formatted.length - suffix - 1]
  ) {
    suffix += 1;
  }

  if (bounded <= prefix) return bounded;
  if (bounded >= source.length - suffix) {
    return formatted.length - (source.length - bounded);
  }
  return Math.min(prefix + (bounded - prefix), formatted.length - suffix);
}

/** Map both ends of a selection through a full-document formatter replacement. */
export function mapSelectionThroughFormat(
  source: string,
  formatted: string,
  selection: TextSelection,
): TextSelection {
  if (source === formatted) return selection;
  const map =
    significantText(source) === significantText(formatted)
      ? (offset: number): number => throughWhitespace(source, formatted, offset)
      : (offset: number): number => throughChangedText(source, formatted, offset);
  return { start: map(selection.start), end: map(selection.end) };
}

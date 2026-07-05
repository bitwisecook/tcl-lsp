// tcl-lsp — a language server and toolchain for Tcl
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Partial / fuzzy / phonetic string matching for the BIG-IP report's global
// search. Substring is kept as the highest-confidence text hit; in-order
// subsequence matching on object names lets `sf` find `vs_safari` and `vs_foo`;
// a phonetic pass (Soundex + a compact Metaphone) catches like-sounding typos.
// Everything is scored so the unified results list can rank hits.

export type MatchKind =
  | "exact"
  | "prefix"
  | "word"
  | "substring"
  | "subsequence"
  | "body"
  | "phonetic"
  | "scope"
  | "none";

export const KIND_SCORE: Record<MatchKind, number> = {
  exact: 100,
  prefix: 85,
  word: 75,
  substring: 60,
  subsequence: 45,
  body: 40,
  phonetic: 30,
  scope: 5,
  none: 0,
};

export interface TextMatch {
  score: number;
  kind: MatchKind;
}

/** Does `hay` contain every character of `needle`, in order (not necessarily
 *  adjacent)? Empty needle matches. Case is assumed already normalised. */
export function subsequenceMatch(hay: string, needle: string): boolean {
  if (!needle) return true;
  let i = 0;
  for (let j = 0; j < hay.length && i < needle.length; j++) {
    if (hay[j] === needle[i]) i++;
  }
  return i === needle.length;
}

/** Split an object name into lowercase alphanumeric tokens
 *  (`vs_safari-01` → ["vs", "safari", "01"]). */
export function nameTokens(name: string): string[] {
  return name
    .toLowerCase()
    .split(/[^a-z0-9]+/)
    .filter(Boolean);
}

// ---- phonetic -------------------------------------------------------------

/** Classic Soundex (letter + 3 digits, zero-padded). Empty for non-alpha. */
export function soundex(word: string): string {
  const s = word.toUpperCase().replace(/[^A-Z]/g, "");
  if (!s) return "";
  const code = (c: string): string => {
    if ("BFPV".includes(c)) return "1";
    if ("CGJKQSXZ".includes(c)) return "2";
    if ("DT".includes(c)) return "3";
    if (c === "L") return "4";
    if ("MN".includes(c)) return "5";
    if (c === "R") return "6";
    return "";
  };
  let out = s[0];
  let prev = code(s[0]);
  for (let i = 1; i < s.length && out.length < 4; i++) {
    const d = code(s[i]);
    // H and W do not reset the "previous code" adjacency rule; vowels do.
    if (d && d !== prev) out += d;
    if (s[i] !== "H" && s[i] !== "W") prev = d;
  }
  return (out + "000").slice(0, 4);
}

/** Compact Metaphone — enough of Lawrence Philips' rules to fold common
 *  spelling variants (ph→f, ck→k, silent leading letters, …) without the full
 *  double-metaphone table. Returns a consonant-ish phonetic key. */
export function metaphone(word: string): string {
  let s = word.toUpperCase().replace(/[^A-Z]/g, "");
  if (!s) return "";
  // Drop silent leading pairs.
  s = s.replace(/^(AE|GN|KN|PN|WR)/, (m) => m[1]);
  if (s.startsWith("X")) s = "S" + s.slice(1);
  s = s.replace(/^WH/, "W");

  const isVowel = (c: string): boolean => "AEIOU".includes(c);
  let out = "";
  for (let i = 0; i < s.length; i++) {
    const c = s[i];
    const next = s[i + 1] || "";
    const prev = s[i - 1] || "";
    if (c === prev && c !== "C") continue; // drop doubled letters (except C)
    switch (c) {
      case "A":
      case "E":
      case "I":
      case "O":
      case "U":
        if (i === 0) out += c; // only leading vowels are kept
        break;
      case "B":
        if (!(i === s.length - 1 && prev === "M")) out += "B";
        break;
      case "C":
        if (next === "H") out += "X";
        else if ("IEY".includes(next)) out += "S";
        else out += "K";
        break;
      case "D":
        out += "T";
        break;
      case "G":
        if (next === "H" && !isVowel(s[i + 2] || "")) break;
        out += "K";
        break;
      case "H":
        if (isVowel(prev) && !isVowel(next)) break;
        out += "H";
        break;
      case "K":
        if (prev !== "C") out += "K";
        break;
      case "P":
        out += next === "H" ? "F" : "P";
        break;
      case "Q":
        out += "K";
        break;
      case "S":
        out += next === "H" ? "X" : "S";
        break;
      case "T":
        out += next === "H" ? "0" : "T";
        break;
      case "V":
        out += "F";
        break;
      case "W":
      case "Y":
        if (isVowel(next)) out += c;
        break;
      case "X":
        out += "KS";
        break;
      case "Z":
        out += "S";
        break;
      default:
        out += c; // F J L M N R
    }
  }
  return out;
}

/** Two words are phonetically close if a Soundex or Metaphone key matches. */
export function phoneticEqual(a: string, b: string): boolean {
  if (a.length < 2 || b.length < 2) return false;
  const sa = soundex(a);
  if (sa && sa === soundex(b)) return true;
  const ma = metaphone(a);
  return !!ma && ma === metaphone(b);
}

// ---- combined scoring -----------------------------------------------------

/** Score a candidate's `name` (identity) and `haystack` (all its indexed
 *  fields) against the query `terms` (already scope-stripped). `terms` is
 *  lowercased by the caller. Returns the best hit; `none` (score 0) = no match.
 *  An empty `terms` means "everything in scope" (e.g. a bare `t0:` query). */
export function scoreText(name: string, haystack: string, terms: string): TextMatch {
  const q = terms.trim();
  if (!q) return { score: KIND_SCORE.scope, kind: "scope" };

  const nameL = name.toLowerCase();
  if (nameL === q) return { score: KIND_SCORE.exact, kind: "exact" };
  if (nameL.startsWith(q)) return { score: KIND_SCORE.prefix, kind: "prefix" };

  const tokens = nameTokens(name);
  if (tokens.some((t) => t.startsWith(q))) return { score: KIND_SCORE.word, kind: "word" };
  if (nameL.includes(q)) return { score: KIND_SCORE.substring, kind: "substring" };
  if (subsequenceMatch(nameL, q)) return { score: KIND_SCORE.subsequence, kind: "subsequence" };

  // Fall back to the object's other indexed fields (address, description,
  // iRule body, data-group records, …) — a real hit, but ranked below name hits.
  if (haystack.includes(q)) return { score: KIND_SCORE.body, kind: "body" };

  // Phonetic: compare the query against each name token (single-word queries
  // only — multi-word phonetic is noise).
  if (!q.includes(" ") && tokens.some((t) => phoneticEqual(t, q))) {
    return { score: KIND_SCORE.phonetic, kind: "phonetic" };
  }
  return { score: 0, kind: "none" };
}

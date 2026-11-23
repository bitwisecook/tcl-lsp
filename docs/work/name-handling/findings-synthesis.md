# What the 199 findings say about the code

**Question:** read all plan items 1–199 together. What do they say about code sprawl, repeated work, hand-rolled parsers and design mistakes, and which structural corrections would retire whole classes of item rather than fixing them one at a time?

**Method:** an independent review classified every item into recurring patterns and derived root causes, then checked the plan for overlaps, missing completion tests and ordering faults. The pattern counts below are from that classification; copy counts are as the items state them.

## Patterns

| Pattern | Items | Examples |
| --- | --- | --- |
| A. Repeated work: one fact computed in several places | 78 | 21, 35, 50, 59, 97, 145, 158, 174 |
| E. Missing or unused registry descriptor, so consumers special-case commands | 51 | 1–17, 80, 153, 158–160, 166–173 |
| J. Plain engine or analyser bugs and missing subsystems | 47 | 18, 26, 30, 137, 153–157, 181 |
| C. Hand-written tables restating registry or upstream data | 40 | 27, 79, 116, 122–130, 147, 185, 188 |
| B. Hand-rolled parsers, scanners, splitters and matchers | 36 | 19, 29, 41, 59, 70, 107, 110, 174, 182, 187 |
| F. Lost information or logic in the wrong layer | 31 | 21, 50, 51, 66, 69, 117, 132, 133 |
| D. Engine duplication without a trait or kernel | 29 | 20, 72, 88, 161, 171, 178, 186 |
| H. Surfaces disagree (LSP, CLI, MCP, explorer) | 21 | 104, 108–110, 113, 144, 146, 148 |
| I. Verification gaps (no oracle drives a consumer; stale tools) | 20 | 25, 75, 131, 198, 199 |
| L. Performance, caching and measurement | 17 | 46–49, 92–95, 101–103 |
| G. Unknown treated as absent; all-or-nothing refusal | 16 | 32, 36, 64, 100, 148, 180 |
| M. Governance, documents and decisions | 10 | 37, 74, 96, 99, 196 |
| K. Code that is not a body unit, so nothing checks it | 8 | 1, 2, 6, 83, 84, 166, 169 |

Patterns A, E and C together cover about 70% of the items. Of the 47 genuine bugs, about 20 are missing descriptors or hand-written tables surfacing in the analyser (3, 5, 9, 12, 14, 45, 173, 197).

## Root causes

1. **The registry describes command syntax, not effects on the name world or on control flow** (1–17, 22, 24, 45, 55, 63, 80, 83, 87, 153, 158–160, 162, 166–173, 176, 179, 180, 184, 189, 192, 197).
   - Every consumer that needs to know that an argument is a script, writes a variable, runs later, suspends, intercepts return codes or is a fixed-signature callback matches command names itself.
   - The items cite `DefinerFamily` matches (86), about 55 `Statement::Catch`/`Try` matchers (174), about eight trace views (158), four deferral facts (169) and `is_unregistered_thread_command` (184).
2. **No per-site record and no query layer** (21, 23, 24, 35, 50, 66–69, 84, 85, 97, 104, 136, 150, 158, 175).
   - The IR carries flat text, so every new feature builds its own resolver.
   - The items cite about 990 lines re-asking the registry by name (50), about 30 positioned query functions (97), 9 frame and 6 namespace choosers (21), and 5 command-identity schemes (35).
3. **Grammars below the lexer are partly owned, and the owners that exist are bypassed** (36 items in pattern B). More bypassed owners exist than missing ones. Host primitives (`split_whitespace`, `trim`, `contains('(')`, `ends_with('\\')`) are always closer to hand than the shared reader. The fault is discoverability and enforcement more than absence.
4. **Release and upstream facts are typed in by hand** (40 items in pattern C). The items cite about 400 `V9_1` branches in 144 files (122), 39 release enums and 28 `format!("tcl{}")` sites (147), two disagreeing TIP 278 facts (191), and hand-written rosters, usage strings, encodings and event orders.
5. **The engines share data, not behaviour** (29 items in pattern D). `tcl-runtime-api` has types but few traits. Frames, traces, interpreters, channels, the event loop, `exec`, coroutines, `try` and start-up are each written twice and drift.
6. **Kernels and answers cannot express partial knowledge** (pattern G, plus 43, 59–61, 81). Analysis cannot call `tcl-cmd-core` on half-known values, so it re-implements folds. A missing "unknown" state becomes silent no-ops (64), whole-scope drops (32) and empty refusals (36, 100, 148).
7. **Identity is carried as strings** (25, 35, 51, 53, 66, 69, 77, 132, 133, 164, 175).
8. **Surfaces own logic instead of adapting one session** (pattern H). Dialect selection, configuration layering, positions, workspace scanning and failure shapes differ between the LSP, the CLI and MCP.
9. **Conformance drives one side only** (pattern I, plus the many "re-run on the current source" caveats). Fixtures exist, but they are not a gate before a consumer migrates, and the MCP tool ran a stale build.

## Hand-rolled parsers

**Exist but bypassed:**
- level words: `tcl_syntax::frame_level` (19, 72);
- variable references: `scan_var_ref` and `split_element_ref_bytes` (70);
- glob and import patterns: `NativeGlobProtocol` (25, 71);
- lists: `tcl-value-syntax::list` (41, 82);
- expression precedence: the `tcl-dialect` table and lowered `tcl-expr` (27, 29);
- numbers: `tcl-numeric` (59, 60);
- constant values: `ExactValue`, which is also owned too high, in `value_transfer` (59);
- versions: the `tclPkg.c` port (31, 147);
- monitor expressions (107);
- URIs: `uri_norm` (152);
- tclsh discovery: `locate_tclsh` (143, 151);
- the `HostError` errno mapping (156);
- comments: `script_comments` (194);
- the regex parse: `tcl-regex` (190);
- partition names: `tcl-bigip::object_names` (111).

**Missing entirely:**
- a purpose-taking writer (39);
- a `TCLLIBPATH` decoder (33);
- option tables and value grammars shared by the registry and `tcl-cmd-core` (15, 44, 87, 128, 188);
- one position and range type (110, 144);
- a waiver matcher (114);
- a docstring parser (149);
- a TSV fixture reader (152);
- a completion-code owner (174);
- a `tclIndex` reader (176);
- an `interp create` reader (179);
- an `exec` pipeline grammar (182);
- a script reader (187);
- a subcommand-prefix rule (71);
- an encoding catalogue (188);
- the BIG-IP name rule (Q8).

## Most repeated facts

1. Release identity and branching (122, 147).
2. Text scanning for nested substitutions: 143 users; expressions re-parsed in about 70 files (82, 29).
3. Role lookup by command name: about 990 lines (50).
4. TSV fixture readers: about 300 in 184 files (152).
5. Completion codes: about 55 matchers and eight word and factory owners (174).
6. "What does this word name here": about 30 queries, 5 identity schemes, 5 rename models (97, 35).
7. `predecessors()` rebuilt on about 18 calls (52).
8. Constants: about 10 types and 3 literal parsers (59).
9. Variable-name text: 3 scanners, 4 splitters, about 10 element tests (70).
10. Frame and namespace choosers: 9 and 6 (21).
11. Writers: about 8 (39).
12. Trace observability views: about 8 (158).
13. Workspace scan policy: 7 consumers (146); presentation inventories: 7 (150).
14. Precedence and version comparators: about 6 each (27, 31).
15. Event-order tables, level-word readers, edit appliers, index grammars, script readers and engine lookup loops: 3–5 each (107, 19, 145, 44, 187, 72).

## Plan quality

- **Overlaps.** These are one piece of work each:
  - Tk `destroy`/`bell` arity: 116, 123, 129;
  - W214 on fixed-signature callbacks: 12, 160, 176;
  - "did you mean `!`": 12, 175, 195;
  - implicit writes reaching W210: 11, 22, 162, 170, 173, 189;
  - body units: 1, 2, 83;
  - nested per-site records: 6, 84, 166;
  - VM silent exits and the runtime library: 30, 38, 155, 177;
  - level grammar: 19, 72, 79;
  - engine traits: 20, 72, 161, 171, 178, 186 under 88;
  - versions: 31 and 147;
  - `tcl_precision`: 63 and 165;
  - positions: 110, 143, 144;
  - tclsh discovery: 143 and 151;
  - refusals with a reason: 36, 100, 105, 148, 180;
  - navigation after `source`: 67, 100, 136;
  - caching: 47–49, 68, 73, 90, 101, 103;
  - BIG-IP event order: 107, 111, Q5;
  - conformance: 25, 28, 37, 65, 131, 198 under 75;
  - gates: 53, 76, 114;
  - rosters: 125, 185, 193.
- **Symptoms that become acceptance tests.** These disappear once their root cause is fixed:
  - Root cause 1: 3, 4, 5, 9, 11, 12, 14, 16, 17, 32, 45 (the W210 part), 160, 162, 166, 167, 170, 173, 175, 189, 197.
  - Root cause 2: 23, 100, 136, 191.
  - Root cause 4: 116, 123–130, 185, 192, which become rows of 131.
  - Root cause 5: 135, 156, 157, 163, 193.

  The real wrong answers still go first: 26, 137, 153–157, 181.
- **No completion test.** Most items before 153 say what to do but not how to tell it is done. Some are grab bags (14, 115, 143, 152) or scope labels (57, 58, 91). Some name no owner crate (40, 109, 113). Some decisions have no owner (38, 142, 196).
- **Ordering:**
  - **Three "first" claims.** The plan's lead paragraphs have three competing "first" claims; [design integration](design-integration.md)'s order is the one to follow.
  - **Unphased items.** Items 107–199 had no phase; they now have one there.
  - **Throwaway work.** 100 needs 67 and 68 (phases 3 and 6), so any earlier version is throwaway.
  - **Quick fixes and the open questions.** 107's quick fixes do not depend on the BIG-IP questions, so the question block should not gate them.
  - **Harness before fixes.** The definitions harness (131) must come before the definition fixes (116, 123–130).
  - **Descriptors first.** Traces and completion codes (158–161, 172–174) need the descriptors of 80 first.

## Corrections

Plan items 200–207 add the structural work that retires these classes:
- effect-descriptor completeness;
- a grammar owner index with a bypass lint;
- generated release facts;
- one engine trait set;
- surfaces as thin session adapters;
- conformance before migration;
- a completion test on every item;
- a merge pass over the overlaps above that keeps item numbers stable.

The [chan design](chan-design.md) (plan items 208–216) is an instance of root causes 1, 3 and 5: three option tables, two engine channel tables with different naming, a shared setter that discards options, and handler protocols described only as "at least two appended words".

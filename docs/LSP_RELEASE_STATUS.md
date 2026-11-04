# LSP release issue sweep

Updated 4 November 2026. Branch: `work`. The LSP is not yet release-ready.

User priority update: the conditional-list S102 follow-up in #940 takes priority.
The user permits evidence comments and closure only for verified fixed issues
authored by bitwisecook. The nine issues #2218, #2254, #2255, #2334, #2351, #2365,
#2434, #2436 and #2440
were verified, commented and closed. All 20 other-reporter issues remain open;
their current assessment is in [LSP_RELEASE_EXTERNAL.md](LSP_RELEASE_EXTERNAL.md).

This sweep follows the user’s LSP/compiler release scope. Unrelated VM, BIG-IP, f5tcl and ancillary feature development is deferred; actual LSP-linked Engine/VM execution and hook confinement remain necessary naming-migration scope. Existing naming-migration acceptance work and its failing gates continue independently; this issue sweep does not certify those gates.

Root integrates reviewed changes every five minutes with normal fetch/merge/push and bounded race-safe retries, without force. Other issue-sweep lanes retain their authorised at-least-15-minute development cadence and pull with rebase before pushing reviewed fixes and status evidence. Development integration may proceed before green tests; required release gates still need to pass on the final source.

Six isolated shallow checkouts use separate Cargo targets. Models: Terra High/Extra High, Sol High/Extra High and Luna High. Tcl 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0 are built from the repository’s pinned references. Latest JimTcl was cloned at `82fb4d0caf90240d94352a31fcbd16adf52658a7` (reports 0.84). Reference provisioning alone certifies no product test.

All naming changes use the shared naming owners. Tcl parsing must use the existing child lexer/parser/segmenter. Original checkout code and pre-existing binaries are stale relative to `work`; only fresh source-bound results count.

| Lane | Status document | Assigned issues |
| --- | --- | --- |
| SERVER | `LSP_RELEASE_SERVER.md` | #1657, #2021, #2295 |
| PROVIDERS | `LSP_RELEASE_PROVIDERS.md` | #1956, #2143, #2254, #2265, #2497 |
| VALUES | `LSP_RELEASE_VALUES.md` | #2131, #2288, #2387, #2427, #2436, #2437, #2438, #2440, #2451, #2452, #2453, #2454, #2498, #2499, #2500, #2503, #2504, #2505, #2506, #2507 |
| BINDINGS | `LSP_RELEASE_BINDINGS.md` | #1778, #1796, #2243, #2246, #2247, #2268, #2286, #2289, #2290, #2294, #2296, #2297, #2305, #2306, #2323, #2324, #2325, #2332, #2367, #2370, #2371, #2374, #2376, #2379, #2407, #2408, #2410, #2412, #2414, #2415, #2441, #2455, #2501, #2502 |
| OPTIMISER | `LSP_RELEASE_OPTIMISER.md` | #1797, #2093, #2216, #2217, #2218, #2231, #2233, #2234, #2248, #2258, #2260, #2261, #2262, #2263, #2264, #2267, #2307, #2308, #2327, #2334, #2339, #2340, #2341, #2347, #2348, #2350, #2351, #2352, #2358, #2373, #2382, #2383, #2384, #2385, #2386, #2390, #2391, #2394, #2396, #2397, #2398, #2399, #2400, #2402, #2403, #2404, #2406, #2411, #2417, #2419, #2423, #2424, #2430, #2431, #2433, #2434, #2439, #2442 |
| PACKS | `LSP_RELEASE_PACKS.md` | #2130, #2137, #2138, #2181, #2242, #2259, #2270, #2299, #2303, #2309, #2365, #2366, #2372, #2422, #2446, #2447, #2448, #2449 |
| COORDINATOR | This document | #1655, #1710, #1712, #1714, #2129, #2183, #2255, #2301, #2426 |

## Current evidence and next actions

Naming acceptance snapshot at14:11UTC: ServerV11 completes26/26 mandatory controls on captured inputs with restored overlays and unchanged captured custody, with32 selections prevalidated. Its sticky Main-currency refusal after bounds-check/Jim source changes prevents current-source acceptance; native protocol workspace-symbol now succeeds, autoload remains null, root references stop at incomplete projection, and all five unchanged existing-library repros still fail. Browser diagnostic [41842464454](https://github.com/bitwisecook/tcl-lsp/actions/runs/41842464454) at `9e986fc8dc9245cb06424b7a6c3ba56b53b9b887` completes failure after a successful build: workspace/symbol positive, autoload null, isolated rename null after successful target checks, bundled-pack1notice. RMWv14 retains its30/31 diagnostic failure; reviewed actual Jim-wrapper repair72bbee3bd awaits RMWv17 after preserved missing-fixture and actual Compiler build failures. Reviewed9a08023b9 adopts the conditional-purpose CompilerV3/CoreHostV2 path; all13 new controls remain unexecuted and fresh RMW builds succeed; RMWv17 completes30/32 (Engine7/7,hooks14/14,VM9/11) with2 Jim control failures retained and original-sequence projection refusal remains open. ClippyV22 predates the latest source; full gates and release acceptance remain open. [Current naming state](work/name-handling/current-state.md) records scoped receipts and retained failures. Finite one-file Browser census setup and opt-in raw pack-notice tracing are adopted, with new Browser41849525146 and manualNative/Browser/WASI41849624576 in progress on01b8c022, no host acceptance yet. Publisher last inspected success14:33:01 at `89936e697`; this is integration evidence only.

- The initial GitHub inventory contains 203 open issues. Assignment does not mean reproduced or fixed; each lane must record its verdict and exact checks.
- The source is a filtered shallow clone. Historical experiment/audit trees are excluded from the working tree; source and regression fixtures remain available through Git when required.
- The first development sync completed at 09:13 UTC, incorporating `961f17a32` and pushing the current head. No product test result is inferred.
- Coordinator #2255: `variable_component` already enables every array-index substitution independently of the enclosing mask. Expanded its existing regression to include Tcl 8.4 and 8.5. All 586 lexer unit tests pass. A fixed native witness agrees on all five Tcl releases and Jim: command and escape substitutions inside the index remain enabled while disabled outside; disabling variables retains the whole literal operand.
- Lexer Clippy is blocked by a pre-existing `assert_is_empty` finding in `source_map.rs:917`; no lint suppression was added. Full repository gates remain pending in the separate gate-fixing work.
- Coordinator #2183: current lexer retains exact inner parse-error terms, and its nested-term/rebasing tests pass in the 586-test run. VM error-info delivery is outside this sweep.
- Coordinator #2436: the analyser now reports E204/E205 for rejected words inside command substitutions through the shared parse-cut owner. Two lexer tests and four production-analyser/reference tests pass, including nested brackets, array indices, multiple commands, duplicate suppression and valid braced-data controls. The reference tests required all five pinned Tcl releases and current JimTcl; source hashes matched before and after. The analyser tests ran through a minimal production-dependency harness containing the committed test file; the full workspace gates remain pending.
- The shared Jim reference is deliberately refreshed to verified upstream HEAD `82fb4d0caf90240d94352a31fcbd16adf52658a7` (3 November 2026), now built with revision-bearing patchlevel `0.84-10-g82fb4d0`. The earlier release-only patchlevel could not satisfy the shared reference validator. The official provisioning helper and the nested parse-cut oracle tests pass with the refreshed build.
- #2436 was closed after its [proof comment](https://github.com/bitwisecook/tcl-lsp/issues/2436#issuecomment-6630162759) was posted and fetched back. The [bounded receipt](LSP_RELEASE_PARSE_CUTS_RECEIPT_2026-11-04.json) records the exact harness, results and six source hashes.
- Targeted production Clippy for `tcl-lexer` and `tcl-compiler` passes with `-D warnings` at `c80aa092d`; this also includes the pushed #2218 optimiser guard. Full workspace gates remain separate and pending.
- #2218 was fixed in `c47d83bda`: retained O115 return/argument normalisation now obeys the shared module command-trust owner. Three focused tests pass, with eight positive and eight negative cases plus equivalent original/rewritten execution on all five Tcl releases and Jim `82fb4d0`. The [proof comment](https://github.com/bitwisecook/tcl-lsp/issues/2218#issuecomment-6630388414) was fetched back before closure.
- #2365 was fixed in `89694c328` and `d58e2952e`: valid stub directives outside a block receive W153 while W123 remains on the unknown call. Clean analyser and Core policy tests, generated-catalogue drift checks and targeted compiler/Core Clippy pass. The new KCS page/index are included; its full index gate remains blocked by 390 unrelated sparse-checkout paths. The [proof comment](https://github.com/bitwisecook/tcl-lsp/issues/2365#issuecomment-6631444932) was fetched back before closure.
- Coordinator #2129: the registry already has a `SOURCE_ENCODING` admissibility axis and declines non-ASCII text where the target’s source reader is unknown. A fresh 11-case native check confirms the Tcl 8.4–8.6 system-encoding distinction, Tcl 9.0–9.1 UTF-8 source reading and current Jim UTF-8 reading; the [receipt](LSP_RELEASE_SOURCE_ENCODING_RECEIPT_2026-11-04.json) retains exact source bytes and launchers. The design example now matches the actual `source_utf8` field and explicitly identifies the missing encoding-profile/decode provenance path. This is scoped reference evidence; #2129 remains open.
- Coordinator #2426: current diagnostics retain typed subjects, and Core includes changed-wording/quoting transport controls. Repository-wide envelope and drift-gate acceptance remain unverified.
- Large design/audit issues #1655, #1710, #1712, #1714 and #2426 require explicit assessment; no blanket completion is claimed.
- #940 native recurrence control: `set x 0; foreach i {1 2 3} {incr x; lindex $x 0}` returns 3 on all five Tcl releases and current Jim. Tcl 8.6, 9.0 and 9.1 additionally expose three integer-to-list conversions on the same object through their representation observer; older Tcl and Jim rows verify output only. The [receipt](LSP_RELEASE_SHIMMER_CYCLE_RECEIPT_2026-11-04.json) retains the script, hash and raw observations. The bounded source-only S102 projection is still pending; this is not a product fix.
- Next: run fresh focused regressions, fix residual defects, preserve positive and refusal cases, update lane evidence, and continue scheduled integration.
- The corrected optimiser differential cohort passes 44/44 on the applicable pinned Tcl releases at `1c15d4ff5`; expanded controls and newer changes require their own results.
- Full locked Cargo metadata succeeds for the LSP server, Spec Studio and Explorer WASM hosts. This verifies those lockfiles, not a WASM binary or editor package build.


## External comment validation completed

All 19 requested external issue comments were posted and verified on GitHub;
#2021 was excluded. [Comment index and outcomes](LSP_RELEASE_EXTERNAL.md).
The comments distinguish passing scoped checks from genuine-control failures.
The source-owner/shimmer gap remains unresolved for #940 and related reports;
no speculative authority or parser workaround was committed.


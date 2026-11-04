# LSP release issue sweep

Updated 4 November 2026. Branch: `work`. The LSP is not yet release-ready.

User priority update: the conditional-list S102 follow-up in #940 takes priority.
The user permits evidence comments and closure only for verified fixed issues
authored by bitwisecook. The six issues #2254, #2255, #2334, #2351, #2434 and #2440
were verified, commented and closed. All 20 other-reporter issues remain open;
their current assessment is in [LSP_RELEASE_EXTERNAL.md](LSP_RELEASE_EXTERNAL.md).

This sweep follows the user’s LSP/compiler release scope. VM, BIG-IP, f5tcl and unrelated tooling work remain outside it. Existing naming-migration acceptance work and its failing gates continue independently; this issue sweep does not certify those gates.

Development integration is authorised at least every 15 minutes regardless of test state. Each lane commits reviewed fixes and status evidence, pulls with rebase and pushes without force. Required release gates still need to pass on the final source.

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

- The initial GitHub inventory contains 203 open issues. Assignment does not mean reproduced or fixed; each lane must record its verdict and exact checks.
- The source is a filtered shallow clone. Historical experiment/audit trees are excluded from the working tree; source and regression fixtures remain available through Git when required.
- The first development sync completed at 09:13 UTC, incorporating `961f17a32` and pushing the current head. No product test result is inferred.
- Coordinator #2255: `variable_component` already enables every array-index substitution independently of the enclosing mask. Expanded its existing regression to include Tcl 8.4 and 8.5. All 586 lexer unit tests pass. A fixed native witness agrees on all five Tcl releases and Jim: command and escape substitutions inside the index remain enabled while disabled outside; disabling variables retains the whole literal operand.
- Lexer Clippy is blocked by a pre-existing `assert_is_empty` finding in `source_map.rs:917`; no lint suppression was added. Full repository gates remain pending in the separate gate-fixing work.
- Coordinator #2183: current lexer retains exact inner parse-error terms, and its nested-term/rebasing tests pass in the 586-test run. VM error-info delivery is outside this sweep.
- Coordinator #2129: the registry already has a `SOURCE_ENCODING` admissibility axis and declines non-ASCII text where the target’s source reader is unknown. Full explicit encoding-profile/decode provenance support still requires assessment; conservative folding alone does not complete the issue.
- Coordinator #2426: current diagnostics retain typed subjects, and Core includes changed-wording/quoting transport controls. Repository-wide envelope and drift-gate acceptance remain unverified.
- Large design/audit issues #1655, #1710, #1712, #1714 and #2426 require explicit assessment; no blanket completion is claimed.
- Next: run fresh focused regressions, fix residual defects, preserve positive and refusal cases, update lane evidence, and continue scheduled integration.
- The corrected optimiser differential cohort passes 44/44 on the applicable pinned Tcl releases at `1c15d4ff5`; expanded controls and newer changes require their own results.
- Full locked Cargo metadata succeeds for the LSP server, Spec Studio and Explorer WASM hosts. This verifies those lockfiles, not a WASM binary or editor package build.


## External comment validation completed

All 19 requested external issue comments were posted and verified on GitHub;
#2021 was excluded. [Comment index and outcomes](LSP_RELEASE_EXTERNAL.md).
The comments distinguish passing scoped checks from genuine-control failures.
The source-owner/shimmer gap remains unresolved for #940 and related reports;
no speculative authority or parser workaround was committed.


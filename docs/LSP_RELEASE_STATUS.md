# LSP release issue sweep

Updated 4 November 2026. Branch: `work`. The LSP is not yet release-ready.

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
- Coordinator #2255: current `variable_component` enables every array-index substitution independently of the enclosing mask and has `outer_substitution_mask_does_not_disable_array_index_commands` coverage. Focused execution remains pending.
- Large design/audit issues #1655, #1710, #1712, #1714 and #2426 require explicit assessment; no blanket completion is claimed.
- Next: run fresh focused regressions, fix residual defects, preserve positive and refusal cases, update lane evidence, and continue scheduled integration.


# Existing inherited Registry method-provider owner migration

Existing baseline: `lsp-completion-catalogue-selected-provider-filter`, remaining `registry_method_items` dependency. This packet makes no baseline or owner-facet count claim; Root reconciles the catalogue.

First refusal: the old caller resolves a genuine factory descriptor, then discards it and passes the receiver package's one version floor to every superclass. Registry's method walk retains actual owning descriptors internally, but the caller cannot supply their independent document package floors. Receiver 1.0 / provider 2.0 incorrectly suppresses a provider method introduced in 2.0; receiver 9.0 / provider 1.0 can incorrectly admit it. No native execution observation is needed for this source ownership mismatch.

Bounded repair: the existing Registry BFS exposes visible `(actual owner descriptor, method)` rows with a per-owner admission/floor callback. Foreign factory identity refuses. Surface, lifecycle, cycle and first-visible-name logic stays centralized. The old uniform-floor API delegates with its original one-floor contract. Only the actual Core `registry_method_items` adopter uses exact context-selected factory + per-provider context availability + DocumentFloor::for_spec.

Outer callback `None` means provider unavailable, inner `None` means admitted with no known version. Neither an unpinned version nor an empty result proves installed/absent methods. The lexical-only completion guard, genuine original-source method-name constructor and actual input/context boundaries remain unchanged. Source rows grant no live receiver, loaded package, construction, dispatch, frame/cell, Native entry, Normal or edit authority.

The three new tests use genuine catalogue descriptors, explicit per-provider floor inputs and actual analysed package-require source; all original test code, fixtures and assertions remain byte-identical. Their selectors are retained in the patched existing proof documents. No tests, Cargo, build, gate or provider executable was run. Rustfmt parsed only the TMP copies; fuzz-zero patch application and exact hashes are structural evidence only.

The signature auto-path prototype is separately withheld at `/tmp/signature-auto-path-target-v1`: it has no production semantic reader and claims no LSP priority, integration or count.

V2 closes independent review findings from V1: provider admission now uses actual ResolvedContext::resolve_spec plus exact pointer correspondence, preserving roster and source-realm selection rather than surface-only availability. The public cross-crate API has an ordinary owner manifest entry; the private same-crate source-route table carries no cross-crate claim. V1 remains immutable.

V3 corrects only the new Jim test's explicit input construction: Jim 0.79 is a projected point under the genuine `jim` environment, not an environment named `jim0.79`. The supported projected_from_point + intern ingress preserves the actual family/release through context_for_profile. Production source and original tests are byte-identical to V2.

# Luna-7 review: Sol-5 `same-document-order-v1.patch`

**Verdict: HOLD.** The design matches decision `export_order`, but two semantic holes must be fixed before admission:

- **H1.** The newly known same-document order switches on the import-conflict rule. That rule then drops an install that real Tcl performs.
- **H2.** The removal guard is open when no query point is given. Exact-import links are therefore revoked for body-local and cross-document consumers, which the patch's own §8.1 says cannot happen.

Several medium soundness and documentation items follow.

## Review basis

- **Patch:** `scratchpad/agents/sol-5/same-document-order-v1.patch`, sha256 `43a5331353a192eaa6a9b3549017395a69602614717b761c5ab568e0abf1bbb5`.
- **Baseline:** the `a/` copies are byte-identical to HEAD `60051c3c2`, and `git apply --check` passes. The working tree is clean.
- **Method:** static review only. Nothing was compiled or run.
- **Probes:**
  - tclsh 8.6 and 9.0 probes are in `scratchpad/agents/luna-7/probe/`. Both versions agree on every probe.
  - Sol-5's `probe.tcl` was re-run on both versions, and all of its claims (A–E) are confirmed.
- **Line references:**
  - `b/...` means the patched copy under `scratchpad/agents/sol-5/b/`.
  - Bare paths mean HEAD.

## Blocking findings

### H1 (HIGH): the conflict rule now drops an install that Tcl performs

How it happens:

1. `navigation_comparison` now returns a known `Less` for two straight-line imports in the same document (`b/rust/tcl-lsp-core/src/source_graph.rs:1337-1349`).
2. That feeds `WorkspaceIndex::conflicting_alias_at` (`rust/tcl-lsp-core/src/workspace_index.rs:8655-8692`). Its liveness check on the earlier alias runs `alias_for_purpose`, which now gates removals through `removal_in_effect` (`b/rust/tcl-lsp-core/src/namespace_import.rs:202-214, 755-763`).
3. A `namespace forget` between the two imports whose position is unknown is therefore treated as *not having happened*. This covers a forget in a `source`d file or in an `if` branch.
4. The earlier alias stays "live", the conflict fires, and the later import installs nothing.

At HEAD the comparison was never known on source navigation, so this conflict never fired.

tclsh 8.6 and 9.0 results (`probe/run3.tcl`):

| shape | Tcl result | patch result |
|---|---|---|
| `import ::A::p` / `source forgetA.tcl` (which forgets `::A::p`) / `import ::B::p` | always `::B::p` | `::A::p`; the B link is dropped |
| `import ::A::p` / `if {$cond} {forget ::A::p}` / `import ::B::p` with cond = 1 | `::B::p` | `::B::p` lost |
| the same with cond = 0 | error; `::A::p` | `::A::p` |

This breaks the family's rule that "an unrankable removal abstains toward answering". It also contradicts §8.1 (`b/docs/design/analysis/import-order-source-graph.md:457`).

**Fix:** in the conflict path, the earlier alias may count as live only when every removal is *known* to fall outside (I1, I2]. Concretely, removals whose position is unknown must count as possibly in effect, through the lenient `in_effect`. Add both probe shapes as controls.

### H2 (HIGH): exact links are revoked when there is no query point

- `removal_in_effect` returns `true` when `query` is `None` (`b/.../namespace_import.rs:210`, `query.is_none_or(..)`).
- `link_alias_live` passes `None` (`workspace_index.rs:8726-8750`), and its result masks every exact import link (`workspace_index.rs:3712`).

The result:

- A straight-line forget or redefinition after a straight-line exact import kills the link for every consumer. That includes calls from a proc body that may run between the two statements, and calls from other documents.
- For the same program written with a glob import, `alias_live_at(.., Some(call))` with a body-local or cross-document call keeps the alias, because the removal's position is Conditional.

So exact and glob imports diverge for identical Tcl, and §8.1:457 ("A body-local or cross-document query therefore still sees every candidate") is false for exact links.

Treating "no query" as "end of load" may be the intended reading: test `an_exact_import_link_dies_on_a_same_file_forget` expects it. But it needs an explicit decision, and it needs either accurate documentation or a consistent rule for both import forms.

## Medium findings

### M1: entering bodies drops HEAD's caller-context refusal and the `Selected` requirement

- At HEAD, the site loop skips every command inside `caller_context_unavailable_bodies`, which includes `namespace eval` bodies (`source_graph.rs:697-704`).
- `DocumentSequence::capture` (`b/source_graph.rs:751-833`) resolves words for those commands and enters their `Sequence`/`ConcatenatedScript` bodies. It does so for any `OriginalRegistrySource`, including Conditional, Vendor, SourceTransitions and ProducedPrefix. It never checks `words.source()` for `Selected`.

tclsh (`probe/shadow.tcl`):

- `namespace eval dst {proc eval args {}}; namespace eval dst {eval {namespace forget ::src::p}}` leaves the alias alive.
- A namespace-local `proc namespace` does the same.

Unless `original_source_registry_barrier` provably blocks these shadows, nested `eval` and `namespace eval` bodies become straight-line, and their forgets revoke.

**Fix:** require `Selected` before entering a body, and either refuse body entry inside caller-context-unavailable bodies or add adversarial controls that prove the barrier holds.

### M2: `uplevel` is not "entered once and immediately"

tclsh results:

- A file sourced at level #0 with top-level `uplevel 1 {..}` fails with `bad level "1"`.
- The same file sourced from a proc runs the body (`probe/uplev.tcl`, `run2.tcl`).

§8.1:452 lists `uplevel`. Gate this on the registry's frame/level selection data rather than admitting every `ConcatenatedScript` (no command name needed).

### M3: early exits are not modelled, although the registry carries the data

- `Traits::TERMINATES_BLOCK` and `TERMINATES_PROCESS` exist (`rust/tcl-registry/src/traits.rs:334,339`) and are set on `return`, `error`, `throw` and `exit`.
- tclsh shows the alias surviving in each of these cases, where the patch would revoke:
  - an unconditional top-level `return` before a forget (`probe/ret.tcl`);
  - a conditional `if {..} return` (`probe/condret.tcl`);
  - errors the analyser itself models:
    - an import conflict (`probe/conflict.tcl`);
    - `namespace export ::q::r`, which the analyser already treats as aborting its own command (`rust/tcl-compiler/src/analyser/handlers.rs:11730-11745`). In Tcl the `-clear` after it never runs (`probe/qual.tcl`).
- §4 of the design doc already lists intra-script abort as a known negative, so the user can accept this.

**Fix:** truncate a straight region's sequence at a command whose registry traits mark it as terminating, and after a command whose nested non-straight region contains one. Otherwise, record explicit acceptance.

### M4: documentation is inaccurate

`docs/design/analysis/import-order-source-graph.md` after the patch:

- `:436` still says `navigation_*` keep Conditional purpose, with no exception.
- `:440` and `:442` say the controls and Definition tests are unchanged, which is now false.
- `:444` says the original-source ownership of both points is checked. The same-document path bypasses `original_point`: it admits unresolved or dynamic heads, gaps between commands, and the document end.
- §8.1:455 says "commands whose words do not resolve are never sequence positions". `contains` (`b/source_graph.rs:835-866`) admits their command word, and NOTES says so.
- §8.1:457 is wrong, per H1 and H2.
- The heading and the code comments carry dated decision ids. That is process narrative, which AGENTS.md requires removing before merge.

Sol-5 named three other docs. All three do need edits:

- **`namespace-original-export-source-advice.md:23-25`:** "chronology assertions also remain unchanged" is now false, and its proof-manifest hash must be updated.
- **`shared-utility-contracts-rust.md:631`:**
  - "conditional results cannot donate ... conflict completion" needs the DocumentSequence exception.
  - The API list should gain `RunOrder::is_source_navigation`.
  - The owner-resolution rows at `:1322-1325` should be checked for the new namespace_import→source_graph edge.
- **`workspace-indexing.md:566`:** minor.

### M5: performance

- **Capture cost:**
  - Assigning each command its owner region costs O(commands × regions) (`b/source_graph.rs:741-746`).
  - Every straight region rescans all commands (`:749-753`).
- **Per-call cost:** `contains` scans linearly, and runs twice per same-URI navigation call on top of `site()`.
- **Per-query recapture:** `definition::in_document_source_order` (`definition.rs:3773`) recaptures for every query.
- **Timing-bounded tests at risk:** `settling_invocations_does_not_scale_with_imports_times_exports` (< 2 s) and test-172 (near 180 s).

**Fix:** sweep the start-sorted `scripts` to assign owners, keep a command list per region, and use binary search in `contains`. Measure before admitting.

## Low findings

- **L1: the cross-document controls are weak.**
  - The new `workspace_order_keeps_cross_document_clear_conditional` (`b/workspace_index.rs:16238`) and the cross-document block in the source_graph control (`b/source_graph.rs:2763-2818`) use unrelated documents. Navigation is therefore `None`, so neither exercises "ordered by the source graph, yet Conditional".
  - Add a variant in which the clearing document `source`s the library. It can keep `is_none_or` while B1 leaves the edge list empty.
  - The unchanged 1b45c540e test `original_source_graph_uses_real_points_and_preserves_cycle_and_re_source_refusal` still pins the cross-document Conditional relation.
- **L2: clippy risks.**
  - The extended source_graph control grows to about 153 code lines. `rust-check` runs clippy with `--all-targets -D warnings`, and `too_many_lines` is pedantic. HEAD has precedents (`completion.rs:7516` at 164 lines, `definition.rs:5101` at 113), so this may not fire, but splitting the test is cleaner.
  - `type CachedRegistryWords = Option<Option<..>>` (`b/source_graph.rs:712`) sidesteps `clippy::option_option`. Prefer a small enum.
- **L3: defence in depth.** `contains` ignores definition-body geometry for the point itself. Retaining `original_definition_body_span` regions at capture, and refusing points inside them, would close the case of a deferred body that SourceStructure did not retain.

## Confirmed sound or compliant

1. **Soundness for a real built-in `namespace eval`:**
   - Nested `namespace eval` bodies are entered straight-line, as Tcl does.
   - `if 1`/`if 0`, `switch`, loops, `catch`, `try`, `dict with`, `apply` and substitutions are withheld. They select Conditional, Alternatives, Loop, Capture, Try, DictionaryScope or Lambda flows, or substitution regions, none of which is ever straight.
   - `Sequence` comes only from `CallerFrameSequence`, which no command authors today.
   - `ConcatenatedScript` requires one complete script word (`source_scoped_body.rs:2001-2013`).
2. **Cross-document pairs:** `document_sequence` returns `None` on a URI mismatch before doing any work (`b/source_graph.rs:1363-1378`). Every cross-document pair then runs the HEAD path unchanged, and SourceHorizon is unchanged.
3. **The guard with a query point:** given `Some(query)`, `removal_in_effect` never revokes for a body-local or cross-document query, and constructed models are untouched. The gaps are only the two above: `None` (H2) and the conflict path (H1).
4. **Compile:** every HEAD API the patch uses exists with a compatible signature and visibility:

   | crate | API |
   |---|---|
   | `tcl-registry` | `script_body_flow_for_invocation` (`pub mod`); `InvocationWords` (`Copy`); `ResolvedInvocation::facts`; `ArgRole::carries_script` |
   | `tcl-compiler` | `OriginalRegistryWords::{roles, operands, with_source_schema, source_script_bodies, matches_source, matches_registry, context}`; `OriginalOperandSource::{span, word}`; `OriginalSourceScriptBody::content_span`; `ResolvedAnalysisInput::{borrowed_context_registry, availability_context, lexer_config}`; `in_effect_within(u32, u32, Option<Span>)`; `SourceNamespaceExport::span` (by value) |
   | `tcl-lexer` | `NativeWord::span` |
   | `tcl-lsp-core` | `load_order(u32, bool)`; `source_contexts: Option<UriMap<SourceOrderContext>>` |

   - Borrows in the new tests are fine.
   - The only remaining doubt is inference of `vec![None; n]` through the slice coercion, which is very likely fine.
5. **Control tests:**
   - Only the same-document shape flips: purpose, `modeled_source_exclusion` and the verdict.
   - Every other original assertion is unchanged, including `has_run`/`cmp_run` returning `None` and the `exported`/`alias_live` checks at `after`.
   - All three 1b45c540e controls would have flipped, so changing all three is necessary. The decision names only the WorkspaceIndex control, though, so the other two need explicit sign-off.
   - The cross-document and branch intent is retained, but weakly (L1).
6. **Other passing tests:**
   - I checked every passing test whose name concerns order in the cp09 `partial-lsp35r2` core run and in luna-workspace `wi-all.txt`, and the import/export/forget tests in definition.rs and references.rs.
   - I found no other pass→fail flip.
   - Constructed `namespace_import::tests` and `source_graph::tests` take the `in_effect` fallback.
   - `exact_import_survives_a_later_export_clear` and `resolve_wildcard_import_survives_a_later_export_clear_in_the_import_file` stay green: the clear is known not yet in effect.
   - `original_export_unknown_operand_blocks_source_absence_and_match_advice` still returns Unknown.
   - `conditional_namespace_forget_preserves_unknowns_and_current_index_owner` is unaffected: the install is cross-document, so `ran_after` stays false.
   - Export and clear rows carry per-token offsets (`handlers.rs:11703-11760`), so no equal-offset self-revocation occurs.
7. **Repository rules:** no `#[allow]` was added, and no non-test code matches on command names. Only `ScriptBodyFlow`/`ArgRole` registry data is used.

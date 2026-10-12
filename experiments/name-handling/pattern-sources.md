# Naming-pattern discovery sources

Use maintained real-world code and the Tcl Wiki to discover familiar naming
idioms, their variants and counterexamples. Source prevalence determines
usefulness; real oracle observations establish runtime semantics. Static
consumer tests establish which conclusions the available evidence permits.

## Existing source owners

| Source | Entry point | Use |
| --- | --- | --- |
| Maintained corpus catalogue | [docs/CORPUS.md](../../docs/CORPUS.md) | Source families, dialects, coding styles and research references |
| Pinned public corpus | [scripts/perf/MANIFEST.toml](../../scripts/perf/MANIFEST.toml), [fetch_corpus.py](../../scripts/perf/fetch_corpus.py) | Authoritative URLs/revisions and verified checkouts; `everything` covers 35 public repositories, `full` covers 21 general Tcl/Tk/EDA repositories and `irules` covers 14 iRules repositories |
| Fetch and workload contract | [performance README](../../scripts/perf/README.md) | Reconstitute existing pins without changing benchmark workloads |
| OO-focused corpus | [experiments/corpus/MANIFEST.md](../corpus/MANIFEST.md) | TclOO projects, tcllib metaobject frameworks and adversarial receiver/dispatch examples; verify actual revisions before using an older fetch result |
| Existing import research | [code-importing-examples.md](../../docs/design/analysis/code-importing-examples.md) | Package/source/autoload/module/alias/wrapping examples and Wiki/manual links |
| Community idioms | [Tcl Wiki](https://wiki.tcl-lang.org/) and Wiki references in the corpus catalogue | Callback/prefix construction, namespace wrappers, aliases, factories, variable linking, object/megawidget patterns and metaprogramming variants |

Use the pinned fetcher for the maintained corpus. Older experiment fetchers
can provide supplemental sources, but a revision mismatch or missing checkout
must be recorded; a warning followed by current HEAD is not the recorded pin.
Keep canonical repository pins in the manifest instead of duplicating them in
new documents. Record the actually used revision in each experimental finding.

```sh
python3 scripts/perf/fetch_corpus.py --list
python3 scripts/perf/fetch_corpus.py --scope everything --dest tmp/name-pattern-corpus
python3 scripts/perf/fetch_corpus.py --scope everything --dest tmp/name-pattern-corpus --verify-only
```

Downloaded third-party trees and generated search indexes stay in ignored
locations. Retain source references, authored minimal probes and oracle results
under the experiment/fixture conventions. Preserve source attribution and
applicable permissions for copied material. Pattern research does not require
changing benchmark pins or regenerating performance results.

## Discovery and validation

1. Record manifest identity, selected scope, verified repository revisions,
   dialects and missing sources. Include old and modern Tcl, packages,
   frameworks and event-driven code rather than relying on one project style.
2. Use `rg` to find candidate idioms, then inspect them through syntax,
   registry roles and execution context. A keyword in braced data is not an
   executed naming operation. Count eligible sites and affected projects,
   deduplicating generated copies and repeated fixtures.
3. Group sites by semantic operation: name construction, command-prefix
   callbacks, namespace capture, alias/import/ensemble lifecycle, variable/frame
   links, source/autoload, finite factories and OO/framework receivers. Keep
   argument positions, namespace/frame, load order and binding assumptions.
4. Use Wiki pages and linked examples to broaden the variants and identify
   historical/version-sensitive behaviour. Record page URL/title, retrieval
   date and revision where available, and a hash of the selected example.
   An explanatory Wiki claim is a hypothesis until checked against the
   relevant real interpreter/context.
5. Reduce an example without removing the condition that makes it interesting:
   preserve scope, list quoting, alias prefix arguments, body execution order,
   mutation and frame reach. Retain an adversarial near miss alongside it.
   Link the reduction to its source location and the original semantic question.
6. Execute the reduced semantic question on the applicable C Tcl matrix,
   latest Jim suite/build, or BIG-IP context. Record dependencies, capability
   absence, acceptance versus execution and raw outcomes. Project code needing
   unavailable packages remains unresolved until its context is established.
7. Test the static model separately: expected proved/conditional identity,
   candidate set or abstention; provenance spans; and permissions for each
   consumer. A real oracle can execute a program without making its target
   statically knowable. Feed recognised effects into shared owners and registry
   contracts rather than private provider-specific matching.
8. Evaluate on held-out projects or source families. Keep close derivatives
   and generated copies in the same group so they cannot inflate coverage.
   Review missed provable sites and wrong targets, and repeat this evaluation
   after changes to recognisers or effect/invalidation rules.

## Finding record

Each candidate pattern records:

- Source catalogue/manifest identity, repository revision, file/line or Wiki
  page/revision/date, and exact input hash.
- Pattern family, concrete variants, representative sites and eligible-site
  denominator, plus breadth across independent projects and dialects.
- Semantic preconditions, binding/value provenance, namespace/frame/interpreter
  context, external dependencies and mutation/invalidation boundaries.
- Positive example, adversarial example and retained real-oracle observations.
- Expected static evidence category and per-surface outcome, including the
  reason for any legitimate abstention.
- Shared semantic owner, registry/summary contract and consumer-test mapping.
- Held-out coverage, false target selections, missed provable targets,
  candidate usefulness and abstention reasons.

Rank work by breadth, prevalence and user impact while preserving correctness.
A frequent pattern can justify investing in a semantic summary, but frequency
does not prove a particular call target. Rename/refactoring and optimisation
still need the proofs in the shared name-handling contract.

This document defines the investigation method. It reports no new corpus
mining counts or Wiki measurements; those belong to retained finding records.

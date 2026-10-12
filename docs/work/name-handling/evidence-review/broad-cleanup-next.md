# Bounded environment/release documentation cleanup

2026-10-12: reviewed three unrelated authored Markdown documents:
`docs/design/contracts/development-environment.md`,
`docs/design/contracts/release-and-publish.md`, and `scripts/perf/README.md`.
Applied two minimal prose corrections; the performance README is unchanged.
No source, registry/F5 evidence, canonical proof, index or root tracking changes.
No Git, network, Cargo, tests or publication actions were executed.

1. `development-environment.md:83–84`, dependency audit guidance:
   removed the completed older-packager/vulnerable-braces incident narrative.
   Retained the current Open VSX / VS Code shared-packager dependency rule.
   Exact evidence: `editors/vscode/package.json:10341` declares `@vscode/vsce`;
   `:10363–10365` overrides `ovsx`'s packager with `$@vscode/vsce`.
   Publisher scripts invoke local committed-lockfile binaries:
   `scripts/release/vsce_publish.sh:75`, `ovsx_publish.sh:80`.

2. `release-and-publish.md:255–257`, release preparation:
   replaced the unconditional claim that preparation stops locally and leaves
   pushing to the maintainer. It now states the default local stop, explicit
   `prepare --push` branch push, and separate PR-opening step.
   Exact evidence: `scripts/release/rust_release.sh::cmd_prepare` parses
   `--push` at `:263–267`, commits at `:287–295`, conditionally pushes at
   `:298–301`, and prints the remaining PR command at `:304–310`.
   This documents existing behaviour; authoring the changelog remains manual.

Kept the performance README's actual benchmark/check order, release procedures,
idempotent steps and retained result provenance. `rust_release.sh::cmd_verify`
(:230–256) rerenders graphs, compares them, and checks notes; the corresponding
documentation is current procedural guidance, not completed implementation chaff.
Environment bootstrap pins also match the inspected installer/session variables.

Validation: all 20 local Markdown file links resolve (4 environment, 15 release,
1 performance); edited prose adds no links and changes no headings/anchors.
Executable behaviour, legal headers, evidence and user procedures are unchanged.
No additional removal candidate was confirmed within these three documents.
The remaining repository-wide currentness audit is still required; compatibility
claims outside the inspected owners are not newly validated by this review.

# Authored source Unicode meaning review (unapplied)

The record `naming.source.original-declared-command-word-contract` contains
three stale ASCII-only descriptions: its scope, one linked assertion, and
the `authored_source_command_candidates` owner purpose. Exact replacements
and preconditions are in [unicode-meaning-review.json](unicode-meaning-review.json).

Propose `Shared bounded UTF-8 source geometry excluding literal NUL` in scope.
The existing candidate-order test's assertion should say `command-slot and
namespace-path projections decline literal NUL and invalid UTF-8`. Its
`name\xff` fixture is invalid UTF-8; it does not establish all non-ASCII refusal.
The owner purpose should also replace ASCII refusal with UTF-8/NUL guards.

`authored_source.rs:9–15` checks valid UTF-8/no literal NUL in written/current
namespace units. Existing positive control `:151–196` retains Unicode, BOM,
constructed components and distinct composed/decomposed bytes; refusal control
`:199–225` rejects invalid written/retained units. All definitions remain linked,
without executed outcomes. Conditional Logical/source geometry supplies no
Native/Normal, installed target, entry, successful publication or edit authority.

No canonical manifest/pages/source changed. Providers, evidence digests, linked
statuses and question/conclusion stay unchanged. No Rust/Cargo/Git, native
execution, full checker or new oracle claim. The original renewal receipt
remains separate and unchanged.

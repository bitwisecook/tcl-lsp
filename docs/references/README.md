# References

Man-page-style canonical reference material for everything a user
touches on the public surfaces of this project.  Each topic has
its own subdirectory so deep-linking and per-topic lookups stay
clean.

Reference docs differ from `docs/design/` in scope:

- **`docs/references/`** — the man pages.  Every function,
  operator, flag, behaviour, and on-disk format a user can
  invoke or read has a stable, anchor-addressable entry here.
- **`docs/design/`** — the engine internals.  Architecture,
  invariants, data-flow, dispatch tables, code organisation —
  the stuff a contributor needs to know but a user doesn't see.

## Topics

- [`f5_query/`](f5_query/) — the `f5 query` DSL: comprehensive
  reference manual, full grammar, every builtin, sample
  configurations, cert-generation one-liners, F5 KB
  cross-references.  Sourced by `f5 query --help-references`.

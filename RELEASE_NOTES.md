# v1.10.10

## New Features

- **W127 — argument value outside a command's closed set.** Some
  command arguments accept only a fixed, exhaustive set of literals
  (e.g. the bareword `HTTP::version` setter takes only `0.9` / `1.0` /
  `1.1`). A literal outside that set is now flagged with the allowed
  values listed in the message. Dynamic values (`$var`, `[cmd]`) and
  option flags are skipped, so `HTTP::version -string $raw` stays
  silent. Marked via a new `FormSpec.closed_value_args` registry
  field — other commands can opt in by listing their closed indices.
- **HTTP/1.x version completions on `HTTP::version`.** The bareword
  setter now offers `0.9`, `1.0`, `1.1` as completions. The
  `-string` form remains unconstrained (HTTP/2 and HTTP/3 live in
  the separate `HTTP2::` / `HTTP3::` namespaces).

## Improvements

- **Bodies of `clientside`, `serverside`, `after`, and `peer` are
  now recursively analysed.** All four iRule commands take an
  optional `NESTING_SCRIPT` but previously declared no `ArgRole.BODY`
  — their script contents were treated as opaque, so nested
  diagnostics, semantic tokens, and scope handling didn't fire
  inside them. Each now carries the correct BodyKind and
  `is_side_switch` flag, with arity tightened from unbounded to
  `(0, 1)` so a stray `clientside a b c` reports E003.
- **`peer` is a first-class side-switch.** It now flips to the
  *opposite* of the current side rather than being treated as
  opaque, so `peer { TCP::collect }` inside a server event correctly
  satisfies a `CLIENT_DATA` payload requirement.
- **W210 (read-before-set) no longer fires inside an existence
  guard.** `info exists X`, `array exists X`, `info vars X` (and the
  `info locals` variant), `[lsearch [info vars] X]`, and
  `catch {set _ $X}` are now recognised as *existence probes*, not
  value reads — and reads of `X` inside the region they prove are
  safe.
- **SCCP folds existence checks both directions.** When the answer
  is statically provable (a local that never persists, or a definite
  assignment / parameter), the I230 diagnostic fires and the dead
  arm is dropped by DCE. `array exists` only folds to false (a
  scalar set is not an array). A conservative per-function gate
  keeps the fold sound: it bails on any barrier / `IRBlock` /
  `IRUpFrame`, any call with an UNKNOWN-target write or inline-body
  argument (e.g. `eval`, `clientside`, …), and excludes array
  elements and qualified names. Resolves #500.
- **Existence-name shape is now derived from the lexer.** The
  legacy `_EXISTENCE_LOCAL_RE` regex was replaced with
  `shared.naming.is_unqualified_var_name`, built on the lexer's
  `is_bare_var_name` rule — single source of truth, no drift, and
  it handles the digit-leading / Unicode names the lexer accepts.

## Bug Fixes

- **Dynamic existence targets are no longer silently exempted.**
  `if {[info exists $name]}` reads `name` to *form* the variable
  name being probed, so the read is real — only literal plain
  scalars are now exempted. `$name`, `A(k)`, `::ns::X` are flagged
  as W210 if they read before set.
- **Nested command substitutions inside an existence guard's value
  or expression are no longer mistaken for absent variables.**
  `set y [set X 1]` and `puts [set X 1]` create a local inside a
  command substitution that has no SSA definition; the folder
  previously treated the resulting `X` as never-set. The
  transparency check now rejects statements whose value / expr /
  args carry a command sub that can create / modify / remove a
  local. `unset` / `array unset` and BODY-running commands are
  treated as mutating; mutation-free subs (`string length`,
  `ILX::call`, …) still fold.

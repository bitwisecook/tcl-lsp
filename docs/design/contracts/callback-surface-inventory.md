# Executable and callback surface inventory

The command registry is the source of truth for every Tcl script, callback,
and command-reference position. `cargo xtask callback-inventory` projects that
metadata into the machine-readable
[`callback-surfaces.json`](../../generated/callback-surfaces.json) inventory
and its human-readable
[`callback-surfaces.md`](../../generated/callback-surfaces.md) report.
`make rust-check` runs the generator in `--check` mode, so a metadata change
cannot silently leave the committed inventory stale.

## What is inventoried

The generator visits every selectable `DialectProfile` plus the additive Tk
profile (Tk is a library surface and intentionally is not selectable on its
own), and records every visible:

- command, subcommand, and object instance-method positional
  `CommandPrefix`, `Body`, or `LambdaLiteral`;
- executable option value, including its exact `ScriptTiming`, appended
  callback arity, and externally supplied callback-taint inputs;
- argument-role, command-prefix, or script-timing resolver. A function pointer
  cannot be enumerated without an invocation, so it receives an explicit
  `dynamic` row instead of being guessed or omitted;
- form/synopsis and lifecycle attached to the owning surface.

Rows whose complete semantics are identical are merged across dialect
profiles. A difference in lifecycle, source, form, timing, arity, or taint
produces a separate row, so dialect-dependent behavior remains explicit.
Command, subcommand, option, and form lifecycles retain their own
package-version axis.

The registry cannot describe a non-Tcl target as a Tcl callback. Audited facts
of that kind live in
[`callback-surface-catalogue.json`](../../references/command-spec/callback-surface-catalogue.json).
The seed currently records `after cancel`'s source-dependent `id|script`
match, plus iRulesLX remote JavaScript method dispatch. `ILX::call` and
`ILX::notify` are intentionally `external-dispatch`, never `CommandPrefix`.
The gate rejects a seed row whose registry owner has disappeared, a duplicate
row, a missing classification, or malformed/unknown fields.

## Adding or changing a surface

1. Put the executable role on `CommandSpec`, `SubCommand`, `OptionSpec`, or the
   object class's `instance_methods`. Use `CommandPrefix` only when Tcl invokes
   the referenced Tcl command; use `Body`/`LambdaLiteral` for executable Tcl.
2. Declare `ScriptTiming`. Use `SameInvocation`, `Deferred`, or
   `ReferenceOnly`; do not infer timing in a compiler or LSP consumer.
3. For `CommandPrefix`, declare the appended arity. Use `Unknown` explicitly
   only when the host's appended argument count cannot be established.
4. Declare only genuinely external callback substitutions in
   `callback_taint_inputs`; framework bookkeeping does not become taint merely
   because a callback can read it.
5. Add lifecycle, dialect, form/synopsis, hover source, and package provenance
   beside the registry declaration.
6. If the target is external or the registry shape cannot represent a proven
   source-dependent union, add a narrowly scoped, sourced seed row. Do not use
   the seed to bypass ordinary registry metadata.
7. Run `cargo xtask callback-inventory`, inspect both generated files, then run
   `cargo xtask callback-inventory --check`.

Changing an executable option to a plain value removes its generated row and
therefore fails the checked-output comparison until the change is explicitly
reviewed and regenerated. Likewise, adding a resolver creates a `dynamic` row;
there is no unclassified resolver fallback.

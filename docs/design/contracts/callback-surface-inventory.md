# Executable and callback surface inventory

The command registry is the source of truth for every Tcl script, callback,
and command-reference position. `cargo xtask callback-inventory` projects that
metadata into the machine-readable
[`callback-surfaces.json`](../../generated/callback-surfaces.json) inventory
and its human-readable
[`callback-surfaces.md`](../../generated/callback-surfaces.md) report.
`make rust-check` runs the generator in `--check` mode, so a metadata change
cannot silently leave the committed inventory stale.

The projection alone is a mirror, and a mirror cannot tell a retired callback
from a lost one: downgrade `fcopy -command` to a plain value and its row simply
disappears, the generator writes the smaller file, and the check passes again.
The second half of the gate is therefore **authored**, in three tiers, and
every projected row must belong to exactly one of them:

| Tier | File | Pins |
|---|---|---|
| `requirements` | [`callback-surface-requirements.json`](../../references/command-spec/callback-surface-requirements.json) | the documented **contract**: kind, timing, appended arity, dialect floor |
| `known_gaps` | the same file | that a documented surface is **not** classified yet, with its evidence and tracking issue |
| baseline | [`callback-surface-baseline.json`](../../references/command-spec/callback-surface-baseline.json) | that the surface **exists** — nothing more |

All three are authored, all three are enforced in `--check` **and** write mode,
and the completeness check ties them to the projection: a row in no tier fails
(a new callback surface arriving unreviewed), and a listed row that stops being
projected fails (the downgrade). That last one is what gives the *whole*
inventory downgrade protection rather than the surfaces someone thought to
document — the review of PR #1727 found `checkbutton -command` could be
downgraded to a plain value with every authored check still green, because no
requirement named it.

Read the tiers as different strengths, not as a ranking of importance. A
baseline row says only "this was an executable position and still is"; what its
kind, timing, or appended arity should be is claimed only by a requirement.
Most of the inventory belongs in the baseline on purpose: the seventeen
`crc::*` `-implementation` options say one thing seventeen times, and writing
seventeen sourced requirements for them would be ceremony rather than review.

## What is inventoried

The generator visits every selectable `DialectProfile` plus the additive Tk
profile (Tk is a library surface and intentionally is not selectable on its
own), and the same profiles again through their **pack-installed** stores — the
EDA vendor libraries ship as bundled `SpecTcl` loadables rather than native
specs, so without that second pass a vendor callback would be invisible to the
audit rather than merely unclassified. It records every visible:

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

## The authored coverage manifest

`callback-surface-requirements.json` has two lists and
`callback-surface-baseline.json` a third. None of them is generated — the
gate only ever reads them, which is exactly why a row vanishing from the
*generated* inventory while still listed here is a failure.

**`requirements`** — one row per documented callback surface that must stay
classified. A row names the surface (`owner` plus `location`, as the inventory
spells them), the `kind`, `timing` and `appended_arity` the registry must
declare, and the `dialects` it must still reach — a *lower bound*, which is
where a version floor lives (`chan push` is 8.6+; `trace vdelete` is gone by
9.0). `source` cites the documentation precisely enough to re-check, `oracle`
records what was measured against a real interpreter or library source, and
`imprecision` records, with its tracking issue, a pinned value that is the
conservative answer rather than the true one — today the positional command
prefixes, whose timing has no per-slot axis and so reports the command-wide
`DEFERS_BODY` answer.

The gate fails when a requirement's row is gone (the downgrade case), when its
kind, timing, or appended arity moved, or when it stopped reaching a dialect it
must. A genuine change is re-pinned in the manifest **in the same commit** as
the registry edit; that edit is the review, and it is why re-running the
generator cannot make a lost callback go quiet.

**`known_gaps`** — documented callback surfaces the registry does *not* classify
yet, each with its evidence and tracking issue, in the shape
`audit-option-dialects`'s `KNOWN_UNSPECIFIED` established. A waiver whose gap
has **closed** fails the gate: once the surface is classified it has to become a
requirement, so a waiver cannot quietly become a parking space. The current
waivers are `http::config -proxyfilter` and `http::register`'s socket-opening
prefix, the flat-table tcllib packages (`websocket::open`/`live`, `ftp::Open`),
and Vivado's `-rule_body` checker procedures.

**the baseline** — `callback-surface-baseline.json`: every remaining projected
surface, by `owner` and `location` alone. It carries no contract and asserts
none; it exists so that the 368 surfaces without a separately citable
contract still cannot lose their declaration unnoticed. Two failures come out
of it: a listed surface that stopped being projected (someone downgraded it),
and a projected surface in no tier at all (a new callback arriving unreviewed,
which is the moment to decide whether it deserves a documented requirement).

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
7. **Account for it.** The gate refuses a projected surface that no tier
   names, so pick one. If the surface has a contract worth citing, add a
   `requirements` row to
   [`callback-surface-requirements.json`](../../references/command-spec/callback-surface-requirements.json)
   naming the classification you just declared, its dialect floor, and the
   documentation that says so; verify the appended arity against a real
   interpreter where Tcl documents one — `tclsh8.4` … `tclsh9.1` are the oracle,
   never memory — and record what you ran in `oracle`. If it is the fifteenth
   member of a family that already has a pinned representative, one line in
   [`callback-surface-baseline.json`](../../references/command-spec/callback-surface-baseline.json)
   is the honest answer. A surface you *cannot* classify at all goes in
   `known_gaps`, with the evidence and a tracking issue.
8. Run `cargo xtask callback-inventory`, inspect both generated files, then run
   `cargo xtask callback-inventory --check`.

Changing an executable option to a plain value removes its generated row, which
fails both the checked-output comparison *and* — the part regeneration cannot
answer — the tier that named it: the requirement if it has one, the baseline
otherwise. Likewise, adding a resolver creates a `dynamic` row; there is no
unclassified resolver fallback.

The halves catch different mistakes, and all of them are needed: the generated
pair notices *any* metadata movement and makes it reviewable in a diff, the
baseline notices a declaration disappearing anywhere in the inventory, and the
requirements notice the movement a diff cannot judge — a callback whose
contract quietly stopped matching what Tcl documents.

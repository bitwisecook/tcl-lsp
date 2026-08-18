# Diagnostic test samples

Minimal reproducible examples for every diagnostic, warning, optimisation,
and shimmer alert the LSP produces. Each example is self-contained and
designed to be testable with tclsh 9.0 where applicable. Tk-based
samples (e.g. W001) require Tk and a working display.

These were initially collected by analysing all 60 `.tcl` files in
[georgtree/SpiceGenTcl](https://github.com/georgtree/SpiceGenTcl).

## Dialect considerations

The LSP supports multiple dialect profiles that affect which commands are
available and which diagnostics fire. Set the dialect with a comment
directive on line 1:

```tcl
# tcl-dialect: tcl9.0
```

Available dialects: `tcl8.4`, `tcl8.5`, `tcl8.6` (default), `tcl9.0`,
`tcl9.1`, `f5-irules`, `f5-iapps`, `f5-tmsh`, `f5-bigip`, `bpf`,
`expect`, `spectcl`, `cadence-eda-tcl`, `intel-quartus-eda-tcl`,
`mentor-eda-tcl`, `microchip-libero-eda-tcl`, `synopsys-eda-tcl`,
`xilinx-eda-tcl`.

Diagnostics affected by dialect are marked with a `[dialect]` tag below.

## Errors

### E003 — Too many arguments

A call supplies more arguments than the resolved target accepts —
generalised beyond the builtin command registry to same-file procs,
`interp alias` (shifted by any prepended arguments), `rename` (inherits
the original's arity unchanged), and `TclOO` methods / `forward`s
(including `forward NAME my TARGET ?ARG…?`, the documented idiom for
forwarding to a sibling or inherited method — a bare method name is never
a valid forward target). The companion **E002** ("too few arguments") is
the same check's other half; no separate sample exists for it since every
case below exercises both directions.

**Validity:** true positive — verified against real tclsh 9.0.4 for
every case in the sample.

## Warnings

### W001 — Unknown subcommand

A subcommand position contains a variable interpolation that cannot be
resolved statically. Often a false positive when the variable holds a
Tk widget path (e.g. `grid ${graph1}`).

**Validity:** false positive for Tk widget paths; true positive when the
value genuinely isn't a valid subcommand.

### W002 — Disabled command in dialect profile `[dialect]`

A command exists in some Tcl versions but is not available in the active
dialect. Key differences:

| Command | Available in | Unavailable in |
|---------|-------------|----------------|
| `oo::configurable` | tcl9.0 | tcl8.4, tcl8.5, tcl8.6 |
| `oo::abstract` | tcl9.0 | tcl8.4, tcl8.5, tcl8.6 |
| `lremove` | tcl9.0 | tcl8.4, tcl8.5, tcl8.6 |
| `open`, `source`, `file`, `socket`, `exit` | tcl8.x, tcl9.0 | f5-irules |
| `dict` | tcl8.5+ | tcl8.4 |
| TclOO (`oo::class`, etc.) | tcl8.6+ | tcl8.4, tcl8.5 |

**Validity:** true positive — the command genuinely won't work in that
dialect. Fix by selecting the correct dialect or guarding with `catch`.

### W101 — eval with substituted arguments

`eval` with variable arguments risks code injection. The substituted
value is re-parsed as Tcl, so `[cmd]` and `$var` inside it execute.

**Validity:** true positive. Use `{*}$cmdList` or a braced body instead.

### W102 — subst with variable argument

`subst` evaluates `$var` and `[cmd]` inside the string. If the content
is untrusted, this is a code injection vector.

**Validity:** true positive. Use `-nocommands -novariables`, `format`,
or `string map` for safe templating.

### W103 — open with variable path

If a filename variable starts with `|`, `open` will execute it as a
command pipeline instead of opening a file.

**Validity:** true positive when the path comes from untrusted input;
false positive when the path is constructed internally.

### W105 — Unbraced eval code block

`eval`'s argument contains `$var` or `[cmd]` without braces. Tcl
substitutes once when building the argument, then `eval` parses again
(double substitution).

**Validity:** true positive. Brace the body or use `{*}`.

### W108 — Non-ASCII character

Characters outside the standard ASCII printable/whitespace range
(e.g. `°`, `—`, `é`). Tcl handles UTF-8 natively so this works at
runtime, but can cause encoding issues across systems.

**Validity:** true positive as a portability warning; no runtime impact
if encoding is consistent.

### W111 — Line exceeds 120 characters

Style warning. No runtime impact.

### W112 — Trailing whitespace

Style hint. No runtime impact.

### W115 — Backslash-newline in comment

A `\` at the end of a comment line causes Tcl to treat the next line as
a continuation of the comment, silently hiding code. This is a common
source of subtle bugs.

**Validity:** true positive. The next line is genuinely swallowed.

### W210 — Variable read before set

A variable is used before any assignment is visible in the current scope.

**Validity:** mixed.
- **True positive:** genuinely unset variable in a proc.
- **False positive:** `lappend` auto-creates the variable (being fixed);
  `$dir` in `pkgIndex.tcl` is set by the package system; variables set
  by `upvar`/`uplevel` from a caller.

### W211 — Variable set but never used

A variable is assigned but never referenced afterward. May indicate dead
code, a typo, or a debugging leftover.

**Validity:** true positive.

### W214 — Unused proc parameter

A proc declares parameters that are never referenced in the body.

**Validity:** true positive. May indicate an incomplete implementation
or wrong signature.

### W220 — Assignment never read

The specific assignment is dead — the value written is never consumed.
Different from W211: the variable might be read elsewhere, but this
particular write is wasted.

**Validity:** true positive.

### W301 — uplevel with unbraced script

Like `eval`, `uplevel` with an unbraced script causes substitution in
the current scope before running in the caller's scope (double
substitution).

**Validity:** true positive. Brace the script body.

### W304 — Missing `--` option terminator

A command that parses leading `-` as options (`lsearch`, `regexp`,
`switch`, `string match`, etc.) receives a substituted argument without
a `--` terminator.

**Validity:** tristate — depends on whether the value can ever start
with `-`:
- **OFF:** value structurally cannot start with `-` (HTTP paths start
  with `/`, SPICE netlist lines start with `.`, simulation vector names
  like `v(node)`, argparse outputs from fixed sets like `{add get set}`).
  Currently a false positive — 19 of 24 sites in SpiceGenTcl.
- **POSSIBLE:** value comes from user input or an unconstrained source.
  Genuine warning — `--` should be inserted.
- **ALWAYS:** value is constructed to start with `-` (e.g. `"-$name"`).
  `--` is mandatory.

The LSP already downgrades to INFO when constant propagation proves the
value is a static literal, but does not yet trace structural constraints
like "starts with `/`" or "from a set that excludes `-` prefixes".

### W306 — Literal expected in regexp pattern `[dialect]`

A regexp pattern contains `$` without braces, meaning the variable is
substituted before `regexp` sees it. The `$` may be intended as an
end-of-line anchor rather than a variable reference.

**Validity:** true positive. Brace regexp patterns: `{pattern$}`.

### W307 — Non-literal command name

A command name is stored in a variable (`$cmd arg1 arg2`). Valid Tcl,
but the LSP cannot determine which command will be called, blocking
static analysis of arguments and side effects.

**Validity:** true positive (valid Tcl, but analysis is limited). Common
in dispatch patterns and OO method calls.

### W313 — file delete with variable path

Deleting files using a variable path without validation could allow path
traversal. The fix is `file normalize` plus a prefix check.

**Validity:** true positive when the path comes from untrusted input.

## Optimisations

### O106 — Loop-invariant expression hoisting

An expression or command call inside a loop produces the same result
every iteration. Hoisting it before the loop avoids redundant work.

**Validity:** true positive. The hoisted expression is demonstrably
invariant.

### O110 — Canonicalise expression (InstCombine)

The optimiser normalises comparison and arithmetic expressions for
better readability or bytecode efficiency (e.g. `{$i<16}` canonical
form).

**Validity:** true positive.

### O116 — Fold constant list command

A `[list ...]` call with all-literal arguments can be replaced with a
braced literal string, avoiding runtime list construction.

**Validity:** true positive. `[list -55 25 85]` becomes `{-55 25 85}`.

## Shimmer warnings

### S101 — Shimmer at merge point

A variable changes internal representation (intrep) between string and
list at a control-flow merge point (e.g. loop exit). Tcl must re-parse
the value, wasting CPU in hot paths.

**Validity:** true positive. Keep variables in one representation
consistently.

### S102 — Type thunking oscillation

A variable alternates between string and list intrep on each loop
iteration. Worse than S101 because it happens every iteration, not just
at boundaries.

**Validity:** true positive. Avoid mixing string concatenation and list
operations on the same variable within a loop.

## Directory layout

```
diagnostics/
├── README.md                          (this file)
├── E003_proc_call_arity/example.tcl
├── O106_loop_invariant/example.tcl
├── O110_canonicalise_expr/example.tcl
├── O116_fold_constant_list/example.tcl
├── S101_shimmer_merge/example.tcl
├── S102_type_thunking/example.tcl
├── W001_unknown_subcommand/example.tcl
├── W002_disabled_command/example.tcl
├── W101_eval_injection/example.tcl
├── W102_subst_injection/example.tcl
├── W103_open_pipeline/example.tcl
├── W105_unbraced_eval/example.tcl
├── W108_non_ascii/example.tcl
├── W111_long_line/example.tcl
├── W112_trailing_whitespace/example.tcl
├── W115_backslash_newline_comment/example.tcl
├── W210_read_before_set/example.tcl
├── W211_set_never_used/example.tcl
├── W214_unused_parameter/example.tcl
├── W220_assignment_never_read/example.tcl
├── W301_uplevel_unbraced/example.tcl
├── W304_missing_double_dash/example.tcl
├── W306_regexp_substitution/example.tcl
├── W307_non_literal_command/example.tcl
└── W313_file_delete_path/example.tcl
```

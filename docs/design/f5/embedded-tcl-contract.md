# F5 embedded Tcl contract

F5 script grammar and command availability are selected independently from
execution context. The `f5-tcl` family describes the embedded Tcl 8.4-derived
fork used by iRules, iApps and tmsh scripts. It does not inherit newer stock
Tcl features from a host-installed interpreter or from a displayed version.

The current implementations are the dialect family's grammar and expression
profiles, the registry's F5 evidence and command surfaces, and the hermetic
lexical vectors. This contract does not make an unobserved appliance build or
execution context equivalent to a supported one.

## Word formation

**R1.** A word beginning with `{` ends at its *matching* close brace; a word
beginning with `"` ends at its closing quote. Brace counting and backslash
escaping are unchanged from Tcl.

**R2.** If the character immediately following that close delimiter is not
whitespace or a command terminator, F5 **ends the word there and begins a new
word** at that character. Stock Tcl raises a parse error.

**R3.** The new word is parsed from scratch under ordinary word-start rules:
a leading `{` makes it brace-quoted (no substitution), a leading `"` makes it
quoted, anything else makes it a **bare word** — in which case any `{` or `}`
inside it are literal characters.

> This is why `}else{log` becomes the single bare word `else{log`, and why the
> `if` branch of `if {0}{a}else{b}` compiles while the `else` branch does not.

**R4.** R2 applies **repeatedly** and in **every word position, including the
command name**. `{set}zz 7` executes `set zz 7`.

**R5.** R2 applies **only** when the word *started* with `{` or `"`. A bare word,
a `$` variable, a `${name}` variable, or a `[` command substitution followed by a
brace is unchanged from Tcl. `a{b}`, `$v{b}`, `${v}b` and `[list a]{b}` all
remain a single word.

**R6.** No diagnostic is emitted. A rule containing `{a}{b}` loads with **zero
warnings**.

**R7.** R1–R6 describe the **script parser only**. The `expr` sub-parser is
*unmodified* and retains the selected expression grammar.


## Brace-line continuation

**N1.** A newline does **not** terminate a command when the next line's first
non-whitespace character is `{`. Such a line is absorbed as further arguments to
the preceding command.

**N2.** N1 is **unconditional**. It does not depend on the command being
incomplete, on the command's identity, or on its arity. Verified: `list a b`
followed by a line `{c}` yields the three-element list `a b c`.

**N3.** N1 applies at **any nesting depth** — inside `when` bodies, `if` bodies,
`proc` bodies.

**N4.** Because the test is purely "does the next line start with `{`", a line
that is **blank, whitespace-only, or a comment** terminates the command normally.
`if {1}` followed by a blank line and then `{body}` fails with
`missing a script after "if"`.

**N5.** `else` / `elseif` are a **separate** lookahead performed by `if` itself:
they are picked up across a single newline (a line starting with `else` is not a
`{` line), but **not** across a blank line, where they fall back to being an
unknown command (`undefined procedure: else`).


## Fork and expression grammar

The script-only delimiter rule does not change expression tokenisation.
The F5 family uses its declared expression operator surface; newer stock Tcl
operators are not enabled by parser permissiveness. In particular, `{*}` is
not argument expansion, and stock Tcl 8.5 command availability must not be
inferred for the embedded fork. The selected registry distinguishes a command
that is absent from one that exists but is refused in an execution context.

## Execution contexts and observations

Traffic event bodies, rule-load initialisation, scriptd, tmsh scripts and host
binaries have independent bindings and capabilities. Procedure declarations,
event selection, runtime state and command availability retain their actual
source and context owners. Host-binary output is not embedded-runtime proof.
Unknown builds and unsupported contexts remain unknown; nearest-known
assistance never supplies execution authority or crosses a context boundary.

Production observation tables are packaged with `tcl-registry`, including
`rust/tcl-registry/data/f5/resolution_contexts/scriptd.txt` and `cli.txt`.
`EvidenceProvenance` retains each observation's probe-set identifier, source
kind and conformance flag. Historical evidence tags such as `§4a` identify
records; they are not links to a development report required at runtime.
The lexical conformance vectors and retained test inputs remain executable
product checks, with their actual bytes and expected outcomes preserved.

## Owners

- [Dialect profiles](../registry/dialect-profile-model.md) select grammar axes.
- [Registry model](../registry/dialect-and-package-registry-redesign.md)
  separates availability, binding and executable backing.
- `rust/tcl-dialect/src/model/family.rs` and `profile.rs` select the F5 family.
- `rust/tcl-registry/src/f5/` owns context-specific evidence and surfaces.
- `rust/tcl-lexer/tests/f5_lexical_conformance.rs` checks word formation.

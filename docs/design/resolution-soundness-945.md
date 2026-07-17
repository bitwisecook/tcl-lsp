# Resolution-model soundness — the issue #945 follow-up

Contract for the resolution surfaces the
[issue #945](https://github.com/bitwisecook/tcl-lsp/issues/945) review
proved unsound and this follow-up replaces.  Companion to
[name-resolution-fix-plan.md](name-resolution-fix-plan.md) (the original
milestone record); C Tcl 9 (tclsh 9.0.4) is the truth oracle for every
behaviour stated here.

## Value provenance (faults 1–2)

A constant-`$cmd` dispatch settles against the compiler's **flow-sensitive
value model**, never a lexical constant map:

- `Statement::AssignConst` carries `value_span: Option<Span>` — the content
  span of the value word when the constant is written verbatim in the
  source.  `None` means *no exact source representation* (a folded or
  desugared constant), and every provenance consumer must abstain from
  writing through it.
- [`tcl_compiler::value_provenance::const_contributors`] answers, for a
  variable use at a program point, the finite set of written constants
  that can reach it: it walks the SSA use-version's reaching definitions
  through φ-joins (`if` / `switch` / loops / `try`) and pure single-`$var`
  copy chains, bottoming out at literal assignments.  Any non-literal
  reaching definition (a computed value, an `upvar`/trace write, a proc
  parameter, an opaque `catch` body) makes the whole site unprovable —
  the sound abstention.
- The settlement (`Analyser::settle_const_dispatches`, in the CFG/SSA
  diagnostic phase where the `CompilationUnit` exists) emits, per resolved
  user-command target: one **indirect** invocation at the `$cmd` head
  (navigation; never rewritten) and one **writable literal-anchored**
  invocation at each contributing definition (the rename edit that keeps
  the dispatch alive).  `SignatureCommandInvocation::rename_safe` is
  `false` when a contributor lacks a writable span; every rename provider
  refuses the whole symbol in that case rather than emit an edit set that
  leaves the dispatch running the old name.

A branch join keeps **every** may-target — `set cmd foo; if {…} {set cmd
bar}; $cmd` references both `::foo` and `::bar`, each with its own
writable literal — and renaming one target rewrites only its own
contributors, keeping both runtime paths correct.

## One-to-many source views (fault 3)

A document sourced under several namespaces is one physical syntax with
**one runtime identity per source-site seed** (`namespace eval ::x
{source b.tcl}` + `namespace eval ::y {source b.tcl}` creates both
`::x::helper` and `::y::helper`).  The server's declaration-side mapping
(`seed_mapped_symbols` / `resolve_workspace_symbols`) returns the **full
identity set**, never an arbitrary first seed; references union every
view's callers, definition dedupes to the physical site, and rename is an
explicit **multi-symbol edit** — the one physical token changes and every
view's call sites follow, with one refusal (collision, unwritable
provenance) aborting all of them.

## TclOO dispatch (faults 4–6)

- The registry owns the visibility semantics:
  `DefinitionBodyGrammar::member_default_exported` is C's
  `PUBLIC_PATTERN` rule (`[a-z]*` — exported iff the first character is
  an ASCII lowercase letter; tclOODefineCmds.c).  The analyser applies it
  at member (re)definition and layers explicit `export` / `unexport`
  (last writer wins; a re-`method` resets to the default — tclsh-pinned).
- The workspace index carries a **typed method table**
  (`WorkspaceMethod`: name, receiver kind, effective export state,
  `private` flag) plus each record's explicit export/unexport deltas.
- [`WorkspaceIndex::method_dispatch_chain`] computes the C-faithful
  chain for a receiver class: the class linearisation (the canonical
  [`tcl_syntax::mro::tcloo_linearise`] — mixins fully linearised first,
  then the class, then superclasses; diamond duplicates keep their late
  placement) filtered by [`MethodAccess`]: `External` (`$obj m`) sees
  exported implementations only; `Internal` (`my m`, declaration-side
  cursors) reaches unexported ones, and `private` definitions only in the
  receiver's own class.  **Go-to-definition returns the chain head** —
  the implementation the call actually enters — never the override
  family; an externally-uncallable method resolves to nothing, mirroring
  C's `unknown method`.  The in-document provider applies the same rule
  through the analyser's own `mro_map`.
- Per-object methods (`oo::objdefine`) key by the receiver's **binding
  identity**: each record carries its objdefine site offset, and the
  lookup matches call sites whose receiver resolves to the same variable
  binding (the innermost proc/method body declaring the name), so two
  unrelated locals both named `o` in different procs never collide.
  Several binding-compatible candidates (an ambiguous reassignment)
  abstain to the class chain.

Rename and references keep the ancestry-closed override-family policy —
a polymorphic name is renamed across the family — which is deliberately
distinct from definition's single-entry policy.

## Interpreter domains (faults 7–8)

The analyser maintains an **interpreter-domain map** driven entirely by
registry hooks (`InterpCreate` / `InterpDelete` / `InterpHide` /
`InterpExpose` / `InterpEval` — no command names in the walker):

- Identity: a literal `interp` path (a Tcl list, relative to the current
  interpreter) keys the domain; paths named inside a child's eval body
  qualify against the enclosing body's path (`interp create t` inside
  `interp eval s {…}` names `s t`).  Evaluation bodies home under the
  synthetic namespace `@interp@<path>` — unrepresentable in Tcl, so a
  real parent namespace of the same name can never collide, and repeated
  evals into the same live interpreter accumulate as in C.
- Temporal identity: `interp delete` bumps the path's **epoch**; a
  re-created interpreter homes under `@interp@<path>#<epoch>` and never
  merges with its predecessor's definitions.
- Existence: an `interp eval` into a literal path never created in the
  file draws **W140** (abstaining when any interp operation used a
  dynamic path).
- Safe visibility: `Traits::SAFE_INTERP_HIDDEN` marks the registry specs
  C hides in a safe interpreter (the non-`CMD_IS_SAFE` set).  A safe
  child's eval body walks under a visibility context; a hidden,
  un-exposed command draws **W129** and is skipped entirely — no
  invocation, no source/package/definition edges, because C raises
  `invalid command name` before any effect.  `interp hide` / `expose`
  layer per-interpreter deltas (a dynamic operand taints the state and
  the gate abstains).
- Cross-domain aliases: `interp alias PATH name TPATH target` records
  the alias under the *source* domain (`::@interp@<path>::name`)
  targeting the *target* domain's command, so child-side calls resolve
  through the ordinary alias links while definitions stay separated.
- Multi-word scripts (`interp eval p w1 w2 …`) concatenate at run time;
  commands can span word boundaries, so the words are **consumed without
  walking** — sound isolation (the previous fall-through analysed them
  in the parent scope).  W312 separately flags the injection-prone shape.

Out of scope here (documented boundary): frame identity inside alias
callbacks and cross-interpreter runtime re-entry live with the VM /
backend work tracked in issue #946.

## Probe references (fault 9)

[`tcl_registry::ArgRole::CommandNameProbe`] is the typed
command-reference role for existence **probes** (`namespace which
-command NAME`; an exact, pattern-free `info commands NAME`): identical
navigation and rename semantics to `CommandName` — a first-class,
exactly-writable reference — with the probe existence policy carried on
the recorded fact (`SignatureCommandInvocation::existence_probe`), which
the W123 unresolved-command pass skips.  A glob pattern names no single
command and abstains.  Reference identity and existence assertion are
orthogonal by construction.

# KCS: Classes and members installed dynamically are missing from the outline

> **Audience:** Contributor
> **Type:** Issue

## Applies to

all-editors, analyser

## Symptom

A class, a method, or a proc that a real interpreter genuinely creates is
absent from the document outline, and go-to-definition, find-references, and
hover all return nothing at its call sites. Sometimes a `W308 Unknown
method` fires on a call that tclsh runs happily, or the outline shows a
nonsense entry named after the unsubstituted source text (`${ptype}`,
`@dynclass@1859`).

All four shapes below come from real corpora (issue #923 audit cluster C3)
and all four run correctly under tclsh 8.6.16 and 9.0.4:

```tcl
# 1. A class factory — a class whose own superclass is oo::class.
oo::class create Megawidget {
    superclass oo::class
    self method create {name superclasses body} {
        next $name [list superclass MegawidgetClass {*}$superclasses]\;$body
    }
}
Megawidget create SimpleWidget {} { method CreateHull {} { my TraceOption a b } }

# 2. A member whose signature arrives through {*} expansion.
oo::define S { method {*}{foo {} {return 1}} }

# 3. A class named by a foreach loop variable over a literal list.
foreach class {chart timeline} { oo::define $class { method RenderTsb {} {…} } }

# 4. A command head built from a namespace held in a variable.
set ns ticklecharts
${ns}::setdef _options id -default $value
```

## Operational context

Each of these is a place where a name is *written* dynamically but is in fact
statically determined. The analyser's job is to tell those apart from names
that genuinely are not knowable until run time, and to abstain only on the
latter.

- **Class factories.** `oo::class`, `oo::abstract`, `oo::configurable`, and
  `oo::singleton` carry `Traits::IS_OO_METACLASS` in the
  [command registry](../design/compiler/command-registry.md). A class whose
  own superclass chain reaches one of them is *also* a class factory —
  that is TclOO language semantics, not per-command knowledge — so
  `Analyser::user_metaclass_of_command` walks the recorded superclass chain
  from the registry seed. Tk's `library/megawidget.tcl` is the canonical
  real user of the idiom.
- **The factory's word layout.** A factory that overrides the manufacturer
  (`self method create {name superclasses body}`) changes where the body
  word sits — argument 3, not the builtin `create Name Body` argument 2 —
  and splices superclasses the caller never wrote. Both facts are read off
  the override's own `next` call, never assumed.
- **`{*}` expansion.** Tcl applies `{*}` while *parsing*, so a `{*}`-marked
  braced literal is not one word but the elements of the list it holds. The
  member grammar's fixed argument layout applies to those elements.
- **`foreach` over a literal list.** The loop variable takes a different
  concrete value each iteration, so a command that installs a name from it
  installs several. Which commands those are is registry data:
  `Traits::INSTALLS_NAMED_DEFINITION`.
- **A composite command head.** The lexer merges `${ns}::setdef` into one
  `Var` token whose text is `ns}::setdef`. The variable's name ends at the
  brace the lexer left in place; everything after is an ordinary word
  suffix.

## Decision rules / contracts

1. **Metaclass-ness propagates; the seed stays in the registry.** Do not
   add a command name to the analyser to recognise a factory. The registry
   trait plus the recorded superclass chain is the whole rule.
2. **Read a factory's argument layout, do not guess it.** A prologue built
   from anything the analyser cannot read yields
   `ClassDef::inheritance_unknown`, which makes W308 abstain the same way an
   out-of-index superclass already does. Recording a guessed superclass
   list is worse than recording none.
3. **Only reference-only members are injected.** `superclass`, `mixin`,
   `filter`, `export`, … (the definition grammar's `all_args_ref` set) name
   existing entities, so each injected word keeps a real source span —
   either in the factory's own body or in the creation call's arguments. A
   *definition* member cannot be injected: it would have no honest span.
4. **`{*}` of a literal splices; `{*}` of a substitution abstains.**
   `method {*}{foo {} {…}}` defines a real `foo`;
   `constructor {*}[info class constructor ::Base]` is equally real but has
   no statically-knowable element list, so the member is left unrecorded
   rather than invented with wrong parameters.
5. **A resolved dynamic head is a reference, never a rename target.** Its
   span spells only part of the name, so the invocation is marked
   `indirect`: find-references reports it, rename skips it. Rewriting the
   span would splice the new name over the substitution — the same
   corruption as
   [the closing-delimiter issue](kcs-issue-highlight-drops-closing-delimiter.md).
6. **A whole-word `$cmd` head belongs to the flow-sensitive engine.** The
   walk must not pre-resolve it from its lexical last-write-wins map; the
   CFG/SSA value model settles it. Only the composite shapes that engine
   skips are folded during the walk.
7. **The `foreach` simulation stays narrow.** It re-dispatches only
   `Traits::INSTALLS_NAMED_DEFINITION` commands, once per literal element.
   Re-walking the whole body per iteration would duplicate every diagnostic
   and scope entry, and there is no fixpoint — the element list is a
   bounded literal.

## File-path anchors

- `rust/tcl-compiler/src/analyser/handlers.rs` —
  `handle_oo_class_command`, `user_metaclass_of_command`,
  `manufacturer_layout`, `manufacturer_word_positions`,
  `manufacturer_injected_members`, `literal_list_words`,
  `handle_oo_define_command`, `handle_foreach_command`,
  `simulate_remaining_foreach_iterations`, `resolve_dynamic_word`
- `rust/tcl-compiler/src/analyser/oo.rs` —
  `splice_static_member_expansions`, `extract_method_def`
- `rust/tcl-compiler/src/analyser/commands.rs` —
  `resolve_dynamic_command_head`, `head_is_whole_word_variable`
- `rust/tcl-compiler/src/analyser/types.rs` —
  `ClassDef::inheritance_unknown`
- `rust/tcl-compiler/src/analyser/diagnostics/var_command.rs` —
  `has_external_super_or_mixin` (the W308 abstention)
- `rust/tcl-registry/src/traits.rs` — `Traits::INSTALLS_NAMED_DEFINITION`,
  `Traits::IS_OO_METACLASS`

## Failure modes

- Assuming the builtin `create Name Body` layout for a factory that
  overrides the manufacturer walks the *superclass* word as a script and
  loses every member of the new class.
- Recording a factory-made class with an empty superclass list when the
  prologue could not be read turns every inherited method into a false
  `W308 Unknown method`. Set `inheritance_unknown` instead.
- Splicing `{*}` over a substituted word invents a parameter list and a body
  span that point at the wrong text — worse than the omission it replaces.
- Folding a whole-word `$cmd` head during the walk makes the walk and the
  flow-sensitive engine disagree about which spans are safe to rewrite, and
  a rename then corrupts the source.
- Reading a composite `Var` token's whole text as a variable name looks up a
  variable that cannot exist, so a statically-resolvable head silently
  resolves to nothing with no diagnostic at all.

## Triage checklist

1. Run the shape under `tclsh9.0` *and* `tclsh8.6` and confirm what the
   interpreter really creates (`info class superclasses`, `info class
   methods -all`, `info class constructor`). The oracle decides, not the
   source text.
2. Check whether the class reached `AnalysisResult::all_classes` at all, and
   under what key — an `@dynclass@<offset>` key means the target word was
   treated as unresolvable.
3. If a factory is involved, check whether its own `ClassDef` was recorded
   *before* the creation call, and whether its superclass chain reaches a
   registry metaclass.
4. If members are missing, check whether the member command's words were
   `{*}`-expanded and whether the expanded word is a braced literal.
5. If a call site resolves to nothing, check whether the head's `Var` token
   text contains a `}` — a composite head whose variable name is the text up
   to that brace.
6. Before claiming a fix, confirm the invocation's `indirect` flag matches
   whether the span really spells the name.

## Test anchors

- `rust/tcl-compiler/tests/analyser.rs` — the `class_factories` module
  (19 TP/TN cases covering all four shapes plus their abstentions)
- `rust/tcl-compiler/src/analyser/handlers.rs` —
  `two_dynamic_oo_define_targets_sharing_a_variable_name_never_merge`,
  `dynamic_oo_define_target_does_not_touch_a_same_named_literal_class`
- `rust/tcl-lsp-core/tests/references_rename.rs` —
  `rename_rewrites_the_defining_literal_and_never_the_dispatch_span_945`

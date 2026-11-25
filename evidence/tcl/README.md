# Tcl execution evidence

One Markdown file per script that actually ran, with the environments it ran in
and what each one returned. The format is defined in
[tcl-execution-evidence.md](../../docs/design/contracts/tcl-execution-evidence.md).

BIG-IP files whose harness is shared link a [fixture](fixtures/README.md) and
record only what the case changes.

```sh
make check-tcl-evidence
python3 scripts/dev/extract-tcl-evidence.py evidence/tcl/<file>.md -o script.tcl
```

[Interpreter limit callbacks use the installing world](interp-limit-callback-installing-world.md) records the global callback entry observed on C Tcl 8.6.18 and 9.0.4.

[Thread script and result-variable worlds](thread-script-and-reply-worlds.md) records source-built Thread 3.0.4 on C Tcl 9.0.4.

[Channel callback protocols](channel-callback-protocols.md) records reflected-handler word counts, the `truncate` release boundary and a transform write-and-pop sequence.

[Inherited stdout transforms](item-14-stdout-observer-entry.md) record how a parent's channel callback can change later lookups in a fresh child on pinned Tcl 8.6, 9.0 and 9.1.

[Procedure body reflection through stdout](optimizer-c14-stdout-reflection.md) records equal numeric output and distinct reflected bodies for a power-to-shift rewrite on C Tcl 8.6, 9.0 and 9.1.

[Procedure call elimination and command counts](constant-call-command-count.md) records equal output and distinct command counts when a caller sources the original and rewritten forms on C Tcl 8.6, 9.0 and 9.1.

[Thread send flag layouts](thread-send-flags-and-surplus-words.md) records combined flags, callback-prefix interpretation and ignored surplus result words in Thread 3.0.4.

[TclOO definition and method activations](tcloo-definition-and-method-activations.md) distinguishes immediate definition namespace entry from a later receiver method.

[TclOO object definition activation](tcloo-object-definition-activation.md) records the standard `::oo::objdefine` namespace separately from method entry.

[Procedure loop-completion conversion order](procedure-loop-completion-order.md) distinguishes direct loop-control errors from codes exposed by `return -code` on C Tcl 8.4–9.1.

[Channel modes and option values](channel-mode-and-option-values.md) records empty creation and event masks, direction-dependent translation checks and direction-independent EOF-character refusals on C Tcl 8.4–9.1.

# Tk and TclOO handoff, 19 November 2026

Paused at the owner's request with the diagnostics worker. No Tk process is running. The shared clean compiler baseline, actual oracle versions, serial-build requirement and all-oracle restart procedure are in [diagnostics.md](diagnostics.md).

## Tk instance factories

The 19 November package-identity ruling requires the actual provider and surviving factory binding at each reaching point. Matching `package provide Tk 8.6; package require Tk` alone can leave `ttk::treeview` unavailable on every Tcl/Jim oracle. A selected document or configured parent entry that installs the original provider remains a positive case. Matching source bytes or a Native compilation entry alone cannot establish the provider. The analyser independently reviewed the allocation-site and package-provider checks. Executed public verification of the new provider code is pending.

The three changes are on the coordinator's local train `c9b656f11e` and were unpublished at the observed remote `b528174d13`. Replay in the order below where the change is absent. These supersede the 18 November package-presence patch. The provider and installed-command crossing must remain separate from deferred configuration/dispatch.

| Patch in patches/ | Source commit | Historical parent | Change |
|---|---|---|---|
| tk-oo-01-instance-provider.patch | cb61c6f542 | 4fe4b5ff0b | Require provider and original factory before reporting the instance. |
| tk-oo-02-matched-installer.patch | 78191c0dc7 | cb61c6f542 | Compare the actual inherited allocation site and installed command; remove the unsupported native-entry shortcut. |
| tk-oo-03-lazy-site.patch | 56504b14cd | 78191c0dc7 | Only acquire dispatch points for package-owned factories. |
| tk-oo-04-verify-only-eval.patch | 500cf463a3 | 5efc2b9cc0 | Separate historical eval WIP with fresh local/global/qualified/trace/input controls. Validate in isolation before publication. The diagnostic W123 patch happened to precede this copy and is independent of its logic. |

The three original areas for formal verification are compiler lib `a_widget_constructor_*` plus bareword/variable/simple widget positives, LSP-core lib `registered_instance_transport_retains_factory_and_applicability_without_message_identity`, and LSP-core it_5 `tk_scale_command_bareword_head_is_a_callback_edge` plus the two `tk_scroll_callback_*` cases. Include the core hover bareword/variable positive endpoints. Preserve positive entry, unbound, fake provide, conditional require, rebinding and same-bytes/different-parent controls. The original-word TK1003 fix is already on the remote. Its six source cases passed a frozen-rlib public replay before pause.

## Guarded TclOO class behind eval

The verify-only commit admits a variable script value to the existing selected native evaluator's delegated script walk, requiring a current variable read and exact frozen argument bytes. The unpatched common compiler `8d6ae9adde` abstains from publishing the class. The analyser independently measured the actual frozen eval operand on this head. Direct global and qualified writes gave `proc M {} {}`; a procedure-local write preserved `set x 1`. This refutes the early stale-value hypothesis at that point. A wrong downstream binding must still be isolated if this candidate publishes a class after a global write.

In the runtime survey C Tcl 8.6.18, 9.0.4 and 9.1.0 created the class after direct, append, uplevel and local-only prefixes. Global, qualified and read-trace prefixes left a procedure. Input selected either result. Stock 8.4.20, 8.5.19 and Jim lacked `::oo::class`. The oracle matrix is in `/workspace/tcl-lsp/tmp/diagnostics-after-8d6ae9adde/guarded-class-oracles.log`, and the analyser probe in `/tmp/bd-stale-guard.log`. Both new tests remain unrun with this WIP. Run compiler it_4 `an_eval_of_a_proved_constant_script_is_not_a_blocker` and `an_eval_of_a_script_the_lattice_cannot_prove_stays_a_blocker`; consult analyser-binding with the exact cases. Do not include this WIP in the ordinary validation train until those cases pass.

The older 18 November ledger still identifies callback, widget and uncalled-method work. Incorrect facts precede missing facts. Parent/child package selection, construction completion, installation of a callback and its later invocation each require their own evidence. Device-dependent iRule/iApp cases go in the BIG-IP ledger.

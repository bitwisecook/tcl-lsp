# naming.namespace.original-counted-namespace-allocation

Kind: `implementation-contract`

## Problem statement

An opaque namespace operand cannot be recreated from a reporting label, and a relative child namespace must retain the actual parent allocation rather than silently using the root namespace.

## Question

Does the original NamespaceEval Ensure transfer retain exact counted path components, actual source allocation lifetimes and the corresponding entered frame?

## Conclusion

The selected C namespace-address purpose consumes the retained whole original operand. Each missing prefix receives an Allocated key at the actual operation source and offset; existing unique keys are reused. Repeated allocation retains a distinct bounded incarnation. The selected frame and ordered source-world geometry consume those same keys. Foreign source, unknown caller geometry, duplicate current paths and command-name-derived qualifiers cannot supply a namespace allocation.

## Scope

Conditional C Tcl source-model NamespaceEval transfer under selected Tcl 8.4, 8.5, 8.6, 9.0 and 9.1 naming policies. Only authentic static whole NamespaceName operands with matching source image, value rules, policy and current source context are admitted. Counted geometry and modeled allocation are separate from native namespace tokens, physical existence, compiler entry, callback closure and Normal completion. Jim, dynamic namespace values and derived rename qualifiers remain outside this allocation issuer. The same NamespaceEval command transfer supplies the variable owner with its actual reached namespace identity. The handoff retains the full original operand, selected NamespaceName role, source site and current namespace; after transfer it rechecks canonical geometry and actual namespace membership, and at consumption it joins the exact operand and site again. It only establishes addressability of that reached identity. It neither closes an unknown cell table nor skips subsequent writes, traces, callbacks, body outcomes or completion checks. The selected namespace member descriptor retains monolithic absent-hook selection for Tcl 8.4 and 8.5. Tcl 8.4 has no public namespace compiler; Tcl 8.5’s monolithic compiler declines eval, inscope and ensemble before preparing operands. Tcl 8.6, 9.0 and 9.1 retain their exact private map and worker identity dependencies. This versioned descriptor supplies no table existence, native registration or successful body completion.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1, jim, bigip

Status: `not-tested`.

No native execution of this Rust implementation question is claimed.

## Source inspection

tcl8.4 8.4.20, revision `Retained release source identified by complete file digest`, `tmp/tcl8.4.20/generic/tclBasic.c`, function `builtInCmds namespace registration`, lines 164–165. Retained reference `namespace-registration-source-0`.

```text
    {"namespace",	(Tcl_CmdProc *) NULL,	Tcl_NamespaceObjCmd,
        (CompileProc *) NULL,		1},

```

tcl8.5 8.5.19, revision `Retained release source identified by complete file digest`, `tmp/tcl8.5.19/generic/tclBasic.c`, function `builtInCmds namespace registration`, lines 157–157. Retained reference `namespace-registration-source-1`.

```text
    {"namespace",	Tcl_NamespaceObjCmd,	TclCompileNamespaceCmd,	1},

```

tcl8.5 8.5.19, revision `Retained release source identified by complete file digest`, `tmp/tcl8.5.19/generic/tclCompCmds.c`, function `TclCompileNamespaceCmd initial decline`, lines 5784–5821. Retained reference `namespace-registration-source-2`.

```text
TclCompileNamespaceCmd(
    Tcl_Interp *interp,		/* Used for error reporting. */
    Tcl_Parse *parsePtr,	/* Points to a parse structure for the command
				 * created by Tcl_ParseCommand. */
    Command *cmdPtr,		/* Points to defintion of command being
				 * compiled. */
    CompileEnv *envPtr)		/* Holds resulting instructions. */
{
    Tcl_Token *tokenPtr, *otherTokenPtr, *localTokenPtr;
    int localIndex, numWords, i;
    DefineLineInformation;	/* TIP #280 */

    if (envPtr->procPtr == NULL) {
	return TCL_ERROR;
    }

    /*
     * Only compile [namespace upvar ...]: needs an odd number of args, >=5
     */

    numWords = parsePtr->numWords;
    if (!(numWords%2) || (numWords < 5)) {
	return TCL_ERROR;
    }

    /*
     * Check if the second argument is "upvar"
     */

    tokenPtr = TokenAfter(parsePtr->tokenPtr);
    if ((tokenPtr->size != 5)  /* 5 == strlen("upvar") */
	    || strncmp(tokenPtr->start, "upvar", 5)) {
	return TCL_ERROR;
    }

    /*
     * Push the namespace
     */

```

tcl8.6 8.6.18, revision `Retained release source identified by complete file digest`, `tmp/tcl8.6.18/generic/tclNamesp.c`, function `namespace implementation map`, lines 141–162. Retained reference `namespace-registration-source-3`.

```text
static const EnsembleImplMap defaultNamespaceMap[] = {
    {"children",   NamespaceChildrenCmd, TclCompileBasic0To2ArgCmd, NULL, NULL, 0},
    {"code",	   NamespaceCodeCmd,	TclCompileNamespaceCodeCmd, NULL, NULL, 0},
    {"current",	   NamespaceCurrentCmd,	TclCompileNamespaceCurrentCmd, NULL, NULL, 0},
    {"delete",	   NamespaceDeleteCmd,	TclCompileBasicMin0ArgCmd, NULL, NULL, 0},
    {"ensemble",   TclNamespaceEnsembleCmd, NULL, NULL, NULL, 0},
    {"eval",	   NamespaceEvalCmd,	NULL, NRNamespaceEvalCmd, NULL, 0},
    {"exists",	   NamespaceExistsCmd,	TclCompileBasic1ArgCmd, NULL, NULL, 0},
    {"export",	   NamespaceExportCmd,	TclCompileBasicMin0ArgCmd, NULL, NULL, 0},
    {"forget",	   NamespaceForgetCmd,	TclCompileBasicMin0ArgCmd, NULL, NULL, 0},
    {"import",	   NamespaceImportCmd,	TclCompileBasicMin0ArgCmd, NULL, NULL, 0},
    {"inscope",	   NamespaceInscopeCmd,	NULL, NRNamespaceInscopeCmd, NULL, 0},
    {"origin",	   NamespaceOriginCmd,	TclCompileNamespaceOriginCmd, NULL, NULL, 0},
    {"parent",	   NamespaceParentCmd,	TclCompileBasic0Or1ArgCmd, NULL, NULL, 0},
    {"path",	   NamespacePathCmd,	TclCompileBasic0Or1ArgCmd, NULL, NULL, 0},
    {"qualifiers", NamespaceQualifiersCmd, TclCompileNamespaceQualifiersCmd, NULL, NULL, 0},
    {"tail",	   NamespaceTailCmd,	TclCompileNamespaceTailCmd, NULL, NULL, 0},
    {"unknown",	   NamespaceUnknownCmd, TclCompileBasic0Or1ArgCmd, NULL, NULL, 0},
    {"upvar",	   NamespaceUpvarCmd,	TclCompileNamespaceUpvarCmd, NULL, NULL, 0},
    {"which",	   NamespaceWhichCmd,	TclCompileNamespaceWhichCmd, NULL, NULL, 0},
    {NULL, NULL, NULL, NULL, NULL, 0}
};

```

tcl9.0 9.0.4, revision `Retained release source identified by complete file digest`, `tmp/tcl9.0.4/generic/tclNamesp.c`, function `namespace implementation map`, lines 159–180. Retained reference `namespace-registration-source-4`.

```text
static const EnsembleImplMap defaultNamespaceMap[] = {
    {"children",   NamespaceChildrenCmd, TclCompileBasic0To2ArgCmd, NULL, NULL, 0},
    {"code",	   NamespaceCodeCmd,	TclCompileNamespaceCodeCmd, NULL, NULL, 0},
    {"current",	   NamespaceCurrentCmd,	TclCompileNamespaceCurrentCmd, NULL, NULL, 0},
    {"delete",	   NamespaceDeleteCmd,	TclCompileBasicMin0ArgCmd, NULL, NULL, 0},
    {"ensemble",   TclNamespaceEnsembleCmd, NULL, NULL, NULL, 0},
    {"eval",	   NamespaceEvalCmd,	NULL, NRNamespaceEvalCmd, NULL, 0},
    {"exists",	   NamespaceExistsCmd,	TclCompileBasic1ArgCmd, NULL, NULL, 0},
    {"export",	   NamespaceExportCmd,	TclCompileBasicMin0ArgCmd, NULL, NULL, 0},
    {"forget",	   NamespaceForgetCmd,	TclCompileBasicMin0ArgCmd, NULL, NULL, 0},
    {"import",	   NamespaceImportCmd,	TclCompileBasicMin0ArgCmd, NULL, NULL, 0},
    {"inscope",	   NamespaceInscopeCmd,	NULL, NRNamespaceInscopeCmd, NULL, 0},
    {"origin",	   NamespaceOriginCmd,	TclCompileNamespaceOriginCmd, NULL, NULL, 0},
    {"parent",	   NamespaceParentCmd,	TclCompileBasic0Or1ArgCmd, NULL, NULL, 0},
    {"path",	   NamespacePathCmd,	TclCompileBasic0Or1ArgCmd, NULL, NULL, 0},
    {"qualifiers", NamespaceQualifiersCmd, TclCompileNamespaceQualifiersCmd, NULL, NULL, 0},
    {"tail",	   NamespaceTailCmd,	TclCompileNamespaceTailCmd, NULL, NULL, 0},
    {"unknown",	   NamespaceUnknownCmd, TclCompileBasic0Or1ArgCmd, NULL, NULL, 0},
    {"upvar",	   NamespaceUpvarCmd,	TclCompileNamespaceUpvarCmd, NULL, NULL, 0},
    {"which",	   NamespaceWhichCmd,	TclCompileNamespaceWhichCmd, NULL, NULL, 0},
    {NULL, NULL, NULL, NULL, NULL, 0}
};

```

tcl9.1 9.1.0, revision `Retained release source identified by complete file digest`, `tmp/tcl9.1.0/generic/tclNamesp.c`, function `namespace implementation map`, lines 159–180. Retained reference `namespace-registration-source-5`.

```text
const EnsembleImplMap tclNamespaceImplMap[] = {
    {"children",   NamespaceChildrenCmd, TclCompileBasic0To2ArgCmd, NULL, NULL, 0},
    {"code",	   NamespaceCodeCmd,	TclCompileNamespaceCodeCmd, NULL, NULL, 0},
    {"current",	   NamespaceCurrentCmd,	TclCompileNamespaceCurrentCmd, NULL, NULL, 0},
    {"delete",	   NamespaceDeleteCmd,	TclCompileBasicMin0ArgCmd, NULL, NULL, 0},
    {"ensemble",   TclNamespaceEnsembleCmd, NULL, NULL, NULL, 0},
    {"eval",	   NamespaceEvalCmd,	NULL, NRNamespaceEvalCmd, NULL, 0},
    {"exists",	   NamespaceExistsCmd,	TclCompileBasic1ArgCmd, NULL, NULL, 0}, // TODO: compile?
    {"export",	   NamespaceExportCmd,	TclCompileBasicMin0ArgCmd, NULL, NULL, 0},
    {"forget",	   NamespaceForgetCmd,	TclCompileBasicMin0ArgCmd, NULL, NULL, 0},
    {"import",	   NamespaceImportCmd,	TclCompileBasicMin0ArgCmd, NULL, NULL, 0},
    {"inscope",	   NamespaceInscopeCmd,	NULL, NRNamespaceInscopeCmd, NULL, 0},
    {"origin",	   NamespaceOriginCmd,	TclCompileNamespaceOriginCmd, NULL, NULL, 0},
    {"parent",	   NamespaceParentCmd,	TclCompileBasic0Or1ArgCmd, NULL, NULL, 0}, // TODO: compile?
    {"path",	   NamespacePathCmd,	TclCompileBasic0Or1ArgCmd, NULL, NULL, 0},
    {"qualifiers", NamespaceQualifiersCmd, TclCompileNamespaceQualifiersCmd, NULL, NULL, 0},
    {"tail",	   NamespaceTailCmd,	TclCompileNamespaceTailCmd, NULL, NULL, 0},
    {"unknown",	   NamespaceUnknownCmd, TclCompileBasic0Or1ArgCmd, NULL, NULL, 0},
    {"upvar",	   NamespaceUpvarCmd,	TclCompileNamespaceUpvarCmd, NULL, NULL, 0},
    {"which",	   NamespaceWhichCmd,	TclCompileNamespaceWhichCmd, NULL, NULL, 0},
    {NULL, NULL, NULL, NULL, NULL, 0}
};

```

## Consumer bindings

- [rust/tcl-compiler/src/command_binding/namespace_slots.rs](../../../../rust/tcl-compiler/src/command_binding/namespace_slots.rs), `ModuleCommandBindings::ensure_original_namespace_key_at`: Canonical NamespaceName address purpose and same-site conditional namespace prefix allocation.
- [rust/tcl-compiler/src/command_binding/original_command_table.rs](../../../../rust/tcl-compiler/src/command_binding/original_command_table.rs), `OriginalCommandOperands::namespace_ensure_input`: Exact selected NamespaceEval whole-operand role; derived command qualifiers remain distinct.
- [rust/tcl-compiler/src/command_binding.rs](../../../../rust/tcl-compiler/src/command_binding.rs), `select_native_frame`: Reuse the independently transferred original namespace key at actual entered body selection.
- [rust/tcl-compiler/src/command_binding/source_command_world.rs](../../../../rust/tcl-compiler/src/command_binding/source_command_world.rs), `CapturedOriginalCommandTransfer::finish`: Retain only the same operation's actual namespace keys and intermediate prefix allocations.
- [rust/tcl-compiler/src/command_binding/original_namespace_ensure.rs](../../../../rust/tcl-compiler/src/command_binding/original_namespace_ensure.rs), `OriginalNamespaceEnsureTransfer::capture`: Retain only the genuine selected NamespaceEval NamespaceName operand and operation source site; no allocation or completion is issued here.
- [rust/tcl-compiler/src/command_binding/original_namespace_ensure.rs](../../../../rust/tcl-compiler/src/command_binding/original_namespace_ensure.rs), `OriginalNamespaceEnsureTransfer::after_command_transfer`: Join the same operation to its actual post-transfer canonical namespace identity; foreign source, missing or unknown geometry refuses.
- [rust/tcl-compiler/src/command_binding/original_namespace_ensure.rs](../../../../rust/tcl-compiler/src/command_binding/original_namespace_ensure.rs), `AppliedOriginalNamespaceEnsures::namespace`: Match the actual reached namespace, original effective operand and operation site at variable transfer.
- [rust/tcl-compiler/src/variable_bindings.rs](../../../../rust/tcl-compiler/src/variable_bindings.rs), `transfer_source_namespace_cells_with_input`: Consume the same-operation original namespace identity through the canonical SourceNamespaceTransfer input, retaining exact script/source order/original operands, allocation-retirement facets and all other cell effects and observer barriers.
- [rust/tcl-registry/src/native_compilation.rs](../../../../rust/tcl-registry/src/native_compilation.rs), `NativeCompilationSpec::implementation_prerequisites`: Selected C8.4/C8.5 monolithic absent-hook registration is independent of the C8.6/C9 private implementation paths; inconsistent body/operation/terminal grammar and unknown engines refuse.
- [rust/tcl-registry/src/commands/tcl/namespace_.rs](../../../../rust/tcl-registry/src/commands/tcl/namespace_.rs), `spec`: Authored namespace eval, inscope and ensemble descriptors use the private namespace_no_hook_compiler macro to retain selected monolithic or exact private-worker registration independently of handler body policy.
- [rust/tcl-compiler/src/command_binding/namespace_slots.rs](../../../../rust/tcl-compiler/src/command_binding/namespace_slots.rs), `command_binding::namespace_slots::tests::original_counted_namespace_allocation_retains_components_and_lifetimes` (linked): Exact opaque and relative path components remain counted; reuse preserves the current key and a subsequent allocation has a different incarnation.
- [rust/tcl-compiler/src/command_binding/namespace_slots.rs](../../../../rust/tcl-compiler/src/command_binding/namespace_slots.rs), `command_binding::namespace_slots::tests::original_counted_namespace_frame_uses_relative_original_children` (linked): The actual entered namespace frame retains the opaque parent and its relative child without creating a root child.
- [rust/tcl-compiler/src/command_binding/namespace_slots.rs](../../../../rust/tcl-compiler/src/command_binding/namespace_slots.rs), `command_binding::namespace_slots::tests::original_counted_namespace_allocation_refuses_foreign_unknown_and_duplicate_geometry` (linked): Foreign original source, unknown current caller geometry and conflicting current namespace lifetimes refuse allocation.
- [rust/tcl-compiler/src/command_binding/original_namespace_ensure.rs](../../../../rust/tcl-compiler/src/command_binding/original_namespace_ensure.rs), `command_binding::original_namespace_ensure::tests::original_namespace_ensure_shares_actual_allocations_without_widening_empty_cells` (linked): Five selected C source contexts retain actual opaque parent and relative child allocations in the variable addressability table without losing previously closed empty cells or inventing traces; original procedure publication remains independently completed.
- [rust/tcl-compiler/src/command_binding/original_namespace_ensure.rs](../../../../rust/tcl-compiler/src/command_binding/original_namespace_ensure.rs), `command_binding::original_namespace_ensure::tests::original_namespace_ensure_does_not_close_unknown_operands_or_body_observers` (linked): Dynamic namespace operands, unknown bodies and unmodelled callbacks cannot obtain a completed original command world from namespace allocation correspondence.
- [rust/tcl-registry/src/native_compilation.rs](../../../../rust/tcl-registry/src/native_compilation.rs), `native_compilation::tests::original_namespace_no_hook_members_select_monolithic_and_private_registration_epochs` (linked): All five selected C points preserve absent-hook admission from authentic complete source words; older releases have no private worker, modern releases require their single exact path, and unknown/Jim descriptor queries decline.
- [rust/tcl-registry/src/native_compilation.rs](../../../../rust/tcl-registry/src/native_compilation.rs), `native_compilation::tests::no_hook_implementation_paths_keep_direct_body_and_reject_inconsistent_descriptors` (linked): A descriptor without the explicit earlier monolithic premise still refuses the older path; a present-hook terminal cannot opt into the absent-hook premise; mismatched body and Jim remain unavailable.

## Evidence

- [rust/tcl-registry/tests/data/native_namespace_source_controls/namespace-compilation-source-anchors.json](../../../../rust/tcl-registry/tests/data/native_namespace_source_controls/namespace-compilation-source-anchors.json)

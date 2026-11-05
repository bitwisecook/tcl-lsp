# naming.procedure.compiled-local-resolver-formal-exclusion

Kind: `source-anchor`

## Problem statement

A procedure formal slot and an unnamed temporary cannot be treated as an ordinary named local merely because an original namespace compiled-variable resolver is present. Exact provider source guards are needed independently of public TclOO links or software formal-cell ownership.

## Question

Which local-slot flags do the pinned C compiled-local resolver initialisation source guards exclude before selecting a namespace or interpreter compiled-variable resolver?

## Conclusion

The five inspected original C implementations exclude VAR_ARGUMENT and VAR_TEMPORARY from the shown compiled-variable resolver selection. C8.4 tclCompile.c also excludes VAR_RESOLVED in that selection guard. C8.5/C8.6 TclInitCompiledLocals and C9.0/C9.1 InitResolvedLocals in tclProc.c clear the cached resolved flag in the shown path, then requires haveResolvers and absence of VAR_ARGUMENT|VAR_TEMPORARY before selecting namespace/interpreter compiled-variable resolver procedures. These are exact source-branch facts: they establish neither a reached resolver invocation nor physical formal/local-cell identity, bytecode selection, runtime shadowing, entered frame or Native compiler admission. The public TclOO const-link question and entered software formal-cell owner remain separate purposes. No Jim or BIG-IP source/execution observation is supplied.

## Scope

Read-only inspection retains five whole original provider source files, exact forty-line guard/resolver excerpts, original paths/ranges and full-source/excerpt SHA256. C8.4 uses tclCompile.c; the four other C releases use tclProc.c. The independently retained request is unchanged. No native/compiler/Rust process is launched, no existing provider/source hash is refreshed and no executed API control is inferred. Jim/BIG-IP and dynamic resolver/current-cell behaviour are not tested.

## Provider answers

### tcl8.4

Status: `inspected`. Channel: Read-only whole source and exact forty-line resolver guard excerpt. Version: tcl8.4 8.4.20.

Inspected selector excludes VAR_ARGUMENT|VAR_TEMPORARY|VAR_RESOLVED. Source-branch fact only; no reached resolver, physical slot, runtime shadowing or Native admission observation.

### tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `inspected`. Channel: Read-only whole source and exact forty-line resolver guard excerpt. Version: tcl8.5 8.5.19; tcl8.6 8.6.18; tcl9.0 9.0.4; tcl9.1 9.1.0.

Inspected path clears VAR_RESOLVED and excludes VAR_ARGUMENT|VAR_TEMPORARY when haveResolvers before namespace/interpreter compiled-variable resolver selection. Source-branch fact only; no reached resolver, physical slot, runtime shadowing or Native admission observation.

### jim, bigip

Status: `not-tested`.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

## Source inspection

tcl8.4 8.4.20, revision `Pinned complete original provider source; read-only inspection`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclCompile.c`, function `TclInitCompiledLocals`, lines 2348–2387. Retained reference `compiled-local-formal-8.4.20-tclCompile.c`.

```text
    for (localPtr = framePtr->procPtr->firstLocalPtr;
	 localPtr != NULL;
	 localPtr = localPtr->nextPtr) {

	/*
	 * Check to see if this local is affected by namespace or
	 * interp resolvers.  The resolver to use is cached for the
	 * next invocation of the procedure.
	 */

	if (!(localPtr->flags & (VAR_ARGUMENT|VAR_TEMPORARY|VAR_RESOLVED))
		&& (nsPtr->compiledVarResProc || iPtr->resolverPtr)) {
	    resPtr = iPtr->resolverPtr;

	    if (nsPtr->compiledVarResProc) {
		result = (*nsPtr->compiledVarResProc)(nsPtr->interp,
			localPtr->name, localPtr->nameLength,
			(Tcl_Namespace *) nsPtr, &vinfo);
	    } else {
		result = TCL_CONTINUE;
	    }

	    while ((result == TCL_CONTINUE) && resPtr) {
		if (resPtr->compiledVarResProc) {
		    result = (*resPtr->compiledVarResProc)(nsPtr->interp,
			    localPtr->name, localPtr->nameLength,
			    (Tcl_Namespace *) nsPtr, &vinfo);
		}
		resPtr = resPtr->nextPtr;
	    }
	    if (result == TCL_OK) {
		localPtr->resolveInfo = vinfo;
		localPtr->flags |= VAR_RESOLVED;
	    }
	}

	/*
	 * Now invoke the resolvers to determine the exact variables that
	 * should be used.
	 */

```

tcl8.5 8.5.19, revision `Pinned complete original provider source; read-only inspection`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclProc.c`, function `TclInitCompiledLocals`, lines 1265–1304. Retained reference `compiled-local-formal-8.5.19-tclProc.c`.

```text
	    if (localPtr->resolveInfo->deleteProc) {
		localPtr->resolveInfo->deleteProc(localPtr->resolveInfo);
	    } else {
		ckfree((char *) localPtr->resolveInfo);
	    }
	    localPtr->resolveInfo = NULL;
	}
	localPtr->flags &= ~VAR_RESOLVED;

	if (haveResolvers &&
		!(localPtr->flags & (VAR_ARGUMENT|VAR_TEMPORARY))) {
	    ResolverScheme *resPtr = iPtr->resolverPtr;
	    Tcl_ResolvedVarInfo *vinfo;
	    int result;

	    if (nsPtr->compiledVarResProc) {
		result = (*nsPtr->compiledVarResProc)(nsPtr->interp,
			localPtr->name, localPtr->nameLength,
			(Tcl_Namespace *) nsPtr, &vinfo);
	    } else {
		result = TCL_CONTINUE;
	    }

	    while ((result == TCL_CONTINUE) && resPtr) {
		if (resPtr->compiledVarResProc) {
		    result = (*resPtr->compiledVarResProc)(nsPtr->interp,
			    localPtr->name, localPtr->nameLength,
			    (Tcl_Namespace *) nsPtr, &vinfo);
		}
		resPtr = resPtr->nextPtr;
	    }
	    if (result == TCL_OK) {
		localPtr->resolveInfo = vinfo;
		localPtr->flags |= VAR_RESOLVED;
	    }
	}
    }
    localPtr = firstLocalPtr;
    codePtr->flags &= ~TCL_BYTECODE_RESOLVE_VARS;
    goto doInitResolvedLocals;

```

tcl8.6 8.6.18, revision `Pinned complete original provider source; read-only inspection`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclProc.c`, function `TclInitCompiledLocals`, lines 1214–1253. Retained reference `compiled-local-formal-8.6.18-tclProc.c`.

```text
	    if (localPtr->resolveInfo->deleteProc) {
		localPtr->resolveInfo->deleteProc(localPtr->resolveInfo);
	    } else {
		ckfree(localPtr->resolveInfo);
	    }
	    localPtr->resolveInfo = NULL;
	}
	localPtr->flags &= ~VAR_RESOLVED;

	if (haveResolvers &&
		!(localPtr->flags & (VAR_ARGUMENT|VAR_TEMPORARY))) {
	    ResolverScheme *resPtr = iPtr->resolverPtr;
	    Tcl_ResolvedVarInfo *vinfo;
	    int result;

	    if (nsPtr->compiledVarResProc) {
		result = nsPtr->compiledVarResProc(nsPtr->interp,
			localPtr->name, localPtr->nameLength,
			(Tcl_Namespace *) nsPtr, &vinfo);
	    } else {
		result = TCL_CONTINUE;
	    }

	    while ((result == TCL_CONTINUE) && resPtr) {
		if (resPtr->compiledVarResProc) {
		    result = resPtr->compiledVarResProc(nsPtr->interp,
			    localPtr->name, localPtr->nameLength,
			    (Tcl_Namespace *) nsPtr, &vinfo);
		}
		resPtr = resPtr->nextPtr;
	    }
	    if (result == TCL_OK) {
		localPtr->resolveInfo = vinfo;
		localPtr->flags |= VAR_RESOLVED;
	    }
	}
    }
    localPtr = firstLocalPtr;
    codePtr->flags &= ~TCL_BYTECODE_RESOLVE_VARS;

```

tcl9.0 9.0.4, revision `Pinned complete original provider source; read-only inspection`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclProc.c`, function `InitResolvedLocals`, lines 1188–1227. Retained reference `compiled-local-formal-9.0.4-tclProc.c`.

```text
	    if (localPtr->resolveInfo->deleteProc) {
		localPtr->resolveInfo->deleteProc(localPtr->resolveInfo);
	    } else {
		Tcl_Free(localPtr->resolveInfo);
	    }
	    localPtr->resolveInfo = NULL;
	}
	localPtr->flags &= ~VAR_RESOLVED;

	if (haveResolvers &&
		!(localPtr->flags & (VAR_ARGUMENT|VAR_TEMPORARY))) {
	    ResolverScheme *resPtr = iPtr->resolverPtr;
	    Tcl_ResolvedVarInfo *vinfo;
	    int result;

	    if (nsPtr->compiledVarResProc) {
		result = nsPtr->compiledVarResProc(nsPtr->interp,
			localPtr->name, localPtr->nameLength,
			(Tcl_Namespace *) nsPtr, &vinfo);
	    } else {
		result = TCL_CONTINUE;
	    }

	    while ((result == TCL_CONTINUE) && resPtr) {
		if (resPtr->compiledVarResProc) {
		    result = resPtr->compiledVarResProc(nsPtr->interp,
			    localPtr->name, localPtr->nameLength,
			    (Tcl_Namespace *) nsPtr, &vinfo);
		}
		resPtr = resPtr->nextPtr;
	    }
	    if (result == TCL_OK) {
		localPtr->resolveInfo = vinfo;
		localPtr->flags |= VAR_RESOLVED;
	    }
	}
    }
    localPtr = firstLocalPtr;
    codePtr->flags &= ~TCL_BYTECODE_RESOLVE_VARS;

```

tcl9.1 9.1.0, revision `Pinned complete original provider source; read-only inspection`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclProc.c`, function `InitResolvedLocals`, lines 1183–1222. Retained reference `compiled-local-formal-9.1.0-tclProc.c`.

```text
	    if (localPtr->resolveInfo->deleteProc) {
		localPtr->resolveInfo->deleteProc(localPtr->resolveInfo);
	    } else {
		Tcl_Free(localPtr->resolveInfo);
	    }
	    localPtr->resolveInfo = NULL;
	}
	localPtr->flags &= ~VAR_RESOLVED;

	if (haveResolvers &&
		!(localPtr->flags & (VAR_ARGUMENT|VAR_TEMPORARY))) {
	    ResolverScheme *resPtr = iPtr->resolverPtr;
	    Tcl_ResolvedVarInfo *vinfo;
	    int result;

	    if (nsPtr->compiledVarResProc) {
		result = nsPtr->compiledVarResProc(nsPtr->interp,
			localPtr->name, localPtr->nameLength,
			(Tcl_Namespace *) nsPtr, &vinfo);
	    } else {
		result = TCL_CONTINUE;
	    }

	    while ((result == TCL_CONTINUE) && resPtr) {
		if (resPtr->compiledVarResProc) {
		    result = resPtr->compiledVarResProc(nsPtr->interp,
			    localPtr->name, localPtr->nameLength,
			    (Tcl_Namespace *) nsPtr, &vinfo);
		}
		resPtr = resPtr->nextPtr;
	    }
	    if (result == TCL_OK) {
		localPtr->resolveInfo = vinfo;
		localPtr->flags |= VAR_RESOLVED;
	    }
	}
    }
    localPtr = firstLocalPtr;
    codePtr->flags &= ~TCL_BYTECODE_RESOLVE_VARS;

```

## Consumer bindings

No implementation binding is claimed by this observation record.

## Evidence

- [rust/tcl-registry/tests/data/native_compiled_local_formal_source272/8.4.20/resolver-excerpt.c](../../../../rust/tcl-registry/tests/data/native_compiled_local_formal_source272/8.4.20/resolver-excerpt.c)
- [rust/tcl-registry/tests/data/native_compiled_local_formal_source272/8.5.19/resolver-excerpt.c](../../../../rust/tcl-registry/tests/data/native_compiled_local_formal_source272/8.5.19/resolver-excerpt.c)
- [rust/tcl-registry/tests/data/native_compiled_local_formal_source272/8.6.18/resolver-excerpt.c](../../../../rust/tcl-registry/tests/data/native_compiled_local_formal_source272/8.6.18/resolver-excerpt.c)
- [rust/tcl-registry/tests/data/native_compiled_local_formal_source272/9.0.4/resolver-excerpt.c](../../../../rust/tcl-registry/tests/data/native_compiled_local_formal_source272/9.0.4/resolver-excerpt.c)
- [rust/tcl-registry/tests/data/native_compiled_local_formal_source272/9.1.0/resolver-excerpt.c](../../../../rust/tcl-registry/tests/data/native_compiled_local_formal_source272/9.1.0/resolver-excerpt.c)

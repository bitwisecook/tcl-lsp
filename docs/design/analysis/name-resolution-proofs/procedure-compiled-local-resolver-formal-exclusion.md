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

Status: `observed`. Version: 8.4.20. Build: Source-only exact pinned tclCompile.c; no interpreter/compiler execution.. Channel: Read-only whole source and exact forty-line resolver guard excerpt.. Dialect: Pinned original C release source.

Inspected selector excludes VAR_ARGUMENT|VAR_TEMPORARY|VAR_RESOLVED. Source-branch fact only; no reached resolver, physical slot, runtime shadowing or Native admission observation.

### tcl8.5

Status: `observed`. Version: 8.5.19. Build: Source-only exact pinned tclProc.c; no interpreter/compiler execution.. Channel: Read-only whole source and exact forty-line resolver guard excerpt.. Dialect: Pinned original C release source.

Inspected path clears VAR_RESOLVED and excludes VAR_ARGUMENT|VAR_TEMPORARY when haveResolvers before namespace/interpreter compiled-variable resolver selection. Source-branch fact only; no reached resolver, physical slot, runtime shadowing or Native admission observation.

### tcl8.6

Status: `observed`. Version: 8.6.18. Build: Source-only exact pinned tclProc.c; no interpreter/compiler execution.. Channel: Read-only whole source and exact forty-line resolver guard excerpt.. Dialect: Pinned original C release source.

Inspected path clears VAR_RESOLVED and excludes VAR_ARGUMENT|VAR_TEMPORARY when haveResolvers before namespace/interpreter compiled-variable resolver selection. Source-branch fact only; no reached resolver, physical slot, runtime shadowing or Native admission observation.

### tcl9.0

Status: `observed`. Version: 9.0.4. Build: Source-only exact pinned tclProc.c; no interpreter/compiler execution.. Channel: Read-only whole source and exact forty-line resolver guard excerpt.. Dialect: Pinned original C release source.

Inspected path clears VAR_RESOLVED and excludes VAR_ARGUMENT|VAR_TEMPORARY when haveResolvers before namespace/interpreter compiled-variable resolver selection. Source-branch fact only; no reached resolver, physical slot, runtime shadowing or Native admission observation.

### tcl9.1

Status: `observed`. Version: 9.1.0. Build: Source-only exact pinned tclProc.c; no interpreter/compiler execution.. Channel: Read-only whole source and exact forty-line resolver guard excerpt.. Dialect: Pinned original C release source.

Inspected path clears VAR_RESOLVED and excludes VAR_ARGUMENT|VAR_TEMPORARY when haveResolvers before namespace/interpreter compiled-variable resolver selection. Source-branch fact only; no reached resolver, physical slot, runtime shadowing or Native admission observation.

### jim

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: jim.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

### bigip

Status: `not-tested`. Version: not tested. Build: not tested. Channel: not tested. Dialect: bigip.

This original-source implementation contract supplies no native process, reached handler, entered frame, contents/value materialisation, completion or edit-authority observation.

## Exact evidence

- `compiled-local-formal-8.4.20-resolver-excerpt.c` (source-anchor): [rust/tcl-registry/tests/data/native_compiled_local_formal_source272/8.4.20/resolver-excerpt.c](../../../../rust/tcl-registry/tests/data/native_compiled_local_formal_source272/8.4.20/resolver-excerpt.c). SHA-256 `a1f1bfec21e2f0937dda2a21ebe7a8a540ac2d234c94e92419d3fc2f93a03a6b`. Exact unchanged independent read-only provider source, selected guard/resolver excerpt or inspection request; no native execution or physical slot observation.
- `compiled-local-formal-8.4.20-tclCompile.c` (source-anchor): [rust/tcl-registry/tests/data/native_compiled_local_formal_source272/8.4.20/tclCompile.c](../../../../rust/tcl-registry/tests/data/native_compiled_local_formal_source272/8.4.20/tclCompile.c). SHA-256 `0bc55b283d6cb62a4298d4dc30e050b587d5d1b4e7f145e4f644e237dd7639f8`. Exact unchanged independent read-only provider source, selected guard/resolver excerpt or inspection request; no native execution or physical slot observation.
- `compiled-local-formal-8.5.19-resolver-excerpt.c` (source-anchor): [rust/tcl-registry/tests/data/native_compiled_local_formal_source272/8.5.19/resolver-excerpt.c](../../../../rust/tcl-registry/tests/data/native_compiled_local_formal_source272/8.5.19/resolver-excerpt.c). SHA-256 `00d8864d624381e7a112cb82246cc99d2a2e83483d77d9a4567db5a002ea2599`. Exact unchanged independent read-only provider source, selected guard/resolver excerpt or inspection request; no native execution or physical slot observation.
- `compiled-local-formal-8.5.19-tclProc.c` (source-anchor): [rust/tcl-registry/tests/data/native_compiled_local_formal_source272/8.5.19/tclProc.c](../../../../rust/tcl-registry/tests/data/native_compiled_local_formal_source272/8.5.19/tclProc.c). SHA-256 `9adc0419de1af0c81c7718e1e682fb50f4e2873f7b2d07a0f3cbe380f7d2dce6`. Exact unchanged independent read-only provider source, selected guard/resolver excerpt or inspection request; no native execution or physical slot observation.
- `compiled-local-formal-8.6.18-resolver-excerpt.c` (source-anchor): [rust/tcl-registry/tests/data/native_compiled_local_formal_source272/8.6.18/resolver-excerpt.c](../../../../rust/tcl-registry/tests/data/native_compiled_local_formal_source272/8.6.18/resolver-excerpt.c). SHA-256 `65846249809bd4bc1980b40f48b64a55c17f4d9acc538c41608dbb82670509ff`. Exact unchanged independent read-only provider source, selected guard/resolver excerpt or inspection request; no native execution or physical slot observation.
- `compiled-local-formal-8.6.18-tclProc.c` (source-anchor): [rust/tcl-registry/tests/data/native_compiled_local_formal_source272/8.6.18/tclProc.c](../../../../rust/tcl-registry/tests/data/native_compiled_local_formal_source272/8.6.18/tclProc.c). SHA-256 `c1ddd801a69b0e39bf923e97838134489ca4937fb7d081eccc0814b6811f71b3`. Exact unchanged independent read-only provider source, selected guard/resolver excerpt or inspection request; no native execution or physical slot observation.
- `compiled-local-formal-9.0.4-resolver-excerpt.c` (source-anchor): [rust/tcl-registry/tests/data/native_compiled_local_formal_source272/9.0.4/resolver-excerpt.c](../../../../rust/tcl-registry/tests/data/native_compiled_local_formal_source272/9.0.4/resolver-excerpt.c). SHA-256 `57f490a48ad62ac09ad843c1675dd7ad6ac9e8ce44655942f9bba17015d9813a`. Exact unchanged independent read-only provider source, selected guard/resolver excerpt or inspection request; no native execution or physical slot observation.
- `compiled-local-formal-9.0.4-tclProc.c` (source-anchor): [rust/tcl-registry/tests/data/native_compiled_local_formal_source272/9.0.4/tclProc.c](../../../../rust/tcl-registry/tests/data/native_compiled_local_formal_source272/9.0.4/tclProc.c). SHA-256 `ef5ef608fbb22c68d35317e09637f099380457e5d01ff8e1b67f2a8eeb70c110`. Exact unchanged independent read-only provider source, selected guard/resolver excerpt or inspection request; no native execution or physical slot observation.
- `compiled-local-formal-9.1.0-resolver-excerpt.c` (source-anchor): [rust/tcl-registry/tests/data/native_compiled_local_formal_source272/9.1.0/resolver-excerpt.c](../../../../rust/tcl-registry/tests/data/native_compiled_local_formal_source272/9.1.0/resolver-excerpt.c). SHA-256 `57f490a48ad62ac09ad843c1675dd7ad6ac9e8ce44655942f9bba17015d9813a`. Exact unchanged independent read-only provider source, selected guard/resolver excerpt or inspection request; no native execution or physical slot observation.
- `compiled-local-formal-9.1.0-tclProc.c` (source-anchor): [rust/tcl-registry/tests/data/native_compiled_local_formal_source272/9.1.0/tclProc.c](../../../../rust/tcl-registry/tests/data/native_compiled_local_formal_source272/9.1.0/tclProc.c). SHA-256 `2c5cf8968a3176aa8e5c316063594186f50109cecd2fbe3c3dff2f3035cefd1a`. Exact unchanged independent read-only provider source, selected guard/resolver excerpt or inspection request; no native execution or physical slot observation.
- `compiled-local-formal-request.json` (source-anchor): [rust/tcl-registry/tests/data/native_compiled_local_formal_source272/request.json](../../../../rust/tcl-registry/tests/data/native_compiled_local_formal_source272/request.json). SHA-256 `746c08449fe3bfd8b85b2c038c4974bbeb89743306eb217740c3b4a033dd5883`. Exact unchanged independent read-only provider source, selected guard/resolver excerpt or inspection request; no native execution or physical slot observation.

## Source inspection

tcl8.4 8.4.20, revision `Pinned complete original provider source; read-only inspection`, `/workspace/tcl-lsp/tmp/tcl8.4.20/generic/tclCompile.c`, function `TclInitCompiledLocals`, lines 2348–2387. Full-source SHA-256 `0bc55b283d6cb62a4298d4dc30e050b587d5d1b4e7f145e4f644e237dd7639f8`; snippet SHA-256 `a1f1bfec21e2f0937dda2a21ebe7a8a540ac2d234c94e92419d3fc2f93a03a6b`; retained evidence `compiled-local-formal-8.4.20-tclCompile.c`.

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

tcl8.5 8.5.19, revision `Pinned complete original provider source; read-only inspection`, `/workspace/tcl-lsp/tmp/tcl8.5.19/generic/tclProc.c`, function `TclInitCompiledLocals`, lines 1265–1304. Full-source SHA-256 `9adc0419de1af0c81c7718e1e682fb50f4e2873f7b2d07a0f3cbe380f7d2dce6`; snippet SHA-256 `00d8864d624381e7a112cb82246cc99d2a2e83483d77d9a4567db5a002ea2599`; retained evidence `compiled-local-formal-8.5.19-tclProc.c`.

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

tcl8.6 8.6.18, revision `Pinned complete original provider source; read-only inspection`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclProc.c`, function `TclInitCompiledLocals`, lines 1214–1253. Full-source SHA-256 `c1ddd801a69b0e39bf923e97838134489ca4937fb7d081eccc0814b6811f71b3`; snippet SHA-256 `65846249809bd4bc1980b40f48b64a55c17f4d9acc538c41608dbb82670509ff`; retained evidence `compiled-local-formal-8.6.18-tclProc.c`.

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

tcl9.0 9.0.4, revision `Pinned complete original provider source; read-only inspection`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclProc.c`, function `InitResolvedLocals`, lines 1188–1227. Full-source SHA-256 `ef5ef608fbb22c68d35317e09637f099380457e5d01ff8e1b67f2a8eeb70c110`; snippet SHA-256 `57f490a48ad62ac09ad843c1675dd7ad6ac9e8ce44655942f9bba17015d9813a`; retained evidence `compiled-local-formal-9.0.4-tclProc.c`.

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

tcl9.1 9.1.0, revision `Pinned complete original provider source; read-only inspection`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclProc.c`, function `InitResolvedLocals`, lines 1183–1222. Full-source SHA-256 `2c5cf8968a3176aa8e5c316063594186f50109cecd2fbe3c3dff2f3035cefd1a`; snippet SHA-256 `57f490a48ad62ac09ad843c1675dd7ad6ac9e8ce44655942f9bba17015d9813a`; retained evidence `compiled-local-formal-9.1.0-tclProc.c`.

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

A named test is a coverage binding, not a claim that it executed.

## Replay

Named Rust coverage is linked, not an executed-result claim. Exact source/executable receipts retain actual outcomes separately. Native evidence and runtime/effect/frame/edit obligations remain independent.

# naming.tcloo.private-dispatch-null-compiler

Kind: `source-anchor`

## Problem statement

An actual stock my command is registered through an object-command constructor rather than a compiler table. Treating its absent compileProc as unknown prevents byte bodies from reaching generic dispatch; borrowing helper compilers or a method name would grant a different preparation.

## Question

Does native TclOO register the per-object my/myclass command with a compiler hook, or only with object/NRE dispatch procedures?

## Conclusion

Pinned Tcl8.6.18/9.0.4/9.1.0 AllocObject creates my through TclNRCreateCommandInNs. Tcl9.0.4/9.1.0 also create myclass. The underlying new object-command allocation initializes compileProc to NULL and the NRE wrapper changes only nreProc/nreProc2. This describes a selected stock registration; later replacement, current lookup, command observers and method execution remain independent. The next/nextto/self helper registrations have separate compiler hooks and are excluded.

## Scope

Exact source constructor windows for C8.6.18, C9.0.4 and C9.1.0. Source inspection only; no build, native/Rust execution, selected method body, Normal completion or arbitrary same-named registration grant.

## Provider answers

### tcl8.4, tcl8.5, jim, bigip

Status: `not-tested`. Channel: No constructor inspected for this question.

No source/native observation for this exact TclOO registration question; no hook metadata is borrowed from another provider.

### tcl8.6

Status: `inspected`. Channel: Inspected object command constructors; no guest input. Version: tcl8.6 8.6.18.

my is created with an absent compileProc. myclass is not present in this 8.6 constructor window.

### tcl9.0, tcl9.1

Status: `inspected`. Channel: Inspected object command constructors; no guest input. Version: tcl9.0 9.0.4; tcl9.1 9.1.0.

my is created with an absent compileProc. myclass uses the same null-compiler constructor independently of its NRE procedure.

## Source inspection

tcl8.6 8.6.18, revision `8.6.18`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOO.c`, function `AllocObject`, lines 766–768. Retained reference `tcl8.6-object-private-command`.

```text
    oPtr->myCommand = TclNRCreateCommandInNs(interp, "my", oPtr->namespacePtr,
	PrivateObjectCmd, PrivateNRObjectCmd, oPtr, MyDeleted);
    return oPtr;

```

tcl8.6 8.6.18, revision `8.6.18`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclBasic.c`, function `TclNRCreateCommandInNs`, lines 8296–8311. Retained reference `tcl8.6-nre-wrapper`.

```text
TclNRCreateCommandInNs(
    Tcl_Interp *interp,
    const char *cmdName,
    Tcl_Namespace *nsPtr,
    Tcl_ObjCmdProc *proc,
    Tcl_ObjCmdProc *nreProc,
    ClientData clientData,
    Tcl_CmdDeleteProc *deleteProc)
{
    Command *cmdPtr = (Command *)
	    TclCreateObjCommandInNs(interp, cmdName, nsPtr, proc, clientData,
		    deleteProc);

    cmdPtr->nreProc = nreProc;
    return (Tcl_Command) cmdPtr;
}

```

tcl8.6 8.6.18, revision `8.6.18`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclBasic.c`, function `TclCreateObjCommandInNs`, lines 2458–2477. Retained reference `tcl8.6-new-object-command`.

```text
    }
    cmdPtr = (Command *)ckalloc(sizeof(Command));
    Tcl_SetHashValue(hPtr, cmdPtr);
    cmdPtr->hPtr = hPtr;
    cmdPtr->nsPtr = nsPtr;
    cmdPtr->refCount = 1;
    cmdPtr->cmdEpoch = 0;
    cmdPtr->compileProc = NULL;
    cmdPtr->objProc = proc;
    cmdPtr->objClientData = clientData;
    cmdPtr->proc = TclInvokeObjectCommand;
    cmdPtr->clientData = cmdPtr;
    cmdPtr->deleteProc = deleteProc;
    cmdPtr->deleteData = clientData;
    cmdPtr->flags = 0;
    cmdPtr->importRefPtr = NULL;
    cmdPtr->tracePtr = NULL;
    cmdPtr->nreProc = NULL;

    /*

```

tcl9.0 9.0.4, revision `9.0.4`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOO.c`, function `AllocObject`, lines 1008–1013. Retained reference `tcl9.0-object-private-command`.

```text
    oPtr->myCommand = TclNRCreateCommandInNs(interp, "my", oPtr->namespacePtr,
	    TclOOPrivateObjectCmd, PrivateNRObjectCmd, oPtr, MyDeleted);
    oPtr->myclassCommand = TclNRCreateCommandInNs(interp, "myclass",
	    oPtr->namespacePtr, TclOOMyClassObjCmd, MyClassNRObjCmd, oPtr,
	    MyClassDeleted);
    oPtr->linkedCmdsList = NULL;

```

tcl9.0 9.0.4, revision `9.0.4`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclBasic.c`, function `TclNRCreateCommandInNs`, lines 8701–8716. Retained reference `tcl9.0-nre-wrapper`.

```text
TclNRCreateCommandInNs(
    Tcl_Interp *interp,
    const char *cmdName,
    Tcl_Namespace *nsPtr,
    Tcl_ObjCmdProc *proc,
    Tcl_ObjCmdProc *nreProc,
    void *clientData,
    Tcl_CmdDeleteProc *deleteProc)
{
    Command *cmdPtr = (Command *)
	    TclCreateObjCommandInNs(interp, cmdName, nsPtr, proc, clientData,
		    deleteProc);

    cmdPtr->nreProc = nreProc;
    return (Tcl_Command) cmdPtr;
}

```

tcl9.0 9.0.4, revision `9.0.4`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclBasic.c`, function `TclCreateObjCommandInNs`, lines 3031–3050. Retained reference `tcl9.0-new-object-command`.

```text
    }
    cmdPtr = (Command *)Tcl_Alloc(sizeof(Command));
    Tcl_SetHashValue(hPtr, cmdPtr);
    cmdPtr->hPtr = hPtr;
    cmdPtr->nsPtr = nsPtr;
    cmdPtr->refCount = 1;
    cmdPtr->cmdEpoch = 0;
    cmdPtr->compileProc = NULL;
    cmdPtr->objProc = proc;
    cmdPtr->objClientData = clientData;
    cmdPtr->proc = NULL;
    cmdPtr->clientData = NULL;
    cmdPtr->deleteProc = deleteProc;
    cmdPtr->deleteData = clientData;
    cmdPtr->flags = 0;
    cmdPtr->importRefPtr = NULL;
    cmdPtr->tracePtr = NULL;
    cmdPtr->nreProc = NULL;

    /*

```

tcl9.1 9.1.0, revision `9.1.0`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOO.c`, function `AllocObject`, lines 996–1001. Retained reference `tcl9.1-object-private-command`.

```text
    oPtr->myCommand = TclNRCreateCommandInNs(interp, "my", oPtr->namespacePtr,
	    TclOOPrivateObjectCmd, PrivateNRObjectCmd, oPtr, MyDeleted);
    oPtr->myclassCommand = TclNRCreateCommandInNs(interp, "myclass",
	    oPtr->namespacePtr, TclOOMyClassObjCmd, MyClassNRObjCmd, oPtr,
	    MyClassDeleted);
    oPtr->linkedCmdsList = NULL;

```

tcl9.1 9.1.0, revision `9.1.0`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclBasic.c`, function `TclNRCreateCommandInNs`, lines 9012–9027. Retained reference `tcl9.1-nre-wrapper`.

```text
TclNRCreateCommandInNs(
    Tcl_Interp *interp,
    const char *cmdName,
    Tcl_Namespace *nsPtr,
    Tcl_ObjCmdProc2 *proc,
    Tcl_ObjCmdProc2 *nreProc,
    void *clientData,
    Tcl_CmdDeleteProc *deleteProc)
{
    Command *cmdPtr = (Command *)
	    TclCreateObjCommandInNs(interp, cmdName, nsPtr, proc, clientData,
		    deleteProc);

    cmdPtr->nreProc2 = nreProc;
    return (Tcl_Command) cmdPtr;
}

```

tcl9.1 9.1.0, revision `9.1.0`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclBasic.c`, function `TclCreateObjCommandInNs`, lines 2987–3006. Retained reference `tcl9.1-new-object-command`.

```text
    }
    cmdPtr = (Command *)Tcl_Alloc(sizeof(Command));
    Tcl_SetHashValue(hPtr, cmdPtr);
    cmdPtr->hPtr = hPtr;
    cmdPtr->nsPtr = nsPtr;
    cmdPtr->refCount = 1;
    cmdPtr->cmdEpoch = 0;
    cmdPtr->compileProc = NULL;
    cmdPtr->objProc2 = proc;
    cmdPtr->objClientData2 = clientData;
    cmdPtr->proc = NULL;
    cmdPtr->clientData = NULL;
    cmdPtr->deleteProc = deleteProc;
    cmdPtr->deleteData = clientData;
    cmdPtr->flags = 0;
    cmdPtr->importRefPtr = NULL;
    cmdPtr->tracePtr = NULL;
    cmdPtr->nreProc2 = NULL;

    /*

```

## Consumer bindings

- [rust/tcl-registry/src/native_tcloo_registration.rs](../../../../rust/tcl-registry/src/native_tcloo_registration.rs), `compilation`: Select independently source-inspected stock registration hook presence; no handler/token/lookup grant.
- [rust/tcl-vm/src/interp.rs](../../../../rust/tcl-vm/src/interp.rs), `Vm::native_hook_for_registry_identity`: Stamp the actual current stock registration sidecar from its retained identity and selected native dialect; replacements and actual command lookup remain separate.
- [rust/tcl-registry/src/native_tcloo_registration.rs](../../../../rust/tcl-registry/src/native_tcloo_registration.rs), `native_tcloo_registration::tests::private_dispatch_registration_keeps_null_hooks_separate_from_helper_compilers` (linked): The exact native registration/version selects null-hook metadata; C84/85, Jim, missing family and unrelated identity decline, while next/nextto/self remain outside the absent-hook owner. No Rust execution is claimed.

## Evidence

- [rust/tcl-registry/tests/data/native_tcloo_private_dispatch_source/tcl8.6-source.json](../../../../rust/tcl-registry/tests/data/native_tcloo_private_dispatch_source/tcl8.6-source.json)
- [rust/tcl-registry/tests/data/native_tcloo_private_dispatch_source/tcl9.0-source.json](../../../../rust/tcl-registry/tests/data/native_tcloo_private_dispatch_source/tcl9.0-source.json)
- [rust/tcl-registry/tests/data/native_tcloo_private_dispatch_source/tcl9.1-source.json](../../../../rust/tcl-registry/tests/data/native_tcloo_private_dispatch_source/tcl9.1-source.json)

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

### tcl8.4

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: No constructor inspected for this question.. Dialect: Tcl.

No source/native observation for this exact TclOO registration question; no hook metadata is borrowed from another provider.

### tcl8.5

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: No constructor inspected for this question.. Dialect: Tcl.

No source/native observation for this exact TclOO registration question; no hook metadata is borrowed from another provider.

### tcl8.6

Status: `inspected`. Version: 8.6.18. Build: Source inspection only; no executed build association.. Channel: Inspected object command constructors; no guest input.. Dialect: Tcl.

my is created with an absent compileProc. myclass is not present in this 8.6 constructor window.

### tcl9.0

Status: `inspected`. Version: 9.0.4. Build: Source inspection only; no executed build association.. Channel: Inspected object command constructors; no guest input.. Dialect: Tcl.

my is created with an absent compileProc. myclass uses the same null-compiler constructor independently of its NRE procedure.

### tcl9.1

Status: `inspected`. Version: 9.1.0. Build: Source inspection only; no executed build association.. Channel: Inspected object command constructors; no guest input.. Dialect: Tcl.

my is created with an absent compileProc. myclass uses the same null-compiler constructor independently of its NRE procedure.

### jim

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: No constructor inspected for this question.. Dialect: Jim Tcl.

No source/native observation for this exact TclOO registration question; no hook metadata is borrowed from another provider.

### bigip

Status: `not-tested`. Version: not recorded. Build: not recorded. Channel: No constructor inspected for this question.. Dialect: F5 iRules.

No source/native observation for this exact TclOO registration question; no hook metadata is borrowed from another provider.

## Exact evidence

- `tcl8.6-object-private-command` (source-anchor): [rust/tcl-registry/tests/data/native_tcloo_private_dispatch_source/tcl8.6-source.json](../../../../rust/tcl-registry/tests/data/native_tcloo_private_dispatch_source/tcl8.6-source.json). SHA-256 `934ecdf4a09f02df2fafcb7dbf2749d13e77e87b18b5d58213a7be773e7b4881`. JSON pointer `/windows/0/snippet`. Exact LF-coordinate source window; full original source and snippet digests retained. No native or Rust launch.
- `tcl8.6-nre-wrapper` (source-anchor): [rust/tcl-registry/tests/data/native_tcloo_private_dispatch_source/tcl8.6-source.json](../../../../rust/tcl-registry/tests/data/native_tcloo_private_dispatch_source/tcl8.6-source.json). SHA-256 `934ecdf4a09f02df2fafcb7dbf2749d13e77e87b18b5d58213a7be773e7b4881`. JSON pointer `/windows/1/snippet`. Exact LF-coordinate source window; full original source and snippet digests retained. No native or Rust launch.
- `tcl8.6-new-object-command` (source-anchor): [rust/tcl-registry/tests/data/native_tcloo_private_dispatch_source/tcl8.6-source.json](../../../../rust/tcl-registry/tests/data/native_tcloo_private_dispatch_source/tcl8.6-source.json). SHA-256 `934ecdf4a09f02df2fafcb7dbf2749d13e77e87b18b5d58213a7be773e7b4881`. JSON pointer `/windows/2/snippet`. Exact LF-coordinate source window; full original source and snippet digests retained. No native or Rust launch.
- `tcl9.0-object-private-command` (source-anchor): [rust/tcl-registry/tests/data/native_tcloo_private_dispatch_source/tcl9.0-source.json](../../../../rust/tcl-registry/tests/data/native_tcloo_private_dispatch_source/tcl9.0-source.json). SHA-256 `91c49b814de60ea321450ace10025f394642fd3d6fff46cd3b41e0496d5a7cca`. JSON pointer `/windows/0/snippet`. Exact LF-coordinate source window; full original source and snippet digests retained. No native or Rust launch.
- `tcl9.0-nre-wrapper` (source-anchor): [rust/tcl-registry/tests/data/native_tcloo_private_dispatch_source/tcl9.0-source.json](../../../../rust/tcl-registry/tests/data/native_tcloo_private_dispatch_source/tcl9.0-source.json). SHA-256 `91c49b814de60ea321450ace10025f394642fd3d6fff46cd3b41e0496d5a7cca`. JSON pointer `/windows/1/snippet`. Exact LF-coordinate source window; full original source and snippet digests retained. No native or Rust launch.
- `tcl9.0-new-object-command` (source-anchor): [rust/tcl-registry/tests/data/native_tcloo_private_dispatch_source/tcl9.0-source.json](../../../../rust/tcl-registry/tests/data/native_tcloo_private_dispatch_source/tcl9.0-source.json). SHA-256 `91c49b814de60ea321450ace10025f394642fd3d6fff46cd3b41e0496d5a7cca`. JSON pointer `/windows/2/snippet`. Exact LF-coordinate source window; full original source and snippet digests retained. No native or Rust launch.
- `tcl9.1-object-private-command` (source-anchor): [rust/tcl-registry/tests/data/native_tcloo_private_dispatch_source/tcl9.1-source.json](../../../../rust/tcl-registry/tests/data/native_tcloo_private_dispatch_source/tcl9.1-source.json). SHA-256 `09d636527307e97514a1113d441f52dea5e868721b13adc740afbc9f9bcad99d`. JSON pointer `/windows/0/snippet`. Exact LF-coordinate source window; full original source and snippet digests retained. No native or Rust launch.
- `tcl9.1-nre-wrapper` (source-anchor): [rust/tcl-registry/tests/data/native_tcloo_private_dispatch_source/tcl9.1-source.json](../../../../rust/tcl-registry/tests/data/native_tcloo_private_dispatch_source/tcl9.1-source.json). SHA-256 `09d636527307e97514a1113d441f52dea5e868721b13adc740afbc9f9bcad99d`. JSON pointer `/windows/1/snippet`. Exact LF-coordinate source window; full original source and snippet digests retained. No native or Rust launch.
- `tcl9.1-new-object-command` (source-anchor): [rust/tcl-registry/tests/data/native_tcloo_private_dispatch_source/tcl9.1-source.json](../../../../rust/tcl-registry/tests/data/native_tcloo_private_dispatch_source/tcl9.1-source.json). SHA-256 `09d636527307e97514a1113d441f52dea5e868721b13adc740afbc9f9bcad99d`. JSON pointer `/windows/2/snippet`. Exact LF-coordinate source window; full original source and snippet digests retained. No native or Rust launch.

## Source inspection

tcl8.6 8.6.18, revision `8.6.18`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOO.c`, function `AllocObject`, lines 766–768. Full-source SHA-256 `e749370dcaeab6d214b811a246536c1cf06f92edf2c51c8f1dbd8f4a6f57b3ef`; snippet SHA-256 `5934644d7c8378e6b3728d210d9f64f3f8401bc0a6fab6d9824a67991b69b0b0`; retained evidence `tcl8.6-object-private-command`.

```text
    oPtr->myCommand = TclNRCreateCommandInNs(interp, "my", oPtr->namespacePtr,
	PrivateObjectCmd, PrivateNRObjectCmd, oPtr, MyDeleted);
    return oPtr;

```

tcl8.6 8.6.18, revision `8.6.18`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclBasic.c`, function `TclNRCreateCommandInNs`, lines 8296–8311. Full-source SHA-256 `19e28c2c9fbcd27e7e538a06960f0d806d04f8c25d94499cf9eb9031f8440198`; snippet SHA-256 `23861938456c0832182861cecee3e0ca7556e2e1a11dfd393479eb23542e3ab4`; retained evidence `tcl8.6-nre-wrapper`.

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

tcl8.6 8.6.18, revision `8.6.18`, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclBasic.c`, function `TclCreateObjCommandInNs`, lines 2458–2477. Full-source SHA-256 `19e28c2c9fbcd27e7e538a06960f0d806d04f8c25d94499cf9eb9031f8440198`; snippet SHA-256 `471928c30ee7dc3243c1069bda8d21eefc981794360c989af1a4781bf319363f`; retained evidence `tcl8.6-new-object-command`.

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

tcl9.0 9.0.4, revision `9.0.4`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOO.c`, function `AllocObject`, lines 1008–1013. Full-source SHA-256 `2e26684094bbe01c988c20386de1a3bf53646f47ce84ad50a807414455d1d3a7`; snippet SHA-256 `9dd24259a48cc056cf6fcf42b010dc1f314c89501c42b726d2d0315fd3921686`; retained evidence `tcl9.0-object-private-command`.

```text
    oPtr->myCommand = TclNRCreateCommandInNs(interp, "my", oPtr->namespacePtr,
	    TclOOPrivateObjectCmd, PrivateNRObjectCmd, oPtr, MyDeleted);
    oPtr->myclassCommand = TclNRCreateCommandInNs(interp, "myclass",
	    oPtr->namespacePtr, TclOOMyClassObjCmd, MyClassNRObjCmd, oPtr,
	    MyClassDeleted);
    oPtr->linkedCmdsList = NULL;

```

tcl9.0 9.0.4, revision `9.0.4`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclBasic.c`, function `TclNRCreateCommandInNs`, lines 8701–8716. Full-source SHA-256 `6d995c1af94fea92717183882ca41775fe8c0e088def163f6ac8fe699cbffc67`; snippet SHA-256 `662139d11a412439c0614e639cb5933dcfb1a3aacc095cc32a49752ddef00a1b`; retained evidence `tcl9.0-nre-wrapper`.

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

tcl9.0 9.0.4, revision `9.0.4`, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclBasic.c`, function `TclCreateObjCommandInNs`, lines 3031–3050. Full-source SHA-256 `6d995c1af94fea92717183882ca41775fe8c0e088def163f6ac8fe699cbffc67`; snippet SHA-256 `c60e5d1e61ed14f8ce06f58f03b32edb885e81b9ed87397754cfc7f9fba744d7`; retained evidence `tcl9.0-new-object-command`.

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

tcl9.1 9.1.0, revision `9.1.0`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOO.c`, function `AllocObject`, lines 996–1001. Full-source SHA-256 `f2abb0b2ca6fa8cf621f158f209a48ee20ec2a08073d2f2c4af541ddf0babbbd`; snippet SHA-256 `9dd24259a48cc056cf6fcf42b010dc1f314c89501c42b726d2d0315fd3921686`; retained evidence `tcl9.1-object-private-command`.

```text
    oPtr->myCommand = TclNRCreateCommandInNs(interp, "my", oPtr->namespacePtr,
	    TclOOPrivateObjectCmd, PrivateNRObjectCmd, oPtr, MyDeleted);
    oPtr->myclassCommand = TclNRCreateCommandInNs(interp, "myclass",
	    oPtr->namespacePtr, TclOOMyClassObjCmd, MyClassNRObjCmd, oPtr,
	    MyClassDeleted);
    oPtr->linkedCmdsList = NULL;

```

tcl9.1 9.1.0, revision `9.1.0`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclBasic.c`, function `TclNRCreateCommandInNs`, lines 9012–9027. Full-source SHA-256 `5f52206bed251629baedd8ef8cc0f7ae1237bf9a7edc7d4bca76d3932f8870d2`; snippet SHA-256 `f1731fce0453b3f2ec722529dcc0207d694f3973264cb8331d3bd96fbcf7551a`; retained evidence `tcl9.1-nre-wrapper`.

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

tcl9.1 9.1.0, revision `9.1.0`, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclBasic.c`, function `TclCreateObjCommandInNs`, lines 2987–3006. Full-source SHA-256 `5f52206bed251629baedd8ef8cc0f7ae1237bf9a7edc7d4bca76d3932f8870d2`; snippet SHA-256 `f584f0c1492c739c63123283ca81dfd7e0272d4ae95d62004d47da9b5fc0a1c6`; retained evidence `tcl9.1-new-object-command`.

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

A named test is a coverage binding, not a claim that it executed.

## Replay

Verify each exact LF window and snippet/full-file SHA against its pinned source. No executable replay or native/Rust execution is asserted.

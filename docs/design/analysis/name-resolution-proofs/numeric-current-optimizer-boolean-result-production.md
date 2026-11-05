# naming.numeric.current-optimizer-boolean-result-production

Kind: `implementation-contract` (current source interpretation; all external execution statuses not tested).

## Problem statement

A reached numeric instruction must execute conversion. An inline Boolean condition may have that instruction removed before execution. Applying one result recipe to both changes original cache/String effects; a public expression API has its own independent result producer.

## Question

Which current configured C Tcl sources select numeric-conversion removal before a Boolean jump, and how does that source-level rule distinguish a reached TRY_CVT_TO_NUMERIC from inline and public-API result production?

## Conclusion

In the inspected configured C8.6.18/9.0.4/9.1.0 source, TclOptimizeBytecode blanks TRY_CVT_TO_NUMERIC to NOP when the next instruction is one of its listed conditional jumps or other listed consumers. The source separately installs the optimizer in Tcl_CreateInterp and calls it from TclSetByteCodeFromAny. C9.1 guards deprecated short jump opcodes with REMOVE_DEPRECATED_OPCODES. A genuinely reached numeric instruction therefore has a distinct current implementation purpose from an inline condition whose source recipe removes conversion; public API result production remains independent. C8.4/C8.5 selected tclOptimize.c absence establishes only that file fact. No Native or Rust execution answers this source interpretation question.

## Scope

Exactly five configured source-tree states with full retained headers, compile/basic/expression sources, Makefile and tclConfig.sh, three whole optimizer files and nine exact rewrite/install/call excerpts. Version macros and independently captured original465 CLI version bytes are retained and matched; each source/header/config/library hash explicitly records whether it was already an original465 launch input. Optimizer files and tclCompile.c are current separately retained source and are not added retroactively to original465. Five existing static-library identities are byte-verified by hash/size and retained as references in the unchanged original465 input receipts; no rebuild/source-to-library causal equivalence is inferred. No opcode listing, Native compiler/execution result, original object/header/cache/frame/issuer, absence of optimisation elsewhere, Jim or BIG-IP behavior or completed software assertion follows. All seven external execution statuses remain not-tested.

The [independent original logical-site observations](numeric-original-logical-operand-truth-sites.md) and [primitive versus expression API observations](numeric-original-primitive-boolean-vs-expression-truth.md) preserve their actual route/cache/result purpose. No source window is added to their original launch inputs.

## Provider answers

| Provider | Version/source state | Execution status | Source answer and strict limit |
| --- | --- | --- | --- |
| `tcl8.4` | 8.4.20 | `not-tested` | Selected tclOptimize.c is absent in the inspected configured tree. This source-file fact supplies no opcode, compile, execution or absence-of-optimisation conclusion. |
| `tcl8.5` | 8.5.19 | `not-tested` | Selected tclOptimize.c is absent in the inspected configured tree. This source-file fact supplies no opcode, compile, execution or absence-of-optimisation conclusion. |
| `tcl8.6` | 8.6.18 | `not-tested` | The retained current source blanks TRY_CVT_TO_NUMERIC before its listed conditional jumps/consumers and separately installs/calls this optimizer; exact macro/config/whole-file state is retained. This is source interpretation only, without measured execution/opcode or source-to-library equivalence. |
| `tcl9.0` | 9.0.4 | `not-tested` | The retained current source blanks TRY_CVT_TO_NUMERIC before its listed conditional jumps/consumers and separately installs/calls this optimizer; exact macro/config/whole-file state is retained. This is source interpretation only, without measured execution/opcode or source-to-library equivalence. |
| `tcl9.1` | 9.1.0 | `not-tested` | The retained current source blanks TRY_CVT_TO_NUMERIC before its listed conditional jumps/consumers and separately installs/calls this optimizer; exact macro/config/whole-file state is retained. This is source interpretation only, without measured execution/opcode or source-to-library equivalence. |
| `jim` | not inspected for this source question | `not-tested` | No source inspection or execution answers this Jim/BIG-IP optimizer question. |
| `bigip` | not inspected for this source question | `not-tested` | No source inspection or execution answers this Jim/BIG-IP optimizer question. |

## Source inspection

tcl8.6 8.6.18, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclOptimize.c`, TclOptimizeBytecode, lines275–330. The source revision was not otherwise recorded.

```c
		break;
	    }
	    break;

	case INST_TRY_CVT_TO_NUMERIC:
	    switch (nextInst) {
	    case INST_JUMP_TRUE1:
	    case INST_JUMP_TRUE4:
	    case INST_JUMP_FALSE1:
	    case INST_JUMP_FALSE4:
	    case INST_INCR_SCALAR1:
	    case INST_INCR_ARRAY1:
	    case INST_INCR_ARRAY_STK:
	    case INST_INCR_SCALAR_STK:
	    case INST_INCR_STK:
	    case INST_LOR:
	    case INST_LAND:
	    case INST_EQ:
	    case INST_NEQ:
	    case INST_LT:
	    case INST_LE:
	    case INST_GT:
	    case INST_GE:
	    case INST_MOD:
	    case INST_LSHIFT:
	    case INST_RSHIFT:
	    case INST_BITOR:
	    case INST_BITXOR:
	    case INST_BITAND:
	    case INST_EXPON:
	    case INST_ADD:
	    case INST_SUB:
	    case INST_DIV:
	    case INST_MULT:
	    case INST_LNOT:
	    case INST_BITNOT:
	    case INST_UMINUS:
	    case INST_UPLUS:
	    case INST_TRY_CVT_TO_NUMERIC:
		blank = size;
		break;
	    }
	    break;
	}

	if (blank > 0) {
	    for (i=0 ; i<blank ; i++) {
		*(currentInstPtr + i) = INST_NOP;
	    }
	    size = blank;
	}
    }
    Tcl_DeleteHashTable(&targets);
}

/*
```

tcl8.6 8.6.18, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclBasic.c`, Tcl_CreateInterp, lines551–575. The source revision was not otherwise recorded.

```c

    iPtr->result = iPtr->resultSpace;
    iPtr->freeProc = NULL;
    iPtr->errorLine = 0;
    TclNewObj(iPtr->objResultPtr);
    Tcl_IncrRefCount(iPtr->objResultPtr);
    iPtr->handle = TclHandleCreate(iPtr);
    iPtr->globalNsPtr = NULL;
    iPtr->hiddenCmdTablePtr = NULL;
    iPtr->interpInfo = NULL;

    TCL_CT_ASSERT(sizeof(iPtr->extra) <= sizeof(Tcl_HashTable));
    iPtr->extra.optimizer = TclOptimizeBytecode;

    iPtr->numLevels = 0;
    iPtr->maxNestingDepth = MAX_NESTING_DEPTH;
    iPtr->framePtr = NULL;	/* Initialise as soon as :: is available */
    iPtr->varFramePtr = NULL;	/* Initialise as soon as :: is available */

    /*
     * TIP #280 - Initialize the arrays used to extend the ByteCode and Proc
     * structures.
     */

    iPtr->cmdFramePtr = NULL;
```

tcl8.6 8.6.18, `/workspace/tcl-lsp/tmp/tcl8.6.18/generic/tclCompile.c`, TclSetByteCodeFromAny, lines843–863. The source revision was not otherwise recorded.

```c
	TclEmitOpcode(INST_DONE, &compEnv);
	assert (compEnv.atCmdStart > 1);
    }

    /*
     * Apply some peephole optimizations that can cross specific/generic
     * instruction generator boundaries.
     */

    if (iPtr->extra.optimizer) {
	(iPtr->extra.optimizer)(&compEnv);
    }

    /*
     * Invoke the compilation hook procedure if one exists.
     */

    if (hookProc) {
	result = hookProc(interp, &compEnv, clientData);
    }
```

tcl9.0 9.0.4, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclOptimize.c`, TclOptimizeBytecode, lines275–330. The source revision was not otherwise recorded.

```c
		break;
	    }
	    break;

	case INST_TRY_CVT_TO_NUMERIC:
	    switch (nextInst) {
	    case INST_JUMP_TRUE1:
	    case INST_JUMP_TRUE4:
	    case INST_JUMP_FALSE1:
	    case INST_JUMP_FALSE4:
	    case INST_INCR_SCALAR1:
	    case INST_INCR_ARRAY1:
	    case INST_INCR_ARRAY_STK:
	    case INST_INCR_SCALAR_STK:
	    case INST_INCR_STK:
	    case INST_EQ:
	    case INST_NEQ:
	    case INST_LT:
	    case INST_LE:
	    case INST_GT:
	    case INST_GE:
	    case INST_MOD:
	    case INST_LSHIFT:
	    case INST_RSHIFT:
	    case INST_BITOR:
	    case INST_BITXOR:
	    case INST_BITAND:
	    case INST_EXPON:
	    case INST_ADD:
	    case INST_SUB:
	    case INST_DIV:
	    case INST_MULT:
	    case INST_LNOT:
	    case INST_BITNOT:
	    case INST_UMINUS:
	    case INST_UPLUS:
	    case INST_TRY_CVT_TO_NUMERIC:
		blank = size;
		break;
	    }
	    break;
	}

	if (blank > 0) {
	    for (i=0 ; i<blank ; i++) {
		currentInstPtr[i] = INST_NOP;
	    }
	    size = blank;
	}
    }
    Tcl_DeleteHashTable(&targets);
}

/*
 * ----------------------------------------------------------------------
 *
```

tcl9.0 9.0.4, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclBasic.c`, Tcl_CreateInterp, lines925–949. The source revision was not otherwise recorded.

```c
    /* Special invalid value: Any attempt to free the legacy result
     * will cause a crash. */
    iPtr->legacyFreeProc = (void (*) (void))-1;
    iPtr->errorLine = 0;
    iPtr->stubTable = &tclStubs;
    TclNewObj(iPtr->objResultPtr);
    Tcl_IncrRefCount(iPtr->objResultPtr);
    iPtr->handle = TclHandleCreate(iPtr);
    iPtr->globalNsPtr = NULL;
    iPtr->hiddenCmdTablePtr = NULL;
    iPtr->interpInfo = NULL;

    iPtr->optimizer = TclOptimizeBytecode;

    iPtr->numLevels = 0;
    iPtr->maxNestingDepth = MAX_NESTING_DEPTH;
    iPtr->framePtr = NULL;	/* Initialise as soon as :: is available */
    iPtr->varFramePtr = NULL;	/* Initialise as soon as :: is available */

    /*
     * TIP #280 - Initialize the arrays used to extend the ByteCode and Proc
     * structures.
     */

    iPtr->cmdFramePtr = NULL;
```

tcl9.0 9.0.4, `/workspace/tcl-lsp/tmp/tcl9.0.4/generic/tclCompile.c`, TclSetByteCodeFromAny, lines858–878. The source revision was not otherwise recorded.

```c
	TclEmitOpcode(INST_DONE, &compEnv);
	assert (compEnv.atCmdStart > 1);
    }

    /*
     * Apply some peephole optimizations that can cross specific/generic
     * instruction generator boundaries.
     */

    if (iPtr->optimizer) {
	(iPtr->optimizer)(&compEnv);
    }

    /*
     * Invoke the compilation hook procedure if there is one.
     */

    if (hookProc) {
	result = hookProc(interp, &compEnv, clientData);
    }
```

tcl9.1 9.1.0, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclOptimize.c`, TclOptimizeBytecode, lines288–343. The source revision was not otherwise recorded.

```c
		break;
	    }
	    break;

	case INST_TRY_CVT_TO_NUMERIC:
	    switch (nextInst) {
#ifndef REMOVE_DEPRECATED_OPCODES
	    case INST_JUMP_TRUE1:
	    case INST_JUMP_FALSE1:
#endif
	    case INST_JUMP_TRUE:
	    case INST_JUMP_FALSE:
	    case INST_INCR_SCALAR1:
	    case INST_INCR_SCALAR:
	    case INST_INCR_ARRAY1:
	    case INST_INCR_ARRAY:
	    case INST_INCR_ARRAY_STK:
	    case INST_INCR_SCALAR_STK:
	    case INST_INCR_STK:
	    case INST_EQ:
	    case INST_NEQ:
	    case INST_LT:
	    case INST_LE:
	    case INST_GT:
	    case INST_GE:
	    case INST_MOD:
	    case INST_LSHIFT:
	    case INST_RSHIFT:
	    case INST_BITOR:
	    case INST_BITXOR:
	    case INST_BITAND:
	    case INST_EXPON:
	    case INST_ADD:
	    case INST_SUB:
	    case INST_DIV:
	    case INST_MULT:
	    case INST_LNOT:
	    case INST_BITNOT:
	    case INST_UMINUS:
	    case INST_UPLUS:
	    case INST_TRY_CVT_TO_NUMERIC:
		blank = size;
		break;
	    }
	    break;
	}

	if (blank > 0) {
	    for (i=0 ; i<blank ; i++) {
		currentInstPtr[i] = INST_NOP;
	    }
	    size = blank;
	}
    }
    Tcl_DeleteHashTable(&targets);
}
```

tcl9.1 9.1.0, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclBasic.c`, Tcl_CreateInterp, lines860–884. The source revision was not otherwise recorded.

```c
    /* Special invalid value: Any attempt to free the legacy result
     * will cause a crash. */
    iPtr->legacyFreeProc = (void (*) (void))-1;
    iPtr->errorLine = 0;
    iPtr->stubTable = &tclStubs;
    TclNewObj(iPtr->objResultPtr);
    Tcl_IncrRefCount(iPtr->objResultPtr);
    iPtr->handle = TclHandleCreate(iPtr);
    iPtr->globalNsPtr = NULL;
    iPtr->hiddenCmdTablePtr = NULL;
    iPtr->interpInfo = NULL;

    iPtr->optimizer = TclOptimizeBytecode;

    iPtr->numLevels = 0;
    iPtr->maxNestingDepth = MAX_NESTING_DEPTH;
    iPtr->framePtr = NULL;	/* Initialise as soon as :: is available */
    iPtr->varFramePtr = NULL;	/* Initialise as soon as :: is available */

    /*
     * TIP #280 - Initialize the arrays used to extend the ByteCode and Proc
     * structures.
     */

    iPtr->cmdFramePtr = NULL;
```

tcl9.1 9.1.0, `/workspace/tcl-lsp/tmp/tcl9.1.0/generic/tclCompile.c`, TclSetByteCodeFromAny, lines1183–1203. The source revision was not otherwise recorded.

```c
	TclEmitOpcode(		INST_DONE,			&compEnv);
	assert (compEnv.atCmdStart > 1);
    }

    /*
     * Apply some peephole optimizations that can cross specific/generic
     * instruction generator boundaries.
     */

    if (iPtr->optimizer) {
	(iPtr->optimizer)(&compEnv);
    }

    /*
     * Invoke the compilation hook procedure if there is one.
     */

    if (hookProc) {
	result = hookProc(interp, &compEnv, clientData);
    }
```

## Consumer bindings

- [rust/tcl-syntax/src/native_boolean_truth.rs](../../../../rust/tcl-syntax/src/native_boolean_truth.rs), `NativeExpressionResultProducer`: Keep an explicitly reached NumericInstruction result producer separate from a Boolean inline or public-API producer; a descriptor supplies no original instruction or completed physical conversion.
- [rust/tcl-registry/src/native_boolean_truth.rs](../../../../rust/tcl-registry/src/native_boolean_truth.rs), `InvocationDialect::native_numeric_instruction_result_protocol`: Select the actual retained C scalar engine for an independently reached numeric instruction, separately from a Boolean condition rewrite or public expression API copy.
- [rust/tcl-cmd-core/src/native_boolean_truth.rs](../../../../rust/tcl-cmd-core/src/native_boolean_truth.rs), `original_boolean_expression_result`: Perform only the selected reached original result stages through supplied original object/getter/publication operations, preserving first Host and independent original ownership; source interpretation supplies no execution authority.

No Rust assertion is attached to this source interpretation record. Current source definitions and any independently pinned implementation executions remain separate.

## Evidence

- [rust/tcl-registry/tests/data](../../../../rust/tcl-registry/tests/data)
- [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.4.20](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.4.20)
- [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.4.20/sources](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.4.20/sources)
- [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.5.19](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.5.19)
- [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.5.19/sources](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.5.19/sources)
- [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.6.18](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.6.18)
- [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.6.18/sources](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/8.6.18/sources)
- [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production)
- [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.0.4](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.0.4)
- [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.0.4/sources](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.0.4/sources)
- [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.1.0](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.1.0)
- [rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.1.0/sources](../../../../rust/tcl-registry/tests/data/current_optimizer_boolean_result_production/providers/9.1.0/sources)

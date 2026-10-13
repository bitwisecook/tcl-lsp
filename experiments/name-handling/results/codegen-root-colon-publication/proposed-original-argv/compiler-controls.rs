// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

// Proposed additions to native_lowering/tests.rs AFTER the additive ABI patch.
// Uncompiled and unexecuted. The existing helper keeps the actual VM owner alive;
// it attests entry registration only, not successful native compilation/execution.

fn defined_original_argv(function: &NativeFunction) -> Vec<(&str, &[super::ir::NativeValueId])> {
    all_ops(function)
        .into_iter()
        .filter_map(|op| match op {
            NativeOp::DefineProc { entry_key, argv } => Some((entry_key.as_str(), argv.as_slice())),
            _ => None,
        })
        .collect()
}

fn original_constant(function: &NativeFunction, id: super::ir::NativeValueId) -> &str {
    all_ops(function)
        .into_iter()
        .find_map(|op| match op {
            NativeOp::ConstStr { dst, text } if *dst == id => Some(text.as_str()),
            _ => None,
        })
        .expect("the admitted literal is transported by its already evaluated value")
}

#[test]
fn original_root_colon_definition_preserves_evaluated_publication_operand() {
    let source = "proc :audit {} {return ORIGINAL}\n:audit\n";
    for environment in ["tcl8.6", "tcl9.0"] {
        let (_actual_owner, context, unit) = original_boolean_unit(source, environment);
        let (function, report) = lower_original_boolean_unit(
            &unit,
            context.commands(),
            &unit.ir_module,
        )
        .expect("the fixed root declaration actually reaches native lowering");
        assert_eq!(
            outcomes(&report, "invoke"),
            vec![StatementOutcome::NativeDefinition, StatementOutcome::GenericInvoke],
            "a QName-only or wholly declined control cannot establish the transport",
        );
        let definitions = defined_original_argv(&function);
        let [(entry_key, argv)] = definitions.as_slice() else {
            panic!("one original definition must reach DefineProc: {definitions:?}");
        };
        assert_eq!(argv.len(), 4);
        assert_eq!(original_constant(&function, argv[0]), "proc");
        assert_eq!(original_constant(&function, argv[1]), ":audit");
        assert_eq!(original_constant(&function, argv[2]), "");
        assert_eq!(original_constant(&function, argv[3]), "return ORIGINAL");
        let point = unit.ir_module.source_entry.native_entry.as_ref()
            .and_then(|entry| entry.execution_point)
            .expect("the independently captured actual entry supplies its point");
        let protocol = tcl_registry::InvocationDialect::of_point(point)
            .native_name_protocol()
            .expect("the actual C86/C9 point selects its original protocol");
        let root = tcl_syntax::naming::NativeNameContext::root();
        let actual_publication = protocol.command_publication_slot(
            root,
            original_constant(&function, argv[1]).as_bytes(),
        ).expect("original literal publication recipe");
        let wrong_projection = protocol.command_publication_slot(
            root,
            entry_key.as_bytes(),
        ).expect("internal key used as new text is a separate publication input");
        assert_eq!(
            actual_publication,
            tcl_syntax::naming::ByteCommandSlot::new(
                tcl_syntax::naming::ByteNamespacePath::root(),
                tcl_syntax::naming::NameBytes::from(":audit"),
            ),
        );
        assert_ne!(actual_publication, wrong_projection);
    }
}

#[test]
fn original_substituted_name_retains_generic_evaluated_argv() {
    // The first declaration is literal. The second must not acquire an entry
    // merely because it eventually produces a name also found in a source map.
    let source = "set calls 0\nproc next_name {} {incr ::calls; return :audit}\n\
                  proc [next_name] {} {return COUNTED}\n:audit\n";
    for environment in ["tcl8.6", "tcl9.0"] {
        let (_actual_owner, context, unit) = original_boolean_unit(source, environment);
        let (function, _report) = lower_original_boolean_unit(
            &unit,
            context.commands(),
            &unit.ir_module,
        ).expect("the structural control builds without replacing admission");
        assert!(defined_original_argv(&function).iter().all(|(_, argv)| {
            original_constant(&function, argv[1]) != ":audit"
        }));
        // This checks IR selection only. Runtime/emitted-WASM tests below must
        // separately demonstrate that [next_name] runs exactly once.
    }
}

// Additional fixed controls for the same APIs, without changing entry/frame
// assumptions to force admission:
//
// 1. Missing evaluated vector or non-exact effective argv: lower_definition
//    returns None; lower_invoke retains its already-built GenericInvoke argv.
//    Do not fabricate a NativeCompilationEntry or erase an admission decline.
// 2. Shadowed definer:
//    rename proc saved_proc
//    saved_proc proc args {return SHADOW}
//    proc :audit {} {return ORIGINAL}
//    -> final command stays generic; no :audit declaration is manufactured.
// 3. Runtime named holder: create actual `holder`, enter its NsId, then call
//    ProcDefineOriginalArgv with caller-owned objects proc/:audit/{}/body and
//    entry=None. Calls to :audit in that actual holder succeed; `audit` and
//    root :audit remain absent. Keep objects alive through completion settle.
// 4. Runtime live prefix: create actual namespaces `:` and `:ns`; enter each
//    live NsId, and define the ordinary literal `audit`. Compare its actual
//    holder/report with the existing original per-release publication rows,
//    rather than parsing the holder's rendered name into a new namespace.
//    Also send `proc missing::audit {} body`: original name validation errors
//    before publication; no missing holder is created.
// 5. Once-only emitted-WASM control: run the substituted-name source and query
//    `list [:audit] $calls`; require `{COUNTED 1}` and absence of plain audit.
//    Assert only next_name received DefineProc and that generic leaf emission
//    calls the existing argv dispatcher once for the second declaration.
// 6. Unknown name: `proc $unknown_name {} {return NEVER}` with the variable
//    absent must preserve the original read error, install no procedure, and
//    never call the new definition ABI. A known replacement `proc` must stay
//    on the ordinary runtime dispatcher even when all operands are literals.

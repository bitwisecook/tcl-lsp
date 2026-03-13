# KCS: Analysis pass authoring checklist

## Use this when

Adding a new compiler pass or materially changing an existing one.

## Checklist

1. **Inputs**
   - Accept `CompilationUnit` or `FunctionUnit` facts where possible.
   - Avoid re-parsing/re-lowering inside the pass core.
2. **Outputs**
   - Emit typed findings with code/message/range.
   - Include related ranges for multi-site findings.
3. **Determinism**
   - Keep output ordering stable for repeatable tests.
4. **Suppression and severity**
   - Define code-family severity mapping in diagnostics layer.
   - Ensure `# noqa` suppression applies uniformly.
5. **Tests**
   - Add direct pass tests plus at least one diagnostics integration test.
   - Add fixture scripts for complex scenarios.
6. **Docs**
   - Update a relevant KCS note in `docs/kcs/compiler/`.
   - Link to tests that anchor expected behaviour.

## Related files

- `core/compiler/compilation_unit.py`
- `lsp/features/diagnostics.py`
- `tests/`

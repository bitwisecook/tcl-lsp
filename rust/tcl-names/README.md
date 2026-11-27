# tcl-names

Source-key descriptors for source analysis: the keys that name a namespace, a
command, a source instance and a variable cell. They intern to
`tcl-core-types`' `*Ref` identities and carry no lookup, execution or binding
authority.

- `command`: command identity and allocation (`CommandIdentity`,
  `CommandAllocationSite`, `CommandAllocation`, `AllocationIncarnation`,
  `RuntimeCommandTokenIdentity`, `CommandRefInterner`), the command-table key
  `SourceCommandKey` and the binding lattice element `BindingKind`.
- `namespace`: `SourceNamespaceKey` and the signature namespace geometry
  `SignatureNamespaceScope`.
- `origin`: `SourceOriginId` and its kind, the source-point origin token, and
  `ExecutedScriptSource` with its mapping.
- `place`: the variable `Place` model, its cell identity and `overlap`.
- `variable`: `VariableCellKey` and its tables, `VariableExecutionFrame`,
  `RawBindingSlotId`, `CallableStorageIdentity` and `SourceObjectAllocation`.
- `word`: the sealed static naming word `SignatureSourceNameKey`.

Methods that reach binding stay in `tcl-compiler` as extension traits:
`SourceOriginLexical` and `ExecutedScriptWords` in `command_binding`,
`SignatureNamespaceChild` in `signature_scan::scope` and
`RawBindingSlotRelocation` in `raw_binding`.

The workspace dependencies are `tcl-core-types`, `tcl-dialect`, `tcl-lexer`,
`tcl-native-model`, `tcl-registry`, `tcl-runtime-api` and `tcl-syntax`; the
registry supplies `FrameLevel`, the F5 storage domains and the native compiler
word capture. Run `cargo test -p tcl-names`.

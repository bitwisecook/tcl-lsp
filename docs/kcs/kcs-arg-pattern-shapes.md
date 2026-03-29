# KCS: Argument pattern shapes

## Symptom

Need to add a new command to the C++ registry but unsure which
`ArgPattern` configuration to use for its argument layout.

## Operational context

`native/src/registry/sample_commands.cpp` contains working examples
of all 10 shape patterns.  `native/tests/test_command_registry.cpp`
has Catch2 tests verifying each.

## Decision rules / contracts

### Shape selection guide

| Your command looks like... | Use this pattern |
|---------------------------|-----------------|
| Fixed args at known positions | `FIXED` with explicit indices |
| Unlimited trailing args (same role) | `TAIL` from start index |
| Repeating pairs/triples | `STRIDE` with step size |
| Option values with semantic roles | `OptionDesc.value_role` |
| Keyword-driven layout (if/switch/try) | `LayoutResolver` enum |
| Getter/setter forms | `FormDesc` array |
| Subcommands with different options | `SubCmdDesc` array |
| Body is always the last arg | `FIXED` with `index = -1` |

### Common patterns

**"All args from N onward are variable names"** (lassign, scan, binary scan):
```cpp
{.kind = TAIL, .role = VAR_NAME, .index = 2}
```

**"Alternating pairs"** (foreach):
```cpp
{.kind = STRIDE, .role = VALUE, .index = 0, .stride = 2},  // even positions
{.kind = STRIDE, .role = VALUE, .index = 1, .stride = 2},  // odd positions
{.kind = FIXED,  .role = BODY,  .index = -1},               // last = body
```

**"Option -body carries a script"** (tcltest):
```cpp
OptionDesc{.name = "-body", .takes_value = true, .value_role = ArgRole::BODY}
```

**"Arg 0 is a list that will shimmer"** (lassign, llength, lindex):
```cpp
ArgTypeDesc{.index = 0, .expected = TclType::LIST, .shimmers = true}
```

## Gotchas

- Stride patterns don't apply to the last arg if it's also caught by
  a `FIXED` with `index = -1`.  The FIXED pattern overwrites the stride.
- `OPTION_VALUE` ArgPattern entries are mostly replaced by
  `OptionDesc.value_role` — prefer the latter for consistency.
- Don't use `LayoutResolver` unless the command's layout truly depends
  on keyword analysis at runtime.  Most commands work with static patterns.

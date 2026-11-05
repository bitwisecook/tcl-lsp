# naming.package.jim-source-operation-table

Kind: `source-anchor`

## Problem statement

Jim package list succeeds in the separately retained native observation while the unknown-operation error omits it. A Registry operation inventory needs a source explanation that distinguishes a hidden supported row from an absent C-family operation, without interpreting an error-message roster as the complete API.

## Question

Which operations and arities does the exact retained Jim package_command_table declare, does list share names implementation and carry the hidden flag, and does this table contain ifneeded?

## Conclusion

The retained source table contains forget, provide, require, list and names, followed by its NULL sentinel. list and names both select package_cmd_names with zero arguments; list carries JIM_MODFLAG_HIDDEN. There is no ifneeded row in this table. This pinned source explains the declared operation inventory and shared implementation pointer; it does not itself execute Jim or prove why a particular binary omits a row from its error output.

## Scope

Offline inspection of the exact complete jim-package.c source bytes at the recorded absolute provider path, with a retained full-source digest and LF-counted table excerpt. Source release, git revision, compiler configuration and source-to-binary correspondence are not recorded. The separate native CLI questions name the observed executable version; their patchlevel is not assigned to this unversioned source snapshot.

## Provider answers

### tcl8.4, tcl8.5, tcl8.6, tcl9.0, tcl9.1

Status: `not-tested`.

No observation for this question.

### jim

Status: `inspected`. Channel: Offline full-source bytes with LF-counted table excerpt.

The complete selected package_command_table declares forget (1..unbounded), provide (1..2), require (1..2), list (0) and names (0). list and names both use package_cmd_names; only list carries JIM_MODFLAG_HIDDEN. The table ends with its NULL sentinel and contains no ifneeded operation. This source inspection supplies no process outcome or source-to-binary proof.

### bigip

Status: `not-tested`.

No appliance observation for this question.

## Source inspection

jim not recorded, revision `not recorded; exact full-source SHA-256 retained`, `/workspace/.proofs/native-providers/jimtcl/jim-package.c`, function `package_command_table`, lines 231–276. Retained reference `source-table`.

```text
static const jim_subcmd_type package_command_table[] = {
    {
        "forget",
        "package ...",
        package_cmd_forget,
        1,
        -1,
        /* Description: Forget that the given packages were loaded */
    },
    {
        "provide",
        "name ?version?",
        package_cmd_provide,
        1,
        2,
        /* Description: Indicates that the current script provides the given package */
    },
    {
        "require",
        "name ?version?",
        package_cmd_require,
        1,
        2,
        /* Description: Loads the given package by looking in standard places */
    },
    {
        "list",
        NULL,
        package_cmd_names,
        0,
        0,
        JIM_MODFLAG_HIDDEN
        /* Description: Deprecated - Lists all known packages */
    },
    {
        "names",
        NULL,
        package_cmd_names,
        0,
        0,
        /* Description: Lists all known packages */
    },
    {
        NULL
    }
};

```

## Consumer bindings

- [rust/tcl-registry/src/commands/tcl/package_.rs](../../../../rust/tcl-registry/src/commands/tcl/package_.rs), `jim_spec`: Authored nearest-family Jim package catalogue retains list and names and excludes C ifneeded; this binding supplies no guest execution claim.
- [rust/tcl-registry/src/commands/tcl/package_.rs](../../../../rust/tcl-registry/src/commands/tcl/package_.rs), `commands::tcl::package_::tests::jim_package_inventory_keeps_hidden_list_and_excludes_ifneeded` (linked): The authored genuine Jim-point catalogue preserves the five declared operations, zero-argument list/names List metadata, and no inherited ifneeded body descriptor; no native process is executed by this test.

## Evidence

- [rust/tcl-registry/tests/data/native_jim_package_hidden_list/source/jim-package.c](../../../../rust/tcl-registry/tests/data/native_jim_package_hidden_list/source/jim-package.c)

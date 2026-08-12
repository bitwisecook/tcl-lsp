# Port of rust/tcl-registry/src/commands/tcllib/snit__type.rs *together with*
# the definition-body grammar it references,
# rust/tcl-registry/src/definer.rs:1536-1687 (`SNIT_GRAMMAR` and the four
# constants it is built from: SNIT_MEMBERS, SNIT_BUILTIN_OBJECT_METHODS,
# SNIT_MEMBER_BODY_COMMANDS, SNIT_MANUFACTURERS).
#
# Why this port exists: tricky-surfaces.md requires definer grammars and
# `body_scope` mini-vocabularies to be expressible "from day one", and the
# first draft of the DSL made `definition_body` reference-only.  Rather than
# amend the rubric, this port lands the construct — and, as the whole
# design-by-porting exercise predicts, the grammar fought back on three
# points that no sketch had anticipated:
#
#   1. `MemberSpec::arg_roles` is a list of (index, role) PAIRS, not a fixed
#      {name, params, body} triple.  The caveat section's sketched
#      `member method -name 0 -params 1 -body 2` needs one new flag word per
#      ArgRole and still cannot spell `onconfigure` (roles at 1 and 2, with
#      index 0 a bare option name that is neither).  `-roles {N ROLE …}` is
#      the field itself, and is what this port uses.
#   2. `bare_word_construction_hint` is a FUNCTION POINTER in the Rust, and
#      is the only such field in the grammar.  Reading it
#      (definer.rs:1651-1653) it is a two-clause predicate over one word: an
#      exact-match set plus a prefix set.  Declared as data here; a family
#      whose hint is not that shape keeps `-native`.
#   3. `builtin_object_methods` carries a per-entry visibility AND a
#      per-entry receiver, so it cannot be the flat name list that
#      `builtin_type_methods` is.  One row per method, `?-unexported?`
#      spelled exactly as on `manufacturer`.
#
# Fidelity: complete except as noted at each site.  Every field of
# `DefinitionBodyGrammar` (14) is either written below or is at its Rust
# default; see README's fidelity table.

speclib tcllib 1.0 {

command snit::type {
    # snit::type carries `dialects: None` in the .rs — deliberately
    # unrestricted, like uri::geturl's.  Absent, not `{all-tcl}`: the
    # required_package gate below is what actually confines it, and writing a
    # set here would wrongly claim the command is *unavailable* in a dialect
    # the shipped spec leaves open.
    traits {CREATES_BARRIER NEVER_INLINE_BODY CREATES_DYNAMIC_BARRIER}

    arity 2

    # Only the body is declared.  The shipped spec does NOT give index 0 an
    # ArgRole::Name — `arg_roles: &[(1, ArgRole::Body)]` — and the port does
    # not add one; the type name reaches the outline through the definer
    # machinery, not through a role.
    arg 1 -role Body

    # A separate definition scope, not enclosing-scope data flow — matches
    # oo::class's own Structural classification.
    body_kind Structural

    tcllib_package  snit
    required_package snit

    side_effect InterpState -writes

    form Default {snit::type name definition}

    hover {
        summary  {Define a new snit type (class).}
        synopsis {snit::type name definition}
        description {Defines a new object type. The definition body contains option, variable, constructor, destructor, method, and typemethod declarations.}
        source   {tcllib snit package}
        example {snit::type Dog {
    option -name Fido
    method bark {} { return "Woof!" }
}}
        # `return_value` is "" in the .rs — absent here, which is the same
        # thing (a HoverSnippet field's default is the empty string).
    }

    # -----------------------------------------------------------------
    # SNIT_GRAMMAR, inline.
    #
    # Written inline rather than as a pack-level
    # `descriptor definition_body snit-type { … }` because this pack
    # declares exactly one definer: `snit::widget` / `snit::widgetadaptor`
    # carry SNIT_WIDGET_GRAMMAR, a *different* constant (same members,
    # plus `win` / `hull` in implicit_vars and `installhull` in
    # member_body_commands).  A pack declaring all three writes the shared
    # part once as a descriptor and references it three times — the same
    # choice `world_effects class-factory-effects` makes in
    # oo-class.tclspec.tcl.
    # -----------------------------------------------------------------
    definition_body {
        family Snit

        # `method` / `typemethod` / `proc` share METHOD_ROLES; `proc` is a
        # type-private procedure with the same NAME PARAMS BODY shape.
        member method         -roles {0 Name 1 ParamList 2 Body}
        member typemethod     -roles {0 Name 1 ParamList 2 Body}
        member proc           -roles {0 Name 1 ParamList 2 Body}

        member constructor    -roles {0 ParamList 1 Body}
        member destructor     -roles {0 Body}
        member typeconstructor -roles {0 Body}

        # snit 1.x option handlers.  Index 0 is the option name in both and
        # carries no role in the shipped spec — which is exactly why the
        # sketched `-name/-params/-body` triple could not express them.
        member onconfigure    -roles {1 VarWrite 2 Body}
        member oncget         -roles {1 Body}

        # snit's `variable v ?value?` declares ONE name (VAR0_ROLES), unlike
        # TclOO's slot-valued `variable a b c`; the difference is real and is
        # the reason `-all-vars` is not written here.
        member variable       -roles {0 VarWrite}
        member typevariable   -roles {0 VarWrite}
        member component      -roles {0 VarWrite}
        member typecomponent  -roles {0 VarWrite}

        # Recognised keywords with nothing to recurse or declare.  A bare
        # `member KEYWORD` is `MemberSpec::keyword_only`.
        member option
        member delegate
        member expose

        implicit_vars {self selfns type options}

        # `member_body_namespace_path` is absent = `&[]`: snit member bodies
        # resolve bare words through the generated type namespace, with no
        # injected helper path (the `::oo::Helpers` fact is TclOO-only).

        # snit(n): "Every snit type has the following type methods: create,
        # info, destroy."  `create` is deliberately NOT listed — it is a
        # construction, not a typemethod call, and lives in `manufacturer`
        # below.
        builtin_type_methods {info destroy}

        # snit generates an ordinary Tcl dispatcher proc rather than using
        # TclOO's export machinery, so there is no unexported tier: every
        # row here is exported (no `-unexported`).
        builtin_object_method configure     -receiver AnyObject   -detail {set one or more of the instance's options}
        builtin_object_method configurelist -receiver AnyObject   -detail {set options from an option/value list}
        builtin_object_method cget          -receiver AnyObject   -detail {retrieve the value of one of the instance's options}
        builtin_object_method destroy       -receiver AnyObject   -detail {destroy the instance}
        builtin_object_method info          -receiver AnyObject   -detail {introspect the instance's type, options and components}
        builtin_object_method create        -receiver ClassObject -detail {construct an instance with the given name}

        # `builtin_terminating_methods` is absent = `&[]`.  TclOO's is
        # `{unknown}`; snit has no such method at all.

        # `install NAME using TYPE ?args…?` — available inside every snit
        # member body and nowhere else, so it is grammar data, not a
        # CommandSpec (a user's own `proc install` must not look like a
        # built-in everywhere).  `-binds-handle` reuses the `binds_handle`
        # flag spelling unchanged.  `installhull` is absent here on purpose:
        # a plain snit::type has no hull.
        member_body_command install -detail {install a snit component into its component variable} \
            -binds-handle {-name-from {Word 0} -class-from {Word 2} -keyword {1 using}}

        # snit(n), "The Type Command": `$type name ?args?` with a
        # non-typemethod first word means `$type create name ?args?`.  The
        # hint is the shipped `snit_bare_word_construction_hint`
        # (definer.rs:1651-1653) written as its two clauses: the exact word
        # `%AUTO%`, or any word starting with `.` (the conventional Tk
        # widget path).
        bare_word_construction -hint-values {%AUTO%} -hint-prefixes {.}

        # snit's wildcard `delegate method *` can install methods no member
        # declaration enumerates, so unknown-method diagnostics must abstain.
        dynamic_method_dispatch

        # snit(n), "The Type Command": `$type create name ?option value…?`.
        # There is no `new` — a snit instance is always named, either
        # explicitly or through `%AUTO%` in the name itself — and no
        # definition body, so `-definition-body-at` is absent.
        manufacturer create -names-instance-at 1 -constructor-args-from 2

        # `unknown_dispatch_method` is absent = None: snit routes an
        # unrecognised method through a declared `delegate method *`, which
        # is a delegation target rather than a member body, so there is no
        # counterpart of TclOO's `unknown`.
        #
        # `property_accessor_methods` is absent = `&[]`: snit's
        # `configure` / `cget` are on every instance of the family (above),
        # not generated from property declarations.
    }
}

}

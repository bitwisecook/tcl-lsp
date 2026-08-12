# Draft spec for three tcllib modules not present in
# rust/tcl-registry/src/commands/tcllib/ (checked exhaustively against all
# 206 files there — only the *generic-language* snit__type/snit__typemethod
# specs exist, which model snit::type's own definer grammar, not any
# specific snit-authored library). Source: github.com/tcltk/tcllib @
# 6093f8d6 (2026-08-07). struct::tree 2.1.3, struct::graph 2.4.4,
# fileutil::traverse 0.7 (versions per modules/*/pkgIndex.tcl).
#
# Exercises: a bare (non-TclOO) handle factory via `interp alias` --
# `creates_instance_at` + `object_class` together, the cleanest grounding
# for that pairing found in any corpus; ensemble-like dispatch via a
# hand-rolled naming convention with RUNTIME introspection, not a native
# ensemble (README §5 item 2); `sub_subcommands` reaching through
# `object_class` (a real third dispatch level: struct::graph's arc/node);
# a `foreach`-shaped loop-body method correctly distinguished from a
# CommandPrefix callback; a library-defined, non-standard completion code
# scoped to one command's body (documents G13); command-prefix options
# with different appended arities on the same snit type (a clean grounding
# for something already expressible); and an option-level
# immutable-after-construction flag with no backing field at all
# (documents G14).
#
# Full evidence and gap numbering: ../external/README.md

speclib tcllib 1.0 {

# ---------------------------------------------------------------------------
# struct::tree (tree_tcl.tcl:49-161). `tree ?name? ?=|:=|as|deserialize
# source?` installs `interp alias {} $name {} ::struct::tree::TreeProc
# $name` (tree_tcl.tcl:131) -- a textbook, maximally literal grounding for
# `creates_instance_at`/`command_table_effect: CreatesAliases`, more direct
# than any TclOO `new`/`create` example (those go through generic
# IS_OO_METACLASS machinery; this is the plain non-OO archetype
# `creates_instance_at`'s own doc comment names: "a namespace factory such
# as `report::report reportName columns ...`").
command struct::tree {
    arity 0..2
    tcllib_package struct::tree
    required_package struct::tree

    arg 0 -role Name  ;# tree instance name (auto-generated "treeN" if omitted)
    # `creates_instance_at` (binds arg 0's name to THIS command's own
    # object_class for method dispatch) is the stronger of the two name-
    # binding fields and supersedes `defines_command_at` ("lighter... the
    # bare 'the name is now a command' fact", fields.md) when object_class
    # is present, as it is here -- so only one is declared.
    creates_instance_at 0
    command_table_effect CreatesAliases

    hover {
        summary  {Create a new tree object.}
        synopsis {struct::tree ?name? ?=|:=|as|deserialize source?}
        description {Returns a dispatchable command bound to `name` (or an auto-generated "treeN") via `interp alias`. Dispatch is hand-rolled: TreeProc (tree_tcl.tcl:200-228) maps `$name cmd args` onto `::struct::tree::_$cmd` by naming convention, and an unrecognised `cmd` is reported by introspecting `info commands ::struct::tree::_*` at runtime rather than reading a declared table (tree_tcl.tcl:208-217) -- no native `namespace ensemble` anywhere in this package.}
        source   {tree_tcl.tcl:38-161}
    }

    # The class NAME is the statement's own word (ObjectClassSpec.class_name).
    # struct::tree is NOT a TclOO class -- the handle is an `interp alias`
    # onto a dispatcher proc -- so the class name is the factory command's
    # own name, no superclasses, and no unknown-method handler.  Because it
    # is not a class command, word 0 of a `struct::tree` call really is the
    # instance name and the `arity`/`arg` rows above stay on the command
    # itself; the manufacturer-row treatment the TclOO drafts need does not
    # apply here.
    object_class ::struct::tree {

        method insert {
            arity 2..
            detail   {Insert one or more new nodes as children of parentNode.}
            synopsis {$tree insert parentNode index ?child ...?}
            mutator
            arg 0 -role Value  ;# existing parent node
            arg 1 -role Index  ;# insertion position, or "end"
            return_type List
            hover { summary {Insert new child node(s) at a given position under parentNode.} source {tree_tcl.tcl:944-1017} }
        }

        method children {
            arity 0..
            detail   {List the immediate children of a node.}
            synopsis {$tree children ?-all|-leaves? node}
            pure
            return_type List
            hover { summary {Return node's children (or, with -all/-leaves, its whole subtree/its leaves).} source {tree_tcl.tcl:451-518} }
        }

        # A `foreach`-shaped loop body, confirmed by reading WalkCall
        # (tree_tcl.tcl:2090-2097): `upvar 2 $avar a; set a $action`,
        # `upvar 2 $nvar n; set n $node`, then `uplevel 2 $cmd`. This is
        # LoopVarList + Body(Plain), NOT CommandPrefix, even though "visit
        # every node" reads like a callback API from the outside -- a spec
        # author has to read the implementation to tell the two apart, and
        # both shapes are equally plausible a priori (fileutil::traverse's
        # options below really ARE CommandPrefix). No gap in the loop-body
        # half; the gap is `prune`, below.
        method walk {
            arity 2..7
            detail   {Walk the subtree rooted at node, running script once per visited node.}
            synopsis {$tree walk node ?-type bfs|dfs? ?-order pre|post|in|both? ?--? loopvar script}
            mutator

            option -type  -takes value -values {bfs dfs} -closed
            option -order -takes value -values {pre post in both} -closed
            option --

            # Node (arg 0) is fixed, but loopvar/script sit at whatever the
            # LAST TWO words are, after a variable-length run of options --
            # the same shape switch's own resolver handles
            # (switch.tclspec.tcl:28-46), so the same tool applies with no
            # new syntax. A resolver supersedes a static `arg_roles` table
            # entirely (per fields.md), so node's role is set here too,
            # not in a separate `arg 0 ...` statement. tree_tcl.tcl:
            # 1712-1721: loopvar is either "nodeVar" or "{actionVar
            # nodeVar}" (1 or 2 names, both VarWrite, bound fresh on every
            # visit); the last word is the loop body, uplevel'd 2 frames up
            # (WalkCall, tree_tcl.tcl:2090-2097).
            arg_role_resolver {words ctx} {
                set n [llength $words]
                if {$n < 3} return
                role 0 Value                       ;# root node to start from
                role [expr {$n - 2}] LoopVarList
                role [expr {$n - 1}] Body
            }
            body_kind Plain

            # The standard codes ARE sayable, as traits -- the authorable
            # half of the completion question (../README.md, "Why
            # `completion` is excluded and `const_fold` is not").  The
            # pairing runs in two directions: `walk` accepts a loop body,
            # so it carries HAS_LOOP_BODY; the commands that PERFORM the
            # break/continue carry BREAKS_LOOP / CONTINUES_LOOP on
            # themselves (rust/tcl-registry/src/commands/tcl/break_.rs:40),
            # not on the loop.
            traits {HAS_LOOP_BODY}

            # G13 is what is left over, and it is recorded, not written.
            # `struct::tree::prune` (tree_tcl.tcl:181-183) is `return -code
            # 5`, a Tcl completion code with NO standard name (fields.md's
            # `completion` vocabulary names only 0-4).  It is meaningful ONLY
            # inside `walk`'s own body -- WalkCall's `switch -exact -- $code
            # { ... 5 {... return -code continue ...} }`
            # (tree_tcl.tcl:2109-2134) is the only place code 5 means
            # anything.  Nothing pairs a library-defined code to the one
            # command whose body accepts it, the way BREAKS_LOOP /
            # CONTINUES_LOOP pair the standard codes to HAS_LOOP_BODY.
            #
            # The first draft of this file wrote an invented
            # `completion { codes … custom_code 5 … }` block here.  That was
            # the one thing a draft must not do: `completion` is an EXCLUDED
            # field, so syntax for it reads as a proposal to un-exclude it
            # however well marked.  Removed; the fact survives as a known
            # limit in ../README.md and as this comment.

            hover {
                summary  {Depth- or breadth-first traversal, running script once per node with loop variables bound.}
                description {script may call `break`/`continue` (ordinary loop control) or `struct::tree::prune` (skip this node's children; only legal outside post/in order — tree_tcl.tcl:2120-2124).}
                source   {tree_tcl.tcl:1698-1856}
            }
        }
    }
}

# ---------------------------------------------------------------------------
# struct::graph (graph_tcl.tcl:52-184, same bare-factory shape as
# struct::tree above -- `graph ?name? ...` / `GraphProc`). Included for two
# things: a genuine THIRD dispatch level, and a real doc/struct DRIFT this
# census surfaced independent of any external library.
#
# `$g arc <op> ...` / `$g node <op> ...` (graph_tcl.tcl:287 `_arc`, 1575
# `_node`) each dispatch dozens of their own `__arc_*`/`__node_*`
# operations (get/getall/keys/set/append/attr/move/setweight/...,
# graph_tcl.tcl:425-1050). `sub_subcommands` is documented *subcommand
# only* in fields.md, so declaring it inside the `method arc { ... }`
# block below (a SubCommand, being a method) is exactly on-label --
# confirms `sub_subcommands` reaches through `object_class`, which no
# existing ported example (all core-command, non-OO) shows.
#
# BUT: fields.md's prose says "Each [sub_subcommand] carries its own
# arity and documentation" -- trying to write `get`'s real arity (exactly
# 2: arc, key) surfaced that `SubSubCommand`
# (rust/tcl-registry/src/spec.rs:2281-2290) is `{name, detail, synopsis,
# dialects}` ONLY: no `arity` field exists at all, and no `pure`/`mutator`
# either (`sub_subcommands: &'static [SubSubCommand]` at spec.rs:2245 is a
# plain slice, not paired with any arity/trait side-table). fields.md's
# own description overclaims what the struct can currently hold -- a real
# drift between docs and implementation, found only by trying to use the
# field for genuine third-party evidence. New finding, tagged G16 in
# ../external/README.md (not really an "external library" finding at
# all, but the census method surfaced it). `arity`/`pure`/`mutator` are
# therefore NOT written on the `sub_subcommand` rows below, even though
# every one of these ~15 ops has a real, known, fixed arity
# (graph_tcl.tcl:425-1050) — there is simply nowhere to put it today.
command struct::graph {
    arity 0..2
    tcllib_package struct::graph
    required_package struct::graph
    arg 0 -role Name  ;# graph instance name (auto-generated "graphN" if omitted)
    creates_instance_at 0
    command_table_effect CreatesAliases

    hover { summary {Create a new graph object (nodes + arcs, both with attribute dicts).} source {graph_tcl.tcl:17-184} }

    object_class ::struct::graph {

        method arc {
            arity 1..
            detail   {Operate on an arc (edge) of the graph.}
            synopsis {$g arc op ?arg ...?}
            mutator
            # `sub_subcommands` is a list of rows, so it gets a singular ROW
            # statement, exactly like `options` -> `option`; the first draft
            # wrapped these in a `sub_subcommands { … }` block, which the
            # ratified grammar does not use.
            sub_subcommand get       -detail {Get one attribute value. Real arity: exactly 2 (arc, key) — see G16, no field to say so.} -synopsis {$g arc get arc key}
            sub_subcommand getall    -detail {Get all attributes as a dict. Real arity: 1..2.} -synopsis {$g arc getall arc ?pattern?}
            sub_subcommand set       -detail {Set one attribute value. Real arity: 2..3. MUTATES — no field to say so either (no pure/mutator on SubSubCommand).} -synopsis {$g arc set arc key ?value?}
            sub_subcommand attr      -detail {Get/set/list a named attribute across every arc.} -synopsis {$g arc attr key ?arg ...?}
            sub_subcommand move      -detail {Change an arc's source and/or target node. Mutates.} -synopsis {$g arc move arc ?newsource? ?newtarget?}
            sub_subcommand setweight -detail {Set an arc's weight. Mutates.} -synopsis {$g arc setweight arc weight}
            hover { summary {Arc sub-ensemble: get/getall/keys/set/append/attr/move/setweight/... (graph_tcl.tcl:425-1050 for the full ~15-op set).} source {graph_tcl.tcl:287-424} }
        }

        method node {
            arity 1..
            detail   {Operate on a node of the graph.}
            synopsis {$g node op ?arg ...?}
            mutator
            # Same G16 limitation as arc's sub_subcommands above: real
            # arity/purity facts exist (graph_tcl.tcl:1756-1834) but
            # SubSubCommand has nowhere to hold them.
            sub_subcommand get    -detail {Get one attribute value. Real arity: exactly 2.} -synopsis {$g node get node key}
            sub_subcommand getall -detail {Get all attributes as a dict. Real arity: 1..2.} -synopsis {$g node getall node ?pattern?}
            sub_subcommand keys   -detail {List attribute names. Real arity: 1..2.} -synopsis {$g node keys node ?pattern?}
            hover { summary {Node sub-ensemble, same shape as arc.} source {graph_tcl.tcl:1575-1834} }
        }
    }
}

# ---------------------------------------------------------------------------
# fileutil::traverse (traverse.tcl:23-282) -- a snit::type, not TclOO. The
# generic `snit::type` definer grammar is ALREADY modelled
# (rust/tcl-registry/src/commands/tcllib/snit__type.rs: definition_body:
# Some(&SNIT_GRAMMAR)) -- folding/navigation inside this file's body work
# today with no registry change. What's missing is data ABOUT this specific
# type: its 3 options (each a documented CommandPrefix with a different
# appended arity) and its 3 methods.
command fileutil::traverse {
    # A snit::type, so its type command manufactures through `create` (and
    # through snit's bare-instance-name shorthand, which `SNIT_GRAMMAR`
    # models as `bare_word_construction`).  Word 0 of a call is therefore
    # the instance name or the `create` keyword, never the base directory:
    # the constructor's shape belongs on `subcommand create`, with a
    # `manufacturer` row saying where the constructor's own arguments start.
    # The first draft put `arg 0 -role Value` (the base directory) on the
    # type command itself, which reads `%AUTO%` as the directory.
    arity 1..
    allow_unknown_subcommands
    tcllib_package fileutil::traverse
    required_package fileutil::traverse

    # snit(n), "The Type Command": `$type create name ?option value…?`.
    # There is no `new` -- a snit instance is always named.  Identical to
    # SNIT_MANUFACTURERS in the shipped grammar; see ../snit-type.tclspec.tcl.
    manufacturer create -names-instance-at 1 -constructor-args-from 2

    subcommand create {
        arity 2..
        detail   {Construct a traverser.}
        synopsis {::fileutil::traverse create name basedir ?-filter cmd? ?-prefilter cmd? ?-errorcmd cmd?}
        arg 0 -role Name   ;# instance name, or %AUTO%
        arg 1 -role Value  ;# base directory to traverse (traverse.tcl:99)
    }

    # traverse.tcl:63-77 (the corpus's own doc comment, quoted almost
    # verbatim -- these three appended arities are stated explicitly in
    # the source, not inferred): "-prefilter ... invoked with a single
    # argument, the path"; "-filter ... invoked with a single argument,
    # the path"; "-errorcmd ... invoked with a two arguments, the path ...
    # and the error message."
    # traverse.tcl:95-97: `option -filter -default {} -readonly 1` (and
    # likewise -prefilter, -errorcmd) -- snit enforces these read-only
    # after construction; `configure`ing them later is an error. `-readonly`
    # below is DSL GAP (G14) -- INVENTED, and unlike most gaps in this
    # census, genuinely UNBACKED: `OptionSpec`
    # (rust/tcl-registry/src/hover.rs:380-415, read in full) has fields for
    # name/value/detail/dialects/aliases/lifecycle/min_abbrev and nothing
    # about post-construction mutability, so there is no field for a real
    # DSL to lower `-readonly` into — written on all three options anyway,
    # to show the shape an author would want.
    option -prefilter -takes cmdPrefix -role CommandPrefix -appends {Exactly 1} -readonly \
        -detail {Run for each directory; a false result skips recursing into it.}
    option -filter -takes cmdPrefix -role CommandPrefix -appends {Exactly 1} -readonly \
        -detail {Run for each path; a false result excludes it from the results.}
    option -errorcmd -takes cmdPrefix -role CommandPrefix -appends {Exactly 2} -readonly \
        -detail {Run on any path the traverser has trouble with (stat/cd failures, filter errors).}

    hover {
        summary  {Incremental or bulk filesystem traversal with pluggable filters.}
        synopsis {::fileutil::traverse create %AUTO% basedir ?-filter cmd? ?-prefilter cmd? ?-errorcmd cmd?}
        description {snit::type (traverse.tcl:23); options are construction-only (read-only). Depth-first pre-order (traverse.tcl:82).}
        source   {traverse.tcl:1-100}
    }

    object_class ::fileutil::traverse {

        method next {
            arity 1
            detail   {Advance to the next path; write it into fvar.}
            synopsis {$t next fvar}
            mutator
            arg 0 -role VarWrite  ;# written with the next path when found
            return_type Boolean
            hover { summary {Advance the incremental iterator; returns false once exhausted.} source {traverse.tcl:148-242} }
        }

        method foreach {
            arity 2
            detail   {Run body once per path found, with fvar bound to each.}
            synopsis {$t foreach fvar body}
            mutator
            arg 0 -role VarWrite
            arg 1 -role Body -body-kind Plain
            hover { summary {Convenience loop wrapper around next.} source {traverse.tcl:111-147} }
        }

        method files {
            arity 0
            detail   {Return every remaining path as one list.}
            synopsis {$t files}
            mutator
            return_type List
            hover { summary {Drain the traversal into a list (non-incremental use).} source {traverse.tcl:105-110} }
        }
    }
}

}

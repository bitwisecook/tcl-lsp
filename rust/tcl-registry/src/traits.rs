//! Behavioural trait flags for commands.
//!
//! Each bit replaces one `bool` field from the Python `CommandSpec`.
//! Consumers query traits via `spec.traits.contains(Traits::CONTROL_FLOW)`
//! instead of matching on command name strings.

use bitflags::bitflags;

bitflags! {
    /// Declarative behavioural traits for a command.
    ///
    /// Packed into a single `u64` — compact storage, fast intersection
    /// and containment queries on a single spec. Whole-registry
    /// trait-membership queries
    /// ([`crate::registry::CommandRegistry::commands_with_trait`])
    /// scan the spec table in O(N) today; a precomputed trait index
    /// is a future optimisation.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Traits: u64 {
        // Control flow
        /// Command is a control-flow construct (`if`, `for`, `while`, `switch`).
        const CONTROL_FLOW              = 1 << 0;
        /// Language keyword for semantic token classification.
        const LANGUAGE_KEYWORD          = 1 << 1;
        /// First expression argument is in boolean context (`if`, `while`, `for`).
        const HAS_BOOLEAN_COND          = 1 << 2;
        /// Unconditionally terminates the current block (`error`, `return`, `exit`).
        const TERMINATES_BLOCK          = 1 << 3;

        // Loop/body structure
        /// Contains a loop body (`for`, `while`, `foreach`).
        const HAS_LOOP_BODY             = 1 << 4;
        /// Forbid inlining body arguments.
        const NEVER_INLINE_BODY         = 1 << 5;
        /// CFG header with list-expression args evaluated once (`foreach`, `lmap`).
        const LOOP_LIST_HEADER          = 1 << 6;

        // Purity and optimisation
        /// Side-effect-free command.
        const PURE                      = 1 << 7;
        /// Candidate for common subexpression elimination.
        const CSE_CANDIDATE             = 1 << 8;
        /// Pure evaluation (`expr` — side-effect-free when braced).
        const PURE_EVALUATION           = 1 << 9;

        // Variable semantics
        /// Defines a procedure (`proc`).
        const DEFINES_PROCEDURE         = 1 << 10;
        /// Destroys/removes a variable (`unset`).
        const DESTROYS_VARIABLE         = 1 << 11;
        /// Reads the target variable before writing (`incr`, `append`, `lappend`).
        const READS_BEFORE_WRITE        = 1 << 12;
        /// Creates a scope alias — upvar-like binding (`upvar`, `global`, `variable`).
        const CREATES_SCOPE_ALIAS       = 1 << 13;
        /// Creates a runtime control-flow barrier (`eval`, `uplevel`, `upvar`).
        const CREATES_BARRIER           = 1 << 14;

        // Analysis check dispatch
        /// Evaluates code dynamically (`eval`, `uplevel`).
        const EVALUATES_CODE            = 1 << 15;
        /// Performs backslash/variable substitution (`subst`).
        const PERFORMS_SUBSTITUTION      = 1 << 16;
        /// Opens a channel (`open`).
        const OPENS_CHANNEL             = 1 << 17;
        /// Sources a file (`source`).
        const SOURCES_FILE              = 1 << 18;
        /// Has a switch body (`switch`).
        const HAS_SWITCH_BODY           = 1 << 19;
        /// String/list confusion risk (`append`).
        const STRING_LIST_CONFUSION     = 1 << 20;
        /// Configures a channel (`fconfigure`, `chan configure`).
        const CONFIGURES_CHANNEL        = 1 << 21;
        /// Has `interp eval` subcommand.
        const HAS_INTERP_EVAL           = 1 << 22;
        /// Has destructive operations (`file delete`, `namespace delete`).
        const HAS_DESTRUCTIVE_OPS       = 1 << 23;
        /// iRules event handler (`when`).
        const IS_EVENT_HANDLER          = 1 << 24;
        /// Returns unnormalised HTTP path/URI/query.
        const UNNORMALISED_HTTP_GETTER  = 1 << 25;

        // Output/value traits
        /// Returns a filesystem path (`pwd`, `file join`).
        const RETURNS_PATH              = 1 << 26;
        /// Performs unescaping/decoding (`subst`, `URI::decode`).
        const IS_UNESCAPE              = 1 << 27;
        /// Produces a canonical Tcl list (`list`, `concat`).
        const PRODUCES_CANONICAL_LIST   = 1 << 28;

        // Safety
        /// Inherently dangerous command.
        const UNSAFE                    = 1 << 29;
        /// Warn about option injection without `--` terminator.
        const WARN_WITHOUT_TERMINATOR   = 1 << 30;
        /// Password option command.
        const PASSWORD_OPTION           = 1 << 31;

        // iRules-specific
        /// Side-switching command (`clientside`/`serverside`).
        const IS_SIDE_SWITCH            = 1 << 32;
        /// Must appear at iRules top level (`proc`, `when`, `timing`).
        const IRULES_TOP_LEVEL_ONLY     = 1 << 33;
        /// TclOO metaclass (`oo::class`, `oo::abstract`).
        const IS_OO_METACLASS           = 1 << 34;

        // Codegen/diagram
        /// Included in diagram extraction.
        const DIAGRAM_ACTION            = 1 << 35;
        /// Needs `startCommand` bytecode instruction.
        const NEEDS_START_CMD           = 1 << 36;

        // Taint
        /// Command is a taint sink (absorbs tainted data).
        const TAINT_SINK                = 1 << 37;
    }
}

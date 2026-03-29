// Sample command descriptors demonstrating all 10 shape patterns.
// These serve as both documentation and test fixtures.

#include "tcl_lsp/registry/command_desc.hpp"

namespace tcl_lsp::sample_commands {

using enum ArgPatternKind;
using enum ArgRole;

// ════════════════════════════════════════════════════════════════════
// Shape 1: Fixed positional — proc name argList body
// ════════════════════════════════════════════════════════════════════

inline constexpr ArgPattern proc_args[] = {
    {.kind = FIXED, .role = NAME,       .index = 0},
    {.kind = FIXED, .role = PARAM_LIST, .index = 1},
    {.kind = FIXED, .role = BODY,       .index = 2},
};

inline constexpr CommandDesc proc_cmd{
    .name = "proc",
    .arity = {3, 3},
    .arg_patterns = proc_args,
    .never_inline_body = true,
    .defines_procedure = true,
    .is_scope_barrier = true,
};

// ════════════════════════════════════════════════════════════════════
// Shape 2: Unlimited tail — lassign list ?varName ...?
// FIXES THE dict[int, ArgRole] BUG: no more hardcoding to index 19.
// ════════════════════════════════════════════════════════════════════

inline constexpr ArgPattern lassign_args[] = {
    {.kind = FIXED, .role = VALUE,    .index = 0},
    {.kind = TAIL,  .role = VAR_NAME, .index = 1},
};

inline constexpr ArgTypeDesc lassign_types[] = {
    {.index = 0, .expected = TclType::LIST, .shimmers = true},
};

inline constexpr CommandDesc lassign_cmd{
    .name = "lassign",
    .arity = {2},
    .arg_patterns = lassign_args,
    .arg_types = lassign_types,
    .return_type = TclType::LIST,
};

// ════════════════════════════════════════════════════════════════════
// Shape 3: Stride pattern — foreach varList list ?varList list ...? body
// ════════════════════════════════════════════════════════════════════

inline constexpr ArgPattern foreach_args[] = {
    {.kind = STRIDE, .role = VALUE, .index = 0, .stride = 2}, // varLists at 0,2,4...
    {.kind = STRIDE, .role = VALUE, .index = 1, .stride = 2}, // lists at 1,3,5...
    {.kind = FIXED,  .role = BODY,  .index = -1},             // last arg = body
};

inline constexpr CommandDesc foreach_cmd{
    .name = "foreach",
    .arity = {.min = 3, .max = std::numeric_limits<std::int32_t>::max(),
              .modulus = 2, .remainder = 1},
    .arg_patterns = foreach_args,
    .is_control_flow = true,
    .has_loop_body = true,
    .loop_list_header = true,
};

// ════════════════════════════════════════════════════════════════════
// Shape 4: Options with value-role mapping — tcltest::test
// ════════════════════════════════════════════════════════════════════

inline constexpr OptionDesc tcltest_options[] = {
    {.name = "-body",    .takes_value = true, .value_role = BODY},
    {.name = "-setup",   .takes_value = true, .value_role = BODY},
    {.name = "-cleanup", .takes_value = true, .value_role = BODY},
    {.name = "-result",  .takes_value = true},
    {.name = "-match",   .takes_value = true, .value_hint = "mode"},
    {.name = "-output",  .takes_value = true},
    {.name = "-errorOutput", .takes_value = true},
    {.name = "-returnCodes", .takes_value = true},
    {.name = "-constraints",  .takes_value = true},
};

inline constexpr ArgPattern tcltest_args[] = {
    {.kind = FIXED, .role = NAME,  .index = 0},
    {.kind = FIXED, .role = VALUE, .index = 1},
};

inline constexpr CommandDesc tcltest_cmd{
    .name = "tcltest::test",
    .arity = {2},
    .arg_patterns = tcltest_args,
    .layout_resolver = LayoutResolver::TCLTEST,
    .options = tcltest_options,
    .required_package = "tcltest",
};

// ════════════════════════════════════════════════════════════════════
// Shape 5: Options with flags, values, and terminator — regexp
// ════════════════════════════════════════════════════════════════════

inline constexpr OptionDesc regexp_options[] = {
    {.name = "-nocase"},
    {.name = "-expanded"},
    {.name = "-line"},
    {.name = "-lineanchor"},
    {.name = "-linestop"},
    {.name = "-about"},
    {.name = "-indices"},
    {.name = "-all"},
    {.name = "-inline"},
    {.name = "-start", .takes_value = true, .value_hint = "index"},
    {.name = "--", .is_terminator = true},
};

inline constexpr ArgPattern regexp_args[] = {
    {.kind = FIXED, .role = PATTERN,  .index = 0},
    {.kind = FIXED, .role = VALUE,    .index = 1},
    {.kind = TAIL,  .role = VAR_NAME, .index = 2},
};

inline constexpr CommandDesc regexp_cmd{
    .name = "regexp",
    .arity = {2},
    .arg_patterns = regexp_args,
    .options = regexp_options,
    .warn_without_terminator = true,
    .pattern_type = PatternType::REGEX,
};

// ════════════════════════════════════════════════════════════════════
// Shape 6: Subcommands with per-subcommand options — string
// ════════════════════════════════════════════════════════════════════

inline constexpr OptionDesc string_compare_opts[] = {
    {.name = "-nocase"},
    {.name = "-length", .takes_value = true, .value_hint = "int"},
};

inline constexpr OptionDesc string_map_opts[] = {
    {.name = "-nocase"},
};

inline constexpr OptionDesc string_is_opts[] = {
    {.name = "-strict"},
    {.name = "-failindex", .takes_value = true, .value_role = VAR_NAME},
};

inline constexpr ArgTypeDesc string_length_types[] = {
    {.index = 0, .expected = TclType::STRING},
};

inline constexpr SubCmdDesc string_subcmds[] = {
    {.name = "compare", .arity = {2}, .options = string_compare_opts,
     .pure = true, .return_type = TclType::INT},
    {.name = "equal", .arity = {2}, .options = string_compare_opts,
     .pure = true, .return_type = TclType::BOOLEAN},
    {.name = "length", .arity = {1, 1}, .arg_types = string_length_types,
     .pure = true, .return_type = TclType::INT},
    {.name = "map", .arity = {2, 2}, .options = string_map_opts,
     .pure = true},
    {.name = "is", .arity = {2, 2}, .options = string_is_opts,
     .pure = true, .return_type = TclType::BOOLEAN},
    {.name = "match", .arity = {2, 2}, .options = string_map_opts,
     .pure = true, .return_type = TclType::BOOLEAN,
     .pattern_type = PatternType::GLOB},
    {.name = "range", .arity = {3, 3}, .pure = true},
    {.name = "repeat", .arity = {2, 2}, .pure = true},
    {.name = "replace", .arity = {3, 4}, .pure = true},
    {.name = "reverse", .arity = {1, 1}, .pure = true},
    {.name = "tolower", .arity = {1, 3}, .pure = true},
    {.name = "toupper", .arity = {1, 3}, .pure = true},
    {.name = "totitle", .arity = {1, 3}, .pure = true},
    {.name = "trim", .arity = {1, 2}, .pure = true},
    {.name = "trimleft", .arity = {1, 2}, .pure = true},
    {.name = "trimright", .arity = {1, 2}, .pure = true},
    {.name = "cat", .arity = {0}, .pure = true},
};

inline constexpr CommandDesc string_cmd{
    .name = "string",
    .subcommands = string_subcmds,
};

// ════════════════════════════════════════════════════════════════════
// Shape 7: Getter/Setter forms — HTTP::uri
// ════════════════════════════════════════════════════════════════════

inline constexpr FormDesc http_uri_forms[] = {
    {.kind = FormKind::GETTER, .arity = {0, 0}, .pure = true},
    {.kind = FormKind::SETTER, .arity = {1, 1}, .mutator = true},
};

inline constexpr CommandDesc http_uri_cmd{
    .name = "HTTP::uri",
    .forms = http_uri_forms,
    .dialects = DialectFlags::IRULES,
    .event_requires = {.transport = Transport::TCP,
                       .profiles = ProfileFlags::HTTP | ProfileFlags::FASTHTTP},
};

// ════════════════════════════════════════════════════════════════════
// Shape 8: Variable-layout — if expr ?then? body ...
// ════════════════════════════════════════════════════════════════════

inline constexpr std::string_view if_keywords[] = {"then", "else", "elseif"};

inline constexpr CommandDesc if_cmd{
    .name = "if",
    .arity = {2},
    .layout_resolver = LayoutResolver::IF,
    .is_control_flow = true,
    .never_inline_body = true,
    .formatting_keywords = if_keywords,
};

// ════════════════════════════════════════════════════════════════════
// Shape 9: TclOO — oo::define
// ════════════════════════════════════════════════════════════════════

inline constexpr OptionDesc method_vis_opts[] = {
    {.name = "-export"},
    {.name = "-unexport"},
    {.name = "-private"},
};

inline constexpr ArgPattern method_args[] = {
    {.kind = FIXED, .role = NAME,       .index = 0},
    {.kind = FIXED, .role = PARAM_LIST, .index = 1},
    {.kind = FIXED, .role = BODY,       .index = 2},
};

inline constexpr ArgPattern constructor_args[] = {
    {.kind = FIXED, .role = PARAM_LIST, .index = 0},
    {.kind = FIXED, .role = BODY,       .index = 1},
};

inline constexpr ArgPattern destructor_args[] = {
    {.kind = FIXED, .role = BODY, .index = 0},
};

inline constexpr OptionDesc slot_ops[] = {
    {.name = "-clear"},
    {.name = "-set",         .takes_value = true},
    {.name = "-append",      .takes_value = true},
    {.name = "-remove",      .takes_value = true},
    {.name = "-prepend",     .takes_value = true},
    {.name = "-appendifnew", .takes_value = true},
};

inline constexpr SubCmdDesc oo_define_subcmds[] = {
    {.name = "method",      .arity = {3, 4}, .arg_patterns = method_args,
     .options = method_vis_opts, .defines_procedure = true},
    {.name = "constructor", .arity = {2, 2}, .arg_patterns = constructor_args,
     .defines_procedure = true},
    {.name = "destructor",  .arity = {1, 1}, .arg_patterns = destructor_args,
     .defines_procedure = true},
    {.name = "superclass",  .arity = {0}, .options = slot_ops,
     .is_slot_command = true},
    {.name = "mixin",       .arity = {0}, .options = slot_ops,
     .is_slot_command = true},
    {.name = "filter",      .arity = {0}, .options = slot_ops,
     .is_slot_command = true},
    {.name = "variable",    .arity = {0}, .options = slot_ops,
     .is_slot_command = true},
    {.name = "forward",     .arity = {2}},
    {.name = "export",      .arity = {1}},
    {.name = "unexport",    .arity = {1}},
    {.name = "deletemethod", .arity = {1}},
    {.name = "renamemethod", .arity = {2, 2}},
};

inline constexpr CommandDesc oo_define_cmd{
    .name = "oo::define",
    .arity = {2},
    .layout_resolver = LayoutResolver::OO_DEFINE,
    .subcommands = oo_define_subcmds,
    .dialects = DialectFlags::TCL86 | DialectFlags::TCL90,
};

// ════════════════════════════════════════════════════════════════════
// Shape 10: Simple iRules — llookup MMAP KEY
// ════════════════════════════════════════════════════════════════════

inline constexpr ArgTypeDesc llookup_types[] = {
    {.index = 0, .expected = TclType::LIST, .shimmers = true},
};

inline constexpr CommandDesc llookup_cmd{
    .name = "llookup",
    .arity = {2, 2},
    .dialects = DialectFlags::IRULES,
    .pure = true,
    .arg_types = llookup_types,
    .return_type = TclType::LIST,
};

// ════════════════════════════════════════════════════════════════════
// Additional commands needed for tests
// ════════════════════════════════════════════════════════════════════

// when EVENT ?priority N? ?timing enable|disable? body
inline constexpr CommandDesc when_cmd{
    .name = "when",
    .arity = {2, 6},
    .layout_resolver = LayoutResolver::WHEN,
    .dialects = DialectFlags::IRULES,
    .is_control_flow = true,
    .never_inline_body = true,
    .is_scope_barrier = true,
    .top_level_only = true,
};

// binary scan string formatString ?varName ...?
inline constexpr ArgPattern binary_scan_args[] = {
    {.kind = FIXED, .role = VALUE,         .index = 0},
    {.kind = FIXED, .role = FORMAT_STRING, .index = 1},
    {.kind = TAIL,  .role = VAR_NAME,      .index = 2},
};

inline constexpr ArgTypeDesc binary_scan_types[] = {
    {.index = 0, .expected = TclType::BYTEARRAY},
};

inline constexpr SubCmdDesc binary_subcmds[] = {
    {.name = "scan", .arity = {2}, .arg_patterns = binary_scan_args,
     .arg_types = binary_scan_types, .return_type = TclType::INT},
    {.name = "format", .arity = {1}, .return_type = TclType::BYTEARRAY},
};

inline constexpr CommandDesc binary_cmd{
    .name = "binary",
    .subcommands = binary_subcmds,
    .format_string_type = FormatType::BINARY,
};

// switch — variable layout
inline constexpr OptionDesc switch_options[] = {
    {.name = "-exact"},
    {.name = "-glob"},
    {.name = "-regexp"},
    {.name = "-nocase"},
    {.name = "-matchvar", .takes_value = true, .value_role = VAR_NAME},
    {.name = "-indexvar", .takes_value = true, .value_role = VAR_NAME},
    {.name = "--", .is_terminator = true},
};

inline constexpr CommandDesc switch_cmd{
    .name = "switch",
    .arity = {2},
    .layout_resolver = LayoutResolver::SWITCH,
    .options = switch_options,
    .is_control_flow = true,
    .never_inline_body = true,
};

// try — variable layout
inline constexpr CommandDesc try_cmd{
    .name = "try",
    .arity = {1},
    .layout_resolver = LayoutResolver::TRY,
    .is_control_flow = true,
    .never_inline_body = true,
};

// set varName ?value?
inline constexpr ArgPattern set_args[] = {
    {.kind = FIXED, .role = VAR_NAME, .index = 0},
    {.kind = FIXED, .role = VALUE,    .index = 1},
};

inline constexpr CommandDesc set_cmd{
    .name = "set",
    .arity = {1, 2},
    .arg_patterns = set_args,
    .assigns_variable_at = 0,
};

// eval arg ?arg ...?
inline constexpr ArgPattern eval_args[] = {
    {.kind = TAIL, .role = BODY, .index = 0},
};

inline constexpr CommandDesc eval_cmd{
    .name = "eval",
    .arity = {1},
    .arg_patterns = eval_args,
    .creates_dynamic_barrier = true,
    .unsafe = true,
    .taint_sink = true,
};

// foreach_in_collection varName collection body (EDA)
inline constexpr ArgPattern foreach_in_collection_args[] = {
    {.kind = FIXED, .role = VAR_NAME, .index = 0},
    {.kind = FIXED, .role = VALUE,    .index = 1},
    {.kind = FIXED, .role = BODY,     .index = 2},
};

inline constexpr CommandDesc foreach_in_collection_cmd{
    .name = "foreach_in_collection",
    .arity = {3, 3},
    .arg_patterns = foreach_in_collection_args,
    .dialects = DialectFlags::EDA_SYNOPSYS | DialectFlags::EDA_CADENCE,
    .is_control_flow = true,
    .has_loop_body = true,
    .loop_list_header = true,
};

// ════════════════════════════════════════════════════════════════════
// Master table of all sample commands
// ════════════════════════════════════════════════════════════════════

inline constexpr const CommandDesc* all_sample_commands[] = {
    &proc_cmd,
    &lassign_cmd,
    &foreach_cmd,
    &tcltest_cmd,
    &regexp_cmd,
    &string_cmd,
    &http_uri_cmd,
    &if_cmd,
    &oo_define_cmd,
    &llookup_cmd,
    &when_cmd,
    &binary_cmd,
    &switch_cmd,
    &try_cmd,
    &set_cmd,
    &eval_cmd,
    &foreach_in_collection_cmd,
};

} // namespace tcl_lsp::sample_commands

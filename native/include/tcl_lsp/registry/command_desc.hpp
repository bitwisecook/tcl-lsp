#pragma once

#include <cstdint>
#include <limits>
#include <span>
#include <string_view>

namespace tcl_lsp {

// ─── Argument roles ────────────────────────────────────────────────
// What role an argument plays in a command.
// Extends the existing ArgRole in command_interface.hpp with the same values.
enum class ArgRole : std::uint8_t {
    BODY,
    EXPR,
    VAR_NAME,
    VAR_READ,
    PARAM_LIST,
    NAME,
    PATTERN,
    OPTION,
    VALUE,
    SUBCOMMAND,
    OPTION_TERMINATOR,
    CHANNEL,
    INDEX,
    FORMAT_STRING,
    LOOP_VAR,
};

// ─── Tcl internal representations ──────────────────────────────────
enum class TclType : std::uint8_t {
    STRING,
    INT,
    DOUBLE,
    BOOLEAN,
    LIST,
    DICT,
    BYTEARRAY,
    NUMERIC, // abstract join of INT and DOUBLE
    OBJECT,  // TclOO object instance
};

// ─── Side effect taxonomy ──────────────────────────────────────────
enum class SideEffectTarget : std::uint8_t {
    VARIABLE,
    SESSION_TABLE,
    PERSISTENCE_TABLE,
    DATA_GROUP,
    HTTP_HEADER,
    HTTP_BODY,
    HTTP_STATUS,
    HTTP_URI,
    HTTP_COOKIE,
    HTTP_METHOD,
    HTTP2_STATE,
    RESPONSE_COMMIT,
    CONNECTION_CONTROL,
    TCP_STATE,
    SSL_STATE,
    SIP_STATE,
    DNS_STATE,
    FTP_STATE,
    DIAMETER_STATE,
    RADIUS_STATE,
    MR_STATE,
    STREAM_STATE,
    LOG_OUTPUT,
    FILE_IO,
    INTERP_STATE,
    UNKNOWN,
};

enum class ConnectionSide : std::uint8_t {
    CLIENT,
    SERVER,
    BOTH,
    GLOBAL,
    NONE,
};

enum class StorageScope : std::uint8_t {
    PROC_LOCAL,
    NAMESPACE,
    GLOBAL,
    UPVAR,
    EVENT,
    CONNECTION,
    STATIC,
    SESSION_TABLE,
    PERSISTENCE,
    DATA_GROUP,
    FILE_SYSTEM,
    NETWORK_SOCKET,
    UNKNOWN,
};

struct SideEffectDesc {
    SideEffectTarget target = SideEffectTarget::UNKNOWN;
    bool reads = false;
    bool writes = false;
    ConnectionSide connection_side = ConnectionSide::NONE;
    StorageScope scope = StorageScope::UNKNOWN;
};

// ─── Taint analysis ────────────────────────────────────────────────
enum class TaintColour : std::uint16_t {
    NONE = 0,
    TAINTED = 1 << 0,
};

struct TaintHints {
    TaintColour source_colour = TaintColour::NONE;
    TaintColour transform_colour = TaintColour::NONE;
    TaintColour safe_colour = TaintColour::NONE;
    std::string_view output_sink_code{};
};

// ─── Format and pattern languages ──────────────────────────────────
enum class FormatType : std::uint8_t {
    NONE,
    SPRINTF, // format, scan — %d %s etc.
    CLOCK,   // clock format, clock scan — %Y %m etc.
    BINARY,  // binary format, binary scan — a A b B etc.
    REGSUB,  // regsub replacement — \& \1 etc.
};

enum class PatternType : std::uint8_t {
    NONE,
    GLOB,  // string match, glob, lsearch, switch (default)
    REGEX, // regexp, regsub, lsearch -regexp, switch -regexp
};

// ─── Argument shape patterns ───────────────────────────────────────
enum class ArgPatternKind : std::uint8_t {
    FIXED,        // Role at a specific index: arg[i] = role
    TAIL,         // From index N onward: arg[N..] = role
    STRIDE,       // Repeating groups: arg[N], arg[N+stride], ... = role
    OPTION_VALUE, // Value of named option has this role
};

struct ArgPattern {
    ArgPatternKind kind = ArgPatternKind::FIXED;
    ArgRole role = ArgRole::VALUE;
    std::int16_t index = 0;          // For FIXED: arg index (-1 = last). For TAIL/STRIDE: start.
    std::int16_t stride = 0;         // For STRIDE: step size (e.g. 2 for foreach pairs).
    std::string_view option_name{};  // For OPTION_VALUE: which option (e.g. "-body").
};

// ─── Per-argument type expectations ────────────────────────────────
// Enables shimmer detection (diagnostic O130) and type propagation.
struct ArgTypeDesc {
    std::int16_t index = 0;       // which arg (0-based after command name)
    TclType expected = TclType::STRING;
    bool shimmers = false;        // does the command force conversion?
};

// ─── Arity with modular constraint ─────────────────────────────────
struct Arity {
    std::int32_t min = 0;
    std::int32_t max = std::numeric_limits<std::int32_t>::max();
    std::int32_t modulus = 0;   // 0 = no modular constraint
    std::int32_t remainder = 0; // (argc) % modulus == remainder

    [[nodiscard]] constexpr auto accepts(std::int32_t n) const -> bool {
        if (n < min || n > max) return false;
        if (modulus > 0 && (n % modulus) != remainder) return false;
        return true;
    }

    [[nodiscard]] constexpr auto is_unlimited() const -> bool {
        return max == std::numeric_limits<std::int32_t>::max();
    }

    constexpr auto operator==(const Arity&) const -> bool = default;
};

// ─── Dialect flags ─────────────────────────────────────────────────
enum class DialectFlags : std::uint16_t {
    NONE = 0,
    TCL84 = 1 << 0,
    TCL85 = 1 << 1,
    TCL86 = 1 << 2,
    TCL90 = 1 << 3,
    IRULES = 1 << 4,
    IAPPS = 1 << 5,
    TMSH = 1 << 6,
    TK = 1 << 7,
    EXPECT = 1 << 8,
    EDA_SYNOPSYS = 1 << 9,
    EDA_CADENCE = 1 << 10,
    EDA_XILINX = 1 << 11,
    EDA_QUARTUS = 1 << 12,
    EDA_MENTOR = 1 << 13,
    ALL = 0x3FFF,

    // Convenience groups
    TCL_ALL = TCL84 | TCL85 | TCL86 | TCL90,
};

[[nodiscard]] constexpr auto operator|(DialectFlags a, DialectFlags b) -> DialectFlags {
    return static_cast<DialectFlags>(
        static_cast<std::uint16_t>(a) | static_cast<std::uint16_t>(b));
}

[[nodiscard]] constexpr auto operator&(DialectFlags a, DialectFlags b) -> DialectFlags {
    return static_cast<DialectFlags>(
        static_cast<std::uint16_t>(a) & static_cast<std::uint16_t>(b));
}

[[nodiscard]] constexpr auto operator~(DialectFlags a) -> DialectFlags {
    return static_cast<DialectFlags>(~static_cast<std::uint16_t>(a));
}

// ─── Layout resolvers ──────────────────────────────────────────────
// Commands with variable-layout arguments that need runtime arg walking.
enum class LayoutResolver : std::uint8_t {
    NONE,
    IF,             // if expr ?then? body ?elseif expr ?then? body ...? ?else? ?body?
    SWITCH,         // switch ?opts? string pattern body ?pattern body ...?
    TRY,            // try body ?on code varList body? ?trap pattern varList body? ?finally body?
    FOREACH,        // foreach varList list ?varList list ...? body
    EXPECT,         // expect ?opts? pattern body ?pattern body ...?
    TCLTEST,        // tcltest::test name desc ?option value ...?
    OO_CLASS,       // oo::class create name ?arg...? / oo::class new ?arg...?
    OO_DEFINE,      // oo::define Class ?defScript | subcommand arg...?
    OO_DEFINITION,  // commands inside oo::define body
    OO_PRIVATE,     // private: subcommand prefix mode or block mode
    OO_SELF,        // self: getter, script, or subcommand dispatch
    WHEN,           // when EVENT ?priority N? ?timing enable|disable? body
};

// ─── Option descriptor ─────────────────────────────────────────────
struct OptionDesc {
    std::string_view name{};        // "-nocase", "-start", "--"
    bool takes_value = false;       // does this option consume the next arg?
    std::string_view value_hint{};  // human hint for the value ("index", "cmd")
    bool is_terminator = false;     // true only for "--"
    std::string_view detail{};      // hover text for option
    ArgRole value_role = ArgRole::VALUE; // semantic role of the consumed value
};

// ─── Getter/setter form descriptor ─────────────────────────────────
enum class FormKind : std::uint8_t {
    DEFAULT,
    GETTER,
    SETTER,
    ACTION,
};

struct FormDesc {
    FormKind kind = FormKind::DEFAULT;
    Arity arity{};
    std::span<const OptionDesc> options{};
    bool pure = false;
    bool mutator = false;
};

// ─── Hover text ────────────────────────────────────────────────────
struct HoverText {
    std::string_view summary{};
    std::span<const std::string_view> synopsis{};
    std::string_view snippet{};
    std::string_view source{};
    std::string_view examples{};
    std::string_view return_value{};
};

// ─── Argument value descriptor (for completions) ───────────────────
struct ArgValueDesc {
    std::string_view value{};       // completion text: "integer", "boolean", etc.
    std::string_view detail{};      // one-line detail for completion item
    std::int32_t hover_index = -1;  // index into hover table (optional)
};

// ─── Event requirements (iRules) ───────────────────────────────────
enum class Transport : std::uint8_t {
    NONE,
    TCP,
    UDP,
    ANY,
};

enum class ProfileFlags : std::uint16_t {
    NONE = 0,
    HTTP = 1 << 0,
    FASTHTTP = 1 << 1,
    SSL = 1 << 2,
    SIP = 1 << 3,
    DNS = 1 << 4,
    FTP = 1 << 5,
    DIAMETER = 1 << 6,
    RADIUS = 1 << 7,
    STREAM = 1 << 8,
    WEBSOCKET = 1 << 9,
    MQTT = 1 << 10,
    ANY = 0x07FF,
};

[[nodiscard]] constexpr auto operator|(ProfileFlags a, ProfileFlags b) -> ProfileFlags {
    return static_cast<ProfileFlags>(
        static_cast<std::uint16_t>(a) | static_cast<std::uint16_t>(b));
}

[[nodiscard]] constexpr auto operator&(ProfileFlags a, ProfileFlags b) -> ProfileFlags {
    return static_cast<ProfileFlags>(
        static_cast<std::uint16_t>(a) & static_cast<std::uint16_t>(b));
}

struct EventRequires {
    Transport transport = Transport::NONE;
    ProfileFlags profiles = ProfileFlags::NONE;
};

// ─── Subcommand descriptor ─────────────────────────────────────────
struct SubCmdDesc {
    std::string_view name{};
    Arity arity{};
    std::span<const ArgPattern> arg_patterns{};
    std::span<const OptionDesc> options{};
    std::span<const FormDesc> forms{};
    std::span<const ArgTypeDesc> arg_types{};
    DialectFlags dialects = DialectFlags::ALL; // NONE = inherit from parent

    // Effect classification
    bool pure = false;
    bool mutator = false;
    bool destructive = false;
    bool loop_list_header = false;
    bool creates_scope_alias = false;
    bool is_slot_command = false;
    bool defines_procedure = false;

    // Type info
    TclType return_type = TclType::STRING;
    PatternType pattern_type = PatternType::NONE;

    // Taint
    TaintHints taint_hints{};

    // Security
    std::int8_t credential_arg = -1;
    std::span<const std::string_view> sensitive_headers{};

    // Deprecation
    std::string_view deprecated_replacement{};

    // Hover
    std::int32_t hover_index = -1;
};

// ─── Command descriptor ────────────────────────────────────────────
struct CommandDesc {
    // ── Identity ──
    std::string_view name{};

    // ── Argument shape ──
    Arity arity{};
    std::span<const ArgPattern> arg_patterns{};
    LayoutResolver layout_resolver = LayoutResolver::NONE;

    // ── Subcommands ──
    std::span<const SubCmdDesc> subcommands{};
    bool allow_unknown_subcommands = false;

    // ── Options ──
    std::span<const OptionDesc> options{};

    // ── Invocation forms (getter/setter) ──
    std::span<const FormDesc> forms{};

    // ── Dialect & event membership ──
    DialectFlags dialects = DialectFlags::ALL;
    EventRequires event_requires{};

    // ── Behavioral traits ──
    bool pure = false;
    bool mutator = false;
    bool is_control_flow = false;
    bool never_inline_body = false;
    bool has_loop_body = false;
    bool loop_list_header = false;
    bool creates_dynamic_barrier = false;
    bool defines_procedure = false;
    bool is_scope_barrier = false;
    bool top_level_only = false;
    bool needs_start_cmd = false;

    // ── Terminator & option traits ──
    bool warn_without_terminator = false;

    // ── Compiler & optimiser traits ──
    bool cse_candidate = false;
    bool compile_time_evaluable = false;
    bool unsafe = false;
    bool taint_sink = false;
    bool diagram_action = false;

    // ── XC translatability ──
    std::int8_t xc_translatable = 0;

    // ── Variable assignment semantics ──
    std::int8_t assigns_variable_at = -1;

    // ── Type info ──
    std::span<const ArgTypeDesc> arg_types{};
    TclType return_type = TclType::STRING;

    // ── Side effects ──
    std::span<const SideEffectDesc> side_effect_hints{};

    // ── Taint analysis ──
    TaintHints taint_hints{};

    // ── Package gating ──
    std::string_view required_package{};
    std::string_view tcllib_package{};
    bool warn_missing_import = true;

    // ── Hover / documentation ──
    std::int32_t hover_index = -1;

    // ── Deprecation ──
    std::string_view deprecated_replacement{};

    // ── Formatting hints ──
    std::span<const std::string_view> formatting_keywords{};
    PatternType pattern_type = PatternType::NONE;
    FormatType format_string_type = FormatType::NONE;

    // ── Credential / security hints ──
    bool password_option_command = false;
    std::span<const std::string_view> credential_options{};
    std::span<const std::string_view> sensitive_headers{};
};

} // namespace tcl_lsp

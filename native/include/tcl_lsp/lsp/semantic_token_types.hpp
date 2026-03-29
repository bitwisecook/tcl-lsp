#pragma once

#include <array>
#include <cstdint>
#include <string_view>

namespace tcl_lsp {

// LSP semantic token type indices.
// These match the SEMANTIC_TOKEN_TYPES legend registered with the client.
// Order is significant — the index is the wire value.
enum class SemanticTokenType : std::uint8_t {
    KEYWORD = 0,
    FUNCTION = 1,
    VARIABLE = 2,
    STRING = 3,
    COMMENT = 4,
    NUMBER = 5,
    OPERATOR = 6,
    PARAMETER = 7,
    NAMESPACE = 8,
    REGEXP = 9,
    EVENT = 10,
    DECORATOR = 11,
    ESCAPE = 12,
    OBJECT = 13,
    FQDN = 14,
    IP_ADDRESS = 15,
    PORT = 16,
    ROUTE_DOMAIN = 17,
    PARTITION = 18,
    USERNAME = 19,
    ENCRYPTED = 20,
    POOL = 21,
    MONITOR = 22,
    PROFILE = 23,
    VLAN = 24,
    INTERFACE = 25,
    REGEXP_GROUP = 26,
    REGEXP_CHAR_CLASS = 27,
    REGEXP_QUANTIFIER = 28,
    REGEXP_ANCHOR = 29,
    REGEXP_ESCAPE = 30,
    REGEXP_BACKREF = 31,
    REGEXP_ALTERNATION = 32,
    BINARY_SPEC = 33,
    BINARY_COUNT = 34,
    BINARY_FLAG = 35,
    FORMAT_PERCENT = 36,
    FORMAT_SPEC = 37,
    FORMAT_FLAG = 38,
    FORMAT_WIDTH = 39,
    CLOCK_PERCENT = 40,
    CLOCK_SPEC = 41,
    CLOCK_MODIFIER = 42,
    APL_SECTION = 43,
    APL_FIELD_TYPE = 44,
    APL_ATTRIBUTE = 45,
    APL_SECTION_NAME = 46,
    APL_FIELD_NAME = 47,
    APL_DEFINE = 48,
    APL_DEFINE_NAME = 49,
    APL_DIRECTIVE = 50,
    APL_OPTIONAL = 51,
    APL_VALIDATOR = 52,
};

inline constexpr std::size_t SEMANTIC_TOKEN_TYPE_COUNT = 53;

// LSP wire names — index == enum value.
inline constexpr std::array<std::string_view, SEMANTIC_TOKEN_TYPE_COUNT>
    SEMANTIC_TOKEN_TYPE_NAMES = {{
        "keyword",
        "function",
        "variable",
        "string",
        "comment",
        "number",
        "operator",
        "parameter",
        "namespace",
        "regexp",
        "event",
        "decorator",
        "escape",
        "object",
        "fqdn",
        "ipAddress",
        "port",
        "routeDomain",
        "partition",
        "username",
        "encrypted",
        "pool",
        "monitor",
        "profile",
        "vlan",
        "interface",
        "regexpGroup",
        "regexpCharClass",
        "regexpQuantifier",
        "regexpAnchor",
        "regexpEscape",
        "regexpBackref",
        "regexpAlternation",
        "binarySpec",
        "binaryCount",
        "binaryFlag",
        "formatPercent",
        "formatSpec",
        "formatFlag",
        "formatWidth",
        "clockPercent",
        "clockSpec",
        "clockModifier",
        "aplSection",
        "aplFieldType",
        "aplAttribute",
        "aplSectionName",
        "aplFieldName",
        "aplDefine",
        "aplDefineName",
        "aplDirective",
        "aplOptional",
        "aplValidator",
    }};

[[nodiscard]] constexpr auto to_string(SemanticTokenType t) -> std::string_view {
    auto i = static_cast<std::size_t>(t);
    if (i < SEMANTIC_TOKEN_TYPE_COUNT) return SEMANTIC_TOKEN_TYPE_NAMES[i];
    return "unknown";
}

// LSP semantic token modifiers (bitmask).
enum class SemanticTokenModifier : std::uint8_t {
    DECLARATION = 0,
    DEFINITION = 1,
    READONLY = 2,
    DEFAULT_LIBRARY = 3,
};

inline constexpr std::size_t SEMANTIC_TOKEN_MODIFIER_COUNT = 4;

inline constexpr std::array<std::string_view, SEMANTIC_TOKEN_MODIFIER_COUNT>
    SEMANTIC_TOKEN_MODIFIER_NAMES = {{
        "declaration",
        "definition",
        "readonly",
        "defaultLibrary",
    }};

[[nodiscard]] constexpr auto to_string(SemanticTokenModifier m) -> std::string_view {
    auto i = static_cast<std::size_t>(m);
    if (i < SEMANTIC_TOKEN_MODIFIER_COUNT) return SEMANTIC_TOKEN_MODIFIER_NAMES[i];
    return "unknown";
}

// Build a modifier bitmask from a single modifier.
[[nodiscard]] constexpr auto modifier_bit(SemanticTokenModifier m) -> std::uint8_t {
    return static_cast<std::uint8_t>(1U << static_cast<unsigned>(m));
}

// A single semantic token in absolute position form.
// Sorted by (line, character) before delta-encoding for the LSP wire format.
struct SemanticToken {
    std::int32_t line = 0;
    std::int32_t character = 0;
    std::int32_t length = 0;
    SemanticTokenType type = SemanticTokenType::STRING;
    std::uint8_t modifiers = 0;

    auto operator==(const SemanticToken&) const -> bool = default;
};

} // namespace tcl_lsp

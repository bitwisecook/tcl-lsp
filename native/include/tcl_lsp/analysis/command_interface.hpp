#pragma once

#include <cstdint>
#include <limits>
#include <optional>
#include <string>
#include <string_view>
#include <unordered_map>
#include <variant>
#include <vector>

namespace tcl_lsp {

// What role an argument plays in a command.
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

// Argument count range for a command or subcommand.
// Supports modular constraints (e.g. foreach: argc must be odd and >= 3).
struct Arity {
    std::int32_t min = 0;
    std::int32_t max = std::numeric_limits<std::int32_t>::max();
    std::int32_t modulus = 0;   // 0 = no modular constraint
    std::int32_t remainder = 0; // (argc) % modulus == remainder

    [[nodiscard]] constexpr auto is_unlimited() const -> bool {
        return max == std::numeric_limits<std::int32_t>::max();
    }

    [[nodiscard]] constexpr auto accepts(std::int32_t n) const -> bool {
        if (n < min || n > max) return false;
        if (modulus > 0 && (n % modulus) != remainder) return false;
        return true;
    }

    constexpr auto operator==(const Arity&) const -> bool = default;
};

// Signature for a simple command.
struct CommandSig {
    Arity arity;
    std::unordered_map<int32_t, ArgRole> arg_roles;
};

// Signature for a command with subcommands (e.g. namespace, dict, string).
struct SubcommandSig {
    std::unordered_map<std::string, CommandSig> subcommands;
    bool allow_unknown = false;
};

using SignatureVariant = std::variant<CommandSig, SubcommandSig>;

// Abstract interface for command metadata.
// Concrete implementations come from Python shim or test harness.
class CommandRegistryInterface {
  public:
    virtual ~CommandRegistryInterface() = default;
    [[nodiscard]] virtual auto command_names() const -> std::vector<std::string> = 0;
    [[nodiscard]] virtual auto validation(std::string_view cmd) const -> std::optional<Arity> = 0;
    [[nodiscard]] virtual auto signature(std::string_view cmd) const
        -> std::optional<SignatureVariant> = 0;
    [[nodiscard]] virtual auto is_known(std::string_view cmd) const -> bool = 0;
};

// Minimal registry for testing -- populated with specific commands.
class TestCommandRegistry final : public CommandRegistryInterface {
  public:
    void add_command(std::string name, Arity arity);
    void add_command(std::string name, CommandSig sig);
    void add_subcommand(std::string name, SubcommandSig sig);

    [[nodiscard]] auto command_names() const -> std::vector<std::string> override;
    [[nodiscard]] auto validation(std::string_view cmd) const -> std::optional<Arity> override;
    [[nodiscard]] auto signature(std::string_view cmd) const
        -> std::optional<SignatureVariant> override;
    [[nodiscard]] auto is_known(std::string_view cmd) const -> bool override;

  private:
    std::unordered_map<std::string, SignatureVariant> sigs_;
};

} // namespace tcl_lsp

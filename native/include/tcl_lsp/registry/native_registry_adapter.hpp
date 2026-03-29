#pragma once

#include "tcl_lsp/analysis/command_interface.hpp"

#include <cstdint>
#include <memory>
#include <string>
#include <string_view>
#include <vector>

namespace tcl_lsp {

// Forward-declare — avoid including command_desc.hpp which redefines ArgRole.
class CommandRegistry;
struct CommandDesc;
struct HoverText;

// Adapter that bridges the generated constexpr CommandRegistry to the
// CommandRegistryInterface expected by Analyser and SemanticTokenCollector.
class NativeCommandRegistry final : public CommandRegistryInterface {
  public:
    NativeCommandRegistry();
    ~NativeCommandRegistry() override;

    [[nodiscard]] auto command_names() const -> std::vector<std::string> override;
    [[nodiscard]] auto validation(std::string_view cmd) const
        -> std::optional<Arity> override;
    [[nodiscard]] auto signature(std::string_view cmd) const
        -> std::optional<SignatureVariant> override;
    [[nodiscard]] auto is_known(std::string_view cmd) const -> bool override;

    // Total number of registered commands.
    [[nodiscard]] auto size() const -> std::size_t;

  private:
    struct Impl;
    std::unique_ptr<Impl> impl_;
};

// Singleton access (lazily constructed).
[[nodiscard]] auto native_registry() -> const NativeCommandRegistry&;

} // namespace tcl_lsp

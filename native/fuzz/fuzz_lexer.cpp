// Fuzz harness for the Tcl lexer.
//
// Feeds arbitrary byte sequences into TclLexer::tokenise_all() and verifies
// the lexer handles them without crashing, asserting, or triggering undefined
// behaviour.  Run with libFuzzer (clang -fsanitize=fuzzer) or AFL++.
//
// Build:  meson setup builddir-fuzz -Dfuzz=true
// Run:    builddir-fuzz/native/fuzz/fuzz_lexer [corpus_dir]

#include "tcl_lsp/parsing/lexer.hpp"

#include <cstddef>
#include <cstdint>
#include <string>
#include <string_view>

extern "C" int LLVMFuzzerTestOneInput(const uint8_t* data, size_t size) {
    // Cap input size to avoid timeouts on huge inputs.
    if (size > 64 * 1024) {
        return 0;
    }

    auto input = std::string_view(reinterpret_cast<const char*>(data), size);

    // Default config (non-strict, expand syntax enabled).
    {
        tcl_lsp::TclLexer lexer(input);
        auto tokens = lexer.tokenise_all();
        (void)tokens;
    }

    // Strict quoting mode — exercises the error-reporting path.
    {
        tcl_lsp::LexerConfig strict_cfg{.strict_quoting = true};
        try {
            tcl_lsp::TclLexer lexer(input, strict_cfg);
            auto tokens = lexer.tokenise_all();
            (void)tokens;
        } catch (const tcl_lsp::TclParseError&) {
            // Expected on malformed input in strict mode.
        }
    }

    // iRules brace separator mode.
    {
        tcl_lsp::LexerConfig irules_cfg{.irules_brace_separator = true};
        tcl_lsp::TclLexer lexer(input, irules_cfg);
        auto tokens = lexer.tokenise_all();
        (void)tokens;
    }

    // Non-zero base offset (exercises offset arithmetic).
    if (size >= 2) {
        auto base_offset = static_cast<int32_t>(data[0]) * 16;
        auto base_line = static_cast<int32_t>(data[1] & 0x0F);
        tcl_lsp::TclLexer lexer(input.substr(2), {}, base_offset, base_line);
        auto tokens = lexer.tokenise_all();
        (void)tokens;
    }

    return 0;
}

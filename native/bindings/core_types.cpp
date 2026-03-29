#include "tcl_lsp/core/diagnostic.hpp"
#include "tcl_lsp/core/document_buffer.hpp"
#include "tcl_lsp/core/memory_stats.hpp"
#include "tcl_lsp/core/range.hpp"
#include "tcl_lsp/core/source_position.hpp"
#include "tcl_lsp/parsing/lexer.hpp"
#include "tcl_lsp/parsing/recovery.hpp"
#include "tcl_lsp/parsing/segmenter.hpp"
#include "tcl_lsp/parsing/token.hpp"

#include <pybind11/pybind11.h>
#include <pybind11/stl.h>

namespace py = pybind11;
using namespace tcl_lsp;

PYBIND11_MODULE(_tcl_lsp_native, m) {
    m.doc() = "Native C++ core types for tcl-lsp";

    // SourcePosition — immutable value type matching the Python frozen dataclass.
    py::class_<SourcePosition>(m, "SourcePosition")
        .def(py::init<int32_t, int32_t, int32_t>(),
             py::arg("line"),
             py::arg("character"),
             py::arg("offset"))
        .def_readonly("line", &SourcePosition::line)
        .def_readonly("character", &SourcePosition::character)
        .def_readonly("offset", &SourcePosition::offset)
        .def("__eq__", [](const SourcePosition& a, const SourcePosition& b) { return a == b; })
        .def("__ne__", [](const SourcePosition& a, const SourcePosition& b) { return a != b; })
        .def("__lt__", [](const SourcePosition& a, const SourcePosition& b) { return a < b; })
        .def("__le__", [](const SourcePosition& a, const SourcePosition& b) { return a <= b; })
        .def("__gt__", [](const SourcePosition& a, const SourcePosition& b) { return a > b; })
        .def("__ge__", [](const SourcePosition& a, const SourcePosition& b) { return a >= b; })
        .def("__hash__", [](const SourcePosition& p) { return SourcePosition::Hash{}(p); })
        .def("__repr__", [](const SourcePosition& p) { return to_string(p); });

    // Range — immutable value type matching the Python frozen dataclass.
    py::class_<Range>(m, "Range")
        .def(py::init([](const SourcePosition& start, const SourcePosition& end) {
                 return Range{start, end};
             }),
             py::arg("start"),
             py::arg("end"))
        .def_readonly("start", &Range::start)
        .def_readonly("end", &Range::end)
        .def_static("zero", &Range::zero)
        .def("__eq__", [](const Range& a, const Range& b) { return a == b; })
        .def("__ne__", [](const Range& a, const Range& b) { return a != b; })
        .def("__lt__", [](const Range& a, const Range& b) { return a < b; })
        .def("__hash__", [](const Range& r) { return Range::Hash{}(r); })
        .def("__repr__", [](const Range& r) { return to_string(r); });

    // DocumentBuffer — mutable (has lazy caches) but source is immutable after creation.
    py::class_<DocumentBuffer>(m, "DocumentBuffer")
        .def_static("from_source",
                    &DocumentBuffer::from_source,
                    py::arg("source"),
                    py::arg("version") = py::none())
        .def_property_readonly("source",
                               [](const DocumentBuffer& buf) { return std::string(buf.source()); })
        .def_property_readonly("version", &DocumentBuffer::version)
        .def_property_readonly("lines",
                               [](const DocumentBuffer& buf) {
                                   auto span = buf.lines();
                                   py::list result;
                                   for (auto sv : span) {
                                       result.append(py::str(std::string(sv)));
                                   }
                                   return result;
                               })
        .def_property_readonly("line_starts",
                               [](const DocumentBuffer& buf) {
                                   // Expose line_starts for compatibility with Python code that
                                   // reads it. Reconstruct from offset_to_line_col probing — or
                                   // just compute. For now, provide via the source.
                                   py::list result;
                                   std::string_view src = buf.source();
                                   result.append(0);
                                   for (std::size_t i = 0; i < src.size(); ++i) {
                                       if (src[i] == '\n') {
                                           result.append(static_cast<int>(i + 1));
                                       }
                                   }
                                   return py::tuple(result);
                               })
        .def("offset_to_position", &DocumentBuffer::offset_to_position, py::arg("offset"))
        .def("position_to_offset",
             &DocumentBuffer::position_to_offset,
             py::arg("line"),
             py::arg("character"))
        .def(
            "offset_to_line_col",
            [](const DocumentBuffer& buf, int32_t offset) {
                auto [line, col] = buf.offset_to_line_col(offset);
                return py::make_tuple(line, col);
            },
            py::arg("offset"))
        .def("range_from_offsets",
             &DocumentBuffer::range_from_offsets,
             py::arg("start"),
             py::arg("end_inclusive"))
        .def(
            "chunk_line_range",
            [](const DocumentBuffer& buf, int32_t start, int32_t end) {
                auto [sl, sc, el, ec] = buf.chunk_line_range(start, end);
                return py::make_tuple(sl, sc, el, ec);
            },
            py::arg("start_offset"),
            py::arg("end_offset"));

    // MemoryStats — C++ heap memory snapshot via mallinfo2().
    py::class_<MemoryStats>(m, "MemoryStats")
        .def_readonly("arena_bytes", &MemoryStats::arena_bytes)
        .def_readonly("mmap_bytes", &MemoryStats::mmap_bytes)
        .def_readonly("used_bytes", &MemoryStats::used_bytes)
        .def_readonly("free_bytes", &MemoryStats::free_bytes)
        .def_readonly("total_allocated", &MemoryStats::total_allocated)
        .def("__repr__", [](const MemoryStats& s) {
            return "MemoryStats(arena=" + std::to_string(s.arena_bytes) +
                   ", mmap=" + std::to_string(s.mmap_bytes) +
                   ", used=" + std::to_string(s.used_bytes) +
                   ", free=" + std::to_string(s.free_bytes) +
                   ", total=" + std::to_string(s.total_allocated) + ")";
        });

    m.def("memory_stats", &memory_stats, "Query C++ heap memory usage (mallinfo2 on Linux).");

    // TokenType — enum matching Python's TokenType.
    py::enum_<TokenType>(m, "TokenType")
        .value("ESC", TokenType::ESC)
        .value("STR", TokenType::STR)
        .value("CMD", TokenType::CMD)
        .value("VAR", TokenType::VAR)
        .value("SEP", TokenType::SEP)
        .value("EOL", TokenType::EOL)
        .value("EOF", TokenType::EOF_)
        .value("COMMENT", TokenType::COMMENT)
        .value("EXPAND", TokenType::EXPAND);

    // Token — immutable value type matching the Python frozen dataclass.
    py::class_<Token>(m, "Token")
        .def(py::init([](TokenType type,
                         const std::string& text,
                         const SourcePosition& start,
                         const SourcePosition& end,
                         bool in_quote) { return Token{type, text, start, end, in_quote}; }),
             py::arg("type"),
             py::arg("text"),
             py::arg("start"),
             py::arg("end"),
             py::arg("in_quote") = false)
        .def_readonly("type", &Token::type)
        .def_readonly("text", &Token::text)
        .def_readonly("start", &Token::start)
        .def_readonly("end", &Token::end)
        .def_readonly("in_quote", &Token::in_quote)
        .def("__eq__", [](const Token& a, const Token& b) { return a == b; })
        .def("__repr__", [](const Token& t) { return to_string(t); });

    // TclParseError — maps to Python exception.
    py::register_exception<TclParseError>(m, "TclParseError", PyExc_RuntimeError);

    // LexerConfig — configuration flags for the lexer.
    py::class_<LexerConfig>(m, "LexerConfig")
        .def(py::init<>())
        .def(py::init([](bool strict_quoting, bool expand_syntax, bool irules_brace_separator) {
                 return LexerConfig{strict_quoting, expand_syntax, irules_brace_separator};
             }),
             py::arg("strict_quoting") = false,
             py::arg("expand_syntax") = true,
             py::arg("irules_brace_separator") = false)
        .def_readwrite("strict_quoting", &LexerConfig::strict_quoting)
        .def_readwrite("expand_syntax", &LexerConfig::expand_syntax)
        .def_readwrite("irules_brace_separator", &LexerConfig::irules_brace_separator);

    // TclLexer — the main lexer class.
    py::class_<TclLexer>(m, "NativeTclLexer")
        .def(py::init([](const std::string& text,
                         LexerConfig config,
                         int32_t base_offset,
                         int32_t base_line,
                         int32_t base_col,
                         py::object ghost_insertions,
                         py::object line_starts_obj) {
                 // Convert Python dict to C++ map.
                 std::unordered_map<int32_t, char> vi;
                 if (!ghost_insertions.is_none()) {
                     auto dict = ghost_insertions.cast<py::dict>();
                     for (auto& [k, v] : dict) {
                         auto offset = k.cast<int32_t>();
                         auto ch_str = v.cast<std::string>();
                         if (!ch_str.empty()) {
                             vi[offset] = ch_str[0];
                         }
                     }
                 }

                 // Convert Python list/tuple of line starts if provided.
                 std::vector<int32_t>* ls_ptr = nullptr;
                 std::vector<int32_t> ls_vec;
                 if (!line_starts_obj.is_none()) {
                     auto ls_list = line_starts_obj.cast<py::sequence>();
                     ls_vec.reserve(static_cast<std::size_t>(py::len(ls_list)));
                     for (auto item : ls_list) {
                         ls_vec.push_back(item.cast<int32_t>());
                     }
                     ls_ptr = &ls_vec;
                 }

                 return TclLexer(std::string(text),
                                 config,
                                 base_offset,
                                 base_line,
                                 base_col,
                                 std::move(vi),
                                 ls_ptr,
                                 OwningTag{});
             }),
             py::arg("text"),
             py::arg("config") = LexerConfig{},
             py::arg("base_offset") = 0,
             py::arg("base_line") = 0,
             py::arg("base_col") = 0,
             py::arg("virtual_insertions") = py::none(),
             py::arg("line_starts") = py::none())
        .def("get_token", &TclLexer::get_token)
        .def("tokenise_all", &TclLexer::tokenise_all)
        .def_property_readonly("remaining", &TclLexer::remaining)
        .def_property_readonly("pos", &TclLexer::pos)
        .def_property_readonly("insidequote", &TclLexer::insidequote)
        .def_property_readonly("warnings",
                               [](const TclLexer& lexer) {
                                   py::list result;
                                   for (auto& [pos, msg] : lexer.warnings()) {
                                       result.append(py::make_tuple(pos, msg));
                                   }
                                   return result;
                               })
        .def_property_readonly("text",
                               [](const TclLexer& lexer) { return std::string(lexer.text()); })
        .def_property_readonly("line_starts",
                               [](const TclLexer& lexer) { return lexer.line_starts(); });

    // --- Phase 3: Segmenter + Recovery types ---

    // Severity enum.
    py::enum_<Severity>(m, "Severity")
        .value("ERROR", Severity::ERROR)
        .value("WARNING", Severity::WARNING)
        .value("INFORMATION", Severity::INFORMATION)
        .value("HINT", Severity::HINT);

    // CodeFix — a quick-fix suggestion.
    py::class_<CodeFix>(m, "CodeFix")
        .def(py::init([](const Range& range,
                         const std::string& new_text,
                         const std::string& description) {
                 return CodeFix{range, new_text, description};
             }),
             py::arg("range"),
             py::arg("new_text"),
             py::arg("description") = "")
        .def_readonly("range", &CodeFix::range)
        .def_readonly("new_text", &CodeFix::new_text)
        .def_readonly("description", &CodeFix::description)
        .def("__eq__", [](const CodeFix& a, const CodeFix& b) { return a == b; })
        .def("__repr__", [](const CodeFix& f) {
            return "CodeFix(new_text=" + f.new_text + ", desc=" + f.description + ")";
        });

    // Diagnostic — error/warning message with optional fixes.
    py::class_<Diagnostic>(m, "Diagnostic")
        .def(py::init([](const Range& range,
                         Severity severity,
                         [[maybe_unused]] const std::string& code_str,
                         const std::string& message,
                         const std::vector<CodeFix>& fixes) {
                 // The code string is ignored here — DiagCode is set by the
                 // C++ analyser, not constructed from Python strings.
                 return Diagnostic{range, severity, DiagCode::E200, message, fixes};
             }),
             py::arg("range"),
             py::arg("severity") = Severity::ERROR,
             py::arg("code") = "",
             py::arg("message") = "",
             py::arg("fixes") = std::vector<CodeFix>{})
        .def_readonly("range", &Diagnostic::range)
        .def_readonly("severity", &Diagnostic::severity)
        .def_property_readonly("code", [](const Diagnostic& d) { return to_string(d.code); })
        .def_readonly("message", &Diagnostic::message)
        .def_readonly("fixes", &Diagnostic::fixes)
        .def("__eq__", [](const Diagnostic& a, const Diagnostic& b) { return a == b; })
        .def("__repr__", [](const Diagnostic& d) {
            return "Diagnostic(code=" + to_string(d.code) + ", message=" + d.message + ")";
        });

    // UnclosedDelimiter enum.
    py::enum_<UnclosedDelimiter>(m, "UnclosedDelimiter")
        .value("BRACE", UnclosedDelimiter::BRACE)
        .value("BRACKET", UnclosedDelimiter::BRACKET)
        .value("QUOTE", UnclosedDelimiter::QUOTE);

    // SegmentedCommand — a single parsed Tcl command.
    py::class_<SegmentedCommand>(m, "NativeSegmentedCommand")
        .def_readonly("range", &SegmentedCommand::range)
        .def_readonly("argv", &SegmentedCommand::argv)
        .def_readonly("texts", &SegmentedCommand::texts)
        .def_readonly("single_token_word", &SegmentedCommand::single_token_word)
        .def_readonly("all_tokens", &SegmentedCommand::all_tokens)
        .def_readonly("preceding_comment", &SegmentedCommand::preceding_comment)
        .def_readonly("is_partial", &SegmentedCommand::is_partial)
        .def_readonly("partial_delimiter", &SegmentedCommand::partial_delimiter)
        .def_readonly("expand_word", &SegmentedCommand::expand_word)
        .def_property_readonly("name", [](const SegmentedCommand& cmd) { return cmd.name(); })
        .def_property_readonly("args", [](const SegmentedCommand& cmd) { return cmd.args(); })
        .def_property_readonly("arg_tokens",
                               [](const SegmentedCommand& cmd) { return cmd.arg_tokens(); })
        .def_property_readonly("arg_single_token",
                               [](const SegmentedCommand& cmd) { return cmd.arg_single_token(); });

    // TopLevelChunk — a region of top-level source.
    py::class_<TopLevelChunk>(m, "TopLevelChunk")
        .def_readonly("index", &TopLevelChunk::index)
        .def_readonly("start_offset", &TopLevelChunk::start_offset)
        .def_readonly("end_offset", &TopLevelChunk::end_offset)
        .def_readonly("source_hash", &TopLevelChunk::source_hash)
        .def_readonly("commands", &TopLevelChunk::commands);

    // segment_commands() — main segmentation entry point.
    m.def(
        "segment_commands",
        [](const std::string& source,
           py::object body_token_obj,
           py::object known_commands_obj,
           py::object ghost_insertions_obj,
           bool recovery) {
            const Token* body_token = nullptr;
            Token body_tok_storage;
            if (!body_token_obj.is_none()) {
                body_tok_storage = body_token_obj.cast<Token>();
                body_token = &body_tok_storage;
            }

            std::unordered_set<std::string>* kc_ptr = nullptr;
            std::unordered_set<std::string> kc_set;
            if (!known_commands_obj.is_none()) {
                auto kc_seq = known_commands_obj.cast<py::sequence>();
                for (auto item : kc_seq) {
                    kc_set.insert(item.cast<std::string>());
                }
                kc_ptr = &kc_set;
            }

            std::unordered_map<int32_t, char>* vi_ptr = nullptr;
            std::unordered_map<int32_t, char> vi_map;
            if (!ghost_insertions_obj.is_none()) {
                auto vi_dict = ghost_insertions_obj.cast<py::dict>();
                for (auto& [k, v] : vi_dict) {
                    auto ch_str = v.cast<std::string>();
                    if (!ch_str.empty()) {
                        vi_map[k.cast<int32_t>()] = ch_str[0];
                    }
                }
                vi_ptr = &vi_map;
            }

            return segment_commands(source, body_token, kc_ptr, vi_ptr, nullptr, recovery);
        },
        py::arg("source"),
        py::arg("body_token") = py::none(),
        py::arg("known_commands") = py::none(),
        py::arg("virtual_insertions") = py::none(),
        py::arg("recovery") = true);

    // segment_top_level_chunks()
    m.def("segment_top_level_chunks", &segment_top_level_chunks, py::arg("source"));

    // find_first_dirty_chunk()
    m.def("find_first_dirty_chunk",
          &find_first_dirty_chunk,
          py::arg("old_chunks"),
          py::arg("new_chunks"));

    // compute_ghost_insertions() — exposed as compute_virtual_insertions for Python compat.
    m.def(
        "compute_virtual_insertions",
        [](const std::string& source, py::object body_token_obj, py::object known_commands_obj) {
            const Token* body_token = nullptr;
            Token body_tok_storage;
            if (!body_token_obj.is_none()) {
                body_tok_storage = body_token_obj.cast<Token>();
                body_token = &body_tok_storage;
            }
            std::unordered_set<std::string>* kc_ptr = nullptr;
            std::unordered_set<std::string> kc_set;
            if (!known_commands_obj.is_none()) {
                auto kc_seq = known_commands_obj.cast<py::sequence>();
                for (auto item : kc_seq) {
                    kc_set.insert(item.cast<std::string>());
                }
                kc_ptr = &kc_set;
            }
            auto result = compute_ghost_insertions(source, body_token, kc_ptr);
            // Convert to Python dict with string values (matching Python API).
            py::dict py_result;
            for (auto& [offset, ch] : result) {
                py_result[py::int_(offset)] = py::str(std::string(1, ch));
            }
            return py_result;
        },
        py::arg("source"),
        py::arg("body_token") = py::none(),
        py::arg("known_commands") = py::none());

    // segment_with_recovery()
    m.def(
        "segment_with_recovery",
        [](const std::string& source, py::object body_token_obj, py::object known_commands_obj) {
            const Token* body_token = nullptr;
            Token body_tok_storage;
            if (!body_token_obj.is_none()) {
                body_tok_storage = body_token_obj.cast<Token>();
                body_token = &body_tok_storage;
            }
            std::unordered_set<std::string>* kc_ptr = nullptr;
            std::unordered_set<std::string> kc_set;
            if (!known_commands_obj.is_none()) {
                auto kc_seq = known_commands_obj.cast<py::sequence>();
                for (auto item : kc_seq) {
                    kc_set.insert(item.cast<std::string>());
                }
                kc_ptr = &kc_set;
            }
            auto result = segment_with_recovery(source, body_token, kc_ptr);
            return py::make_tuple(std::move(result.commands), std::move(result.diagnostics));
        },
        py::arg("source"),
        py::arg("body_token") = py::none(),
        py::arg("known_commands") = py::none());

    // position_from_relative()
    m.def("position_from_relative",
          &position_from_relative,
          py::arg("text"),
          py::arg("rel_offset"),
          py::arg("base_line"),
          py::arg("base_col"),
          py::arg("base_offset"));
}

#include <pybind11/pybind11.h>
#include <pybind11/stl.h>

#include "tcl_lsp/core/source_position.hpp"
#include "tcl_lsp/core/range.hpp"
#include "tcl_lsp/core/document_buffer.hpp"
#include "tcl_lsp/core/memory_stats.hpp"
#include "tcl_lsp/parsing/token.hpp"
#include "tcl_lsp/parsing/lexer.hpp"

namespace py = pybind11;
using namespace tcl_lsp;

PYBIND11_MODULE(_tcl_lsp_native, m) {
    m.doc() = "Native C++ core types for tcl-lsp";

    // SourcePosition — immutable value type matching the Python frozen dataclass.
    py::class_<SourcePosition>(m, "SourcePosition")
        .def(py::init<int32_t, int32_t, int32_t>(),
             py::arg("line"), py::arg("character"), py::arg("offset"))
        .def_readonly("line", &SourcePosition::line)
        .def_readonly("character", &SourcePosition::character)
        .def_readonly("offset", &SourcePosition::offset)
        .def("__eq__", [](const SourcePosition& a, const SourcePosition& b) {
            return a == b;
        })
        .def("__ne__", [](const SourcePosition& a, const SourcePosition& b) {
            return a != b;
        })
        .def("__lt__", [](const SourcePosition& a, const SourcePosition& b) {
            return a < b;
        })
        .def("__le__", [](const SourcePosition& a, const SourcePosition& b) {
            return a <= b;
        })
        .def("__gt__", [](const SourcePosition& a, const SourcePosition& b) {
            return a > b;
        })
        .def("__ge__", [](const SourcePosition& a, const SourcePosition& b) {
            return a >= b;
        })
        .def("__hash__", [](const SourcePosition& p) {
            return SourcePosition::Hash{}(p);
        })
        .def("__repr__", [](const SourcePosition& p) {
            return to_string(p);
        });

    // Range — immutable value type matching the Python frozen dataclass.
    py::class_<Range>(m, "Range")
        .def(py::init([](const SourcePosition& start, const SourcePosition& end) {
            return Range{start, end};
        }), py::arg("start"), py::arg("end"))
        .def_readonly("start", &Range::start)
        .def_readonly("end", &Range::end)
        .def_static("zero", &Range::zero)
        .def("__eq__", [](const Range& a, const Range& b) {
            return a == b;
        })
        .def("__ne__", [](const Range& a, const Range& b) {
            return a != b;
        })
        .def("__lt__", [](const Range& a, const Range& b) {
            return a < b;
        })
        .def("__hash__", [](const Range& r) {
            return Range::Hash{}(r);
        })
        .def("__repr__", [](const Range& r) {
            return to_string(r);
        });

    // DocumentBuffer — mutable (has lazy caches) but source is immutable after creation.
    py::class_<DocumentBuffer>(m, "DocumentBuffer")
        .def_static("from_source", &DocumentBuffer::from_source,
             py::arg("source"), py::arg("version") = py::none())
        .def_property_readonly("source", [](const DocumentBuffer& buf) {
            return std::string(buf.source());
        })
        .def_property_readonly("version", &DocumentBuffer::version)
        .def_property_readonly("lines", [](const DocumentBuffer& buf) {
            auto span = buf.lines();
            py::list result;
            for (auto sv : span) {
                result.append(py::str(std::string(sv)));
            }
            return result;
        })
        .def_property_readonly("line_starts", [](const DocumentBuffer& buf) {
            // Expose line_starts for compatibility with Python code that reads it.
            // Reconstruct from offset_to_line_col probing — or just compute.
            // For now, provide via the source.
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
        .def("offset_to_position", &DocumentBuffer::offset_to_position,
             py::arg("offset"))
        .def("position_to_offset", &DocumentBuffer::position_to_offset,
             py::arg("line"), py::arg("character"))
        .def("offset_to_line_col", [](const DocumentBuffer& buf, int32_t offset) {
            auto [line, col] = buf.offset_to_line_col(offset);
            return py::make_tuple(line, col);
        }, py::arg("offset"))
        .def("range_from_offsets", &DocumentBuffer::range_from_offsets,
             py::arg("start"), py::arg("end_inclusive"))
        .def("chunk_line_range", [](const DocumentBuffer& buf, int32_t start, int32_t end) {
            auto [sl, sc, el, ec] = buf.chunk_line_range(start, end);
            return py::make_tuple(sl, sc, el, ec);
        }, py::arg("start_offset"), py::arg("end_offset"));

    // MemoryStats — C++ heap memory snapshot via mallinfo2().
    py::class_<MemoryStats>(m, "MemoryStats")
        .def_readonly("arena_bytes", &MemoryStats::arena_bytes)
        .def_readonly("mmap_bytes", &MemoryStats::mmap_bytes)
        .def_readonly("used_bytes", &MemoryStats::used_bytes)
        .def_readonly("free_bytes", &MemoryStats::free_bytes)
        .def_readonly("total_allocated", &MemoryStats::total_allocated)
        .def("__repr__", [](const MemoryStats& s) {
            return "MemoryStats(arena=" + std::to_string(s.arena_bytes)
                 + ", mmap=" + std::to_string(s.mmap_bytes)
                 + ", used=" + std::to_string(s.used_bytes)
                 + ", free=" + std::to_string(s.free_bytes)
                 + ", total=" + std::to_string(s.total_allocated) + ")";
        });

    m.def("memory_stats", &memory_stats,
          "Query C++ heap memory usage (mallinfo2 on Linux).");

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
        .def(py::init([](TokenType type, const std::string& text,
                         const SourcePosition& start, const SourcePosition& end,
                         bool in_quote) {
            return Token{type, text, start, end, in_quote};
        }),
             py::arg("type"), py::arg("text"),
             py::arg("start"), py::arg("end"),
             py::arg("in_quote") = false)
        .def_readonly("type", &Token::type)
        .def_readonly("text", &Token::text)
        .def_readonly("start", &Token::start)
        .def_readonly("end", &Token::end)
        .def_readonly("in_quote", &Token::in_quote)
        .def("__eq__", [](const Token& a, const Token& b) {
            return a == b;
        })
        .def("__repr__", [](const Token& t) {
            return to_string(t);
        });

    // TclParseError — maps to Python exception.
    py::register_exception<TclParseError>(m, "TclParseError", PyExc_RuntimeError);

    // LexerConfig — configuration flags for the lexer.
    py::class_<LexerConfig>(m, "LexerConfig")
        .def(py::init<>())
        .def(py::init([](bool strict_quoting, bool expand_syntax,
                         bool irules_brace_separator) {
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
                         py::object virtual_insertions,
                         py::object line_starts_obj) {
            // Convert Python dict to C++ map.
            std::unordered_map<int32_t, char> vi;
            if (!virtual_insertions.is_none()) {
                auto dict = virtual_insertions.cast<py::dict>();
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

            return TclLexer(std::string(text), config, base_offset, base_line, base_col,
                            std::move(vi), ls_ptr, OwningTag{});
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
        .def_property_readonly("warnings", [](const TclLexer& lexer) {
            py::list result;
            for (auto& [pos, msg] : lexer.warnings()) {
                result.append(py::make_tuple(pos, msg));
            }
            return result;
        })
        .def_property_readonly("text", [](const TclLexer& lexer) {
            return std::string(lexer.text());
        })
        .def_property_readonly("line_starts", [](const TclLexer& lexer) {
            return lexer.line_starts();
        });
}

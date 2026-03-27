#include <pybind11/pybind11.h>
#include <pybind11/stl.h>

#include "tcl_lsp/core/source_position.hpp"
#include "tcl_lsp/core/range.hpp"
#include "tcl_lsp/core/document_buffer.hpp"

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
}

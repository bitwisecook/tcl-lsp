#include "tcl_lsp/lsp/python_bridge.hpp"

#include <pybind11/embed.h>
#include <pybind11/functional.h>
#include <pybind11/stl.h>

#include <cstdio>
#include <mutex>
#include <stdexcept>
#include <utility>

namespace py = pybind11;

namespace tcl_lsp {

struct PythonBridge::Impl {
    py::scoped_interpreter guard{};
    py::module_ bridge;
    NotificationCallback notification_cb;
    std::mutex mutex;  // protects notification_cb

    Impl() {
        // Add the project root to sys.path so Python can find our modules.
        py::module_::import("sys").attr("path").attr("insert")(0, ".");
        bridge = py::module_::import("lsp._native_bridge");
    }
};

PythonBridge::PythonBridge()
    : impl_{new Impl{}}
{
}

PythonBridge::~PythonBridge() {
    delete impl_;
}

void PythonBridge::set_notification_callback(NotificationCallback cb) {
    {
        std::lock_guard lock{impl_->mutex};
        impl_->notification_cb = std::move(cb);
    }

    // Register a Python-callable wrapper that invokes our C++ callback.
    py::gil_scoped_acquire gil;
    auto py_callback = py::cpp_function(
        [this](const std::string& method, const std::string& params_json) {
            NotificationCallback cb;
            {
                std::lock_guard lock{impl_->mutex};
                cb = impl_->notification_cb;
            }
            if (cb) {
                cb(method, params_json);
            }
        });
    impl_->bridge.attr("set_notification_callback")(py_callback);
}

void PythonBridge::on_initialized(const std::string& workspace_folders_json,
                                  const std::string& settings_json) {
    py::gil_scoped_acquire gil;
    impl_->bridge.attr("on_initialized")(workspace_folders_json,
                                          settings_json);
}

void PythonBridge::on_did_open(const std::string& uri,
                               const std::string& language_id,
                               const std::string& text,
                               int32_t version) {
    py::gil_scoped_acquire gil;
    impl_->bridge.attr("on_did_open")(uri, language_id, text, version);
}

void PythonBridge::on_did_change(const std::string& uri,
                                 const std::string& text,
                                 int32_t version) {
    py::gil_scoped_acquire gil;
    impl_->bridge.attr("on_did_change")(uri, text, version);
}

void PythonBridge::on_did_close(const std::string& uri) {
    py::gil_scoped_acquire gil;
    impl_->bridge.attr("on_did_close")(uri);
}

void PythonBridge::on_did_change_configuration(
    const std::string& settings_json) {
    py::gil_scoped_acquire gil;
    impl_->bridge.attr("on_did_change_configuration")(settings_json);
}

auto PythonBridge::call_feature(const std::string& method,
                                const std::string& params_json)
    -> std::optional<std::string> {
    py::gil_scoped_acquire gil;
    try {
        auto result = impl_->bridge.attr("handle_feature")(method,
                                                            params_json);
        if (result.is_none()) return std::nullopt;
        return result.cast<std::string>();
    } catch (const py::error_already_set& e) {
        std::fprintf(stderr, "[tcl-lsp] Python exception in %s: %s\n",
                     method.c_str(), e.what());
        return std::nullopt;
    }
}

auto PythonBridge::execute_command(const std::string& command,
                                   const std::string& arguments_json)
    -> std::optional<std::string> {
    py::gil_scoped_acquire gil;
    try {
        auto result = impl_->bridge.attr("handle_command")(command,
                                                            arguments_json);
        if (result.is_none()) return std::nullopt;
        return result.cast<std::string>();
    } catch (const py::error_already_set& e) {
        std::fprintf(stderr, "[tcl-lsp] Python exception in command %s: %s\n",
                     command.c_str(), e.what());
        return std::nullopt;
    }
}

} // namespace tcl_lsp

#pragma once

#include <functional>
#include <optional>
#include <string>
#include <string_view>

namespace tcl_lsp {

// Callback type for server-to-client notifications originating from Python.
// The C++ server registers this so Python can send diagnostics, log messages, etc.
using NotificationCallback = std::function<void(std::string_view method,
                                                const std::string& params_json)>;

// Bridge between the C++ LSP server and the Python feature implementations.
//
// Embeds a Python interpreter, imports the bridge module, and provides
// methods to call Python feature functions.  All data crosses the boundary
// as JSON strings — no complex type conversion needed since LSP is JSON-RPC.
//
// Thread safety: all public methods acquire the GIL internally.
class PythonBridge {
public:
    PythonBridge();
    ~PythonBridge();

    PythonBridge(const PythonBridge&) = delete;
    PythonBridge& operator=(const PythonBridge&) = delete;
    PythonBridge(PythonBridge&&) = delete;
    PythonBridge& operator=(PythonBridge&&) = delete;

    // Set the callback for server→client notifications from Python.
    void set_notification_callback(NotificationCallback cb);

    // Lifecycle: called once after initialize handshake completes.
    void on_initialized(const std::string& workspace_folders_json,
                        const std::string& settings_json);

    // Document sync: forward to Python workspace state.
    void on_did_open(const std::string& uri, const std::string& language_id,
                     const std::string& text, int32_t version);
    void on_did_change(const std::string& uri, const std::string& text,
                       int32_t version);
    void on_did_close(const std::string& uri);

    // Configuration change.
    void on_did_change_configuration(const std::string& settings_json);

    // Generic feature call: takes a method name and JSON params, returns JSON result.
    // Returns nullopt if the feature is disabled or returned null.
    [[nodiscard]] auto call_feature(const std::string& method,
                                    const std::string& params_json)
        -> std::optional<std::string>;

    // Execute a custom command (workspace/executeCommand).
    [[nodiscard]] auto execute_command(const std::string& command,
                                      const std::string& arguments_json)
        -> std::optional<std::string>;

private:
    struct Impl;
    Impl* impl_;  // prevent pybind11 header leaking into public header
};

} // namespace tcl_lsp

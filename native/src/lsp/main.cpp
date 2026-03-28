#include "tcl_lsp/lsp/server.hpp"

#include <lsp/connection.h>
#include <lsp/io/standardio.h>

#include <exception>
#include <iostream>

int main() {
    try {
        auto& io = lsp::io::standardIO();
        auto connection = lsp::Connection(io);
        auto server = tcl_lsp::TclLspServer(connection);
        server.run();
    } catch (const std::exception& e) {
        std::cerr << "tcl-lsp: fatal error: " << e.what() << std::endl;
        return 1;
    }
    return 0;
}

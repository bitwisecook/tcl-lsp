# KCS: How do I build and install tcl-lsp on an unsupported platform?

> **Audience:** User
> **Type:** How-To

## Applies to

all-editors, MCP, claude-skill

## Question

How do I build and install tcl-lsp on a platform without an installer build?

## Before you start

- Install Git, GNU Make, and the current stable Rust toolchain from `rustup`.
- Install the platform's native linker and C build tools. This is usually Xcode
  Command Line Tools on macOS, `build-essential` on Debian or Ubuntu, the
  Development Tools group on Fedora or RHEL, or the equivalent for your
  platform.

## Answer

The shell installer supports macOS on x86-64 and Arm64, and Linux on x86-64,
Arm64, and RISC-V 64. Releases also contain x86-64 and Arm64 Windows binaries,
which Windows users can download and install manually. On another platform,
build the native programs you need from source. You do not need the
dependencies for features you will not build.

### Build the native programs

Every native program needs:

- the current stable Rust toolchain from `rustup`, including `cargo` and
  `rustc`; and
- the native build tools listed above.

Clone the `rust` branch, then build only the programs you want:

```sh
git clone --branch rust https://github.com/bitwisecook/tcl-lsp.git
cd tcl-lsp

cargo build --release -p tcl-lsp-server  # language server
cargo build --release -p tcl-cli         # tcl command
cargo build --release -p f5-cli          # f5-query command
cargo build --release -p tcl-mcp         # MCP server for AI clients
```

The outputs are `target/release/tcl-lsp-server`, `target/release/tcl`,
`target/release/f5-query`, and `target/release/tcl-mcp`. Install only those
you built:

```sh
install -d "$HOME/.local/bin"
install -m 0755 target/release/tcl-lsp-server "$HOME/.local/bin/"
install -m 0755 target/release/tcl "$HOME/.local/bin/"
install -m 0755 target/release/f5-query "$HOME/.local/bin/f5"
install -m 0755 target/release/tcl-mcp "$HOME/.local/bin/"
```

Add `$HOME/.local/bin` to `PATH`. A standalone native binary contains the
shipped command specifications, so a separate Python installation is not
needed.

Register the optional MCP server with the client you use:

```sh
claude mcp add tcl-lsp -- "$HOME/.local/bin/tcl-mcp"
codex mcp add tcl_lsp -- "$HOME/.local/bin/tcl-mcp"
```

### Extra dependencies by feature

| Feature to build | Additional dependencies | Build command or note |
|---|---|---|
| Native server, `tcl`, `f5`, or MCP server | None beyond the native requirements above | Use the matching `cargo build` command above. |
| VS Code extension | Node.js 24+, Corepack, npm 12, `rsync`, `zip`, and `unzip` | Run `corepack enable npm`, `npm ci` in `editors/vscode`, and `make package-vsix BUNDLED_TARGETS=`. Then set `tclLsp.rustServerPath` to your locally built server. The normal release package targets need the published target binaries instead. |
| JetBrains extension | A Java Development Kit 17, Node.js for the optional compiler-explorer page, `zip`, and `unzip` | Run `./gradlew buildPlugin` in `editors/jetbrains`, then set **Settings → Tools → Tcl Language Server → Server path** to your locally built server. The Gradle wrapper downloads Gradle and the Kotlin compiler. A separately installed `kotlinc` is needed only for the repository's Kotlin catalogue check. |
| Zed extension | `rustup`, the Rust `wasm32-wasip2` target, Node.js, and `zip` | Run `rustup target add wasm32-wasip2`, then `make build-editor-zed`. The extension downloads a native server at run time; an unsupported platform also needs a Zed-side server-download mapping before it can be distributed. |
| Sublime Text package | `zip` and `unzip` | Run `make build-editor-sublime`. The package carries no server: the plugin downloads the published build for the host, so on an unsupported platform build the server yourself and set `server_path` in the TclLsp LSP settings. |
| Neovim, Emacs, Helix, Vim, and other external-client configurations | No build dependency beyond the native server | Point the client configuration at `tcl-lsp-server`. The editor itself is needed only to use or test that integration. |
| Compiler Explorer browser module | `wasm-pack`, the Rust `wasm32-unknown-unknown` target, and Node.js for verification | Run `rustup target add wasm32-unknown-unknown`, install `wasm-pack`, then run `make explorer-wasm`. |
| Rust WebAssembly runtime and linked WebAssembly tests | the Rust `wasm32-wasip1` target, wasi-sdk, Binaryen (`wasm-merge` and `wasm-opt`), and Wasmtime | These are not needed for the native server or command-line programs. wasi-sdk supplies the WebAssembly C compiler and system root used for the numeric runtime. Wasmtime executes the result; it is not needed merely to compile the native tools. |

### Full contributor test suite

The complete test suite has a wider dependency set than a product build. Run
this check to see what the repository would install without changing the
machine:

```sh
bash scripts/dev/ensure-test-deps.sh --check
```

The full suite additionally uses Tcl 8.6 and 9.0 shells, tcllib, Emacs, Xvfb
on headless Linux, `tshark`, OpenSSL, `ping`, `rgxg`, Python with Tk, and `uv`.
Those tools support tests and reports; none is a runtime dependency of the
native tcl-lsp programs.

## How to tell it worked

Run each installed program with `--help`. For the MCP server, start the client
you registered and confirm that its tcl-lsp tools are available.

## Related

- [KCS index](README.md)
- [Build the multi-platform VS Code extension](kcs-howto-build-multiplatform-vsix.md)

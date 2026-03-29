#include "tcl_lsp/lsp/known_commands.hpp"

#include <string_view>
#include <unordered_set>

namespace tcl_lsp {
namespace {

// Built from REGISTRY.command_names() + control-flow words + TclOO keywords.
// This static set covers the standard Tcl/Tk/TclOO command inventory and is
// used when no dynamic CommandRegistryInterface is available.
const std::unordered_set<std::string_view>& keyword_set() {
    static const std::unordered_set<std::string_view> s = {
        // Core Tcl commands
        "after", "append", "apply", "array", "auto_execok", "auto_import",
        "auto_load", "auto_mkindex", "auto_qualify", "auto_reset",
        "binary", "break",
        "catch", "cd", "chan", "clock", "close", "concat", "continue",
        "coroutine",
        "dict",
        "encoding", "eof", "error", "eval", "exec", "exit", "expr",
        "fblocked", "fconfigure", "fcopy", "file", "fileevent", "flush",
        "for", "foreach", "format",
        "gets", "glob", "global",
        "history",
        "if", "incr", "info", "interp",
        "join",
        "lappend", "lassign", "lindex", "linsert", "list", "llength",
        "lmap", "load", "lrange", "lrepeat", "lreplace", "lreverse",
        "lsearch", "lset", "lsort",
        "namespace",
        "open",
        "package", "parray", "pid", "proc", "puts", "pwd",
        "read", "regexp", "regsub", "rename", "return",
        "scan", "seek", "set", "socket", "source", "split", "string",
        "subst", "switch",
        "tailcall", "tell", "throw", "time", "trace", "try",
        "unknown", "unload", "unset", "update", "uplevel", "upvar",
        "variable", "vwait",
        "while",
        "yield", "yieldto",

        // TclOO
        "oo::class", "oo::copy", "oo::define", "oo::objdefine", "oo::object",

        // Tk commands
        "bell", "bind", "bindtags", "bitmap", "button",
        "canvas", "checkbutton", "clipboard", "colors",
        "console", "cursors", "destroy",
        "entry", "event",
        "focus", "font", "frame",
        "grab", "grid",
        "image",
        "keysyms",
        "label", "labelframe", "listbox", "lower",
        "menu", "menubutton", "message",
        "option",
        "pack", "panedwindow", "photo", "place",
        "radiobutton", "raise",
        "scale", "scrollbar", "selection", "send", "spinbox",
        "text", "tk", "tk_bisque", "tk_chooseColor",
        "tk_chooseDirectory", "tk_dialog", "tk_focusFollowsMouse",
        "tk_focusNext", "tk_focusPrev", "tk_getOpenFile",
        "tk_getSaveFile", "tk_messageBox", "tk_optionMenu",
        "tk_popup", "tk_setPalette", "tk_textCopy", "tk_textCut",
        "tk_textPaste", "tkerror", "tkwait",
        "toplevel", "ttk::button", "ttk::checkbutton", "ttk::combobox",
        "ttk::entry", "ttk::frame", "ttk::intro", "ttk::label",
        "ttk::labelframe", "ttk::menubutton", "ttk::notebook",
        "ttk::panedwindow", "ttk::progressbar", "ttk::radiobutton",
        "ttk::scale", "ttk::scrollbar", "ttk::separator", "ttk::sizegrip",
        "ttk::spinbox", "ttk::style", "ttk::treeview", "ttk::widget",
        "winfo", "wm",

        // Control-flow words (appear as arguments, not standalone)
        "else", "elseif",

        // TclOO definition-context keywords
        "constructor", "destructor", "method", "my", "next", "self",
        "forward", "mixin", "filter", "superclass",
        "renamemethod", "deletemethod", "export", "unexport",

        // tcltest (imported unqualified)
        "test", "cleanupTests", "makeFile", "removeFile",
        "makeDirectory", "removeDirectory", "viewFile",
        "customMatch", "testConstraint",
    };
    return s;
}

const std::unordered_set<std::string_view>& operator_set() {
    static const std::unordered_set<std::string_view> s = {
        "+", "-", "*", "/", ">", ">=", "<", "<=", "==", "!=",
    };
    return s;
}

} // anonymous namespace

auto is_keyword(std::string_view name) -> bool {
    return keyword_set().count(name) > 0;
}

auto is_operator(std::string_view name) -> bool {
    return operator_set().count(name) > 0;
}

} // namespace tcl_lsp

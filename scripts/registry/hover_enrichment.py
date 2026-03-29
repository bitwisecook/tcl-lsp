"""Supplementary hover text enrichment for commands that lack snippets/examples.

All text here is ORIGINAL — not copied from any vendor documentation.
These descriptions and examples are written from scratch to document
the same functionality in our own words.
"""

# ─── Expect commands ────────────────────────────────────────────────

EXPECT_ENRICHMENT: dict[str, dict[str, str]] = {
    "spawn": {
        "snippet": (
            "Launches an external program and sets up bidirectional communication "
            "channels for send/expect interaction. The spawned process runs "
            "asynchronously. On success, the spawn_id variable is set to the "
            "connection handle, and the process id is available via exp_pid."
        ),
        "examples": (
            "# Launch an SSH session and wait for the password prompt\n"
            "spawn ssh admin@192.168.1.1\n"
            "expect \"password:\"\n"
            "send \"secret123\\r\"\n"
            "expect \"$ \""
        ),
    },
    "expect": {
        "snippet": (
            "Blocks until the spawned process produces output matching one of "
            "the given patterns, a timeout occurs, or end-of-file is reached. "
            "Patterns can be glob-style (default), regular expressions (-re), "
            "or exact strings (-exact). When a pattern matches, its associated "
            "body script is evaluated."
        ),
        "examples": (
            "# Match multiple patterns with different actions\n"
            "expect {\n"
            "    \"login:\" {\n"
            "        send \"admin\\r\"\n"
            "        exp_continue\n"
            "    }\n"
            "    \"Password:\" {\n"
            "        send \"$password\\r\"\n"
            "    }\n"
            "    timeout {\n"
            "        puts \"Connection timed out\"\n"
            "        exit 1\n"
            "    }\n"
            "    eof {\n"
            "        puts \"Connection closed unexpectedly\"\n"
            "    }\n"
            "}"
        ),
    },
    "send": {
        "snippet": (
            "Transmits a string to the current spawned process. The string is "
            "sent exactly as provided — use \\r for carriage return (simulating "
            "pressing Enter). Use -i spawn_id to target a specific process when "
            "multiple are active."
        ),
        "examples": (
            "# Send a command followed by Enter\n"
            "send \"show version\\r\"\n"
            "\n"
            "# Send to a specific spawned process\n"
            "send -i $router_id \"show interfaces\\r\"\n"
            "\n"
            "# Send slowly (one char at a time) for sensitive prompts\n"
            "send -s \"delicate_input\\r\""
        ),
    },
    "interact": {
        "snippet": (
            "Transfers control of the spawned process to the user, allowing "
            "direct keyboard interaction. Optionally intercepts specific key "
            "sequences to trigger script actions. Control returns to the "
            "script when the process exits or an escape sequence is typed."
        ),
        "examples": (
            "# Hand off to user after automated login\n"
            "spawn ssh admin@server\n"
            "expect \"password:\"\n"
            "send \"$pass\\r\"\n"
            "expect \"$ \"\n"
            "interact\n"
            "\n"
            "# Intercept Ctrl+C to perform cleanup\n"
            "interact {\n"
            "    \\003 {\n"
            "        send_user \"Caught interrupt, cleaning up...\\n\"\n"
            "        send \"logout\\r\"\n"
            "    }\n"
            "}"
        ),
    },
    "exp_continue": {
        "examples": (
            "# Keep matching after handling each prompt\n"
            "expect {\n"
            "    \"username:\" {\n"
            "        send \"admin\\r\"\n"
            "        exp_continue\n"
            "    }\n"
            "    \"password:\" {\n"
            "        send \"$pass\\r\"\n"
            "        exp_continue\n"
            "    }\n"
            "    \"$ \" {\n"
            "        # Logged in, stop matching\n"
            "    }\n"
            "}"
        ),
    },
    "close": {
        "snippet": (
            "Terminates the connection to the spawned process. The process "
            "may continue running; use wait afterwards to collect its exit "
            "status and prevent zombie processes."
        ),
        "examples": (
            "# Close the connection and collect exit status\n"
            "close\n"
            "set result [wait]\n"
            "set exit_code [lindex $result 3]"
        ),
    },
    "wait": {
        "examples": (
            "# Wait for process and check exit status\n"
            "set result [wait]\n"
            "set pid [lindex $result 0]\n"
            "set spawn [lindex $result 1]\n"
            "set os_error [lindex $result 2]\n"
            "set exit_status [lindex $result 3]\n"
            "if {$exit_status != 0} {\n"
            "    puts \"Process $pid exited with status $exit_status\"\n"
            "}"
        ),
    },
    "log_file": {
        "snippet": (
            "Opens, closes, or appends to a log file that captures all "
            "session input and output. Useful for audit trails and "
            "debugging automated interactions."
        ),
        "examples": (
            "# Start logging to a file\n"
            "log_file -a session.log\n"
            "\n"
            "# Stop logging\n"
            "log_file"
        ),
    },
    "log_user": {
        "examples": (
            "# Suppress output during password entry\n"
            "log_user 0\n"
            "send \"$password\\r\"\n"
            "expect \"$ \"\n"
            "log_user 1"
        ),
    },
    "exp_internal": {
        "examples": (
            "# Enable debug tracing of pattern matching\n"
            "exp_internal 1\n"
            "expect \"ready\"\n"
            "exp_internal 0"
        ),
    },
    "debug": {
        "snippet": (
            "Activates the interactive Expect debugger, which allows "
            "stepping through scripts, inspecting variables, and "
            "diagnosing pattern matching issues."
        ),
        "examples": (
            "# Enable the debugger\n"
            "debug 1\n"
            "\n"
            "# Disable the debugger\n"
            "debug 0"
        ),
    },
    "fork": {
        "snippet": (
            "Creates a child copy of the current Expect process. Returns "
            "zero in the child process and the child's PID in the parent. "
            "Both parent and child retain access to all spawned connections."
        ),
        "examples": (
            "set child_pid [fork]\n"
            "if {$child_pid == 0} {\n"
            "    # In child process\n"
            "    send \"child task\\r\"\n"
            "    expect \"done\"\n"
            "    exit 0\n"
            "} else {\n"
            "    # In parent process\n"
            "    puts \"Forked child: $child_pid\"\n"
            "}"
        ),
    },
    "disconnect": {
        "examples": (
            "# Daemonize: disconnect from terminal and run in background\n"
            "if {[fork] != 0} exit\n"
            "disconnect\n"
            "# Now running as a background daemon\n"
            "spawn tail -f /var/log/syslog\n"
            "expect -re {error|warning} {\n"
            "    exec mail -s \"Alert\" admin@example.com << $expect_out(0,string)\n"
            "    exp_continue\n"
            "}"
        ),
    },
    "trap": {
        "snippet": (
            "Registers a handler script that runs when the specified signal "
            "is received. Commonly used to perform cleanup on SIGINT or "
            "SIGTERM during long-running automation."
        ),
        "examples": (
            "# Clean up on Ctrl+C\n"
            "trap {\n"
            "    send \"exit\\r\"\n"
            "    expect eof\n"
            "    exit 0\n"
            "} SIGINT"
        ),
    },
    "stty": {
        "snippet": (
            "Queries or modifies terminal settings such as raw mode, echo, "
            "and terminal dimensions. Useful for controlling how input is "
            "processed during interactive sessions."
        ),
        "examples": (
            "# Disable echo for password entry\n"
            "stty -echo\n"
            "expect_user -re \"(.*)\\n\"\n"
            "set password $expect_out(1,string)\n"
            "stty echo"
        ),
    },
    "match_max": {
        "snippet": (
            "Controls the size of the buffer used for pattern matching. "
            "Increase this when expecting large output blocks, or decrease "
            "it to reduce memory usage."
        ),
        "examples": (
            "# Increase buffer to handle large output\n"
            "match_max 100000\n"
            "\n"
            "# Query current buffer size\n"
            "set current [match_max]"
        ),
    },
    "sleep": {
        "examples": (
            "# Wait 2 seconds between commands\n"
            "send \"reboot\\r\"\n"
            "sleep 30\n"
            "spawn ssh admin@192.168.1.1"
        ),
    },
    "timestamp": {
        "snippet": (
            "Returns the current epoch time or formats a timestamp. "
            "Supports -format for strftime-style formatting and -seconds "
            "to convert a specific epoch value."
        ),
        "examples": (
            "# Get current epoch time\n"
            "set now [timestamp]\n"
            "\n"
            "# Format as human-readable\n"
            "set formatted [timestamp -format \"%Y-%m-%d %H:%M:%S\"]"
        ),
    },
    "send_user": {
        "snippet": (
            "Writes a string directly to standard output (the user's terminal), "
            "bypassing the spawned process. Useful for displaying status messages "
            "during automated sessions."
        ),
        "examples": (
            "send_user \"Connecting to server...\\n\"\n"
            "spawn ssh admin@server\n"
            "send_user \"Waiting for login prompt...\\n\""
        ),
    },
    "send_error": {
        "snippet": (
            "Writes a string to standard error. Useful for error messages "
            "that should appear even when stdout is redirected."
        ),
        "examples": (
            "send_error \"Error: failed to connect to host\\n\""
        ),
    },
    "send_log": {
        "snippet": (
            "Writes a string to the log file only, without sending it to "
            "the spawned process or displaying it to the user."
        ),
        "examples": (
            "send_log \"=== Session started at [timestamp -format \"%c\"] ===\\n\""
        ),
    },
    "send_tty": {
        "snippet": (
            "Sends a string directly to the controlling terminal, bypassing "
            "both the spawned process and stdout redirection."
        ),
        "examples": (
            "send_tty \"\\a\"  ;# Ring the terminal bell"
        ),
    },
    "exp_pid": {
        "examples": (
            "spawn ssh admin@server\n"
            "set pid [exp_pid]\n"
            "puts \"SSH process ID: $pid\""
        ),
    },
    "exp_version": {
        "examples": (
            "# Require Expect 5.45 or newer\n"
            "exp_version 5.45"
        ),
    },
    "system": {
        "snippet": (
            "Runs a command through the system shell (/bin/sh -c). Unlike "
            "Tcl's exec, the system command connects the child directly to "
            "the terminal for I/O."
        ),
        "examples": (
            "# Run a shell command\n"
            "system \"date >> /tmp/expect_log.txt\""
        ),
    },
    "overlay": {
        "snippet": (
            "Replaces the current Expect process entirely with the specified "
            "program (similar to Unix exec). All file descriptors from "
            "spawned processes remain open and accessible to the new program."
        ),
        "examples": (
            "# After setup, hand off to the real application\n"
            "spawn ssh -L 8080:localhost:80 admin@server\n"
            "expect \"password:\"\n"
            "send \"$pass\\r\"\n"
            "expect \"$ \"\n"
            "overlay -0 $spawn_id firefox http://localhost:8080"
        ),
    },
    "expect_after": {
        "snippet": (
            "Defines fallback patterns that are checked after the patterns "
            "in each expect command. Commonly used for global error handling "
            "or timeout behavior across all expect calls."
        ),
        "examples": (
            "# Handle unexpected disconnects globally\n"
            "expect_after {\n"
            "    timeout {\n"
            "        send_user \"Timed out waiting for response\\n\"\n"
            "        exit 1\n"
            "    }\n"
            "    eof {\n"
            "        send_user \"Connection lost\\n\"\n"
            "        exit 1\n"
            "    }\n"
            "}"
        ),
    },
    "expect_before": {
        "snippet": (
            "Defines patterns that are checked before the patterns in each "
            "expect command. Useful for intercepting prompts or messages "
            "that can appear at any point during a session."
        ),
        "examples": (
            "# Automatically handle \"more\" paging prompts\n"
            "expect_before {\n"
            "    -re \"--More--\" {\n"
            "        send \" \"\n"
            "        exp_continue\n"
            "    }\n"
            "}"
        ),
    },
    "expect_background": {
        "examples": (
            "# Monitor a process without blocking\n"
            "spawn tail -f /var/log/messages\n"
            "expect_background {\n"
            "    -re \"CRITICAL:(.*)\" {\n"
            "        puts \"ALERT: $expect_out(1,string)\"\n"
            "    }\n"
            "}"
        ),
    },
    "expect_user": {
        "snippet": (
            "Reads input from the user's terminal (stdin) using expect-style "
            "pattern matching. Useful for interactive prompts in scripts."
        ),
        "examples": (
            "send_user \"Enter hostname: \"\n"
            "expect_user -re \"(.*)\\n\"\n"
            "set hostname $expect_out(1,string)"
        ),
    },
    "expect_tty": {
        "snippet": (
            "Reads input from the controlling terminal using expect-style "
            "pattern matching, even when stdin has been redirected."
        ),
        "examples": (
            "send_tty \"Press Enter to continue...\"\n"
            "expect_tty \"\\n\""
        ),
    },
    "remove_nulls": {
        "snippet": (
            "Controls whether null (\\0) bytes are stripped from spawned "
            "process output before pattern matching. Enabled by default."
        ),
        "examples": (
            "# Keep null bytes in binary data\n"
            "remove_nulls 0"
        ),
    },
    "parity": {
        "snippet": (
            "Controls whether the parity bit is preserved on data from "
            "spawned processes. Most modern systems use 8-bit clean "
            "connections and don't need parity adjustment."
        ),
        "examples": (
            "# Strip parity bits from serial device output\n"
            "parity 0"
        ),
    },
    "strace": {
        "snippet": (
            "Enables internal tracing of Expect statement execution at "
            "the specified detail level. Level 0 disables tracing."
        ),
        "examples": (
            "# Trace expect internals at maximum detail\n"
            "strace 4"
        ),
    },
}

# ─── iApps/TMSH commands ───────────────────────────────────────────

IAPPS_ENRICHMENT: dict[str, dict[str, str]] = {
    "tmsh::create": {
        "snippet": (
            "Mirrors the tmsh create command, accepting the same components, "
            "object identifiers, and properties. When tmsh::stateless is "
            "enabled (the default in iApps), create acts as create-or-modify."
        ),
        "examples": (
            "# Create a pool with two members\n"
            "tmsh::create ltm pool my_pool \\\n"
            "    members add { 10.0.0.1:80 10.0.0.2:80 } \\\n"
            "    monitor http"
        ),
        "return_value": "",
    },
    "tmsh::modify": {
        "snippet": (
            "Mirrors the tmsh modify command. Unlike tmsh::create in "
            "stateless mode, modify will fail if the target object does "
            "not already exist. Use when you specifically need to ensure "
            "the object was previously created."
        ),
        "examples": (
            "# Add a member to an existing pool\n"
            "tmsh::modify ltm pool my_pool \\\n"
            "    members add { 10.0.0.3:80 }"
        ),
        "return_value": "",
    },
    "tmsh::delete": {
        "snippet": (
            "Mirrors the tmsh delete command. Removes a configuration "
            "object from the running configuration."
        ),
        "examples": (
            "tmsh::delete ltm pool my_pool"
        ),
        "return_value": "",
    },
    "tmsh::list": {
        "snippet": (
            "Returns configuration as a raw string. For structured access "
            "to configuration fields, use tmsh::get_config instead, which "
            "returns Tcl objects compatible with tmsh::get_field_value."
        ),
        "examples": (
            "# List a specific pool configuration\n"
            "set config [tmsh::list ltm pool my_pool]\n"
            "\n"
            "# List all virtual servers\n"
            "set vs_list [tmsh::list ltm virtual]"
        ),
        "return_value": "String \u2014 raw configuration text of the specified object(s).",
    },
    "tmsh::show": {
        "snippet": (
            "Returns runtime statistics and status as a raw string. "
            "For structured access to status fields, use tmsh::get_status "
            "instead, which returns Tcl objects compatible with "
            "tmsh::get_field_value."
        ),
        "examples": (
            "# Show pool statistics\n"
            "set stats [tmsh::show ltm pool my_pool]"
        ),
        "return_value": "String \u2014 runtime statistics and status text.",
    },
    "tmsh::get_field_value": {
        "snippet": (
            "Extracts a field value from a tmsh::get_config or "
            "tmsh::get_status object. Important: without the optional "
            "variable argument, a missing field throws a fatal error. "
            "With a variable argument, it sets the variable and returns "
            "1 if found, 0 if missing \u2014 use this form for optional fields."
        ),
        "examples": (
            "set obj [tmsh::get_config ltm pool my_pool]\n"
            "# Safe form: returns 0/1, sets variable\n"
            "if { [tmsh::get_field_value [lindex $obj 0] monitor mon_val] } {\n"
            "    puts \"Monitor: $mon_val\"\n"
            "}"
        ),
        "return_value": (
            "When called with a variable: Integer \u2014 1 if field exists "
            "(variable set), 0 if not. Without variable: String \u2014 the "
            "field value, or fatal error if missing."
        ),
    },
    "tmsh::get_field_names": {
        "snippet": (
            "Returns field names from a tmsh::get_config or "
            "tmsh::get_status object. Accepts a type selector: 'value' "
            "for scalar fields, 'nested' for sub-objects."
        ),
        "examples": (
            "set obj [tmsh::get_config ltm pool my_pool]\n"
            "set fields [tmsh::get_field_names value [lindex $obj 0]]\n"
            "foreach f $fields {\n"
            "    puts \"Field: $f\"\n"
            "}"
        ),
        "return_value": "List \u2014 field names of the specified type from the object.",
    },
    "tmsh::get_name": {
        "snippet": (
            "Extracts the object name from a configuration block."
        ),
        "examples": (
            "set config [tmsh::list ltm pool my_pool]\n"
            "set name [tmsh::get_name $config]"
        ),
    },
    "tmsh::get_type": {
        "snippet": (
            "Returns the object type from a configuration block."
        ),
        "examples": (
            "set config [tmsh::list ltm pool my_pool]\n"
            "set type [tmsh::get_type $config]"
        ),
    },
    "tmsh::get_status": {
        "snippet": (
            "Returns the runtime status of a configuration object."
        ),
        "examples": (
            "set status [tmsh::get_status ltm pool my_pool]\n"
            "puts \"Pool status: $status\""
        ),
    },
    "tmsh::get_config": {
        "snippet": (
            "Returns configuration as structured Tcl objects, unlike "
            "tmsh::list which returns raw text. The returned objects "
            "work with tmsh::get_name, tmsh::get_field_names, and "
            "tmsh::get_field_value for structured field access."
        ),
        "examples": (
            "set objs [tmsh::get_config ltm pool my_pool]\n"
            "set pool_obj [lindex $objs 0]\n"
            "set name [tmsh::get_name $pool_obj]"
        ),
        "return_value": "List \u2014 a list of Tcl configuration objects.",
    },
    "tmsh::begin_transaction": {
        "snippet": (
            "Starts an atomic transaction. All subsequent create/modify/"
            "delete operations are staged until committed. Important: "
            "read commands (tmsh::list, tmsh::show, tmsh::get_config, "
            "tmsh::get_status) cannot be used inside a transaction."
        ),
        "examples": (
            "tmsh::begin_transaction\n"
            "tmsh::create ltm pool web_pool members add { 10.0.0.1:80 }\n"
            "tmsh::create ltm virtual web_vs destination 10.0.1.1:80 pool web_pool\n"
            "tmsh::commit_transaction"
        ),
        "return_value": "",
    },
    "tmsh::commit_transaction": {
        "snippet": (
            "Commits all staged changes atomically. If any command fails, "
            "the entire transaction is rolled back. Same restriction as "
            "begin_transaction: no read commands during the transaction."
        ),
        "examples": (
            "tmsh::begin_transaction\n"
            "tmsh::modify ltm pool my_pool members add { 10.0.0.4:80 }\n"
            "tmsh::commit_transaction"
        ),
        "return_value": "",
    },
    "tmsh::cancel_transaction": {
        "snippet": (
            "Discards all staged changes from the current transaction "
            "without applying them."
        ),
        "examples": (
            "tmsh::begin_transaction\n"
            "# Oops, wrong pool name\n"
            "tmsh::cancel_transaction"
        ),
    },
    "tmsh::cd": {
        "snippet": (
            "Changes the current working folder within the TMOS "
            "configuration hierarchy."
        ),
        "examples": (
            "tmsh::cd /Common\n"
            "tmsh::list ltm pool"
        ),
    },
    "tmsh::pwd": {
        "snippet": (
            "Returns the current working folder in the TMOS configuration "
            "hierarchy."
        ),
        "examples": (
            "set current_folder [tmsh::pwd]"
        ),
    },
    "tmsh::log": {
        "snippet": (
            "Writes a message to the system log with the specified facility "
            "and severity level."
        ),
        "examples": (
            "tmsh::log \"iApp deployment completed successfully\""
        ),
    },
    "tmsh::log_dest": {
        "snippet": (
            "Sets or queries the log destination for tmsh::log output."
        ),
        "examples": (
            "tmsh::log_dest file /var/log/iapp_deploy.log"
        ),
    },
    "tmsh::log_level": {
        "snippet": (
            "Sets or queries the minimum log severity level. Messages "
            "below this level are suppressed."
        ),
        "examples": (
            "tmsh::log_level crit"
        ),
    },
    "tmsh::include": {
        "snippet": (
            "Sources and evaluates another TMSH script file within the "
            "current execution context."
        ),
        "examples": (
            "tmsh::include /config/iapps/lib/common_utils.tcl"
        ),
    },
    "tmsh::version": {
        "snippet": (
            "Returns the TMOS software version string."
        ),
        "examples": (
            "set tmos_ver [tmsh::version]\n"
            "puts \"Running TMOS: $tmos_ver\""
        ),
    },
    "tmsh::stateless": {
        "snippet": (
            "Marks the current tmsh session as stateless, meaning it does "
            "not maintain persistent configuration context between commands."
        ),
        "examples": (
            "tmsh::stateless enable"
        ),
    },
    "tmsh::display": {
        "snippet": (
            "Controls the output display mode for tmsh results."
        ),
        "examples": (
            "tmsh::display wide"
        ),
    },
    "tmsh::display_threshold": {
        "snippet": (
            "Sets the threshold for display truncation of large results."
        ),
        "examples": (
            "tmsh::display_threshold 1000"
        ),
    },
    "tmsh::reset_stats": {
        "snippet": (
            "Resets runtime statistics counters for the specified object."
        ),
        "examples": (
            "tmsh::reset_stats ltm pool my_pool"
        ),
    },
    "tmsh::clear_screen": {
        "snippet": "Clears the terminal screen in an interactive TMSH session.",
    },
    "tmsh::add_help": {
        "snippet": "Registers custom help text for a TMSH command or context.",
    },
    "tmsh::add_tabc": {
        "snippet": "Registers tab-completion entries for a TMSH command or context.",
    },
    "tmsh::builtin_help": {
        "snippet": "Displays built-in help text for the specified TMSH component.",
    },
    "tmsh::builtin_tabc": {
        "snippet": "Returns built-in tab-completion options for a TMSH component.",
    },
    "iapp::conf": {
        "snippet": (
            "Executes a tmsh command with integrated logging, caching, "
            "and password obscuring. Wraps tmsh::create, tmsh::modify, "
            "and tmsh::delete with debug-friendly output. Passwords in "
            "arguments are automatically masked in log output."
        ),
        "examples": (
            "iapp::conf create ltm pool my_pool \\\n"
            "    members add { 10.0.0.1:80 } monitor http"
        ),
        "return_value": "",
    },
    "iapp::template": {
        "snippet": (
            "Returns information about the current iApp template, such "
            "as its name and version."
        ),
        "examples": (
            "set tmpl [iapp::template name]"
        ),
    },
    "iapp::destination": {
        "snippet": (
            "Formats an IP address and port as a tmsh-compatible "
            "destination/mask string. Supports options for CIDR "
            "notation and route domain handling. Used to construct "
            "virtual server destination addresses from APL variables."
        ),
        "examples": (
            "set dest [iapp::destination $::pool__addr $::pool__port]\n"
            "iapp::conf create ltm virtual $::app_name \\\n"
            "    destination $dest"
        ),
        "return_value": "String \u2014 formatted destination address (e.g. 10.0.0.1:80 or 10.0.0.1%2:80).",
    },
    "iapp::debug": {
        "snippet": (
            "Logs a message with automatic obscuring of sensitive "
            "words listed in the $::SENSITIVES variable. Used for "
            "debug output during template deployment that won't "
            "accidentally leak passwords or keys to the log."
        ),
        "examples": (
            "set ::SENSITIVES [list password secret key]\n"
            "iapp::debug \"Creating user with password $pw\"\n"
            "# Logs: Creating user with password ********"
        ),
        "return_value": "",
    },
    "iapp::pool_members": {
        "snippet": (
            "Converts an APL table variable containing server addresses "
            "into the tmsh pool member format. Maps table columns to "
            "member properties (address, port, connection limit, etc.) "
            "for direct use in tmsh::create ltm pool commands."
        ),
        "examples": (
            "set members [iapp::pool_members pool__members]\n"
            "iapp::conf create ltm pool $::pool__name \\\n"
            "    members add { $members } monitor http"
        ),
        "return_value": "String \u2014 formatted pool member list for tmsh commands.",
    },
    "iapp::tmos_version": {
        "snippet": (
            "Returns the TMOS software version or performs version "
            "comparison. Called without arguments, returns the version "
            "string. Called with a version number, returns a boolean "
            "indicating whether the running version is greater or equal."
        ),
        "examples": (
            "# Version comparison for feature gating\n"
            "if {[iapp::tmos_version 12.1]} {\n"
            "    # Use features available in 12.1+\n"
            "}\n"
            "# Get raw version string\n"
            "set ver [iapp::tmos_version]"
        ),
        "return_value": "String (no args) or Integer (with version arg: 1 if running >= given, 0 otherwise).",
    },
    "iapp::is": {
        "snippet": (
            "Performs safe variable comparison, returning 1 if a variable "
            "matches any of the given strings, 0 otherwise. Crucially, "
            "returns 0 without error if the variable does not exist \u2014 "
            "essential for handling optional APL presentation variables "
            "that vanish when unchecked."
        ),
        "examples": (
            "if {[iapp::is basic__optimization \"wan\"]} {\n"
            "    iapp::conf create ltm profile tcp wan_opt\n"
            "}"
        ),
        "return_value": "Integer \u2014 1 if the variable matches any argument, 0 otherwise.",
    },
    "iapp::substa": {
        "snippet": (
            "Array-based string substitution with wildcard default support. "
            "Given an array of key-value pairs and a template string, "
            "replaces matching keys. Supports a single '*' wildcard entry "
            "as a default value. Eliminates long if-then-else chains for "
            "mapping APL selections to configuration values."
        ),
        "examples": (
            "array set profiles {\n"
            "    wan     /Common/tcp-wan-optimized\n"
            "    lan     /Common/tcp-lan-optimized\n"
            "    *       /Common/tcp\n"
            "}\n"
            "set tcp_profile [iapp::substa profiles $::basic__optimization]"
        ),
        "return_value": "String \u2014 the substituted value, or the wildcard default if no key matches.",
    },
    "iapp::get_items": {
        "snippet": (
            "Versatile configuration query utility. Supports options: "
            "-exists (boolean test), -nocomplain (suppress errors), "
            "-list (return as list), -norecursive (shallow query), "
            "-local (current folder only), -dir (directory listing), "
            "and -filter (match pattern)."
        ),
        "examples": (
            "# Check if a profile exists before using it\n"
            "if {[iapp::get_items -exists ltm profile http $prof_name]} {\n"
            "    iapp::conf modify ltm virtual $vs profiles add { $prof_name }\n"
            "}"
        ),
        "return_value": "Varies \u2014 boolean with -exists, list with -list, string otherwise.",
    },
    "iapp::make_safe_password": {
        "snippet": (
            "Escapes single quotes, hash signs, and backslashes in "
            "password strings so they can be safely embedded in tmsh "
            "commands without breaking argument parsing."
        ),
        "examples": (
            "set safe_pw [iapp::make_safe_password $raw_password]\n"
            "iapp::conf create auth user svc_acct password $safe_pw"
        ),
        "return_value": "String \u2014 the escaped password safe for tmsh arguments.",
    },
    "iapp::apm_config": {
        "snippet": (
            "Accesses APM (Access Policy Manager) configuration data "
            "within an iApp template."
        ),
    },
    "iapp::upgrade": {
        "snippet": (
            "Performs upgrade logic for an iApp application service, "
            "migrating from a previous template version."
        ),
    },
    "iapp::upgrade_template": {
        "snippet": (
            "Returns information about the template version being "
            "upgraded from."
        ),
    },
    "iapp::downgrade": {
        "snippet": (
            "Performs downgrade logic to revert an iApp application "
            "service to a previous template version."
        ),
    },
    "iapp::downgrade_template": {
        "snippet": (
            "Returns information about the template version being "
            "downgraded to."
        ),
    },
    "script::init": {
        "snippet": (
            "Initializes a TMSH script execution context with required "
            "dependencies and settings."
        ),
    },
    "script::run": {
        "snippet": (
            "Executes a named TMSH script."
        ),
        "examples": (
            "script::run /Common/my_maintenance_script"
        ),
    },
    "script::help": {
        "snippet": (
            "Displays or registers help text for a TMSH script."
        ),
    },
    "script::tabc": {
        "snippet": (
            "Provides tab-completion support for TMSH script arguments."
        ),
    },
}

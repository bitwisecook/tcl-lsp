//! A Debug Adapter Protocol (DAP) server over the [`crate::VmBackend`].
//!
//! Speaks the subset of DAP an editor needs to drive the record-and-replay
//! debugger: `initialize` / `launch` / `setBreakpoints` / `configurationDone` /
//! `threads` / `stackTrace` / `scopes` / `variables` / `continue` / `next` /
//! `stepIn` / `stepOut` / `evaluate` / `disconnect`, plus the `initialized` /
//! `stopped` / `terminated` / `output` events. Messages are
//! `Content-Length`-framed JSON over a reader/writer pair (stdin/stdout for a
//! real adapter), so this is fully testable in-process.
//!
//! The backend is record-and-replay: `launch` runs the script once and the
//! adapter then navigates the captured trace, so there is a single thread and
//! one inspectable (top) scope.

use std::io::{BufRead, Write};

use serde_json::{Value, json};

use crate::backend::{DebugBackend, DebugError, VmBackend};
use crate::types::StopReason;

/// Read one `Content-Length`-framed message. Returns `None` at clean EOF.
pub fn read_message(reader: &mut impl BufRead) -> Option<Value> {
    let mut content_length: Option<usize> = None;
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line).ok()? == 0 {
            return None; // EOF
        }
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            break; // end of headers
        }
        if let Some(v) = trimmed.strip_prefix("Content-Length:") {
            content_length = v.trim().parse().ok();
        }
    }
    let len = content_length?;
    let mut buf = vec![0u8; len];
    reader.read_exact(&mut buf).ok()?;
    serde_json::from_slice(&buf).ok()
}

/// Write one `Content-Length`-framed message.
pub fn write_message(writer: &mut impl Write, msg: &Value) -> std::io::Result<()> {
    let body = serde_json::to_vec(msg)?;
    write!(writer, "Content-Length: {}\r\n\r\n", body.len())?;
    writer.write_all(&body)?;
    writer.flush()
}

/// DAP server state: the backend plus the outgoing-message sequence counter.
struct Server {
    backend: VmBackend,
    seq: i64,
    /// Accumulated breakpoint lines (DAP replaces the whole set per file).
    breakpoints: Vec<u32>,
}

impl Server {
    fn new() -> Self {
        Self {
            backend: VmBackend::new(),
            seq: 0,
            breakpoints: Vec::new(),
        }
    }

    fn next_seq(&mut self) -> i64 {
        self.seq += 1;
        self.seq
    }

    /// Build a `response` to `request`.
    #[allow(clippy::needless_pass_by_value)] // `body` is moved into the JSON.
    fn response(&mut self, request: &Value, success: bool, body: Value) -> Value {
        json!({
            "seq": self.next_seq(),
            "type": "response",
            "request_seq": request.get("seq").and_then(Value::as_i64).unwrap_or(0),
            "success": success,
            "command": request.get("command").and_then(Value::as_str).unwrap_or(""),
            "body": body,
        })
    }

    /// Build an `event`.
    #[allow(clippy::needless_pass_by_value)] // `body` is moved into the JSON.
    fn event(&mut self, event: &str, body: Value) -> Value {
        json!({
            "seq": self.next_seq(),
            "type": "event",
            "event": event,
            "body": body,
        })
    }

    /// The `stopped` (or `terminated`) event for the current backend state.
    fn stop_event(&mut self) -> Value {
        if let Some(stop) = self.backend.last_stop() {
            let reason = match stop.reason {
                StopReason::Breakpoint => "breakpoint",
                StopReason::Step => "step",
                StopReason::Entry => "entry",
            };
            self.event(
                "stopped",
                json!({ "reason": reason, "threadId": 1, "allThreadsStopped": true }),
            )
        } else {
            self.event("terminated", json!({}))
        }
    }
}

/// Run the DAP server loop over `reader`/`writer` until `disconnect` or EOF.
///
/// # Errors
/// Propagates write failures.
#[allow(clippy::too_many_lines)] // One request-dispatch match; splitting obscures it.
pub fn run_server(reader: &mut impl BufRead, writer: &mut impl Write) -> std::io::Result<()> {
    let mut srv = Server::new();
    while let Some(req) = read_message(reader) {
        let command = req.get("command").and_then(Value::as_str).unwrap_or("");
        let mut outgoing: Vec<Value> = Vec::new();

        match command {
            "initialize" => {
                let resp = srv.response(
                    &req,
                    true,
                    json!({
                        "supportsConfigurationDoneRequest": true,
                        "supportsEvaluateForHovers": true,
                    }),
                );
                outgoing.push(resp);
                let ev = srv.event("initialized", json!({}));
                outgoing.push(ev);
            }
            "launch" => {
                let program = req
                    .pointer("/arguments/program")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                let source = req.pointer("/arguments/source").and_then(Value::as_str);
                match srv.backend.launch(program, source) {
                    Ok(()) => {
                        let resp = srv.response(&req, true, json!({}));
                        outgoing.push(resp);
                        let ev = srv.stop_event();
                        outgoing.push(ev);
                    }
                    Err(e) => {
                        let resp = srv.response(&req, false, json!({ "error": e.to_string() }));
                        outgoing.push(resp);
                    }
                }
            }
            "setBreakpoints" => {
                let lines: Vec<u32> = req
                    .pointer("/arguments/breakpoints")
                    .and_then(Value::as_array)
                    .map(|bps| {
                        bps.iter()
                            .filter_map(|b| b.get("line").and_then(Value::as_u64))
                            .map(|l| u32::try_from(l).unwrap_or(0))
                            .collect()
                    })
                    .unwrap_or_default();
                srv.breakpoints = srv.backend.set_breakpoints(&lines).unwrap_or_default();
                let verified: Vec<Value> = srv
                    .breakpoints
                    .iter()
                    .map(|l| json!({ "verified": true, "line": l }))
                    .collect();
                let resp = srv.response(&req, true, json!({ "breakpoints": verified }));
                outgoing.push(resp);
            }
            "configurationDone" => {
                let resp = srv.response(&req, true, json!({}));
                outgoing.push(resp);
            }
            "threads" => {
                let resp = srv.response(
                    &req,
                    true,
                    json!({ "threads": [{ "id": 1, "name": "main" }] }),
                );
                outgoing.push(resp);
            }
            "stackTrace" => {
                let frames: Vec<Value> = srv
                    .backend
                    .stack_trace()
                    .unwrap_or_default()
                    .into_iter()
                    .map(|f| json!({ "id": f.id, "name": f.name, "line": f.line, "column": 1 }))
                    .collect();
                let resp = srv.response(
                    &req,
                    true,
                    json!({ "totalFrames": frames.len(), "stackFrames": frames }),
                );
                outgoing.push(resp);
            }
            "scopes" => {
                // One inspectable scope: the top frame's locals.
                let resp = srv.response(
                    &req,
                    true,
                    json!({ "scopes": [{
                        "name": "Locals",
                        "variablesReference": 1,
                        "expensive": false,
                    }] }),
                );
                outgoing.push(resp);
            }
            "variables" => {
                let vars: Vec<Value> = srv
                    .backend
                    .variables(0)
                    .unwrap_or_default()
                    .into_iter()
                    .map(|v| json!({ "name": v.name, "value": v.value, "variablesReference": 0 }))
                    .collect();
                let resp = srv.response(&req, true, json!({ "variables": vars }));
                outgoing.push(resp);
            }
            "continue" | "next" | "stepIn" | "stepOut" => {
                let result = match command {
                    "continue" => srv.backend.continue_execution(),
                    "next" => srv.backend.step_over(),
                    "stepIn" => srv.backend.step_in(),
                    _ => srv.backend.step_out(),
                };
                let body = if command == "continue" {
                    json!({ "allThreadsContinued": true })
                } else {
                    json!({})
                };
                let resp = srv.response(&req, true, body);
                outgoing.push(resp);
                let ev = match result {
                    // The program ran off the end — terminate.
                    Err(DebugError::Finished) => srv.event("terminated", json!({})),
                    // Stopped (step/breakpoint) or a recoverable error — report
                    // the new stop location.
                    Ok(()) | Err(_) => srv.stop_event(),
                };
                outgoing.push(ev);
            }
            "evaluate" => {
                let expr = req
                    .pointer("/arguments/expression")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                match srv.backend.evaluate(expr) {
                    Ok(v) => {
                        let resp = srv.response(
                            &req,
                            true,
                            json!({ "result": v, "variablesReference": 0 }),
                        );
                        outgoing.push(resp);
                    }
                    Err(e) => {
                        let resp = srv.response(&req, false, json!({ "error": e.to_string() }));
                        outgoing.push(resp);
                    }
                }
            }
            "disconnect" | "terminate" => {
                let resp = srv.response(&req, true, json!({}));
                outgoing.push(resp);
                for msg in &outgoing {
                    write_message(writer, msg)?;
                }
                return Ok(());
            }
            _ => {
                // Unknown request — acknowledge so the client isn't wedged.
                let resp = srv.response(&req, true, json!({}));
                outgoing.push(resp);
            }
        }

        for msg in &outgoing {
            write_message(writer, msg)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    /// Frame a sequence of DAP request `Value`s into a `Content-Length` stream.
    fn frame(requests: &[Value]) -> Vec<u8> {
        let mut buf = Vec::new();
        for r in requests {
            write_message(&mut buf, r).unwrap();
        }
        buf
    }

    /// Run the server over framed `requests` and return the parsed responses /
    /// events it emits, in order.
    fn drive(requests: &[Value]) -> Vec<Value> {
        let input = frame(requests);
        let mut reader = Cursor::new(input);
        let mut out: Vec<u8> = Vec::new();
        run_server(&mut reader, &mut out).unwrap();
        let mut cursor = Cursor::new(out);
        let mut msgs = Vec::new();
        while let Some(m) = read_message(&mut cursor) {
            msgs.push(m);
        }
        msgs
    }

    #[allow(clippy::needless_pass_by_value)] // test helper; `arguments` is moved into the json
    fn req(seq: i64, command: &str, arguments: Value) -> Value {
        json!({ "seq": seq, "type": "request", "command": command, "arguments": arguments })
    }

    #[test]
    fn initialize_then_launch_stops_at_entry() {
        let msgs = drive(&[
            req(1, "initialize", json!({})),
            req(2, "launch", json!({ "source": "set x 1\nset y 2\n" })),
            req(3, "disconnect", json!({})),
        ]);
        // initialize response + initialized event + launch response + stopped event…
        assert!(msgs.iter().any(|m| m["event"] == "initialized"));
        let stopped = msgs
            .iter()
            .find(|m| m["event"] == "stopped")
            .expect("stopped event");
        assert_eq!(stopped["body"]["reason"], "entry");
    }

    #[test]
    fn breakpoint_then_continue_stops_there_with_variables() {
        let msgs = drive(&[
            req(1, "initialize", json!({})),
            req(
                2,
                "launch",
                json!({ "source": "set a 1\nset b 2\nset c 3\n" }),
            ),
            req(
                3,
                "setBreakpoints",
                json!({ "breakpoints": [{ "line": 3 }] }),
            ),
            req(4, "continue", json!({})),
            req(5, "stackTrace", json!({})),
            req(6, "variables", json!({})),
            req(7, "disconnect", json!({})),
        ]);

        // setBreakpoints verified line 3.
        let setbp = msgs
            .iter()
            .find(|m| m["command"] == "setBreakpoints")
            .expect("setBreakpoints response");
        assert_eq!(setbp["body"]["breakpoints"][0]["line"], 3);

        // continue → stopped at the breakpoint.
        let stopped = msgs
            .iter()
            .rfind(|m| m["event"] == "stopped")
            .expect("stopped after continue");
        assert_eq!(stopped["body"]["reason"], "breakpoint");

        // stackTrace shows line 3.
        let st = msgs
            .iter()
            .find(|m| m["command"] == "stackTrace")
            .expect("stackTrace response");
        assert_eq!(st["body"]["stackFrames"][0]["line"], 3);

        // variables include a=1 and b=2 (set before line 3 ran).
        let vars = msgs
            .iter()
            .find(|m| m["command"] == "variables")
            .expect("variables response");
        let arr = vars["body"]["variables"].as_array().unwrap();
        assert!(arr.iter().any(|v| v["name"] == "a" && v["value"] == "1"));
        assert!(arr.iter().any(|v| v["name"] == "b" && v["value"] == "2"));
    }

    #[test]
    fn continue_to_end_terminates() {
        let msgs = drive(&[
            req(1, "initialize", json!({})),
            req(2, "launch", json!({ "source": "set x 1\n" })),
            req(3, "continue", json!({})),
            req(4, "disconnect", json!({})),
        ]);
        assert!(msgs.iter().any(|m| m["event"] == "terminated"));
    }
}

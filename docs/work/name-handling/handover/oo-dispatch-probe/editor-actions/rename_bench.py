import sys, time, json
sys.path.insert(0, "./.claude/skills/lsp-client")
import lsp_client as L

W = sys.argv[1]
server = "./target/release/tcl-lsp-server"
c = L.LspClient(W, [server])
c.start()
t = time.time(); L.initialize(c); print(f"initialize {time.time()-t:.2f}s")
t = time.time(); c.wait_for_workspace_scan(timeout=600); print(f"workspace-scan {time.time()-t:.2f}s")

def timed(label, method, params, timeout=300):
    t = time.time()
    try:
        r = c.send_request(method, params, timeout=timeout)
    except Exception as e:
        r = f"ERROR {e}"
    dt = time.time() - t
    if isinstance(r, dict) and "changes" in r:
        summary = {u.rsplit('/',1)[-1]: len(v) for u, v in r["changes"].items()}
    elif isinstance(r, dict) and "documentChanges" in r:
        summary = {d["textDocument"]["uri"].rsplit('/',1)[-1]: len(d["edits"]) for d in r["documentChanges"]}
    elif isinstance(r, list):
        summary = {}
        for x in r:
            k = x.get("uri", "?").rsplit('/',1)[-1]; summary[k] = summary.get(k, 0) + 1
    else:
        summary = json.dumps(r)[:160] if r is not None else None
    print(f"{label}: {dt*1000:.0f} ms -> {summary}")

def pos_of(text, needle, delta=0):
    off = text.index(needle) + delta
    line = text[:off].count("\n"); col = off - (text.rfind("\n", 0, off) + 1)
    return {"line": line, "character": col}

def run(path, cases):
    t = time.time(); uri, text = L.open_document(c, path); c.wait_for_diagnostics(uri, timeout=300)
    print(f"open+diagnostics {path.rsplit('/',1)[-1]}: {time.time()-t:.2f}s")
    for label, needle, delta in cases:
        p = pos_of(text, needle, delta)
        td = {"textDocument": {"uri": uri}, "position": p}
        timed(f"  prepareRename {label}", "textDocument/prepareRename", td)
        timed(f"  rename {label}", "textDocument/rename", {**td, "newName": "zz_new"})
        timed(f"  references {label}", "textDocument/references", {**td, "context": {"includeDeclaration": True}})
        timed(f"  definition {label}", "textDocument/definition", td)
    return uri, text

uri, text = run(f"{W}/main.tcl", [("proc-call ::util::scale", "::util::scale $n 2", 8), ("local-var doubled", "set doubled", 4), ("proc-decl compute", "proc compute", 5)])
uri2, text2 = run(f"{W}/lib/http.tcl", [("proc-decl http::geturl", "proc http::geturl", 11), ("proc-decl http::Finish", "proc http::Finish", 11)])
# edit one line in the middle of http.tcl, then rename immediately
lines = text2.split("\n"); mid = len(lines)//2; lines.insert(mid, "# edited"); new = "\n".join(lines)
c.send_notification("textDocument/didChange", {"textDocument": {"uri": uri2, "version": 2}, "contentChanges": [{"text": new}]})
p = pos_of(new, "proc http::Finish", 11)
timed("  after-edit rename http::Finish", "textDocument/rename", {"textDocument": {"uri": uri2}, "position": p, "newName": "zz_new"})
timed("  after-edit references http::Finish", "textDocument/references", {"textDocument": {"uri": uri2}, "position": p, "context": {"includeDeclaration": True}})
c.shutdown()

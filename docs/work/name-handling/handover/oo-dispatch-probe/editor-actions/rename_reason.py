import sys, time, json
sys.path.insert(0, "./.claude/skills/lsp-client")
import lsp_client as L
W = sys.argv[1]
c = L.LspClient(W, ["./target/release/tcl-lsp-server"]); c.start(); L.initialize(c)
t=time.time(); c.wait_for_workspace_scan(timeout=300); print(f"scan {time.time()-t:.2f}s")
uri, text = L.open_document(c, f"{W}/main.tcl"); c.wait_for_diagnostics(uri, timeout=60)
def pos(n, d):
    o = text.index(n)+d; return {"line": text[:o].count("\n"), "character": o-(text.rfind("\n",0,o)+1)}
for label, n, d in [("call ::util::scale", "::util::scale $n 2", 8), ("local doubled", "set doubled", 4), ("decl compute", "proc compute", 5), ("call compute", "[compute 5", 1)]:
    td = {"textDocument": {"uri": uri}, "position": pos(n, d)}
    for m, extra in [("textDocument/prepareRename", {}), ("textDocument/rename", {"newName": "zz_new"}), ("textDocument/definition", {}), ("textDocument/references", {"context": {"includeDeclaration": True}})]:
        t = time.time()
        try: r = c.send_request(m, {**td, **extra}, timeout=120)
        except Exception as e: r = f"ERROR {e}"
        print(f"{label:20s} {m.split('/')[1]:14s} {1000*(time.time()-t):7.0f} ms  {json.dumps(r)[:150]}")
    for nmsg in c.collect_notifications("window/showMessage", timeout=0.5):
        print("   showMessage:", json.dumps(nmsg)[:200])
c.shutdown()

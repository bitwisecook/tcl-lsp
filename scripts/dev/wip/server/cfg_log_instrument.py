"""Temporary instrumentation for #2543: log every lowered_document_module
execution and every fallback CfgContext's per-field hashes to the file named by
TCL_LSP_DEBUG_CFG. Apply from the repo root on a scratch tree, build, measure,
then `git checkout -- rust/tcl-lsp-db/src/memoised_units.rs`. Never commit."""

from pathlib import Path

p = Path("rust/tcl-lsp-db/src/memoised_units.rs")
s = p.read_text()
dbg = '''
pub(crate) fn debug_cfg_log(line: std::fmt::Arguments<'_>) {
    use std::io::Write;
    if let Ok(path) = std::env::var("TCL_LSP_DEBUG_CFG")
        && let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path)
    {
        let _ = writeln!(f, "{line}");
    }
}
fn debug_hash<T: std::hash::Hash + ?Sized>(value: &T) -> u64 {
    use std::hash::Hasher;
    let mut h = std::collections::hash_map::DefaultHasher::new();
    value.hash(&mut h);
    h.finish()
}
'''
anchor = "#[salsa::tracked(lru = 512, returns(clone))]\npub fn lowered_document_module"
assert anchor in s, "lowered_document_module moved; adapt the anchor"
s = s.replace(anchor, dbg + anchor, 1)
s = s.replace(
    "    let inputs = key.inputs(db).with_source(file.text(db));\n",
    "    let inputs = key.inputs(db).with_source(file.text(db));\n"
    '    debug_cfg_log(format_args!("lower file={:?} key={:?} src={:x}", file.path(db), '
    "salsa::plumbing::AsId::as_id(&key), debug_hash(file.text(db).as_str())));\n",
    1,
)
old = """    CfgContext::new(
        db,
        upvar,
        proc_params,
        global_write,"""
new = """    debug_cfg_log(format_args!(
        "ctx body={:?} upvar={:x} params={:x} gw={:x} owners={:x} upvar_ids={:?}",
        req.body_id,
        debug_hash(&upvar.iter().map(|x| &x.1).collect::<Vec<_>>()),
        debug_hash(&proc_params.iter().map(|x| &x.1).collect::<Vec<_>>()),
        debug_hash(&format!("{:?}", global_write.iter().map(|x| &x.1).collect::<Vec<_>>())),
        debug_hash(&req.procedure_summary_owners),
        upvar.iter().map(|x| x.0).collect::<Vec<_>>(),
    ));
""" + old
assert old in s, "lattice_memo_context moved; adapt the anchor"
s = s.replace(old, new, 1)
p.write_text(s)

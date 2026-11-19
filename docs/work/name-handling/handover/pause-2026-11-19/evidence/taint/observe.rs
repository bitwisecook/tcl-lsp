// Temporary measurement hook used identically in both builds.
use crate::analyses::{ConstValue,LatticeValue};
use crate::cfg::Function;
use crate::sccp::SccpResult;
use crate::ssa::SsaFunction;
use std::fmt::Write as _;
use std::io::Write as _;
use std::sync::{Mutex,OnceLock};

fn emit(kind: &str, cfg:&Function, row:&str) {
    static OUTPUT: OnceLock<Option<Mutex<std::fs::File>>> = OnceLock::new();
    let Some(file) = OUTPUT.get_or_init(|| {
        let dir=std::env::var_os("TCL_STRICT_OBSERVATIONS")?;
        let path=std::path::PathBuf::from(dir).join(format!("{}.tsv",std::process::id()));
        Some(Mutex::new(std::fs::File::create(path).expect("observation log")))
    }) else {return};
    let label=std::env::var("TCL_STRICT_CASE").ok().or_else(||std::env::args().skip(1).find(|s|!s.starts_with('-'))).unwrap_or_else(||"<lib>".to_owned());
    let mut blocks=cfg.blocks.iter().collect::<Vec<_>>(); blocks.sort_unstable_by_key(|(id,_)|id.0);
    let mut shape=String::new();
    for (id,block) in blocks { write!(shape,"{}:",id.0).unwrap(); for st in &block.statements { write!(shape, "{}-{},",st.span().start(),st.span().end()).unwrap(); } shape.push('|'); }
    writeln!(file.lock().unwrap(),"{label:?}\t{kind}\t{:?}\t{shape}\t{row}",cfg.name).unwrap();
}

pub(crate) fn solve(kind: &str, cfg:&Function) {emit(kind,cfg,"");}

fn value(out:&mut String,c:&ConstValue) {
    match c {
        ConstValue::Int(i)=>write!(out,"i{i};").unwrap(),
        ConstValue::Float(f)=>write!(out,"f{:016x};",f.to_bits()).unwrap(),
        ConstValue::Bool(b)=>write!(out,"b{};",u8::from(*b)).unwrap(),
        ConstValue::String(s)=>{write!(out,"s{}:",s.len()).unwrap();for byte in s.as_bytes(){write!(out,"{byte:02x}").unwrap();}out.push(';');}
    }
}
pub(crate) fn strict(cfg:&Function, ssa:&SsaFunction, r:&SccpResult) {
    let mut entries=r.values.iter().collect::<Vec<_>>();entries.sort_unstable_by_key(|((symbol,version),_)|(symbol.0,*version));
    let mut row=String::new();
    for ((symbol,version),v) in entries {
        write!(row,"{}#{} {:?}=",symbol.0,version,ssa.var_name(*symbol)).unwrap();
        match v {
            LatticeValue::Unknown=>row.push('U'), LatticeValue::Overdefined=>row.push('O'),
            LatticeValue::Const(c)=>{row.push('C');value(&mut row,c);},
            LatticeValue::ConstSet(cs)=>{write!(row,"S{}[",cs.len()).unwrap();for c in cs{value(&mut row,c);}row.push(']');}
        }
        row.push('|');
    }
    emit("values",cfg,&row);
}

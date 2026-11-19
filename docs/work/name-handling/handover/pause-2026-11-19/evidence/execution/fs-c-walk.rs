use tcl_cmd_core::filesystem_glob::{GlobOptions,GlobTypes};
use tcl_dialect::TclVersion;
use tcl_host_native::NativeHost;
use tcl_platform::Host;
fn main() {
 let a:Vec<_>=std::env::args().collect();let jim=a[1]=="jim";let v:usize=a[1].parse().unwrap_or(2);let version=TclVersion::ALL[v];let options=GlobOptions{base:None,join:false,tails:false,no_complain:true,types:GlobTypes::default(),patterns_start:0};let host=NativeHost::default();let fs=host.filesystem().unwrap();
 match options.unix_matches(Some(fs),None,&[a[2].as_bytes().to_vec()],version,jim) {
 Ok(mut hits)=>{hits.sort();for h in hits {print!(" {}",h.iter().map(|b|format!("{b:02x}")).collect::<String>());}println!();} Err(e)=>println!("ERR {}",String::from_utf8_lossy(e.message_bytes())),
 }
}

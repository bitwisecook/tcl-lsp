fn main() {
 for input in ["[list simple]","[list {]} yes]","[list \"x]y\" value]",r"[list a\]b]","[list [list x] tail]","[list {]}","[list {𐐀]} tail]"] {
    let source=format!("section main {{\n  optional ( {input} ) {{ string sni }}\n}}\n");println!("case {input}");
    for (start,end) in tcl_bigip::apl::embedded_tcl_regions(&source) {
      println!("capture {}..{} {}",start-1,end+1,&source[(start-1)as usize..(end+1)as usize]);
    }
 }
}

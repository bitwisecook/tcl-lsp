//! EXPERIMENT (throwaway): which Rust structure for the Tcl dict value type?
//!
//! Question: the dict needs by-key get/set (hot, incl. `dict set` build loops)
//! AND **insertion-ordered** iteration (`dict keys`/`dict for`, Tcl 8.5+). Its
//! internal rep is free-to-choose (the ABI is function-mediated:
//! Tcl_DictObjFirst/Next), so pick by evidence. Candidates:
//!
//!   A  Vec<(key,val)>                      linear get/set, ordered, O(n^2) build
//!   B  Vec<(key,val)> + BTreeMap<key,idx>  O(log n) get/set, O(n) ordered iter
//!   C  BTreeMap<key,(seq,val)> + counter   O(log n) ops, O(n log n) iter (sort)
//!   D  HashMap<key,(seq,val)> (FNV, fixed) O(1) ops, O(n log n) iter (sort)
//!
//! Measured: build (N inserts), lookup (N gets), ordered iterate (R passes).
//! Built to wasm32-wasip1 and run under wasmtime — the real target, whose
//! constant factors (libc malloc, no SIMD) differ from the host.
//!
//! Build + run:
//!   rustc -O --edition 2021 --target wasm32-wasip1 dict_rep.rs -o /tmp/dict_rep.wasm
//!   wasmtime /tmp/dict_rep.wasm
//!   # (and natively:  rustc -O --edition 2021 dict_rep.rs -o /tmp/dict_rep && /tmp/dict_rep)

use std::collections::{BTreeMap, HashMap};
use std::hash::{BuildHasherDefault, Hasher};
use std::time::Instant;

type Key = Vec<u8>;
type Val = u64;

// ---- a deterministic FNV-1a hasher (no RandomState nondeterminism) ----
#[derive(Default)]
struct Fnv(u64);
impl Hasher for Fnv {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        let mut h = if self.0 == 0 { 0xcbf29ce484222325 } else { self.0 };
        for &b in bytes {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
        self.0 = h;
    }
}
type FnvMap = HashMap<Key, (u64, Val), BuildHasherDefault<Fnv>>;

// ---- candidate dict structures ----
trait Dict {
    fn new() -> Self;
    fn set(&mut self, k: &[u8], v: Val);
    fn get(&self, k: &[u8]) -> Option<Val>;
    fn iter_sum(&self) -> u64; // visit all values in insertion order, fold
}

struct A {
    v: Vec<(Key, Val)>,
}
impl Dict for A {
    fn new() -> Self {
        A { v: Vec::new() }
    }
    fn set(&mut self, k: &[u8], val: Val) {
        for e in &mut self.v {
            if e.0 == k {
                e.1 = val;
                return;
            }
        }
        self.v.push((k.to_vec(), val));
    }
    fn get(&self, k: &[u8]) -> Option<Val> {
        self.v.iter().find(|e| e.0 == k).map(|e| e.1)
    }
    fn iter_sum(&self) -> u64 {
        self.v.iter().fold(0u64, |a, e| a.wrapping_add(e.1))
    }
}

struct B {
    v: Vec<(Key, Val)>,
    idx: BTreeMap<Key, usize>,
}
impl Dict for B {
    fn new() -> Self {
        B { v: Vec::new(), idx: BTreeMap::new() }
    }
    fn set(&mut self, k: &[u8], val: Val) {
        if let Some(&i) = self.idx.get(k) {
            self.v[i].1 = val;
        } else {
            self.idx.insert(k.to_vec(), self.v.len());
            self.v.push((k.to_vec(), val));
        }
    }
    fn get(&self, k: &[u8]) -> Option<Val> {
        self.idx.get(k).map(|&i| self.v[i].1)
    }
    fn iter_sum(&self) -> u64 {
        self.v.iter().fold(0u64, |a, e| a.wrapping_add(e.1))
    }
}

struct C {
    m: BTreeMap<Key, (u64, Val)>,
    seq: u64,
}
impl Dict for C {
    fn new() -> Self {
        C { m: BTreeMap::new(), seq: 0 }
    }
    fn set(&mut self, k: &[u8], val: Val) {
        if let Some(e) = self.m.get_mut(k) {
            e.1 = val;
        } else {
            self.m.insert(k.to_vec(), (self.seq, val));
            self.seq += 1;
        }
    }
    fn get(&self, k: &[u8]) -> Option<Val> {
        self.m.get(k).map(|e| e.1)
    }
    fn iter_sum(&self) -> u64 {
        let mut entries: Vec<(u64, Val)> = self.m.values().copied().collect();
        entries.sort_by_key(|e| e.0);
        entries.iter().fold(0u64, |a, e| a.wrapping_add(e.1))
    }
}

struct D {
    m: FnvMap,
    seq: u64,
}
impl Dict for D {
    fn new() -> Self {
        D { m: FnvMap::default(), seq: 0 }
    }
    fn set(&mut self, k: &[u8], val: Val) {
        if let Some(e) = self.m.get_mut(k) {
            e.1 = val;
        } else {
            self.m.insert(k.to_vec(), (self.seq, val));
            self.seq += 1;
        }
    }
    fn get(&self, k: &[u8]) -> Option<Val> {
        self.m.get(k).map(|e| e.1)
    }
    fn iter_sum(&self) -> u64 {
        let mut entries: Vec<(u64, Val)> = self.m.values().copied().collect();
        entries.sort_by_key(|e| e.0);
        entries.iter().fold(0u64, |a, e| a.wrapping_add(e.1))
    }
}

// E: ordered Vec (fast insertion-order iterate) + FNV HashMap index (O(1)
// by-key). We never iterate the HashMap, so output order = Vec order = fully
// deterministic regardless of hash internals. The "best of both" candidate.
struct E {
    v: Vec<(Key, Val)>,
    idx: HashMap<Key, usize, BuildHasherDefault<Fnv>>,
}
impl Dict for E {
    fn new() -> Self {
        E { v: Vec::new(), idx: HashMap::default() }
    }
    fn set(&mut self, k: &[u8], val: Val) {
        if let Some(&i) = self.idx.get(k) {
            self.v[i].1 = val;
        } else {
            self.idx.insert(k.to_vec(), self.v.len());
            self.v.push((k.to_vec(), val));
        }
    }
    fn get(&self, k: &[u8]) -> Option<Val> {
        self.idx.get(k).map(|&i| self.v[i].1)
    }
    fn iter_sum(&self) -> u64 {
        self.v.iter().fold(0u64, |a, e| a.wrapping_add(e.1))
    }
}

fn keys(n: usize) -> Vec<Key> {
    (0..n).map(|i| format!("key{i:08}").into_bytes()).collect()
}

fn bench<T: Dict>(name: &str, n: usize, ks: &[Key]) {
    // build
    let t = Instant::now();
    let mut d = T::new();
    for (i, k) in ks.iter().enumerate() {
        d.set(k, i as u64);
    }
    let build = t.elapsed();

    // lookup: N deterministic-"random" gets
    let t = Instant::now();
    let mut lcg: u64 = 0x9e3779b97f4a7c15;
    let mut acc = 0u64;
    for _ in 0..n {
        lcg = lcg.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let i = (lcg >> 33) as usize % n;
        if let Some(v) = d.get(&ks[i]) {
            acc = acc.wrapping_add(v);
        }
    }
    let lookup = t.elapsed();

    // ordered iterate: R passes (R scaled so total work ~ constant)
    let passes = (1_000_000 / n.max(1)).max(1);
    let t = Instant::now();
    let mut isum = 0u64;
    for _ in 0..passes {
        isum = isum.wrapping_add(d.iter_sum());
    }
    let iterate = t.elapsed();

    let per_iter = iterate.as_nanos() as f64 / passes as f64;
    println!(
        "{name:>2}  N={n:>6}  build={:>9.1}us  lookup={:>9.1}us  iter={:>9.1}us/pass  (acc={acc} isum={isum})",
        build.as_nanos() as f64 / 1000.0,
        lookup.as_nanos() as f64 / 1000.0,
        per_iter / 1000.0,
    );
}

fn main() {
    for &n in &[8usize, 64, 512, 4096, 65536] {
        let ks = keys(n);
        bench::<A>("A", n, &ks);
        bench::<B>("B", n, &ks);
        bench::<C>("C", n, &ks);
        bench::<D>("D", n, &ks);
        bench::<E>("E", n, &ks);
        println!();
    }
}

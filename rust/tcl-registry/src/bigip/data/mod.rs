//! Generated BIG-IP object-spec data modules (one per kind-name
//! initial). Aggregated by [`all_specs`].
use super::BigipObjectSpec;

mod a;
mod c;
mod g;
mod i;
mod l;
mod m;
mod n;
mod p;
mod s;
mod u;
mod v;
mod w;

/// All generated BIG-IP object specs, in kind-name order.
#[must_use]
pub fn all_specs() -> Vec<&'static BigipObjectSpec> {
    let mut v: Vec<&'static BigipObjectSpec> = Vec::new();
    v.extend(a::SPECS.iter());
    v.extend(c::SPECS.iter());
    v.extend(g::SPECS.iter());
    v.extend(i::SPECS.iter());
    v.extend(l::SPECS.iter());
    v.extend(m::SPECS.iter());
    v.extend(n::SPECS.iter());
    v.extend(p::SPECS.iter());
    v.extend(s::SPECS.iter());
    v.extend(u::SPECS.iter());
    v.extend(v::SPECS.iter());
    v.extend(w::SPECS.iter());
    v
}

#[path="object-owner-source.txt"] mod object_names;
mod tcl_bigip {pub mod object_names {pub use crate::object_names::*;} pub mod model {#[derive(Debug,PartialEq)] pub struct BigipVirtualServer {pub full_path:String}}}
use tcl_bigip::model::BigipVirtualServer;
struct Rule {source:String}
enum ModelObject {Rule(Rule),VirtualServer(BigipVirtualServer)}
struct Placed {table_name:&'static str, full_path:String, object:ModelObject}
struct Config {objects:Vec<Placed>}
#[derive(Debug, PartialEq)] struct RuleSource {path:String,source:String}
struct RuleIdentity(String);
impl RuleIdentity {fn new(path:&str)->Result<Self,()>{if path.is_empty(){Err(())}else{Ok(Self(path.into()))}}}
impl RuleSource {fn named(identity:RuleIdentity,source:&str)->Self{Self{path:identity.0,source:source.into()}}}
fn short_name(path:&str)->&str {path.rsplit('/').next().unwrap_or(path)}
struct BaseTopology {config:Config}
impl BaseTopology {
pub fn resolve_virtual(&self, name: &str) -> Option<&tcl_bigip::model::BigipVirtualServer> {
        let mut by_short: Option<&tcl_bigip::model::BigipVirtualServer> = None;
        for placed in &self.config.objects {
            if placed.table_name != "virtual_servers" {
                continue;
            }
            let ModelObject::VirtualServer(vs) = &placed.object else {
                continue;
            };
            if placed.full_path == name {
                return Some(vs);
            }
            if short_name(&placed.full_path) == name
                || placed.full_path == format!("/Common/{name}")
            {
                by_short.get_or_insert(vs);
            }
        }
        by_short
    }
pub fn rule_source(&self, rule_ref: &str) -> Option<RuleSource> {
        let rules: Vec<_> = self
            .config
            .objects
            .iter()
            .filter_map(|placed| {
                let ModelObject::Rule(rule) = &placed.object else {
                    return None;
                };
                (placed.table_name == "rules")
                    .then_some((placed.full_path.as_str(), rule.source.as_str()))
            })
            .collect();
        let selected = rules
            .iter()
            .find(|(path, _)| *path == rule_ref)
            .copied()
            .or_else(|| {
                let mut matching = rules
                    .iter()
                    .filter(|(path, _)| short_name(path) == rule_ref);
                let first = matching.next().copied()?;
                matching.next().is_none().then_some(first)
            })?;
        Some(RuleSource::named(
            RuleIdentity::new(selected.0).ok()?,
            selected.1,
        ))
    }
}
struct WorkTopology {config:Config}
impl WorkTopology {
pub fn resolve_virtual(&self, name: &str) -> Option<&tcl_bigip::model::BigipVirtualServer> {
        tcl_bigip::object_names::object_name_candidates(
            name,
            self.config.objects.iter().filter_map(|placed| {
                let ModelObject::VirtualServer(server) = &placed.object else {
                    return None;
                };
                (placed.table_name == "virtual_servers")
                    .then_some((placed.full_path.as_str(), server))
            }),
            tcl_bigip::object_names::ObjectNamePolicy::ExactThenShortNameOrCommon,
        )
        .first_in_order()
    }
pub fn rule_source(&self, rule_ref: &str) -> Option<RuleSource> {
        let rules: Vec<_> = self
            .config
            .objects
            .iter()
            .filter_map(|placed| {
                let ModelObject::Rule(rule) = &placed.object else {
                    return None;
                };
                (placed.table_name == "rules")
                    .then_some((placed.full_path.as_str(), rule.source.as_str()))
            })
            .collect();
        let selected = tcl_bigip::object_names::object_name_candidates(
            rule_ref,
            rules.iter().map(|(path, source)| (*path, (*path, *source))),
            tcl_bigip::object_names::ObjectNamePolicy::ExactThenShortName,
        )
        .first_exact_in_order_or_unique()?;
        Some(RuleSource::named(
            RuleIdentity::new(selected.0).ok()?,
            selected.1,
        ))
    }
}
fn config(rows:&[(&str,&str)])->Config{Config{objects:rows.iter().flat_map(|(path,source)|[
 Placed{table_name:"rules",full_path:(*path).into(),object:ModelObject::Rule(Rule{source:(*source).into()})},
 Placed{table_name:"virtual_servers",full_path:(*path).into(),object:ModelObject::VirtualServer(BigipVirtualServer{full_path:format!("{path}:{source}")})}
]).collect()}}
fn main(){
 let cases:[(&str,&[(&str,&str)]);8]=[
 ("/Tenant/p",&[("/Tenant/p","FIRST"),("/Tenant/p","SECOND")]),
 ("p",&[("/Tenant/p","FIRST"),("/Tenant/p","SECOND")]),
 ("p",&[("/Tenant/p","FIRST"),("/Other/p","SECOND")]),
 ("p",&[("/Tenant/p","ONLY")]),
 ("missing",&[("/Tenant/p","ONLY")]),
 ("/Other/p",&[("/Tenant/p","FIRST"),("/Other/p","EXACT")]),
 ("p",&[("/Tenant/p","FIRST"),("/Common/p","COMMON")]),
 ("/Absent/p",&[("/Tenant/p","OTHER")]),
 ];
 let mut compared=0;
 for (reference,rows) in cases {
  let original=BaseTopology{config:config(rows)};let migrated=WorkTopology{config:config(rows)};
  assert_eq!(original.rule_source(reference),migrated.rule_source(reference),"rule {reference}");compared+=1;
  assert_eq!(original.resolve_virtual(reference),migrated.resolve_virtual(reference),"virtual {reference}");compared+=1;
 }
 let exact=WorkTopology{config:config(&[("/Tenant/p","FIRST"),("/Tenant/p","SECOND")])};
 assert_eq!(exact.rule_source("/Tenant/p"),Some(RuleSource{path:"/Tenant/p".into(),source:"FIRST".into()}));
 assert_eq!(exact.rule_source("p"),None);
 println!("{compared} extracted actual topology-helper comparisons passed, including duplicate exact rows and ambiguous short names; fixture model adapters only, no native identity grant");
}

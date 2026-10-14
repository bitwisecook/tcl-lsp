#[path="object-owner-source.txt"] mod object_names;
use object_names::{ObjectNamePolicy,object_name_candidates};
use std::collections::BTreeMap;
#[derive(Clone)] struct Placed {full_path:String}
struct BigipConfig {default_partition:String,objects:Vec<Placed>}
mod lint {
pub(crate) struct KindMap<'a, T> {
    /// `(full_path, value)` in first-seen order.
    entries: Vec<(&'a str, &'a T)>,
    /// `full_path -> index into entries`.
    index: std::collections::HashMap<&'a str, usize>,
}

impl<T> Default for KindMap<'_, T> {
    fn default() -> Self {
        Self {
            entries: Vec::new(),
            index: std::collections::HashMap::new(),
        }
    }
}

impl<'a, T> KindMap<'a, T> {
    /// Insert / update with map-assignment semantics: a new key appends
    /// (keeping first-seen position), an existing key updates the value in
    /// place.
    fn insert(&mut self, key: &'a str, value: &'a T) {
        if let Some(&i) = self.index.get(key) {
            self.entries[i].1 = value;
        } else {
            self.index.insert(key, self.entries.len());
            self.entries.push((key, value));
        }
    }

    fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Look up the value for `key`.
    pub(crate) fn get(&self, key: &str) -> Option<&'a T> {
        self.index.get(key).map(|&i| self.entries[i].1)
    }

    pub(crate) fn iter(&self) -> impl Iterator<Item = (&'a str, &'a T)> + '_ {
        self.entries.iter().copied()
    }
}

pub fn resolve_name<T>(
    name: &str,
    table: &KindMap<'_, T>,
    default_partition: &str,
) -> Option<String> {
    crate::object_names::object_name_candidates(
        name,
        table.iter().map(|(key, _)| (key, key)),
        crate::object_names::ObjectNamePolicy::Configuration { default_partition },
    )
    .first_in_order()
    .map(ToOwned::to_owned)
}
 pub fn run(name:&str,paths:&[&str],partition:&str)->Option<String>{let mut map=KindMap::default();for path in paths {map.insert(path,path);}resolve_name(name,&map,partition)}
}
mod graph { use super::Placed;
pub fn resolve_name<'a>(
    name: &str,
    table: &[&'a Placed],
    default_partition: &str,
) -> Option<&'a Placed> {
    crate::object_names::object_name_candidates(
        name,
        table
            .iter()
            .map(|placed| (placed.full_path.as_str(), *placed)),
        crate::object_names::ObjectNamePolicy::Configuration { default_partition },
    )
    .first_in_order()
}
}
mod convert { use super::BigipConfig;
fn find<'a>(cfg:&'a BigipConfig,_table:&str,path:&str)->Option<&'a super::Placed>{cfg.objects.iter().find(|p|p.full_path==path)}
fn find_object<'a>(cfg:&'a BigipConfig,table:&str,path:&str)->Option<&'a super::Placed>{find(cfg,table,path)}
fn table_paths<'a>(cfg:&'a BigipConfig,_table:&str)->impl Iterator<Item=&'a str>{cfg.objects.iter().map(|p|p.full_path.as_str())}
fn typed_objects<'a>(cfg:&'a BigipConfig,table:&str)->impl Iterator<Item=(&'a str,())>{table_paths(cfg,table).map(|p|(p,()))}
pub fn resolve_name(cfg: &BigipConfig, name: &str, table: &str) -> Option<String> {
    crate::object_names::object_name_candidates(
        name,
        table_paths(cfg, table).map(|path| (path, path)),
        crate::object_names::ObjectNamePolicy::Configuration {
            default_partition: &cfg.default_partition,
        },
    )
    .first_in_order()
    .map(ToOwned::to_owned)
}
}
mod pcap_enrich { use super::BigipConfig;
fn find<'a>(cfg:&'a BigipConfig,_table:&str,path:&str)->Option<&'a super::Placed>{cfg.objects.iter().find(|p|p.full_path==path)}
fn find_object<'a>(cfg:&'a BigipConfig,table:&str,path:&str)->Option<&'a super::Placed>{find(cfg,table,path)}
fn table_paths<'a>(cfg:&'a BigipConfig,_table:&str)->impl Iterator<Item=&'a str>{cfg.objects.iter().map(|p|p.full_path.as_str())}
fn typed_objects<'a>(cfg:&'a BigipConfig,table:&str)->impl Iterator<Item=(&'a str,())>{table_paths(cfg,table).map(|p|(p,()))}
pub fn resolve_name(cfg: &BigipConfig, name: &str, table: &'static str) -> Option<String> {
    crate::object_names::object_name_candidates(
        name,
        typed_objects(cfg, table).map(|(path, _)| (path, path)),
        crate::object_names::ObjectNamePolicy::Configuration {
            default_partition: &cfg.default_partition,
        },
    )
    .first_in_order()
    .map(ToOwned::to_owned)
}
}
mod wireshark_profile { use super::BigipConfig;
fn find<'a>(cfg:&'a BigipConfig,_table:&str,path:&str)->Option<&'a super::Placed>{cfg.objects.iter().find(|p|p.full_path==path)}
fn find_object<'a>(cfg:&'a BigipConfig,table:&str,path:&str)->Option<&'a super::Placed>{find(cfg,table,path)}
fn table_paths<'a>(cfg:&'a BigipConfig,_table:&str)->impl Iterator<Item=&'a str>{cfg.objects.iter().map(|p|p.full_path.as_str())}
fn typed_objects<'a>(cfg:&'a BigipConfig,table:&str)->impl Iterator<Item=(&'a str,())>{table_paths(cfg,table).map(|p|(p,()))}
pub fn resolve_name(cfg: &BigipConfig, name: &str, table: &'static str) -> Option<String> {
    crate::object_names::object_name_candidates(
        name,
        typed_objects(cfg, table).map(|(path, _)| (path, path)),
        crate::object_names::ObjectNamePolicy::Configuration {
            default_partition: &cfg.default_partition,
        },
    )
    .first_in_order()
    .map(ToOwned::to_owned)
}
}
mod irule_context {
pub fn resolve_in<T>(name: &str, entries: &[(&str, &T)], default_partition: &str) -> Option<usize> {
    crate::object_names::object_name_candidates(
        name,
        entries
            .iter()
            .enumerate()
            .map(|(index, entry)| (entry.0, index)),
        crate::object_names::ObjectNamePolicy::Configuration { default_partition },
    )
    .first_in_order()
}
}
mod tls {use std::collections::BTreeMap;
pub fn resolve_reference<T>(reference: &str, objects: &BTreeMap<String, T>) -> Option<String> {
    crate::object_names::object_name_candidates(
        reference,
        objects.keys().map(|path| (path.as_str(), path.as_str())),
        crate::object_names::ObjectNamePolicy::ExactThenReferenceLeaf,
    )
    .unique()
    .map(ToOwned::to_owned)
}
}
mod gtm {use std::collections::BTreeMap;type GtmServer=();
pub fn resolve_gtm_path<'a, T>(name: &str, pool: &'a [(String, T)]) -> Option<&'a str> {
    crate::object_names::object_name_candidates(
        name,
        pool.iter().map(|(path, _)| (path.as_str(), path.as_str())),
        crate::object_names::ObjectNamePolicy::CommonThenSuffix,
    )
    .first_in_order()
}
pub fn resolve_gtm_server<'a>(name: &str, servers: &'a BTreeMap<String, GtmServer>) -> Option<&'a str> {
    crate::object_names::object_name_candidates(
        name,
        servers.keys().map(|path| (path.as_str(), path.as_str())),
        crate::object_names::ObjectNamePolicy::CommonThenSuffix,
    )
    .first_in_order()
}
}
fn main(){
 let paths=["/Z/p","/Common/p","/Tenant/p","p","/Tenant/a::b","/Tenant/a//b","/Tenant/a/../b","/Other/p","/Nested/Tenant/p","/Common//Tenant/p"];
 let references=["p","/Tenant/p","missing","a::b","a//b","a/../b","Tenant/p","/Absent/p", "", "p:"];
 let partitions=["Tenant", "/Tenant/", "Common", "", "///"];
 let mut controls=0;
 for name in references {
  for partition in partitions {
   let cfg=BigipConfig{default_partition:partition.into(),objects:paths.iter().map(|p|Placed{full_path:(*p).into()}).collect()};
   let matched=object_name_candidates(name,paths.iter().map(|p|(*p,(*p).to_owned())),ObjectNamePolicy::Configuration{default_partition:partition}).first_in_order();
   assert_eq!(lint::run(name,&paths,partition),matched,"lint {name} {partition}");controls+=1;
   let rows:Vec<_>=cfg.objects.iter().collect();
   assert_eq!(graph::resolve_name(name,&rows,partition).map(|p|p.full_path.clone()),matched);controls+=1;
   assert_eq!(convert::resolve_name(&cfg,name,"x"),matched);controls+=1;
   assert_eq!(pcap_enrich::resolve_name(&cfg,name,"x"),matched);controls+=1;
   assert_eq!(wireshark_profile::resolve_name(&cfg,name,"x"),matched);controls+=1;
   let entries:Vec<_>=paths.iter().enumerate().map(|(i,p)|(*p,i)).collect();
   let borrowed:Vec<_>=entries.iter().map(|(p,i)|(*p,i)).collect();
   assert_eq!(irule_context::resolve_in(name,&borrowed,partition),object_name_candidates(name,borrowed.iter().enumerate().map(|(i,e)|(e.0,i)),ObjectNamePolicy::Configuration{default_partition:partition}).first_in_order());controls+=1;
  }
  let map:BTreeMap<String,()>=paths.iter().map(|p|((*p).into(),())).collect();
  assert_eq!(tls::resolve_reference(name,&map),object_name_candidates(name,map.keys().map(|p|(p.as_str(),p.clone())),ObjectNamePolicy::ExactThenReferenceLeaf).unique());controls+=1;
  let pool:Vec<_>=paths.iter().map(|p|((*p).to_owned(),())).collect();
  assert_eq!(gtm::resolve_gtm_path(name,&pool),object_name_candidates(name,pool.iter().map(|(p,_)|(p.as_str(),p.as_str())),ObjectNamePolicy::CommonThenSuffix).first_in_order());controls+=1;
  assert_eq!(gtm::resolve_gtm_server(name,&map),object_name_candidates(name,map.keys().map(|p|(p.as_str(),p.as_str())),ObjectNamePolicy::CommonThenSuffix).first_in_order());controls+=1;
 }
 println!("{controls} original-source software-policy comparisons passed; no appliance oracle claim");
}

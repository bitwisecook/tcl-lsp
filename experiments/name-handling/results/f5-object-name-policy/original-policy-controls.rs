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

    pub(crate) fn contains_key(&self, key: &str) -> bool {
        self.index.contains_key(key)
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
    if table.contains_key(name) {
        return Some(name.to_owned());
    }
    if !name.starts_with('/') {
        let partition = {
            let p = default_partition.trim_matches('/');
            if p.is_empty() { "Common" } else { p }
        };
        let candidate = format!("/{partition}/{name}");
        if table.contains_key(&candidate) {
            return Some(candidate);
        }
        if partition != "Common" {
            let candidate = format!("/Common/{name}");
            if table.contains_key(&candidate) {
                return Some(candidate);
            }
        }
    }
    let suffix = format!("/{name}");
    table
        .iter()
        .find(|(k, _)| k.ends_with(&suffix))
        .map(|(k, _)| k.to_owned())
}
 pub fn run(name:&str,paths:&[&str],partition:&str)->Option<String>{let mut map=KindMap::default();for path in paths {map.insert(path,path);}resolve_name(name,&map,partition)}
}
mod graph { use super::Placed;
pub fn resolve_name<'a>(
    name: &str,
    table: &[&'a Placed],
    default_partition: &str,
) -> Option<&'a Placed> {
    if let Some(p) = table.iter().copied().find(|p| p.full_path == name) {
        return Some(p);
    }
    if !name.starts_with('/') {
        let trimmed = default_partition.trim_matches('/');
        let partition = if trimmed.is_empty() {
            "Common"
        } else {
            trimmed
        };
        let candidate = format!("/{partition}/{name}");
        if let Some(p) = table.iter().copied().find(|p| p.full_path == candidate) {
            return Some(p);
        }
        if partition != "Common" {
            let candidate = format!("/Common/{name}");
            if let Some(p) = table.iter().copied().find(|p| p.full_path == candidate) {
                return Some(p);
            }
        }
    }
    let suffix = format!("/{name}");
    table
        .iter()
        .copied()
        .find(|p| p.full_path.ends_with(&suffix))
}
}
mod convert { use super::BigipConfig;
fn find<'a>(cfg:&'a BigipConfig,_table:&str,path:&str)->Option<&'a super::Placed>{cfg.objects.iter().find(|p|p.full_path==path)}
fn find_object<'a>(cfg:&'a BigipConfig,table:&str,path:&str)->Option<&'a super::Placed>{find(cfg,table,path)}
fn table_paths<'a>(cfg:&'a BigipConfig,_table:&str)->impl Iterator<Item=&'a str>{cfg.objects.iter().map(|p|p.full_path.as_str())}
fn typed_objects<'a>(cfg:&'a BigipConfig,table:&str)->impl Iterator<Item=(&'a str,())>{table_paths(cfg,table).map(|p|(p,()))}
pub fn resolve_name(cfg: &BigipConfig, name: &str, table: &str) -> Option<String> {
    if find(cfg, table, name).is_some() {
        return Some(name.to_owned());
    }
    if !name.starts_with('/') {
        let partition = {
            let p = if cfg.default_partition.is_empty() {
                "Common"
            } else {
                cfg.default_partition.as_str()
            };
            p.trim_matches('/').to_owned()
        };
        if !partition.is_empty() {
            let candidate = format!("/{partition}/{name}");
            if find(cfg, table, &candidate).is_some() {
                return Some(candidate);
            }
        }
        if partition != "Common" {
            let candidate = format!("/Common/{name}");
            if find(cfg, table, &candidate).is_some() {
                return Some(candidate);
            }
        }
    }
    let suffix = format!("/{name}");
    table_paths(cfg, table)
        .find(|key| key.ends_with(&suffix))
        .map(ToOwned::to_owned)
}
}
mod pcap_enrich { use super::BigipConfig;
fn find<'a>(cfg:&'a BigipConfig,_table:&str,path:&str)->Option<&'a super::Placed>{cfg.objects.iter().find(|p|p.full_path==path)}
fn find_object<'a>(cfg:&'a BigipConfig,table:&str,path:&str)->Option<&'a super::Placed>{find(cfg,table,path)}
fn table_paths<'a>(cfg:&'a BigipConfig,_table:&str)->impl Iterator<Item=&'a str>{cfg.objects.iter().map(|p|p.full_path.as_str())}
fn typed_objects<'a>(cfg:&'a BigipConfig,table:&str)->impl Iterator<Item=(&'a str,())>{table_paths(cfg,table).map(|p|(p,()))}
pub fn resolve_name(cfg: &BigipConfig, name: &str, table: &'static str) -> Option<String> {
    if find_object(cfg, table, name).is_some() {
        return Some(name.to_owned());
    }
    if !name.starts_with('/') {
        let partition = {
            let p = if cfg.default_partition.is_empty() {
                "Common"
            } else {
                cfg.default_partition.as_str()
            };
            p.trim_matches('/').to_owned()
        };
        if !partition.is_empty() {
            let candidate = format!("/{partition}/{name}");
            if find_object(cfg, table, &candidate).is_some() {
                return Some(candidate);
            }
        }
        if partition != "Common" {
            let candidate = format!("/Common/{name}");
            if find_object(cfg, table, &candidate).is_some() {
                return Some(candidate);
            }
        }
    }
    let suffix = format!("/{name}");
    typed_objects(cfg, table)
        .map(|(p, _)| p)
        .find(|key| key.ends_with(&suffix))
        .map(ToOwned::to_owned)
}
}
mod wireshark_profile { use super::BigipConfig;
fn find<'a>(cfg:&'a BigipConfig,_table:&str,path:&str)->Option<&'a super::Placed>{cfg.objects.iter().find(|p|p.full_path==path)}
fn find_object<'a>(cfg:&'a BigipConfig,table:&str,path:&str)->Option<&'a super::Placed>{find(cfg,table,path)}
fn table_paths<'a>(cfg:&'a BigipConfig,_table:&str)->impl Iterator<Item=&'a str>{cfg.objects.iter().map(|p|p.full_path.as_str())}
fn typed_objects<'a>(cfg:&'a BigipConfig,table:&str)->impl Iterator<Item=(&'a str,())>{table_paths(cfg,table).map(|p|(p,()))}
pub fn resolve_name(cfg: &BigipConfig, name: &str, table: &'static str) -> Option<String> {
    if find_object(cfg, table, name).is_some() {
        return Some(name.to_owned());
    }
    if !name.starts_with('/') {
        let partition = {
            let p = if cfg.default_partition.is_empty() {
                "Common"
            } else {
                cfg.default_partition.as_str()
            };
            p.trim_matches('/').to_owned()
        };
        if !partition.is_empty() {
            let candidate = format!("/{partition}/{name}");
            if find_object(cfg, table, &candidate).is_some() {
                return Some(candidate);
            }
        }
        if partition != "Common" {
            let candidate = format!("/Common/{name}");
            if find_object(cfg, table, &candidate).is_some() {
                return Some(candidate);
            }
        }
    }
    let suffix = format!("/{name}");
    typed_objects(cfg, table)
        .map(|(p, _)| p)
        .find(|key| key.ends_with(&suffix))
        .map(ToOwned::to_owned)
}
}
mod irule_context {
pub fn resolve_in<T>(name: &str, entries: &[(&str, &T)], default_partition: &str) -> Option<usize> {
    if let Some(i) = entries.iter().position(|e| e.0 == name) {
        return Some(i);
    }
    if !name.starts_with('/') {
        // `(default_partition or "Common").strip("/")`
        let base = if default_partition.is_empty() {
            "Common"
        } else {
            default_partition
        };
        let partition = base.trim_matches('/');
        if !partition.is_empty() {
            let candidate = format!("/{partition}/{name}");
            if let Some(i) = entries.iter().position(|e| e.0 == candidate) {
                return Some(i);
            }
        }
        if partition != "Common" {
            let candidate = format!("/Common/{name}");
            if let Some(i) = entries.iter().position(|e| e.0 == candidate) {
                return Some(i);
            }
        }
    }
    let suffix = format!("/{name}");
    entries.iter().position(|e| e.0.ends_with(&suffix))
}
}
mod tls {use std::collections::BTreeMap;
pub fn resolve_reference<T>(reference: &str, objects: &BTreeMap<String, T>) -> Option<String> {
    if objects.contains_key(reference) {
        return Some(reference.to_owned());
    }
    let leaf = reference.rsplit('/').next().unwrap_or(reference);
    let matches: Vec<&String> = objects
        .keys()
        .filter(|path| path.rsplit('/').next().unwrap_or(path) == leaf)
        .collect();
    (matches.len() == 1).then(|| matches[0].clone())
}
}
mod gtm {use std::collections::BTreeMap;type GtmServer=();
pub fn resolve_gtm_path<'a, T>(name: &str, pool: &'a [(String, T)]) -> Option<&'a str> {
    if pool.iter().any(|(k, _)| k == name) {
        return Some(pool.iter().find(|(k, _)| k == name).unwrap().0.as_str());
    }
    let candidate = format!("/Common/{name}");
    if let Some((k, _)) = pool.iter().find(|(k, _)| *k == candidate) {
        return Some(k.as_str());
    }
    let suffix = format!("/{name}");
    pool.iter()
        .find(|(k, _)| k.ends_with(&suffix))
        .map(|(k, _)| k.as_str())
}
pub fn resolve_gtm_server<'a>(name: &str, servers: &'a BTreeMap<String, GtmServer>) -> Option<&'a str> {
    if servers.contains_key(name) {
        return servers
            .keys()
            .find(|k| k.as_str() == name)
            .map(String::as_str);
    }
    let candidate = format!("/Common/{name}");
    if let Some(k) = servers.keys().find(|k| **k == candidate) {
        return Some(k.as_str());
    }
    let suffix = format!("/{name}");
    servers
        .keys()
        .find(|k| k.ends_with(&suffix))
        .map(String::as_str)
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

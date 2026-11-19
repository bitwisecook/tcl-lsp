use crate::Backend;

impl Backend {
    pub(in super::super) async fn debug_memory_report(&self) -> serde_json::Value {
        let documents = self.documents.lock("debug_memory").await.len();
        let db_files = self.db_files.lock().await.len();
        let mut db = self.db.lock().await;
        let usage = <dyn salsa::Database>::memory_usage(&mut *db);
        let mut structs = std::collections::BTreeMap::<String, (usize, usize)>::new();
        for s in &usage.structs {
            let e = structs.entry(s.debug_name().to_string()).or_default();
            e.0 += s.count();
            e.1 += s.size_of_fields() + s.heap_size_of_fields().unwrap_or(0);
        }
        let mut queries = std::collections::BTreeMap::<String, (usize, usize)>::new();
        for q in usage.queries.values() {
            let e = queries.entry(q.debug_name().to_string()).or_default();
            e.0 += q.count();
            e.1 += q.size_of_fields() + q.heap_size_of_fields().unwrap_or(0);
        }
        drop(db);
        serde_json::json!({
            "documents": documents,
            "db_files": db_files,
            "structs": structs,
            "queries": queries,
        })
    }
}

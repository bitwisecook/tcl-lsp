//! Typed BIG-IP scalar values — IP addresses, networks, ports,
//! partitions, destinations, folders, attachments, and the typed list.
//! Rust port of `dialects/f5/bigip/types/`.
//!
//! Each type round-trips to its canonical F5 spelling via `Display` and
//! parses via `parse` / `try_parse`, mirroring the Python value layer so
//! reconstructed objects compare equal.

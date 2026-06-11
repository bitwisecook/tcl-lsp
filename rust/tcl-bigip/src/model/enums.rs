//! Model enums shared across BIG-IP kinds. Mirrors
//! `dialects/f5/bigip/model/_enums.py`.

/// Whether a data-group is stored inline or in an external file.
/// Mirrors Python `DataGroupType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DataGroupType {
    /// Inline (`internal`) data-group.
    #[default]
    Internal,
    /// External-file (`external`) data-group.
    External,
}

/// Broad classification of BIG-IP profile types. Mirrors Python
/// `ProfileType`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProfileType {
    /// `http`.
    Http,
    /// `tcp`.
    Tcp,
    /// `udp`.
    Udp,
    /// `client-ssl`.
    ClientSsl,
    /// `server-ssl`.
    ServerSsl,
    /// `ftp`.
    Ftp,
    /// `dns`.
    Dns,
    /// `sip`.
    Sip,
    /// `diameter`.
    Diameter,
    /// `fix`.
    Fix,
    /// `radius`.
    Radius,
    /// `mqtt`.
    Mqtt,
    /// `websocket`.
    Websocket,
    /// `stream`.
    Stream,
    /// `html`.
    Html,
    /// `rewrite`.
    Rewrite,
    /// `fasthttp`.
    Fasthttp,
    /// `fastl4`.
    Fastl4,
    /// `one-connect`.
    OneConnect,
    /// `persistence`.
    Persistence,
    /// Unclassified / other.
    #[default]
    Other,
}

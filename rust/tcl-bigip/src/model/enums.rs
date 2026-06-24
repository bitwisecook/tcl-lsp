//! Model enums shared across BIG-IP kinds.

/// Whether a data-group is stored inline or in an external file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DataGroupType {
    /// Inline (`internal`) data-group.
    #[default]
    Internal,
    /// External-file (`external`) data-group.
    External,
}

impl DataGroupType {
    /// The canonical `DataGroupType` member name (`"INTERNAL"` / `"EXTERNAL"`).
    #[must_use]
    pub const fn py_name(self) -> &'static str {
        match self {
            Self::Internal => "INTERNAL",
            Self::External => "EXTERNAL",
        }
    }
}

/// Broad classification of BIG-IP profile types.
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

impl ProfileType {
    /// The canonical `ProfileType` member name (`"HTTP"`, `"CLIENT_SSL"`, …).
    #[must_use]
    pub const fn py_name(self) -> &'static str {
        match self {
            Self::Http => "HTTP",
            Self::Tcp => "TCP",
            Self::Udp => "UDP",
            Self::ClientSsl => "CLIENT_SSL",
            Self::ServerSsl => "SERVER_SSL",
            Self::Ftp => "FTP",
            Self::Dns => "DNS",
            Self::Sip => "SIP",
            Self::Diameter => "DIAMETER",
            Self::Fix => "FIX",
            Self::Radius => "RADIUS",
            Self::Mqtt => "MQTT",
            Self::Websocket => "WEBSOCKET",
            Self::Stream => "STREAM",
            Self::Html => "HTML",
            Self::Rewrite => "REWRITE",
            Self::Fasthttp => "FASTHTTP",
            Self::Fastl4 => "FASTL4",
            Self::OneConnect => "ONE_CONNECT",
            Self::Persistence => "PERSISTENCE",
            Self::Other => "OTHER",
        }
    }
}

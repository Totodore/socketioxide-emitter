//! Custom request and response types for the Redis adapter
//! with custom serialization/deserialization to reduce the size of the messages.
//!
//! This code is part of the socketioxide-redis crate.

use serde::Serialize;
use socketioxide_core::{
    Sid, Uid,
    adapter::{BroadcastOptions, Room},
    packet::Packet,
};

#[derive(Debug, PartialEq)]
pub enum RequestType {
    /// Broadcast a packet to matching sockets.
    #[allow(unused)]
    Broadcast(Packet),
    /// Disconnect matching sockets.
    DisconnectSockets,
    /// Add matching sockets to the rooms.
    AddSockets(Vec<Room>),
    /// Remove matching sockets from the rooms.
    DelSockets(Vec<Room>),
}
impl RequestType {
    fn to_u8(&self) -> u8 {
        match self {
            Self::Broadcast(_) => 0,
            Self::DisconnectSockets => 2,
            Self::AddSockets(_) => 4,
            Self::DelSockets(_) => 5,
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Request {
    pub node_id: Uid,
    pub id: Sid,
    pub r#type: RequestType,
    pub opts: BroadcastOptions,
}
impl Request {
    pub fn new(r#type: RequestType, opts: BroadcastOptions) -> Self {
        Self {
            node_id: Uid::new(),
            id: Sid::new(),
            r#type,
            opts,
        }
    }
}

/// Custom implementation to serialize enum variant as u8.
impl Serialize for Request {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        #[derive(Debug, Serialize)]
        struct RawRequest<'a> {
            node_id: Uid,
            id: Sid,
            r#type: u8,
            packet: Option<&'a Packet>,
            rooms: Option<&'a [Room]>,
            opts: &'a BroadcastOptions,
        }
        let raw = RawRequest {
            node_id: self.node_id,
            id: self.id,
            r#type: self.r#type.to_u8(),
            packet: match &self.r#type {
                RequestType::Broadcast(p) => Some(p),
                _ => None,
            },
            rooms: match &self.r#type {
                RequestType::AddSockets(r) | RequestType::DelSockets(r) => Some(r),
                _ => None,
            },
            opts: &self.opts,
        };
        raw.serialize(serializer)
    }
}

#[cfg(test)]
mod tests {
    use socketioxide_core::Value;

    use super::*;

    #[test]
    fn request_broadcast_serde() {
        let packet = Packet::event("foo", Value::Str("bar".into(), None));
        let opts = BroadcastOptions::new(Sid::new());
        let req = Request::new(RequestType::Broadcast(packet), opts);
        let serialized = rmp_serde::to_vec(&req).unwrap();
        assert_eq!(serialized, serialized);
    }

    #[test]
    fn request_add_sockets_serde() {
        let opts = BroadcastOptions::new(Sid::new());
        let rooms = vec!["foo".into(), "bar".into()];
        let req = Request::new(RequestType::AddSockets(rooms), opts);
        let serialized = rmp_serde::to_vec(&req).unwrap();
        assert_eq!(serialized, serialized);
    }

    #[test]
    fn request_del_sockets_serde() {
        let opts = BroadcastOptions::new(Sid::new());
        let rooms = vec!["foo".into(), "bar".into()];
        let req = Request::new(RequestType::DelSockets(rooms), opts);
        let serialized = rmp_serde::to_vec(&req).unwrap();
        assert_eq!(serialized, serialized);
    }

    #[test]
    fn request_disconnect_sockets_serde() {
        let opts = BroadcastOptions::new(Sid::new());
        let req = Request::new(RequestType::DisconnectSockets, opts);
        let serialized = rmp_serde::to_vec(&req).unwrap();
        assert_eq!(serialized, serialized);
    }
}

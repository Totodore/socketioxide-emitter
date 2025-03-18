//! SocketIO Redis Emitter
//!
//! You can use this crate to emit events to a cluster of Socket.IO servers through Redis.
use std::fmt;

use requests::{Request, RequestType};
use socketioxide_core::{
    Str,
    adapter::{BroadcastFlags, BroadcastOptions, RoomParam},
};

mod requests;

pub trait Driver {
    type Error: std::error::Error;
    fn emit(&self, channel: String, data: Vec<u8>)
    -> impl Future<Output = Result<(), Self::Error>>;
}

pub enum EmitError<D: Driver> {
    Driver(D::Error),
    #[cfg(any(feature = "common-parser", feature = "msgpack-parser"))]
    Parser(socketioxide_core::parser::ParserError),
}
impl<D: Driver> fmt::Debug for EmitError<D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EmitError::Driver(err) => write!(f, "Driver error: {}", err),
            #[cfg(any(feature = "common-parser", feature = "msgpack-parser"))]
            EmitError::Parser(err) => write!(f, "Serialization error: {}", err),
        }
    }
}
impl<D: Driver> fmt::Display for EmitError<D> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self, f)
    }
}
impl<D: Driver> std::error::Error for EmitError<D> {}

#[cfg(any(feature = "common-parser", feature = "msgpack-parser"))]
#[derive(Debug, Clone, Copy)]
enum Parser {
    #[cfg(feature = "common-parser")]
    Common,
    #[cfg(feature = "msgpack-parser")]
    MsgPack,
}
#[cfg(any(feature = "common-parser", feature = "msgpack-parser"))]
impl Default for Parser {
    fn default() -> Self {
        #[cfg(all(feature = "msgpack-parser", not(feature = "common-parser")))]
        {
            Parser::MsgPack
        }
        #[cfg(feature = "common-parser")]
        {
            Parser::Common
        }
    }
}

#[derive(Clone, Debug)]
pub struct IoEmitter {
    opts: BroadcastOptions,
    ns: Option<Str>,
    prefix: Option<String>,
    #[cfg(any(feature = "common-parser", feature = "msgpack-parser"))]
    parser: Parser,
}

impl Default for IoEmitter {
    fn default() -> Self {
        let mut io = Self {
            opts: Default::default(),
            ns: None,
            prefix: None,
            #[cfg(any(feature = "common-parser", feature = "msgpack-parser"))]
            parser: Parser::default(),
        };
        io.opts.add_flag(BroadcastFlags::Broadcast);
        io
    }
}

impl IoEmitter {
    pub fn new() -> Self {
        Self::default()
    }
    #[cfg(feature = "msgpack-parser")]
    pub fn new_msgpack() -> Self {
        Self {
            parser: Parser::MsgPack,
            ..Default::default()
        }
    }
    pub fn of(mut self, ns: impl Into<String>) -> IoEmitter {
        self.ns = Some(Str::from(ns.into()));
        self
    }
    pub fn to(mut self, rooms: impl RoomParam) -> IoEmitter {
        self.opts.rooms.extend(rooms.into_room_iter());
        self
    }
    pub fn within(self, rooms: impl RoomParam) -> IoEmitter {
        self.to(rooms)
    }
    pub fn except(mut self, rooms: impl RoomParam) -> IoEmitter {
        self.opts.except.extend(rooms.into_room_iter());
        self
    }
    pub fn prefix(mut self, prefix: impl Into<String>) -> IoEmitter {
        self.prefix = Some(prefix.into());
        self
    }
}

impl IoEmitter {
    pub async fn join<D: Driver>(self, rooms: impl RoomParam, driver: &D) -> Result<(), D::Error> {
        let rooms = rooms.into_room_iter().collect();
        let chan = self.get_channel();
        let data = self.serialize(RequestType::AddSockets(rooms));
        driver.emit(chan, data).await
    }
    pub async fn leave<D: Driver>(self, rooms: impl RoomParam, driver: &D) -> Result<(), D::Error> {
        let rooms = rooms.into_room_iter().collect();
        let chan = self.get_channel();
        let data = self.serialize(RequestType::DelSockets(rooms));
        driver.emit(chan, data).await
    }
    pub async fn disconnect<D: Driver>(self, driver: &D) -> Result<(), D::Error> {
        let chan = self.get_channel();
        let data = self.serialize(RequestType::DisconnectSockets);
        driver.emit(chan, data).await
    }

    #[cfg(any(feature = "msgpack-parser", feature = "common-parser"))]
    pub async fn emit<D: Driver, T: serde::Serialize + ?Sized>(
        self,
        event: &str,
        msg: &T,
        driver: &D,
    ) -> Result<(), EmitError<D>> {
        use socketioxide_core::{
            packet::{Packet, PacketData},
            parser::Parse,
        };

        let value = match self.parser {
            #[cfg(feature = "common-parser")]
            Parser::Common => {
                socketioxide_parser_common::CommonParser.encode_value(msg, Some(event))
            }
            #[cfg(feature = "msgpack-parser")]
            Parser::MsgPack => {
                socketioxide_parser_msgpack::MsgPackParser.encode_value(msg, Some(event))
            }
        }
        .map_err(EmitError::Parser)?;

        let packet = Packet {
            inner: PacketData::Event(value, None),
            ns: self.get_namespace(),
        };
        let chan = self.get_channel();
        let data = self.serialize(RequestType::Broadcast(packet));
        driver.emit(chan, data).await.map_err(EmitError::Driver)?;
        Ok(())
    }
}

impl IoEmitter {
    fn serialize(self, req_type: RequestType) -> Vec<u8> {
        let req = Request::new(req_type, self.opts);
        rmp_serde::to_vec(&req).unwrap()
    }
    /// The request channel used to broadcast requests to all the servers.
    /// format: `{prefix}-request#{path}#`.
    fn get_channel(&self) -> String {
        let prefix = self.prefix.as_deref().unwrap_or("socket.io");
        format!("{}-request#{}#", prefix, self.get_namespace())
    }
    fn get_namespace(&self) -> Str {
        self.ns.clone().unwrap_or(Str::from("/"))
    }
}

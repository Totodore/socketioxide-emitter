#![doc(
    html_logo_url = "https://raw.githubusercontent.com/Totodore/socketioxide/refs/heads/main/.github/logo_dark.svg"
)]
#![doc(
    html_favicon_url = "https://raw.githubusercontent.com/Totodore/socketioxide/refs/heads/main/.github/logo_dark.ico"
)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![warn(
    clippy::all,
    clippy::todo,
    clippy::empty_enum,
    clippy::mem_forget,
    clippy::unused_self,
    clippy::filter_map_next,
    clippy::needless_continue,
    clippy::needless_borrow,
    clippy::match_wildcard_for_single_variants,
    clippy::if_let_mutex,
    clippy::await_holding_lock,
    clippy::match_on_vec_items,
    clippy::imprecise_flops,
    clippy::suboptimal_flops,
    clippy::lossy_float_literal,
    clippy::rest_pat_in_fully_bound_structs,
    clippy::fn_params_excessive_bools,
    clippy::exit,
    clippy::inefficient_to_string,
    clippy::linkedlist,
    clippy::macro_use_imports,
    clippy::option_option,
    clippy::verbose_file_reads,
    clippy::unnested_or_patterns,
    rust_2018_idioms,
    future_incompatible,
    nonstandard_style,
    missing_docs
)]
//! The Socketioxide Emitter crate allows you to easily communicate with a group of Socket.IO servers
//! from another rust process (server-side). It must be used in conjunction with [socketioxide-redis](https://docs.rs/socketioxide-redis).
//!
//! <div class="warning">
//!     Socketioxide Emitter is not compatible with <code>@socketio/redis-adapter</code>
//!     and <code>@socketio/redis-emitter</code>. They use completely different protocols and
//!     cannot be used together. Do not mix socket.io JS servers with socketioxide rust servers.
//!     If you are looking for a way to emit events to a cluster of node socket.io servers in rust,
//!     you should use the <a href="https://github.com/epli2/socketio-rust-emitter">socketio-rust-emitter</a> package.
//! </div>
//!
//! # Diagram taken from the socket.io documentation:
//! <img src="https://raw.githubusercontent.com/socketio/socket.io-redis-emitter/refs/heads/main/assets/emitter.png" width="600" />
//!
//! # Features and parsers
//! The emitter supports two parsers: Common and MessagePack. You can enable/disable them with the `parser-common`
//! and `parser-msgpack` feature flags. If you disable all features, you won't be able to emit events.
//! It will be only possible to manipulate sockets (join/leave rooms, disconnect).
//!
//! # Emit cheat sheet (example with redis)
//! ```no_run
//! use redis::{AsyncCommands, aio::MultiplexedConnection};
//! use socketioxide_emitter::{Driver, IoEmitter};
//!
//! struct RedisConnection(MultiplexedConnection);
//! impl Driver for RedisConnection {
//!     type Error = redis::RedisError;
//!
//!     async fn emit(&self, channel: String, data: Vec<u8>) -> Result<(), Self::Error> {
//!         self.0
//!             .clone()
//!             .publish::<_, _, redis::Value>(channel, data)
//!             .await?;
//!         Ok(())
//!     }
//! }
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let client = redis::Client::open("redis://127.0.0.1").unwrap();
//!     let conn = client.get_multiplexed_tokio_connection().await?;
//!     let conn = RedisConnection(conn);
//!     // sending to all clients
//!     IoEmitter::new().emit("event", "hello!", &conn).await?;
//!
//!     // sending to all clients in 'room1' room
//!     IoEmitter::new().to("room1").emit("event", "message", &conn).await?;
//!
//!     // sending to all clients in 'room1' except those in 'room2'
//!     IoEmitter::new().to("room1").except("room2").emit("event", "message", &conn).await?;
//!
//!     // sending to individual socketid (private message).
//!     // (You will have to make the socket join a room corresponding to its id when it connects.)
//!     IoEmitter::new().to("tK3lxSproMuTbioPAAAB").emit("event", "message", &conn).await?;
//!
//!     let nsp = IoEmitter::new().of("/admin");
//!
//!     // sending to all clients in 'admin' namespace
//!     nsp.clone().emit("event", "message", &conn).await?;
//!
//!     // sending to all clients in 'admin' namespace and in 'notifications' room
//!     nsp.to("notifications").emit("event", "message", &conn).await?;
//!
//!     let msgpack = IoEmitter::new_msgpack();
//!
//!     // sending to all clients and encode message with the msgpack format.
//!     msgpack.clone().emit("event", "message", &conn).await?;
//!
//!     // sending to all clients in 'notifications' room and encode message with the msgpack format.
//!     msgpack.to("notifications").emit("event", "message", &conn).await?;
//!
//!     Ok(())
//! }
use requests::{Request, RequestType};
use socketioxide_core::{
    Str,
    adapter::{BroadcastFlags, BroadcastOptions, RoomParam},
};

mod requests;

#[cfg(any(feature = "msgpack-parser", feature = "common-parser"))]
mod emit;
#[cfg(any(feature = "msgpack-parser", feature = "common-parser"))]
pub use emit::EmitError;

/// The abstraction between the socketio emitter and the underlying system.
/// You must implement it for your specific
/// [`Adapter`](https://docs.rs/socketioxide/latest/socketioxide/#adapters) driver.
///
/// For a redis emitter you would implement the driver trait to emit events to a pubsub channel.
/// The only requirement is that the driver must be able to emit data to specified channels.
///
/// # Example with the [redis](https://docs.rs/redis) crate
/// ```
/// use redis::{AsyncCommands, aio::MultiplexedConnection};
/// use socketioxide_emitter::{Driver, IoEmitter};
///
/// struct RedisConnection(MultiplexedConnection);
/// impl Driver for RedisConnection {
///     type Error = redis::RedisError;
///
///     async fn emit(&self, channel: String, data: Vec<u8>) -> Result<(), Self::Error> {
///         self.0
///             .clone()
///             .publish::<_, _, redis::Value>(channel, data)
///             .await?;
///         Ok(())
///     }
/// }
/// ```
///
/// # Example with the [fred](https://docs.rs/fred) crate
/// ```
/// use fred::{
///     clients::SubscriberClient,
///     prelude::{ClientLike, PubsubInterface},
/// };
/// use socketioxide_emitter::{Driver, IoEmitter};
///
/// struct FredConnection(SubscriberClient);
/// impl Driver for FredConnection {
///     type Error = fred::error::Error;
///
///     async fn emit(&self, channel: String, data: Vec<u8>) -> Result<(), Self::Error> {
///         self.0.publish::<u16, _, _>(channel, data).await?;
///         Ok(())
///     }
/// }
/// ```
pub trait Driver {
    /// The error type returned by the driver.
    type Error: std::error::Error;
    /// Emit data to a given channel.
    fn emit(&self, channel: String, data: Vec<u8>)
    -> impl Future<Output = Result<(), Self::Error>>;
}

/// The [`IoEmitter`] is the main structure for emitting events to a socket.io cluster.
/// It provides a convenient way to broadcast events to all connected nodes and clients.
/// It acts as a simple builder for creating socket.io messages to send through the driver.
#[derive(Clone, Debug)]
pub struct IoEmitter {
    opts: BroadcastOptions,
    ns: Str,
    prefix: Option<String>,
    #[cfg(any(feature = "common-parser", feature = "msgpack-parser"))]
    parser: emit::Parser,
}

impl Default for IoEmitter {
    fn default() -> Self {
        let mut io = Self {
            opts: Default::default(),
            ns: Str::from("/"),
            prefix: None,
            #[cfg(any(feature = "common-parser", feature = "msgpack-parser"))]
            parser: emit::Parser::default(),
        };
        io.opts.add_flag(BroadcastFlags::Broadcast);
        io
    }
}

impl IoEmitter {
    /// Creates a new [`IoEmitter`] with the default settings.
    pub fn new() -> Self {
        Self::default()
    }
    /// Creates a new [`IoEmitter`] with the msgpack parser.
    #[cfg(feature = "msgpack-parser")]
    pub fn new_msgpack() -> Self {
        Self {
            parser: emit::Parser::MsgPack,
            ..Default::default()
        }
    }
    /// Sets the namespace for this [`IoEmitter`]. By default, the namespace is set to `/`.
    pub fn of(mut self, ns: impl Into<Str>) -> IoEmitter {
        self.ns = Str::from(ns.into());
        self
    }
    /// Sets the rooms for this [`IoEmitter`]. By default, events are sent to all rooms.
    pub fn to(mut self, rooms: impl RoomParam) -> IoEmitter {
        self.opts.rooms.extend(rooms.into_room_iter());
        self
    }
    /// Alias for [`IoEmitter::to`].
    pub fn within(self, rooms: impl RoomParam) -> IoEmitter {
        self.to(rooms)
    }
    /// Excludes the specified rooms.
    pub fn except(mut self, rooms: impl RoomParam) -> IoEmitter {
        self.opts.except.extend(rooms.into_room_iter());
        self
    }
    /// You may have set a custom prefix on your adapter config,
    /// which will be used as a prefix for the channel name.
    /// By default, the prefix is `socket.io`.
    pub fn prefix(mut self, prefix: impl Into<String>) -> IoEmitter {
        self.prefix = Some(prefix.into());
        self
    }
}

impl IoEmitter {
    /// Makes the selected sockets join the specified rooms.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Makes the sockets in the root namespace and in the room1 and room2, join the room4.
    /// IoEmitter::new()
    ///     .to(["room1", "room2"])
    ///     .join("room4", &driver)
    ///     .await?;
    /// ```
    pub async fn join<D: Driver>(self, rooms: impl RoomParam, driver: &D) -> Result<(), D::Error> {
        let rooms = rooms.into_room_iter().collect();
        let chan = self.get_channel();
        let data = serialize(self.opts, RequestType::AddSockets(rooms));
        driver.emit(chan, data).await
    }
    /// Makes the selected sockets leave the specified rooms.
    ///
    /// # Example
    ///
    /// ```ignore
    /// // Makes the sockets in the root namespace and in the room1 and room2, leave the room4.
    /// IoEmitter::new()
    ///     .to(["room1", "room2"])
    ///     .leave("room4", &driver)
    ///     .await?;
    /// ```
    pub async fn leave<D: Driver>(self, rooms: impl RoomParam, driver: &D) -> Result<(), D::Error> {
        let rooms = rooms.into_room_iter().collect();
        let chan = self.get_channel();
        let data = serialize(self.opts, RequestType::DelSockets(rooms));
        driver.emit(chan, data).await
    }
    /// Disconnects the selected sockets from their namespace.
    ///
    /// ```ignore
    /// // Makes the sockets in the root namespace and in the room1 and room2, disconnect.
    /// IoEmitter::new()
    ///     .to(["room1", "room2"])
    ///     .disconnect(&driver)
    ///     .await?;
    /// ```
    pub async fn disconnect<D: Driver>(self, driver: &D) -> Result<(), D::Error> {
        let chan = self.get_channel();
        let data = serialize(self.opts, RequestType::DisconnectSockets);
        driver.emit(chan, data).await
    }

    /// Emits a socket.io event to the selected sockets.
    ///
    /// ```ignore
    /// // Emits the event "message" with the message "Hello, world!" to the root namespace sockets
    /// // that are in the room1 and room2
    /// IoEmitter::new()
    ///     .to(["room1", "room2"])
    ///     .emit("message", "Hello, world!", &driver)
    ///     .await?;
    /// ```
    #[cfg(any(feature = "msgpack-parser", feature = "common-parser"))]
    pub async fn emit<D: Driver, T: serde::Serialize + ?Sized>(
        self,
        event: &str,
        msg: &T,
        driver: &D,
    ) -> Result<(), emit::EmitError<D>> {
        use emit::{EmitError, Parser};
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

        let chan = self.get_channel();
        let packet = Packet {
            inner: PacketData::Event(value, None),
            ns: self.ns,
        };

        let data = serialize(self.opts, RequestType::Broadcast(packet));
        driver.emit(chan, data).await.map_err(EmitError::Driver)?;
        Ok(())
    }
}

impl IoEmitter {
    /// The request channel used to broadcast requests to all the servers.
    /// Format: `{prefix}-request#{path}#`.
    fn get_channel(&self) -> String {
        let prefix = self.prefix.as_deref().unwrap_or("socket.io");
        format!("{}-request#{}#", prefix, &self.ns)
    }
}
fn serialize(opts: BroadcastOptions, req_type: RequestType) -> Vec<u8> {
    let req = Request::new(req_type, opts);
    rmp_serde::to_vec(&req).unwrap()
}

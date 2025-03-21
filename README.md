# Socketioxide Emitter

🚀 **Seamless communication with your Socketioxide cluster!**

The **Socketioxide Emitter** lets you easily send messages to a group of Socketioxide servers from a separate Rust process.
Designed for **socketioxide-redis**, this emitter is **fully agnostic** to the underlying system,
meaning support for future adapters will be possible!

> [!WARNING]
> Socketioxide Emitter is not compatible with <code>@socketio/redis-adapter</code>
> and <code>@socketio/redis-emitter</code>. They use completely different protocols and
> cannot be used together. Do not mix socket.io JS servers with socketioxide rust servers.
> If you are looking for a way to emit events to a cluster of node socket.io servers in rust,
> you should use the [socketio-rust-emitter](https://github.com/epli2/socketio-rust-emitter) package

## How It Works

The emitter allows you to **broadcast events** to all connected servers in your cluster.
It operates independently, meaning you can use it outside your main Socket.IO process to
**send messages**, **manage rooms**, and **disconnect sockets**.

![emitter schema](https://raw.githubusercontent.com/socketio/socket.io-redis-emitter/refs/heads/main/assets/emitter.png)

## Parsers

Socketioxide Emitter supports two data encoding formats:

- **Common Parser** (default, widely compatible)
- **MessagePack Parser** (binary-optimized, efficient for large payloads)

Enable or disable them with feature flags: `parser-common` and `parser-msgpack`.

Make sure to use the same parser on the socketioxide servers and the emitter.

## 💡 Quick Start: Emit Cheat Sheet

[_Check the examples for more details._](./examples)

### 1️⃣ Setup a Redis Connection and implement the `Driver` trait

```rust
use redis::{AsyncCommands, aio::MultiplexedConnection};
use socketioxide_emitter::{Driver, IoEmitter};

struct RedisConnection(MultiplexedConnection);
impl Driver for RedisConnection {
    type Error = redis::RedisError;

    async fn emit(&self, channel: String, data: Vec<u8>) -> Result<(), Self::Error> {
        self.0.clone().publish::<_, _, redis::Value>(channel, data).await?;
        Ok(())
    }
}
```

### 2️⃣ Emit Messages

```rust
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = redis::Client::open("redis://127.0.0.1")?;
    let conn = client.get_multiplexed_tokio_connection().await?;
    let conn = RedisConnection(conn);

    // Emit to all clients
    IoEmitter::new().emit("event", "hello!", &conn).await?;

    // Emit to a specific room
    IoEmitter::new().to("room1").emit("event", "message", &conn).await?;

    // Exclude a room
    IoEmitter::new().to("room1").except("room2").emit("event", "message", &conn).await?;

    // Private message to a socket ID
    IoEmitter::new().to("tK3lxSproMuTbioPAAAB").emit("event", "message", &conn).await?;

    // Use a namespace
    let nsp = IoEmitter::new().of("/admin");
    nsp.clone().emit("event", "message", &conn).await?;

    // MessagePack encoding
    let msgpack = IoEmitter::new_msgpack();
    msgpack.emit("event", "message", &conn).await?;

    Ok(())
}
```

## Adapter-Agnostic Design

While **currently available for `socketioxide-redis`**,
the Emitter is designed with **flexibility in mind**. Support for future adapters (such as Kafka, MongoDB, PostgreSQL)
is planned!

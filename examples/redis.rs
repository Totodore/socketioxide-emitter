use redis::{AsyncCommands, aio::MultiplexedConnection};
use socketioxide_emitter::{Driver, IoEmitter};

struct RedisConnection(MultiplexedConnection);
impl Driver for RedisConnection {
    type Error = redis::RedisError;

    async fn emit(&self, channel: String, data: Vec<u8>) -> Result<(), Self::Error> {
        self.0
            .clone()
            .publish::<_, _, redis::Value>(channel, data)
            .await?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = redis::Client::open("redis://127.0.0.1").unwrap();
    let conn = client.get_multiplexed_tokio_connection().await?;
    let conn = RedisConnection(conn);

    IoEmitter::new().emit("event", "hello", &conn).await?;
    IoEmitter::new()
        .of("/admin")
        .emit("event", "hello", &conn)
        .await?;
    IoEmitter::new()
        .within("room")
        .emit("event", "hello", &conn)
        .await?;
    IoEmitter::new().to("test1").disconnect(&conn).await?;
    IoEmitter::new()
        .to("test1")
        .except("room1")
        .join(["blabla", "azidnazdoi"], &conn)
        .await?;
    Ok(())
}

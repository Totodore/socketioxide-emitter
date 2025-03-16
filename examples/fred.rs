use fred::{
    clients::SubscriberClient,
    prelude::{ClientLike, PubsubInterface},
};
use socketioxide_emitter::{Driver, IoEmitter};

struct FredConnection(SubscriberClient);
impl Driver for FredConnection {
    type Error = fred::error::Error;

    async fn emit(&self, channel: String, data: Vec<u8>) -> Result<(), Self::Error> {
        self.0.publish::<u16, _, _>(channel, data).await?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = fred::prelude::Config::from_url("redis://127.0.0.1:6379")?;
    let client = fred::prelude::Builder::from_config(config).build_subscriber_client()?;
    client.init().await?;
    let conn = FredConnection(client);

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

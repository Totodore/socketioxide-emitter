use std::{
    collections::HashMap,
    future::Future,
    sync::{Arc, RwLock},
};
use tokio::sync::mpsc;

use socketioxide::{SocketIo, adapter::Emitter};
use socketioxide_redis::{
    CustomRedisAdapter, RedisAdapterConfig, RedisAdapterCtr,
    drivers::{Driver, MessageStream},
};

pub struct StubEmitterDriver(mpsc::Sender<(String, Vec<u8>)>);

impl socketioxide_emitter::Driver for StubEmitterDriver {
    type Error = mpsc::error::SendError<(String, Vec<u8>)>;

    async fn emit(&self, channel: String, data: Vec<u8>) -> Result<(), Self::Error> {
        self.0.send((channel, data)).await
    }
}

/// Spawns a number of servers with a stub driver for testing.
/// Every server will be connected to every other server.
/// Spawn also an emit-only driver that will simulate the emitting behavior of socketioxide-emitter
pub fn spawn_servers<const N: usize>() -> (
    [SocketIo<CustomRedisAdapter<Emitter, StubDriver>>; N],
    StubEmitterDriver,
) {
    let sync_buff = Arc::new(RwLock::new(Vec::with_capacity(N)));

    let ios = [0; N].map(|_| {
        let (driver, mut rx, tx) = StubDriver::new(N as u16);

        // pipe messages to all other servers
        sync_buff.write().unwrap().push(tx);
        let sync_buff = sync_buff.clone();
        tokio::spawn(async move {
            while let Some((chan, data)) = rx.recv().await {
                for tx in sync_buff.read().unwrap().iter() {
                    tx.try_send((chan.clone(), data.clone())).unwrap();
                }
            }
        });

        let adapter = RedisAdapterCtr::new_with_driver(driver, RedisAdapterConfig::default());
        let (_svc, io) = SocketIo::builder()
            .with_adapter::<CustomRedisAdapter<_, _>>(adapter)
            .build_svc();
        io
    });

    // Create a new driver that will only emit messages to the other servers.
    // This driver will not receive any messages from other servers.
    let (driver, mut rx, _) = StubDriver::new(N as u16);
    let sync_buff = sync_buff.clone();
    tokio::spawn(async move {
        while let Some((chan, data)) = rx.recv().await {
            for tx in sync_buff.read().unwrap().iter() {
                tx.try_send((chan.clone(), data.clone())).unwrap();
            }
        }
    });

    (ios, StubEmitterDriver(driver.tx))
}

type ChanItem = (String, Vec<u8>);
type ResponseHandlers = HashMap<String, mpsc::Sender<ChanItem>>;
#[derive(Debug, Clone)]
pub struct StubDriver {
    tx: mpsc::Sender<ChanItem>,
    handlers: Arc<RwLock<ResponseHandlers>>,
    num_serv: u16,
}

async fn pipe_handlers(mut rx: mpsc::Receiver<ChanItem>, handlers: Arc<RwLock<ResponseHandlers>>) {
    while let Some((chan, data)) = rx.recv().await {
        let handlers = handlers.read().unwrap();
        if let Some(tx) = handlers.get(&chan) {
            tx.try_send((chan, data)).unwrap();
        }
    }
}
impl StubDriver {
    pub fn new(num_serv: u16) -> (Self, mpsc::Receiver<ChanItem>, mpsc::Sender<ChanItem>) {
        let (tx, rx) = mpsc::channel(255); // driver emitter
        let (tx1, rx1) = mpsc::channel(255); // driver receiver
        let handlers = Arc::new(RwLock::new(HashMap::new()));

        tokio::spawn(pipe_handlers(rx1, handlers.clone()));

        let driver = Self {
            tx,
            num_serv,
            handlers,
        };
        (driver, rx, tx1)
    }
}

impl Driver for StubDriver {
    type Error = std::convert::Infallible;

    fn publish(
        &self,
        chan: String,
        val: Vec<u8>,
    ) -> impl Future<Output = Result<(), Self::Error>> + Send {
        self.tx.try_send((chan, val)).unwrap();
        async move { Ok(()) }
    }

    async fn subscribe(
        &self,
        pat: String,
        _: usize,
    ) -> Result<MessageStream<ChanItem>, Self::Error> {
        let (tx, rx) = mpsc::channel(255);
        self.handlers.write().unwrap().insert(pat, tx);
        Ok(MessageStream::new(rx))
    }

    async fn unsubscribe(&self, pat: String) -> Result<(), Self::Error> {
        self.handlers.write().unwrap().remove(&pat);
        Ok(())
    }

    async fn num_serv(&self, _chan: &str) -> Result<u16, Self::Error> {
        Ok(self.num_serv)
    }
}

#[macro_export]
macro_rules! timeout_rcv_err {
    ($srx:expr) => {
        tokio::time::timeout(std::time::Duration::from_millis(10), $srx.recv())
            .await
            .unwrap_err();
    };
}

#[macro_export]
macro_rules! timeout_rcv {
    ($srx:expr) => {
        TryInto::<String>::try_into(
            tokio::time::timeout(std::time::Duration::from_millis(10), $srx.recv())
                .await
                .unwrap()
                .unwrap(),
        )
        .unwrap()
    };
    ($srx:expr, $t:expr) => {
        TryInto::<String>::try_into(
            tokio::time::timeout(std::time::Duration::from_millis($t), $srx.recv())
                .await
                .unwrap()
                .unwrap(),
        )
        .unwrap()
    };
}

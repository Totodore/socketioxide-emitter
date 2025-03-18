use socketioxide::extract::SocketRef;
use socketioxide_emitter::IoEmitter;
mod fixture;

#[tokio::test]
pub async fn add_sockets() {
    let handler = |room: &'static str| move |socket: SocketRef<_>| socket.join(room);
    let ([io1, io2], emitter) = fixture::spawn_servers();

    io1.ns("/", handler("room1")).await.unwrap();
    io2.ns("/", handler("room3")).await.unwrap();

    let ((_tx1, mut rx1), (_tx2, mut rx2)) =
        tokio::join!(io1.new_dummy_sock("/", ()), io2.new_dummy_sock("/", ()));

    timeout_rcv!(&mut rx1); // Connect "/" packet
    timeout_rcv!(&mut rx2); // Connect "/" packet

    IoEmitter::new().join("room2", &emitter).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    let mut rooms = io1.rooms().await.unwrap();
    rooms.sort();
    assert_eq!(rooms, ["room1", "room2", "room3"]);

    timeout_rcv_err!(&mut rx1);
    timeout_rcv_err!(&mut rx2);
}

#[tokio::test]
pub async fn del_sockets() {
    let handler = |rooms: &'static [&'static str]| move |socket: SocketRef<_>| socket.join(rooms);
    let ([io1, io2], emitter) = fixture::spawn_servers();

    io1.ns("/", handler(&["room1", "room2"])).await.unwrap();
    io2.ns("/", handler(&["room3", "room2"])).await.unwrap();

    let ((_tx1, mut rx1), (_tx2, mut rx2)) =
        tokio::join!(io1.new_dummy_sock("/", ()), io2.new_dummy_sock("/", ()));

    timeout_rcv!(&mut rx1); // Connect "/" packet
    timeout_rcv!(&mut rx2); // Connect "/" packet

    IoEmitter::new().leave("room2", &emitter).await.unwrap();
    tokio::time::sleep(std::time::Duration::from_millis(10)).await;

    let mut rooms = io1.rooms().await.unwrap();
    rooms.sort();
    assert_eq!(rooms, ["room1", "room3"]);

    timeout_rcv_err!(&mut rx1);
    timeout_rcv_err!(&mut rx2);
}

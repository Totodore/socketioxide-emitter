use socketioxide::extract::SocketRef;
use socketioxide_emitter::IoEmitter;

mod fixture;

#[tokio::test]
pub async fn broadcast() {
    let ([io1, io2], emitter) = fixture::spawn_servers();

    io1.ns("/", || ()).await.unwrap();
    io2.ns("/", || ()).await.unwrap();

    let ((_tx1, mut rx1), (_tx2, mut rx2)) =
        tokio::join!(io1.new_dummy_sock("/", ()), io2.new_dummy_sock("/", ()));

    timeout_rcv!(&mut rx1); // Connect "/" packet
    timeout_rcv!(&mut rx2); // Connect "/" packet

    IoEmitter::new().emit("test", &2, &emitter).await.unwrap();
    assert_eq!(timeout_rcv!(&mut rx1), r#"42["test",2]"#);
    assert_eq!(timeout_rcv!(&mut rx2), r#"42["test",2]"#);

    timeout_rcv_err!(&mut rx1);
    timeout_rcv_err!(&mut rx2);
}

#[tokio::test]
pub async fn broadcast_rooms() {
    let ([io1, io2, io3], emitter) = fixture::spawn_servers();
    let handler = |rooms: &'static [&'static str]| {
        move |socket: SocketRef<_>| async move {
            // delay to ensure all socket/servers are connected
            socket.join(rooms);
        }
    };

    io1.ns("/", handler(&["room1", "room4"])).await.unwrap();
    io2.ns("/", handler(&["room2", "room4"])).await.unwrap();
    io3.ns("/", handler(&["room3", "room4"])).await.unwrap();

    let ((_tx1, mut rx1), (_tx2, mut rx2), (_tx3, mut rx3)) = tokio::join!(
        io1.new_dummy_sock("/", ()),
        io2.new_dummy_sock("/", ()),
        io3.new_dummy_sock("/", ())
    );

    timeout_rcv!(&mut rx1); // Connect "/" packet
    timeout_rcv!(&mut rx2); // Connect "/" packet
    timeout_rcv!(&mut rx3); // Connect "/" packet

    let emit_room = async |rooms: &'static [&'static str]| {
        IoEmitter::new()
            .to(rooms)
            .emit("test", rooms, &emitter)
            .await
            .unwrap()
    };

    emit_room(&["room1"]).await;
    assert_eq!(timeout_rcv!(&mut rx1), r#"42["test",["room1"]]"#);
    emit_room(&["room2"]).await;
    assert_eq!(timeout_rcv!(&mut rx2), r#"42["test",["room2"]]"#);
    emit_room(&["room3"]).await;
    assert_eq!(timeout_rcv!(&mut rx3), r#"42["test",["room3"]]"#);

    IoEmitter::new()
        .to("room4")
        .except("room3")
        .emit("test", "chippons froutés", &emitter)
        .await
        .unwrap();
    assert_eq!(timeout_rcv!(&mut rx1), r#"42["test","chippons froutés"]"#);
    assert_eq!(timeout_rcv!(&mut rx2), r#"42["test","chippons froutés"]"#);

    IoEmitter::new()
        .within("room4")
        .emit("test", "Barnabouche", &emitter)
        .await
        .unwrap();

    assert_eq!(timeout_rcv!(&mut rx1), r#"42["test","Barnabouche"]"#);
    assert_eq!(timeout_rcv!(&mut rx2), r#"42["test","Barnabouche"]"#);
    assert_eq!(timeout_rcv!(&mut rx3), r#"42["test","Barnabouche"]"#);

    timeout_rcv_err!(&mut rx1);
    timeout_rcv_err!(&mut rx2);
    timeout_rcv_err!(&mut rx3);
}

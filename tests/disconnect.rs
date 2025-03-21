use socketioxide::extract::SocketRef;
use socketioxide_emitter::IoEmitter;

mod fixture;

#[tokio::test]
pub async fn disconnect() {
    let ([io1, io2], emitter) = fixture::spawn_servers();

    io1.ns("/", || ()).await.unwrap();
    io2.ns("/", || ()).await.unwrap();

    let ((_tx1, mut rx1), (_tx2, mut rx2)) =
        tokio::join!(io1.new_dummy_sock("/", ()), io2.new_dummy_sock("/", ()));

    timeout_rcv!(&mut rx1); // Connect "/" packet
    timeout_rcv!(&mut rx2); // Connect "/" packet

    IoEmitter::new().disconnect(&emitter).await.unwrap();
    assert_eq!(timeout_rcv!(&mut rx1), r#"41"#);
    assert_eq!(timeout_rcv!(&mut rx2), r#"41"#);

    timeout_rcv_err!(&mut rx1);
    timeout_rcv_err!(&mut rx2);
}

#[tokio::test]
pub async fn disconnect_of() {
    let ([io1, io2], emitter) = fixture::spawn_servers();

    io1.ns("/", || ()).await.unwrap();
    io2.ns("/test", || ()).await.unwrap();

    let ((_tx1, mut rx1), (_tx2, mut rx2)) =
        tokio::join!(io1.new_dummy_sock("/", ()), io2.new_dummy_sock("/test", ()));

    timeout_rcv!(&mut rx1); // Connect "/" packet
    timeout_rcv!(&mut rx2); // Connect "/" packet

    IoEmitter::new()
        .of("/test")
        .disconnect(&emitter)
        .await
        .unwrap();
    assert_eq!(timeout_rcv!(&mut rx2), r#"41/test,"#);

    timeout_rcv_err!(&mut rx1);
    timeout_rcv_err!(&mut rx2);
}

#[tokio::test]
pub async fn disconnect_to() {
    let ([io1, io2], emitter) = fixture::spawn_servers();

    io1.ns("/", |s: SocketRef<_>| s.join("bar")).await.unwrap();
    io2.ns("/", |s: SocketRef<_>| s.join("foo")).await.unwrap();

    let ((_tx1, mut rx1), (_tx2, mut rx2)) =
        tokio::join!(io1.new_dummy_sock("/", ()), io2.new_dummy_sock("/", ()));

    timeout_rcv!(&mut rx1); // Connect "/" packet
    timeout_rcv!(&mut rx2); // Connect "/" packet

    IoEmitter::new()
        .to("foo")
        .disconnect(&emitter)
        .await
        .unwrap();
    assert_eq!(timeout_rcv!(&mut rx2), r#"41"#);

    timeout_rcv_err!(&mut rx1);
    timeout_rcv_err!(&mut rx2);
}

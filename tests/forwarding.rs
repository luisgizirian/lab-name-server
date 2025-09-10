use lab_name_server::forward_udp;
use tokio::net::UdpSocket;

#[tokio::test]
async fn forward_udp_echo() {
    let server = UdpSocket::bind("127.0.0.1:0").await.expect("bind");
    let addr = server.local_addr().unwrap();
    let server_task = tokio::spawn(async move {
        let mut buf = [0u8; 512];
        if let Ok((n, peer)) = server.recv_from(&mut buf).await {
            let _ = server.send_to(&buf[..n], &peer).await;
        }
    });

    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
    let data = b"ping";
    let resp = forward_udp(&format!("{}:{}", addr.ip(), addr.port()), data)
        .await
        .expect("forward_udp");
    assert_eq!(resp, data);
    let _ = server_task.await;
}

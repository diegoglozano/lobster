use lobster_engine::{EngineHandle, MatchingEngine};
use lobster_proto::{order_generated::lobster::root_as_order, to_core_order};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

#[tokio::main]
async fn main() {
    let listener = TcpListener::bind("127.0.0.1:7777").await.unwrap();
    let (engine, handle) = MatchingEngine::new();
    engine.run();

    loop {
        let (socket, _) = listener.accept().await.unwrap();
        let cloned_handle = handle.clone();
        tokio::spawn(async move {
            process(socket, cloned_handle).await;
        });
    }
}

async fn process(mut socket: TcpStream, handle: EngineHandle) {
    while let Ok(n) = socket.read_u32().await {
        let prefix = n;
        let mut buffer = vec![0u8; prefix as usize];
        socket.read_exact(&mut buffer).await.unwrap();
        let fb_order = root_as_order(&buffer).unwrap();
        let order = to_core_order(&fb_order);
        let result = handle.submit(order);
        match result {
            None => {
                socket.write_u8(0u8).await.unwrap();
            }
            Some((bid_id, ask_id)) => {
                socket.write_u8(1u8).await.unwrap();
                socket.write_all(bid_id.as_bytes()).await.unwrap();
                socket.write_all(ask_id.as_bytes()).await.unwrap();
            }
        };
    }
}

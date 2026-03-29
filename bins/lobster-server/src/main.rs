use lobster_db::TradeRepository;
use lobster_engine::{EngineHandle, MatchingEngine};
use lobster_proto::{order_generated::lobster::root_as_order, to_core_order};
use std::net::SocketAddr;
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpListener, TcpStream},
};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let listener = TcpListener::bind("127.0.0.1:7777").await.unwrap();
    tracing::info!("Listening on 127.0.0.1:7777");
    let (engine, handle, mut trade_rx) = MatchingEngine::new();
    engine.run();

    let repo = TradeRepository::new(&database_url).await;
    repo.run_migrations().await;

    tokio::spawn(async move {
        while let Some(trade) = trade_rx.recv().await {
            repo.insert_trade(&trade).await.unwrap();
        }
    });

    loop {
        let (socket, addr) = listener.accept().await.unwrap();
        tracing::info!("new connection from {}", addr);
        let cloned_handle = handle.clone();
        tokio::spawn(async move {
            process(socket, cloned_handle, addr).await;
        });
    }
}

async fn process(mut socket: TcpStream, handle: EngineHandle, addr: SocketAddr) {
    tracing::info!("new connection from {}", addr);
    while let Ok(n) = socket.read_u32().await {
        tracing::debug!("order received, {} bytes", n);
        let prefix = n;
        let mut buffer = vec![0u8; prefix as usize];
        socket.read_exact(&mut buffer).await.unwrap();
        let fb_order = root_as_order(&buffer).unwrap();
        let order = to_core_order(&fb_order);
        let handle_clone = handle.clone();
        let result = tokio::task::spawn_blocking(move || handle_clone.submit(order))
            .await
            .unwrap();
        match result {
            None => {
                tracing::debug!("no match");
                socket.write_u8(0u8).await.unwrap();
            }
            Some(trade) => {
                tracing::info!(bid_id = %trade.bid_id(), ask_id = %trade.ask_id(), "match");
                socket.write_u8(1u8).await.unwrap();
                socket.write_all(trade.bid_id().as_bytes()).await.unwrap();
                socket.write_all(trade.ask_id().as_bytes()).await.unwrap();
            }
        };
    }
    tracing::info!("client disconnected");
}

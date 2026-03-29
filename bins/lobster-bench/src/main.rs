use lobster_core::{Order, OrderSide, OrderType};
use lobster_proto::to_fb_order;
use rust_decimal::Decimal;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
const TASKS: u16 = 10;
const ORDERS_PER_TASK: u16 = 1000;

async fn task(id: u16) -> Vec<Duration> {
    let mut duration = vec![];
    let mut stream = TcpStream::connect("127.0.0.1:7777").await.unwrap();

    for _ in 0..ORDERS_PER_TASK {
        let start = Instant::now();
        let order = match id % 2 {
            0 => Order::new(OrderSide::Ask, OrderType::Limit, 50, Decimal::new(50, 0)),
            _ => Order::new(OrderSide::Bid, OrderType::Limit, 50, Decimal::new(51, 0)),
        };
        let fb_order = to_fb_order(&order);
        stream.write_u32(fb_order.len() as u32).await.unwrap();
        stream.write_all(&fb_order).await.unwrap();

        stream.read_u8().await.unwrap();
        let elapsed = start.elapsed();
        duration.push(elapsed);
    }
    duration
}

#[tokio::main]
async fn main() {
    let mut handles = vec![];
    let mut all = vec![];

    for id in 0..TASKS {
        handles.push(tokio::spawn(async move { task(id).await }));
    }
    for h in handles {
        let durations = h.await.unwrap();
        all.extend(durations);
    }

    all.sort();
    let p50 = all[all.len() / 2];
    let p99 = all[all.len() * 99 / 100];

    println!("p50: {}", p50.as_micros());
    println!("p99: {}", p99.as_micros());
}

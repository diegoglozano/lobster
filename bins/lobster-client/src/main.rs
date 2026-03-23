use lobster_core::{Order, OrderSide, OrderType};
use lobster_proto::to_fb_order;
use rust_decimal::Decimal;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use uuid::Uuid;

#[tokio::main]
async fn main() {
    let mut stream = TcpStream::connect("127.0.0.1:7777").await.unwrap();

    let order_1 = Order::new(OrderSide::Ask, OrderType::Limit, 50, Decimal::new(50, 0));
    let bytes = to_fb_order(&order_1);
    stream.write_u32(bytes.len() as u32).await.unwrap();
    stream.write_all(&bytes).await.unwrap();

    let flag = stream.read_u8().await.unwrap();
    match flag {
        1 => {
            let ask_id = stream.read_u128().await.unwrap();
            let ask_uuid = Uuid::from_u128(ask_id);
            let bid_id = stream.read_u128().await.unwrap();
            let bid_uuid = Uuid::from_u128(bid_id);

            println!("match! bid: {} ask: {}", bid_uuid, ask_uuid);
        }
        0 => println!("no match!"),
        _ => println!("unknown response"),
    }

    let order_2 = Order::new(OrderSide::Bid, OrderType::Limit, 50, Decimal::new(50, 0));
    let bytes_2 = to_fb_order(&order_2);
    stream.write_u32(bytes_2.len() as u32).await.unwrap();
    stream.write_all(&bytes_2).await.unwrap();
    let flag = stream.read_u8().await.unwrap();
    match flag {
        1 => {
            let ask_id = stream.read_u128().await.unwrap();
            let ask_uuid = Uuid::from_u128(ask_id);
            let bid_id = stream.read_u128().await.unwrap();
            let bid_uuid = Uuid::from_u128(bid_id);

            println!("match! bid: {} ask: {}", bid_uuid, ask_uuid);
        }
        0 => println!("no match!"),
        _ => println!("unknown response"),
    }
}

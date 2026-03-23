pub mod order_generated;
use crate::order_generated::lobster::{self, Order as FbOrder, OrderArgs, finish_order_buffer};
use chrono::DateTime;
use lobster_core::Order;
use rust_decimal::Decimal;
use std::str::FromStr;
use uuid::Uuid;

pub fn to_core_order(fb_order: &FbOrder) -> Order {
    let side = match fb_order.side() {
        lobster::OrderSide::Ask => lobster_core::OrderSide::Ask,
        lobster::OrderSide::Bid => lobster_core::OrderSide::Bid,
        _ => panic!("unknown order side"),
    };
    let order_type = match fb_order.order_type() {
        lobster::OrderType::Limit => lobster_core::OrderType::Limit,
        lobster::OrderType::Market => lobster_core::OrderType::Market,
        _ => panic!("unknown order type"),
    };
    Order::from_parts(
        fb_order.id().unwrap().parse::<Uuid>().unwrap(),
        DateTime::from_timestamp_nanos(fb_order.timestamp()),
        side,
        order_type,
        fb_order.units(),
        Decimal::from_str(fb_order.price().unwrap_or("0")).unwrap(),
    )
}

pub fn to_fb_order(order: &Order) -> Vec<u8> {
    let mut builder = flatbuffers::FlatBufferBuilder::with_capacity(1024);

    let id = builder.create_string(&order.id().to_string());
    let price = builder.create_string(&order.price().to_string());

    let side = match order.side() {
        lobster_core::OrderSide::Ask => lobster::OrderSide::Ask,
        lobster_core::OrderSide::Bid => lobster::OrderSide::Bid,
    };
    let order_type = match order.order_type() {
        lobster_core::OrderType::Limit => lobster::OrderType::Limit,
        lobster_core::OrderType::Market => lobster::OrderType::Market,
    };

    let fb_order = FbOrder::create(
        &mut builder,
        &OrderArgs {
            id: Some(id),
            side,
            order_type,
            units: order.units(),
            price: Some(price),
            timestamp: order.timestamp().timestamp_nanos_opt().unwrap_or(0),
        },
    );
    finish_order_buffer(&mut builder, fb_order);
    builder.finished_data().to_vec()
}

#[cfg(test)]
mod tests {
    use crate::order_generated::lobster::root_as_order;

    use super::*;
    use lobster_core::{OrderSide, OrderType};

    #[test]
    fn serialize_deserialize() {
        let raw_order = Order::new(OrderSide::Ask, OrderType::Limit, 50, Decimal::new(50, 0));
        let fb_bytes = to_fb_order(&raw_order);
        let fb_order = root_as_order(&fb_bytes).unwrap();
        let order = to_core_order(&fb_order);
        assert_eq!(raw_order.id(), order.id());
        assert_eq!(raw_order.side(), order.side());
        assert_eq!(raw_order.order_type(), order.order_type());
        assert_eq!(raw_order.units(), order.units());
        assert_eq!(raw_order.price(), order.price());
        assert_eq!(raw_order.timestamp(), order.timestamp());
    }
}

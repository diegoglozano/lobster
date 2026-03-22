use chrono::{DateTime, Utc};
use rust_decimal::prelude::Decimal;
use std::collections::{BTreeMap, VecDeque};
use uuid::Uuid;

type Price = Decimal;

#[derive(PartialEq, Debug)]
pub enum OrderSide {
    Ask,
    Bid,
}

#[derive(PartialEq, Debug)]
pub enum OrderType {
    Limit,
    Market,
}

pub struct Order {
    id: Uuid,
    timestamp: DateTime<Utc>,
    side: OrderSide,
    order_type: OrderType,
    units: u64,
    price: Price,
}

impl Order {
    pub fn new(side: OrderSide, order_type: OrderType, units: u64, price: Price) -> Self {
        Order {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            side,
            order_type,
            units,
            price,
        }
    }
    pub fn from_parts(
        id: Uuid,
        timestamp: DateTime<Utc>,
        side: OrderSide,
        order_type: OrderType,
        units: u64,
        price: Price,
    ) -> Self {
        Order {
            id,
            timestamp,
            side,
            order_type,
            units,
            price,
        }
    }

    pub fn id(&self) -> Uuid {
        self.id
    }
    pub fn side(&self) -> &OrderSide {
        &self.side
    }
    pub fn order_type(&self) -> &OrderType {
        &self.order_type
    }
    pub fn units(&self) -> u64 {
        self.units
    }
    pub fn price(&self) -> Price {
        self.price
    }
    pub fn timestamp(&self) -> DateTime<Utc> {
        self.timestamp
    }
}

pub struct OrderBook {
    bids: BTreeMap<Price, VecDeque<Order>>,
    asks: BTreeMap<Price, VecDeque<Order>>,
}

impl Default for OrderBook {
    fn default() -> Self {
        Self::new()
    }
}

impl OrderBook {
    pub fn new() -> Self {
        OrderBook {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
        }
    }

    pub fn units_at_price(&self, side: OrderSide, price: Price) -> Option<u64> {
        match side {
            OrderSide::Ask => {
                let ask_queue = self.asks.get(&price)?;
                let units = ask_queue.front()?;
                Some(units.units)
            }
            OrderSide::Bid => {
                let bid_queue = self.bids.get(&price)?;
                let units = bid_queue.front()?;
                Some(units.units)
            }
        }
    }

    pub fn add_order(&mut self, order: Order) {
        match order.side {
            OrderSide::Ask => {
                self.asks.entry(order.price).or_default().push_back(order);
            }
            OrderSide::Bid => {
                self.bids.entry(order.price).or_default().push_back(order);
            }
        }
    }

    pub fn match_orders(&mut self) -> Option<(Uuid, Uuid)> {
        if self.bids.is_empty() || self.asks.is_empty() {
            return None;
        }
        let (bid_price, _) = self.bids.last_key_value()?;
        let (ask_price, _) = self.asks.first_key_value()?;

        let bid_price = *bid_price;
        let ask_price = *ask_price;

        if bid_price < ask_price {
            None
        } else {
            let bid_queue = self.bids.get_mut(&bid_price)?;
            let ask_queue = self.asks.get_mut(&ask_price)?;
            let bid_order = bid_queue.front_mut()?;
            let ask_order = ask_queue.front_mut()?;

            let bid_id = bid_order.id;
            let bid_units = bid_order.units;
            let ask_id = ask_order.id;
            let ask_units = ask_order.units;

            if bid_units < ask_units {
                bid_queue.pop_front();
                if bid_queue.is_empty() {
                    self.bids.remove_entry(&bid_price);
                }
                ask_order.units -= bid_units;
            } else if bid_units > ask_units {
                ask_queue.pop_front();
                if ask_queue.is_empty() {
                    self.asks.remove_entry(&ask_price);
                }
                bid_order.units -= ask_units;
            } else {
                ask_queue.pop_front();
                if ask_queue.is_empty() {
                    self.asks.remove_entry(&ask_price);
                }
                bid_queue.pop_front();
                if bid_queue.is_empty() {
                    self.bids.remove_entry(&bid_price);
                }
            };
            Some((bid_id, ask_id))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_order_has_correct_units() {
        let order = Order::new(OrderSide::Ask, OrderType::Limit, 50, Decimal::new(50, 0));
        assert_eq!(order.units, 50);
    }

    #[test]
    fn new_order_has_correct_price() {
        let order = Order::new(OrderSide::Ask, OrderType::Limit, 50, Decimal::new(50, 0));
        assert_eq!(order.price, Decimal::new(50, 0))
    }

    #[test]
    fn no_match_in_book() {
        let mut order_book = OrderBook::new();
        let ask = Order::new(OrderSide::Ask, OrderType::Limit, 50, Decimal::new(51, 0));
        let bid = Order::new(OrderSide::Bid, OrderType::Limit, 50, Decimal::new(50, 0));
        order_book.add_order(ask);
        order_book.add_order(bid);

        let is_match = order_book.match_orders();
        assert_eq!(is_match, None);
    }

    #[test]
    fn exact_match_in_book() {
        let mut order_book = OrderBook::new();
        let ask = Order::new(OrderSide::Ask, OrderType::Limit, 50, Decimal::new(50, 0));
        let bid = Order::new(OrderSide::Bid, OrderType::Limit, 50, Decimal::new(50, 0));
        let ask_id = ask.id;
        let bid_id = bid.id;

        order_book.add_order(ask);
        order_book.add_order(bid);

        let is_match = order_book.match_orders();
        assert_eq!(is_match, Some((bid_id, ask_id)));
        assert_eq!(
            order_book.units_at_price(OrderSide::Bid, Decimal::new(50, 0)),
            None,
        );
        assert_eq!(
            order_book.units_at_price(OrderSide::Ask, Decimal::new(50, 0)),
            None,
        );
    }

    #[test]
    fn partial_match_in_book_bid_smaller() {
        let mut order_book = OrderBook::new();
        let ask = Order::new(OrderSide::Ask, OrderType::Limit, 50, Decimal::new(50, 0));
        let bid = Order::new(OrderSide::Bid, OrderType::Limit, 49, Decimal::new(50, 0));
        let ask_id = ask.id;
        let bid_id = bid.id;

        order_book.add_order(ask);
        order_book.add_order(bid);

        let is_match = order_book.match_orders();
        assert_eq!(is_match, Some((bid_id, ask_id)));

        assert_eq!(
            order_book.units_at_price(OrderSide::Bid, Decimal::new(50, 0)),
            None
        );
        assert_eq!(
            order_book.units_at_price(OrderSide::Ask, Decimal::new(50, 0)),
            Some(1)
        );
    }
    #[test]
    fn partial_match_in_book_ask_smaller() {
        let mut order_book = OrderBook::new();
        let ask = Order::new(OrderSide::Ask, OrderType::Limit, 49, Decimal::new(50, 0));
        let bid = Order::new(OrderSide::Bid, OrderType::Limit, 50, Decimal::new(50, 0));
        let ask_id = ask.id;
        let bid_id = bid.id;

        order_book.add_order(ask);
        order_book.add_order(bid);

        let is_match = order_book.match_orders();
        assert_eq!(is_match, Some((bid_id, ask_id)));
        assert_eq!(
            order_book.units_at_price(OrderSide::Bid, Decimal::new(50, 0)),
            Some(1)
        );
        assert_eq!(
            order_book.units_at_price(OrderSide::Ask, Decimal::new(50, 0)),
            None
        );
    }
}

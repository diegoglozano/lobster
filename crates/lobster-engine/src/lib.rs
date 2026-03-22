use crossbeam::channel::{Receiver, Sender};
use lobster_core::{Order, OrderBook};
use uuid::Uuid;

pub struct Message {
    order: Order,
    response: Sender<Option<(Uuid, Uuid)>>,
}

pub struct EngineHandle {
    sender: Sender<Message>,
}

impl EngineHandle {
    pub fn submit(&self, order: Order) -> Option<(Uuid, Uuid)> {
        let (response_tx, response_rx) = crossbeam::channel::bounded(1);
        self.sender
            .send(Message {
                order,
                response: response_tx,
            })
            .unwrap();
        response_rx.recv().unwrap()
    }
}

pub struct MatchingEngine {
    order_book: OrderBook,
    receiver: Receiver<Message>,
}

impl MatchingEngine {
    pub fn new() -> (Self, EngineHandle) {
        let (tx, rx) = crossbeam::channel::unbounded();
        let engine = MatchingEngine {
            order_book: OrderBook::new(),
            receiver: rx,
        };
        let handle = EngineHandle { sender: tx };
        (engine, handle)
    }

    pub fn run(mut self) {
        std::thread::spawn(move || {
            while let Ok(msg) = self.receiver.recv() {
                self.order_book.add_order(msg.order);
                let is_match = self.order_book.match_orders();
                msg.response.send(is_match).ok();
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lobster_core::{Order, OrderSide, OrderType};
    use rust_decimal::Decimal;

    #[test]
    fn submit_orders() {
        let (engine, handle) = MatchingEngine::new();
        engine.run();
        handle.submit(Order::new(
            OrderSide::Ask,
            OrderType::Limit,
            50,
            Decimal::new(50, 0),
        ));
        let result = handle.submit(Order::new(
            OrderSide::Bid,
            OrderType::Limit,
            50,
            Decimal::new(50, 0),
        ));
        assert!(result.is_some());
    }
}

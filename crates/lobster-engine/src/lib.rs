use crossbeam::channel::{Receiver as CrossbeamReceiver, Sender as CrossbeamSender, bounded};
use lobster_core::{Order, OrderBook, Trade};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

pub struct Message {
    order: Order,
    response: CrossbeamSender<Option<Trade>>,
}

#[derive(Clone)]
pub struct EngineHandle {
    sender: CrossbeamSender<Message>,
}

impl EngineHandle {
    pub fn submit(&self, order: Order) -> Option<Trade> {
        let (response_tx, response_rx) = bounded(1);
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
    receiver: CrossbeamReceiver<Message>,
    trade_tx: UnboundedSender<Trade>,
}

impl MatchingEngine {
    pub fn new() -> (Self, EngineHandle, UnboundedReceiver<Trade>) {
        let (tx, rx) = crossbeam::channel::unbounded();

        let (trade_tx, trade_rx) = unbounded_channel();

        let engine = MatchingEngine {
            order_book: OrderBook::new(),
            receiver: rx,
            trade_tx,
        };
        let handle = EngineHandle { sender: tx };
        (engine, handle, trade_rx)
    }

    pub fn run(mut self) {
        std::thread::spawn(move || {
            while let Ok(msg) = self.receiver.recv() {
                self.order_book.add_order(msg.order);
                let is_match = self.order_book.match_orders();

                // TODO: avoid two clones per request
                msg.response.send(is_match.clone()).ok();
                if let Some(trade) = is_match {
                    self.trade_tx.send(trade.clone()).ok();
                }
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
        let (engine, handle, _) = MatchingEngine::new();
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

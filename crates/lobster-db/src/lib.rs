use lobster_core::Trade;
use sqlx;

pub struct TradeRepository {
    pool: sqlx::PgPool,
}

impl TradeRepository {
    pub async fn new(database_url: &str) -> Self {
        let pool = sqlx::PgPool::connect(database_url).await.unwrap();
        TradeRepository { pool }
    }

    pub async fn run_migrations(&self) {
        sqlx::migrate!("./migrations")
            .run(&self.pool)
            .await
            .unwrap();
    }

    pub async fn insert_trade(&self, trade: &Trade) -> Result<(), sqlx::Error> {
        sqlx::query(
            "INSERT INTO trades (bid_id, ask_id, price, quantity, timestamp) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(trade.bid_id())
        .bind(trade.ask_id())
        .bind(trade.price())
        .bind(trade.quantity() as i64)
        .bind(trade.timestamp())
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lobster_core::{Order, OrderBook, OrderSide, OrderType};
    use rust_decimal::Decimal;
    use testcontainers::runners::AsyncRunner;
    use testcontainers_modules::postgres::Postgres;

    #[tokio::test]
    async fn insert_trade_persists() {
        let container = Postgres::default().start().await.unwrap();
        let url = format!(
            "postgres://postgres:postgres@127.0.0.1:{}/postgres",
            container.get_host_port_ipv4(5432).await.unwrap()
        );

        let repo = TradeRepository::new(&url).await;
        repo.run_migrations().await;

        // 3. create a trade
        let ask = Order::new(OrderSide::Ask, OrderType::Limit, 50, Decimal::new(50, 0));
        let bid = Order::new(OrderSide::Bid, OrderType::Limit, 50, Decimal::new(50, 0));

        let mut order_book = OrderBook::new();
        order_book.add_order(ask);
        order_book.add_order(bid);

        let trade = order_book.match_orders().unwrap();

        repo.insert_trade(&trade).await.unwrap();
    }
}

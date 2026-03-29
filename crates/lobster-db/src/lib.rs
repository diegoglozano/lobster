use chrono::{DateTime, Utc};
use lobster_core::Trade;
use rust_decimal::Decimal;
use serde;
use sqlx;
use uuid::Uuid;

#[derive(Clone)]
pub struct TradeRepository {
    pool: sqlx::PgPool,
}

#[derive(sqlx::FromRow, serde::Serialize)]
pub struct TradeRow {
    id: Uuid,
    bid_id: Uuid,
    ask_id: Uuid,
    price: Decimal,
    quantity: i64,
    timestamp: DateTime<Utc>,
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

    pub async fn get_trades(&self, limit: i64, offset: i64) -> Result<Vec<TradeRow>, sqlx::Error> {
        sqlx::query_as::<_, TradeRow>(
            "SELECT * FROM trades ORDER BY timestamp DESC LIMIT $1 OFFSET $2",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await
    }

    pub async fn get_trade(&self, id: Uuid) -> Result<TradeRow, sqlx::Error> {
        sqlx::query_as::<_, TradeRow>("SELECT * FROM trades WHERE id = $1")
            .bind(id)
            .fetch_one(&self.pool)
            .await
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

        // 4. insert trade
        repo.insert_trade(&trade).await.unwrap();

        // 5. list trades
        let trades = repo.get_trades(100, 0);
        assert_eq!(trades.await.unwrap().len(), 1)
    }
}

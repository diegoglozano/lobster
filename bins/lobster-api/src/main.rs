use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::get,
};
use lobster_db::{TradeRepository, TradeRow};
use uuid::Uuid;

#[derive(serde::Deserialize)]
struct TradeQuery {
    limit: Option<i64>,
    offset: Option<i64>,
}

#[tokio::main]
async fn main() {
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let repo = TradeRepository::new(&database_url).await;
    let app = Router::new()
        .route("/", get(root))
        .route("/trades", get(get_trades))
        .route("/trades/:id", get(get_trade))
        .with_state(repo);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.expect("server error");
}

async fn root() -> &'static str {
    "Hello, World!"
}

async fn get_trades(
    State(repo): State<TradeRepository>,
    Query(params): Query<TradeQuery>,
) -> Json<Vec<TradeRow>> {
    Json(
        repo.get_trades(params.limit.unwrap_or(10), params.offset.unwrap_or(0))
            .await
            .unwrap(),
    )
}

async fn get_trade(State(repo): State<TradeRepository>, Path(id): Path<Uuid>) -> Json<TradeRow> {
    Json(repo.get_trade(id).await.unwrap())
}

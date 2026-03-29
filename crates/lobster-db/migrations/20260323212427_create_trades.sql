-- Add migration script here
CREATE EXTENSION IF NOT EXISTS pgcrypto;

CREATE TABLE trades (
    id          UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    bid_id      UUID NOT NULL,
    ask_id      UUID NOT NULL,
    price       NUMERIC NOT NULL,
    quantity    BIGINT NOT NULL,
    timestamp   TIMESTAMPTZ NOT NULL
);

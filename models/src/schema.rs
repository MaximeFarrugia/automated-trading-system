// @generated automatically by Diesel CLI.

diesel::table! {
    candles (pair, open_time, timeframe) {
        pair -> Text,
        open_time -> Timestamptz,
        timeframe -> Text,
        open -> Numeric,
        high -> Numeric,
        low -> Numeric,
        close -> Numeric,
        size_in_millis -> Int8,
    }
}

diesel::table! {
    fvgs (pair, open_time, timeframe) {
        pair -> Text,
        open_time -> Timestamptz,
        timeframe -> Text,
        high -> Numeric,
        low -> Numeric,
        flow -> Text,
        close_time -> Nullable<Timestamptz>,
    }
}

diesel::table! {
    trades (pair, open_time, timeframe) {
        pair -> Text,
        open_time -> Timestamptz,
        timeframe -> Text,
        fill_time -> Nullable<Timestamptz>,
        quantity -> Numeric,
        entry -> Numeric,
        stop_loss -> Numeric,
        take_profit -> Numeric,
        flow -> Text,
        close_time -> Nullable<Timestamptz>,
        close -> Nullable<Numeric>,
    }
}

diesel::allow_tables_to_appear_in_same_query!(
    candles,
    fvgs,
    trades,
);

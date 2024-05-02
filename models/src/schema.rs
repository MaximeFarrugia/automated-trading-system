// @generated automatically by Diesel CLI.

diesel::table! {
    candles (pair, open_time, timeframe) {
        pair -> Text,
        open -> Numeric,
        high -> Numeric,
        low -> Numeric,
        close -> Numeric,
        open_time -> Timestamptz,
        timeframe -> Text,
        size_in_millis -> Int8,
    }
}

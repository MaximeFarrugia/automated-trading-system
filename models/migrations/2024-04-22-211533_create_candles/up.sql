create table candles (
    pair text not null,
    open decimal not null,
    high decimal not null,
    low decimal not null,
    close decimal not null,
    open_time timestamptz not null,
    timeframe text not null,
    size_in_millis bigint not null,
    primary key (pair, open_time, timeframe)
);

select create_hypertable('candles', by_range('open_time'));

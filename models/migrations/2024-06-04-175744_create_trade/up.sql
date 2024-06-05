create table trades (
    pair text not null,
    open_time timestamptz not null,
    timeframe text not null,
    fill_time timestamptz default null,
    quantity decimal not null,
    entry decimal not null,
    stop_loss decimal not null,
    take_profit decimal not null,
    flow text not null,
    close_time timestamptz default null,
    close decimal default null,
    primary key (pair, open_time, timeframe)
);

select create_hypertable('trades', by_range('open_time'));

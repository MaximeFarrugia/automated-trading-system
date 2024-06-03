create table fvgs (
    pair text not null,
    open_time timestamptz not null,
    timeframe text not null,
    high decimal not null,
    low decimal not null,
    flow text not null,
    close_time timestamptz default null,
    primary key (pair, open_time, timeframe)
);

select create_hypertable('fvgs', by_range('open_time'));

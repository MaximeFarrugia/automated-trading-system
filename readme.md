## Architecture

### collector

Connected to coinbase websocket api, subscribed to ticker channel.

Forwards every ticker message to Redis' `ticker` channel.

### data-processor

Subscribed to Redis' `ticker` channel.

Creates / Updates candles in database on multiple time frames.

Emits candle updates to Redis' `candle` channel.

Emits candle info on close to Redis' `candle_close` channel.

### indicators

Subscribed to Redis' `candle_close` channel.

Creates / Updates indicators in database on multiple time frames.

Emits indicator updates to Redis' `indicator` channel.

### strategy

Runs strategies.

## Strategies

### iFVG

Gets last FVG that is still open.

If candle closes below a bullish FVG, go short at candle close, ST at FVG's top.

If candle closes above a bearish FVG, go long at candle close, ST at FVG's bottom.

### The Strat

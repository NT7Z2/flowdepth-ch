# News Tracker

Open **Add View → News Tracker**. No exchange ticker is required. The pane works
in dashboard splits and popout windows and preserves its filters in the layout.

The compact headline tape combines CoinDesk, Cointelegraph and Decrypt RSS feeds.
CryptoPanic can contribute additional publishers when configured. Every row shows
Publication time in the computer's local time zone, publisher, detected assets,
event category and relevance score. Local times follow the operating system time
zone, including daylight saving time; UTC is used internally for storage only.
Select a headline to inspect the score explanation and open its source link.
Wide panes show the detail beside the feed. Small panes show a scrollable detail
with a **Back to feed / Close** button, keeping every headline reachable.

Use the asset field for comma-separated symbols (`BTC,ETH,SOL`), the search field
for headlines/topics, the publisher selector and the minimum-impact threshold.
Open **Filters** for publisher, category, relevance threshold, time period and
watchlist controls. Asset names and market pairs such as `Ethereum` and `BTCUSDT`
are normalized. Quick asset buttons add or remove watchlist symbols. Macro events
can be included or excluded explicitly. **Reset** clears active filters. Search
matches all entered words, regardless of their order.

Highest-relevance sorting breaks ties by publication time; otherwise the tape is
newest first. Compact rows are the default; **Expanded rows** increases spacing.
Pause suspends polling. Refresh fetches once while preserving the paused state.
Refresh shows its loading state and repeated requests are disabled while loading.
New headlines wait behind an **N new** button when the user has scrolled down or
opened a detail; showing them returns to the top of the feed.

## Sources and refresh

- CoinDesk: <https://www.coindesk.com/arc/outboundfeeds/rss/>
- Cointelegraph: <https://cointelegraph.com/rss>
- Decrypt: <https://decrypt.co/feed>
- CryptoPanic API reference: <https://cryptopanic.com/developers/api/>

Polling starts immediately and repeats 120 seconds after each fetch completes,
while the layout is active. Panes share a process-wide cache for 120 seconds;
manual refresh bypasses it. News requests respect the application's proxy.
The **N/N sources** button opens provider health, errors and the last successful
update in local computer time. A provider failure leaves previously received
headlines available and marks affected rows as **STALE**.
Identical URLs (ignoring tracking parameters) are deduplicated. Identical titles
are grouped in the feed and all distinct source links remain in the detail. This
groups exact normalized titles; semantic grouping of differently worded headlines
is not performed. The
in-memory tape is limited to 500 headlines from the past seven days; publishers
may provide a much shorter history. Headlines are fetched again after restarting.

## Optional CryptoPanic setup

Set the following environment variables **before starting Flowsurface**:

```powershell
$env:CRYPTOPANIC_AUTH_TOKEN = "your-api-token"
$env:CRYPTOPANIC_API_PLAN = "growth"
cargo run
```

The adapter requests `https://cryptopanic.com/api/{plan}/v2/posts/` using the
token, `public=true` and `kind=news`. The plan defaults to `growth`; `growth_weekly`
and `enterprise` are also accepted as endpoint segments. Choose the segment
assigned to your account and check availability/quota in the provider dashboard.
Without a token, the RSS sources keep working and CryptoPanic is shown as
unconfigured. Tokens are never saved in the layout or included in error messages.
The original publisher URL is preferred when supplied by the API; otherwise the
available CryptoPanic URL is shown explicitly in the detail area.

## Meaning of the impact percentage

The 0–100 value is a **headline-based editorial relevance score**, not a
probability, sentiment signal, prediction of returns, or observed price change.
The detail area explains the matched rule. No LLM or price-reaction model is used.

| Headline event | Base score |
| --- | ---: |
| Security exploit / systemic risk | 90% |
| Macro (rates, inflation, FOMC, tariffs) | 80% |
| Regulatory action / approval | 75% |
| ETF / institutional flows | 65% |
| Supply change / listing | 60% |
| Network upgrade / outage | 55% |
| General asset coverage | 35% |
| General market coverage | 20% |

Specific event rules add five points when an asset is identified. The first
affirmative event rule wins. A negated or denied event is reduced to 25, and
headline wording suggesting rumors or allegations reduces affirmative scores by
20 and displays an uncertainty marker. Security terms include phishing,
unauthorized access, malware and compromised systems. Macro aliases include Fed
and ECB; capitalized LINK and HYPE tickers are recognized without interpreting
ordinary lowercase words as those assets. Binance mentions do not automatically
assign BNB. CryptoPanic instrument metadata can add other symbols.

These are lexical heuristics, not full semantic understanding or independent
verification. Headlines can still miss context, complex negation or unknown
assets. Read the linked articles to assess the event.

## Observed market response

The article detail includes an asset selector, percentage price changes at
**+5 minutes, +1 hour and +24 hours**, and a small return chart. The default is an
asset identified in the article; otherwise BTC is used as a clearly labeled
market reference. BTC, ETH and SOL can also be selected for comparison. Loading
an observation adds its asset/+1h return (or another available horizon) to that
headline in the feed, keeping the observed value separate from editorial relevance.

Market prices come from Binance spot [one-minute historical klines](https://developers.binance.com/docs/binance-spot-api-docs/rest-api/market-data-endpoints).
Requests are paginated to cover the full 24-hour window, respect the application
proxy, and are made only for the selected article. The baseline is the last closed
one-minute candle at or before the reported publication time, at most one minute
earlier. Horizon prices likewise use the last closed candle at or before the
target time. The return is `(horizon close / baseline close - 1) * 100`.

The display uses the computer's local time zone for the baseline and sampling
time. UTC millisecond timestamps are used only in API requests and calculations.
Open candles and future prices are excluded. **Awaiting** means a horizon has not
elapsed; **No data** means a required historical quote is absent. Missing prices
are never shown as a zero return, and the chart breaks across missing minute bars.

For assets other than BTC, **vs BTC** is the asset return minus the BTC return in
percentage points over the same window. This gives context for a broader market
move; it is not statistical attribution. USDT is measured against USDC using the
inverse USDCUSDT price, with BTCUSDC as its same-quote benchmark. Other supported
assets use their USDT pairs. Unavailable Binance spot pairs show an explicit error
and can be replaced using the selector; another asset is not silently substituted.

Observations refresh every minute while the detail is open, including when the
news feed is paused. Results are cached for one minute while horizons develop,
and one hour after the 24-hour window has completed. **Reload** bypasses the cache.
The cache and the feed's remembered observation badges are bounded to 64 entries.
These measurements describe price movements following the publication timestamp;
they do not establish that the article caused the movement or predict returns.

## Visual hierarchy

Feed entries now separate the headline, relevance score, metadata, and local
publication time. Source, category, assets, provenance/status, and observed
price response use distinct badges. Price responses show the asset and elapsed
horizon with green/red direction; asset badges use the accent color. Badges wrap
to fit narrow panes and opening a row still shows the complete source and
market response details. Comfortable mode increases text size and spacing.

### Automatic feed price response

The feed requests observed price response in the background without requiring
an article to be opened. Requests follow the filtered feed order and use the
first detected asset; headlines without a detected asset use BTC, explicitly
marked as a market comparison. A directional badge displays the asset, horizon,
and observed percentage movement as soon as data arrives. Loading, paused,
pending +5-minute observations, and unavailable market data are shown explicitly.

Automatic requests are sequential and paced, share a bounded market-data cache,
and recheck the queue after a 60-second interval. Pause stops the automatic feed
requests. Detail asset selection remains independent of the feed calculations.
The feed reports observed movement after publication, not proof of causation.

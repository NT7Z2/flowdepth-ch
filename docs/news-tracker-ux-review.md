# News Tracker UI and UX review

Review of the `feature/news-tracker` implementation, updated October 11, 2026.

## Implemented improvements

- A clear feed hierarchy separates the headline, relevance score, publisher,
  category, asset badges, source/status badges, and local publication time.
  Observed price response appears as a directional, green/red badge with the
  asset and measurement horizon. Badges wrap inside narrow panes.
- Compact and expanded row modes adjust headline size and spacing. Selecting
  a row opens its source links, relevance explanation, and market response.
- Wide panes show detail beside the feed. Narrow panes provide a scrollable
  detail with a persistent back button so the feed remains reachable.
- Publication, refresh, and market observation times use the operating system's
  local time zone, including daylight saving time.
- Filters cover publisher, category, relevance, period, and normalized asset
  watchlists. Multiword search matches every entered word and reset clears
  active filters. Macro inclusion is explicit.
- Refresh preserves pause. Newly received headlines wait behind an explicit
  button while the user reads a detail or scrolls through older headlines.
- Provider health shows failures and last successful refresh times. Affected
  headlines carry a stale badge. Shared polling caches respect the app proxy.
- Exact normalized titles preserve all distinct source links. Scores account
  for rumors, denials, asset aliases, and macro events.
- Historical market response uses closed one-minute candles before publication
  and after +5 minutes, +1 hour, and +24 hours, with a BTC benchmark and excess
  return when appropriate. Missing and pending observations remain explicit.
  The UI distinguishes observed movement from causal attribution.

## Visual verification

Ignored rendering tests draw the real Iced widgets with the application theme
and live news data. Screenshots are written to `target/news-ux-improved/`.
Checks cover 960×700, 480×420, and 320×400 panes, selected details, filters,
source health, empty results, and historical market response badges.

A separate GEX header rendering check covers widths from 180 to 960 pixels.
It verifies that every laid-out child remains inside the pane. The liquidity
label becomes shorter as space decreases, while its tooltip retains the full
market name. The asset picker and liquidity selector remain available; pane
commands use a compact menu at narrower widths. Rendered images are written
to `target/gex-layout/`.

The native desktop automation service was unavailable. These checks render
actual widgets offscreen; they do not verify physical mouse/keyboard interaction,
Windows scaling, external browser launches, or the complete application window.
CryptoPanic requires credentials and was not included in the live verification.

## Future improvements

Credential setup in the UI, semantic clustering of differently worded events,
configurable alerts, keyboard feed navigation, and linked chart navigation remain
possible extensions. They are not part of the current implementation.

## Regression checks

The GEX header is constrained to 28 pixels plus title-bar padding. The rendering
check also exercises the complete dashboard pane with a populated GEX chart at
180–960 pixel widths and verifies the title and body heights. Testing the header
alone previously missed a height-filling responsive widget that hid the chart.

The News Tracker preview consumes the actual automatic price subscription before
opening any article, verifies that results are stored, and renders its directional
badges. Unit tests cover filtered background requests, exact-title deduplication,
and rejection of results from an obsolete publication time. Feed responses no
longer depend on selecting an article to trigger market-data requests.

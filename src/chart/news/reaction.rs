use super::{Message, NewsNetwork, read_capped};
use data::news::reaction::{Candle, HORIZONS, Reading, Response, response};
use iced::{
    Element, Length, Point,
    widget::{
        canvas::{self, Canvas, Frame, Path, Stroke},
        column, row, text,
    },
};
use std::{
    collections::HashMap,
    sync::OnceLock,
    time::{Duration, Instant},
};

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Request {
    pub article_url: String,
    pub asset: String,
    pub published: i64,
}

#[derive(Debug, Clone)]
pub struct Snapshot {
    pub asset: String,
    pub pair: String,
    pub published: i64,
    pub fetched: i64,
    pub response: Response,
    pub benchmark: Option<Response>,
    pub benchmark_error: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub enum State {
    #[default]
    Idle,
    Loading,
    Ready(Box<Snapshot>),
    Failed(String),
}

fn pair(asset: &str) -> Result<String, String> {
    if asset.is_empty()
        || asset.len() > 20
        || !asset
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
    {
        return Err("Invalid asset symbol.".into());
    }
    Ok(if asset == "USDT" {
        "USDCUSDT".into()
    } else {
        format!("{asset}USDT")
    })
}

pub async fn fetch(
    request: &Request,
    network: &NewsNetwork,
    force: bool,
) -> Result<Snapshot, String> {
    type Cache = HashMap<(String, i64, u64), (Instant, Snapshot)>;
    static CACHE: OnceLock<tokio::sync::Mutex<Cache>> = OnceLock::new();
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    std::hash::Hash::hash(network, &mut hasher);
    let key = (
        request.asset.clone(),
        request.published,
        std::hash::Hasher::finish(&hasher),
    );
    let mut cache = CACHE
        .get_or_init(|| tokio::sync::Mutex::new(HashMap::new()))
        .lock()
        .await;
    let now = chrono::Utc::now().timestamp();
    let ttl = if now - request.published > 86_520 {
        3600
    } else {
        60
    };
    if !force
        && let Some((loaded, snapshot)) = cache.get(&key)
        && loaded.elapsed() < Duration::from_secs(ttl)
    {
        return Ok(snapshot.clone());
    }
    let builder = reqwest::Client::builder()
        .timeout(Duration::from_secs(20))
        .user_agent("Flowsurface NewsReaction/1.0");
    let client = exchange::adapter::proxy::try_apply_proxy(builder, network.0.as_ref())
        .build()
        .map_err(|_| "Market data connection could not be initialized.")?;
    let snapshot = fetch_with_client(&client, request, now).await?;
    if cache.len() >= 512
        && let Some(oldest) = cache
            .iter()
            .min_by_key(|(_, (loaded, _))| *loaded)
            .map(|(key, _)| key.clone())
    {
        cache.remove(&oldest);
    }
    cache.insert(key, (Instant::now(), snapshot.clone()));
    Ok(snapshot)
}

async fn fetch_with_client(
    client: &reqwest::Client,
    request: &Request,
    now: i64,
) -> Result<Snapshot, String> {
    let pair = pair(&request.asset)?;
    if request.published < 120 || request.published > now {
        return Err("Reaction requires a past publication time.".into());
    }
    let start = (request.published / 60 * 60 - 120) * 1000;
    let end = (request.published + 86400).min(now) * 1000;
    let mut candles = fetch_candles(client, "https://api.binance.com", &pair, start, end).await?;
    if request.asset == "USDT" {
        for candle in &mut candles {
            candle.price = 1.0 / candle.price;
        }
    }
    let result = response(&candles, request.published, now).map_err(str::to_owned)?;
    let (benchmark, benchmark_error) = if request.asset == "BTC" {
        (None, None)
    } else {
        match fetch_candles(
            client,
            "https://api.binance.com",
            if request.asset == "USDT" {
                "BTCUSDC"
            } else {
                "BTCUSDT"
            },
            start,
            end,
        )
        .await
        .and_then(|candles| response(&candles, request.published, now).map_err(str::to_owned))
        {
            Ok(result) => (Some(result), None),
            Err(error) => (None, Some(error)),
        }
    };
    Ok(Snapshot {
        asset: request.asset.clone(),
        pair: if request.asset == "USDT" {
            "USDT/USDC (inverse USDCUSDT)".into()
        } else {
            pair
        },
        published: request.published,
        fetched: now,
        response: result,
        benchmark,
        benchmark_error,
    })
}

async fn fetch_candles(
    client: &reqwest::Client,
    base: &str,
    symbol: &str,
    start: i64,
    end: i64,
) -> Result<Vec<Candle>, String> {
    let mut cursor = start;
    let mut candles = Vec::new();
    // A 24h window plus two baseline minutes needs at most two 1000-bar pages.
    for _ in 0..3 {
        let response = client
            .get(format!("{base}/api/v3/klines"))
            .query(&[
                ("symbol", symbol.to_owned()),
                ("interval", "1m".into()),
                ("startTime", cursor.to_string()),
                ("endTime", end.to_string()),
                ("limit", "1000".into()),
            ])
            .send()
            .await
            .map_err(|_| "Market data request failed. Check the connection or proxy.")?;
        match response.status().as_u16() {
            200 => {}
            400 => {
                return Err(format!(
                    "{symbol} is unavailable on Binance spot. Choose another asset."
                ));
            }
            429 | 418 => return Err("Market data rate limit reached. Retry later.".into()),
            451 | 403 => {
                return Err("Binance market data is unavailable from this connection.".into());
            }
            code => return Err(format!("Market data HTTP error {code}.")),
        }
        let bytes = read_capped(response).await.map_err(str::to_owned)?;
        let value: serde_json::Value =
            serde_json::from_slice(&bytes).map_err(|_| "Invalid market data response.")?;
        let mut page = parse_candles(&value)?;
        let length = page.len();
        let Some(last) = page.iter().max_by_key(|c| c.open_ms) else {
            break;
        };
        let next = last.open_ms + 60_000;
        if next <= cursor {
            return Err("Market data pagination did not advance.".into());
        }
        page.retain(|c| c.open_ms >= start && c.open_ms <= end);
        candles.extend(page);
        if length < 1000 || next > end {
            break;
        }
        cursor = next;
    }
    candles.sort_by_key(|c| c.close_ms);
    candles.dedup_by_key(|c| c.close_ms);
    Ok(candles)
}

fn parse_candles(value: &serde_json::Value) -> Result<Vec<Candle>, String> {
    value
        .as_array()
        .ok_or("Invalid candle response.")?
        .iter()
        .map(|row| {
            let open_ms = row[0].as_i64().ok_or("Invalid candle time.")?;
            let close_ms = row[6].as_i64().ok_or("Invalid candle close time.")?;
            let price = row[4]
                .as_str()
                .and_then(|s| s.parse::<f64>().ok())
                .ok_or("Invalid candle price.")?;
            if open_ms < 0
                || open_ms.checked_add(59_999) != Some(close_ms)
                || open_ms % 60_000 != 0
                || !price.is_finite()
                || price <= 0.0
            {
                return Err("Invalid one-minute candle data.".into());
            }
            Ok(Candle {
                open_ms,
                close_ms,
                price,
            })
        })
        .collect()
}

pub fn summary(snapshot: &Snapshot) -> Option<String> {
    [1, 0, 2]
        .into_iter()
        .find_map(|index| match snapshot.response.readings[index] {
            Reading::Observed(value) => Some(format!(
                "{} {} {value:+.2}%",
                snapshot.asset, HORIZONS[index].1
            )),
            _ => None,
        })
}

pub fn view(snapshot: &Snapshot) -> Element<'_, Message> {
    let mut cards = row![].spacing(8);
    for (index, (_, label)) in HORIZONS.iter().enumerate() {
        let mut card = column![
            text(*label).size(12),
            reading(snapshot.response.readings[index])
        ]
        .spacing(4)
        .width(Length::Fill);
        if let Some(benchmark) = &snapshot.benchmark
            && let (Reading::Observed(asset), Reading::Observed(btc)) =
                (snapshot.response.readings[index], benchmark.readings[index])
        {
            card = card.push(text(format!("vs BTC {:+.2}pp", asset - btc)).size(10));
        }
        cards = cards.push(card);
    }
    let baseline_time =
        chrono::DateTime::from_timestamp(snapshot.response.baseline.close_ms / 1000, 0)
            .map(|t| {
                t.with_timezone(&chrono::Local)
                    .format("%d %b %H:%M:%S")
                    .to_string()
            })
            .unwrap_or_default();
    let sampled_time = chrono::DateTime::from_timestamp(snapshot.fetched, 0)
        .map(|t| {
            t.with_timezone(&chrono::Local)
                .format("%H:%M:%S")
                .to_string()
        })
        .unwrap_or_default();
    let quote = if snapshot.asset == "USDT" {
        "USDC"
    } else {
        "USDT"
    };
    let mut body = column![
        text(format!("{} | Binance spot", snapshot.pair)).size(12),
        cards,
        Canvas::new(Plot { snapshot }).width(Length::Fill).height(145),
        text(if snapshot.benchmark.is_some() { "Asset return (color) | BTC return (gray)" } else { "Asset return since publication baseline" }).size(10),
        text(format!("Baseline {:.6} {quote} | {baseline_time} local", snapshot.response.baseline.price)).size(11),
        text(format!("Sampled at {sampled_time} local")).size(10),
        text("Closed 1m candles; baseline up to 1m before publication. Movement in this window does not establish causation.").size(11),
    ].spacing(8);
    if let Some(error) = &snapshot.benchmark_error {
        body = body.push(text(format!("BTC comparison unavailable: {error}")).size(11));
    }
    body.into()
}

fn reading(value: Reading) -> iced::widget::Text<'static> {
    let label = match value {
        Reading::Observed(value) => format!("{value:+.2}%"),
        Reading::Pending => "Awaiting".into(),
        Reading::Missing => "No data".into(),
    };
    text(label)
        .size(15)
        .style(move |theme: &iced::Theme| iced::widget::text::Style {
            color: Some(match value {
                Reading::Observed(value) if value > 0.0 => theme.palette().success.base.color,
                Reading::Observed(value) if value < 0.0 => theme.palette().danger.base.color,
                _ => theme.palette().background.base.text,
            }),
        })
}

struct Plot<'a> {
    snapshot: &'a Snapshot,
}
impl canvas::Program<Message> for Plot<'_> {
    type State = ();
    fn draw(
        &self,
        _: &(),
        renderer: &iced::Renderer,
        theme: &iced::Theme,
        bounds: iced::Rectangle,
        _: iced::mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        if bounds.width < 80.0 || bounds.height < 60.0 {
            return vec![frame.into_geometry()];
        }
        let asset = &self.snapshot.response.path;
        let btc = self
            .snapshot
            .benchmark
            .as_ref()
            .map(|r| r.path.as_slice())
            .unwrap_or(&[]);
        let end = asset.iter().chain(btc).map(|p| p.0).fold(300.0, f64::max);
        let min = asset.iter().chain(btc).map(|p| p.1).fold(0.0, f64::min);
        let max = asset.iter().chain(btc).map(|p| p.1).fold(0.0, f64::max);
        let padding = ((max - min) * 0.12).max(0.02);
        let (low, high) = (min - padding, max + padding);
        let (left, top, width, height) = (43.0, 13.0, bounds.width - 53.0, bounds.height - 35.0);
        let point = |time: f64, value: f64| {
            Point::new(
                left + (time / end) as f32 * width,
                top + ((high - value) / (high - low)) as f32 * height,
            )
        };
        let muted = theme.palette().background.base.text.scale_alpha(0.5);
        frame.stroke(
            &Path::line(point(0.0, 0.0), point(end, 0.0)),
            Stroke::default().with_color(muted).with_width(1.0),
        );
        for (points, color, thickness) in [
            (btc, muted, 1.0),
            (
                asset.as_slice(),
                if asset.last().is_some_and(|p| p.1 < 0.0) {
                    theme.palette().danger.base.color
                } else {
                    theme.palette().success.base.color
                },
                1.8,
            ),
        ] {
            let path = Path::new(|builder| {
                let mut previous = None;
                for &(time, value) in points {
                    let position = point(time, value);
                    if previous.is_none_or(|last| time - last > 61.0) {
                        builder.move_to(position);
                    } else {
                        builder.line_to(position);
                    }
                    previous = Some(time);
                }
            });
            frame.stroke(
                &path,
                Stroke::default().with_color(color).with_width(thickness),
            );
        }
        for (content, position) in [
            (format!("{high:+.2}%"), Point::new(0.0, top)),
            (format!("{low:+.2}%"), Point::new(0.0, top + height - 10.0)),
            ("News".into(), Point::new(left, top + height + 5.0)),
            (
                if end >= 3600.0 {
                    format!("+{:.1}h", end / 3600.0)
                } else {
                    format!("+{:.0}m", end / 60.0)
                },
                Point::new(left + width - 37.0, top + height + 5.0),
            ),
        ] {
            frame.fill_text(canvas::Text {
                content,
                position,
                color: theme.palette().background.base.text,
                size: iced::Pixels(10.0),
                ..Default::default()
            });
        }
        vec![frame.into_geometry()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pagination_covers_twenty_four_hours_and_ignores_open_candles() {
        use std::io::{Read, Write};
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            for (page, range) in [(0, 0..1000), (1, 1000..1443)] {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut request = String::new();
                let mut buffer = [0u8; 4096];
                while !request.contains("\r\n\r\n") {
                    let length = stream.read(&mut buffer).unwrap();
                    assert!(length > 0);
                    request.push_str(std::str::from_utf8(&buffer[..length]).unwrap());
                }
                assert!(request.contains(if page == 0 {
                    "startTime=0"
                } else {
                    "startTime=60000000"
                }));
                let rows: Vec<_> = range
                    .map(|index| {
                        serde_json::json!([
                            index * 60_000,
                            "100",
                            "100",
                            "100",
                            format!("{}", 100.0 + f64::from(index) * 0.01),
                            "1",
                            index * 60_000 + 59_999
                        ])
                    })
                    .collect();
                let body = serde_json::to_string(&rows).unwrap();
                write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
            }
        });
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let candles = runtime
            .block_on(fetch_candles(
                &reqwest::Client::new(),
                &base,
                "BTCUSDT",
                0,
                86_520_000,
            ))
            .unwrap();
        server.join().unwrap();
        assert_eq!(candles.len(), 1443);
        let result = response(&candles, 120, 86_520).unwrap();
        assert!(
            result
                .readings
                .iter()
                .all(|r| matches!(r, Reading::Observed(_)))
        );
        assert_eq!(result.path.len(), 1441);
        assert!(result.path.last().unwrap().0 < 86_400.0);
    }

    #[test]
    fn parsing_rejects_invalid_prices_and_wrong_intervals() {
        assert!(
            parse_candles(&serde_json::json!([[0, "1", "1", "1", "NaN", "1", 59999]])).is_err()
        );
        assert!(
            parse_candles(&serde_json::json!([[0, "1", "1", "1", "10", "1", 299999]])).is_err()
        );
        assert_eq!(
            parse_candles(&serde_json::json!([[0, "1", "1", "1", "10", "1", 59999]])).unwrap()[0]
                .price,
            10.0
        );
        assert_eq!(pair("USDT").unwrap(), "USDCUSDT");
        assert!(pair("BTC/USDT").is_err());
    }

    #[test]
    #[ignore = "contacts public Binance market data"]
    fn live_market_reaction() {
        let request = Request {
            article_url: "https://example.com/news".into(),
            asset: "ETH".into(),
            published: chrono::Utc::now().timestamp() - 90_000,
        };
        let result = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(fetch(&request, &NewsNetwork(None), false))
            .unwrap();
        assert!(result.benchmark.is_some());
        assert!(
            result
                .response
                .readings
                .iter()
                .all(|r| matches!(r, Reading::Observed(_)))
        );
        println!(
            "{} {:?}, {} closed samples",
            result.pair,
            result.response.readings,
            result.response.path.len()
        );
    }
}

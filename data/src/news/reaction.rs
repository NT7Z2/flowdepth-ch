/// Observations use closed one-minute candles. A horizon is measured against
/// the last close before publication, with at most one minute of sampling lag.
#[derive(Debug, Clone, Copy)]
pub struct Candle {
    pub open_ms: i64,
    pub close_ms: i64,
    pub price: f64,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Reading {
    Pending,
    Missing,
    Observed(f64),
}

pub const HORIZONS: [(i64, &str); 3] = [(300, "+5m"), (3600, "+1h"), (86400, "+24h")];

#[derive(Debug, Clone)]
pub struct Response {
    pub baseline: Candle,
    pub readings: [Reading; 3],
    /// Seconds after publication and percentage return from the baseline.
    pub path: Vec<(f64, f64)>,
}

fn percentage(price: f64, baseline: f64) -> Option<f64> {
    let value = (price / baseline - 1.0) * 100.0;
    value.is_finite().then_some(value)
}

fn at_or_before(candles: &[Candle], target: i64, now: i64) -> Option<Candle> {
    candles
        .iter()
        .copied()
        .filter(|c| {
            c.close_ms >= 0
                && c.close_ms <= target
                && c.close_ms <= now
                && target - c.close_ms <= 60_000
                && c.price.is_finite()
                && c.price > 0.0
        })
        .max_by_key(|c| c.close_ms)
}

pub fn response(candles: &[Candle], published: i64, now: i64) -> Result<Response, &'static str> {
    if published < 0 || published > now {
        return Err("Publication time is in the future or invalid.");
    }
    let event_ms = published
        .checked_mul(1000)
        .ok_or("Invalid publication time.")?;
    let now_ms = now.checked_mul(1000).ok_or("Invalid clock time.")?;
    let window_end = event_ms
        .checked_add(86_400_000)
        .ok_or("Invalid observation window.")?;
    let baseline = at_or_before(candles, event_ms, now_ms)
        .ok_or("No closed candle immediately before publication.")?;
    let readings = HORIZONS.map(|(seconds, _)| {
        let target = event_ms + seconds * 1000;
        if target > now_ms {
            Reading::Pending
        } else if let Some(candle) = at_or_before(candles, target, now_ms) {
            percentage(candle.price, baseline.price).map_or(Reading::Missing, Reading::Observed)
        } else {
            Reading::Missing
        }
    });
    let mut path = vec![(0.0, 0.0)];
    let mut closed: Vec<_> = candles
        .iter()
        .filter(|c| {
            c.close_ms > event_ms
                && c.close_ms <= now_ms
                && c.close_ms <= window_end
                && c.price.is_finite()
                && c.price > 0.0
        })
        .collect();
    closed.sort_by_key(|c| c.close_ms);
    closed.dedup_by_key(|c| c.close_ms);
    path.extend(closed.into_iter().filter_map(|c| {
        percentage(c.price, baseline.price)
            .map(|value| ((c.close_ms - event_ms) as f64 / 1000.0, value))
    }));
    Ok(Response {
        baseline,
        readings,
        path,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candle(close_ms: i64, price: f64) -> Candle {
        Candle {
            open_ms: close_ms - 59_999,
            close_ms,
            price,
        }
    }

    #[test]
    fn uses_pre_event_close_and_never_future_or_open_candles() {
        let event = 3_600;
        let candles = [
            candle(3_599_999, 100.0),
            candle(3_899_999, 102.0),
            candle(7_199_999, 110.0),
            candle(89_999_999, 120.0),
        ];
        let result = response(&candles, event, event + 600).unwrap();
        assert!(
            (match result.readings[0] {
                Reading::Observed(v) => v,
                _ => panic!("expected observed value"),
            } - 2.0)
                .abs()
                < 1e-8
        );
        assert_eq!(result.readings[1], Reading::Pending);
        assert_eq!(result.readings[2], Reading::Pending);
        assert!(result.path.iter().all(|(time, _)| *time <= 600.0));
        let complete = response(&candles, event, event + 86400).unwrap();
        assert!(matches!(complete.readings[2], Reading::Observed(v) if (v-20.0).abs() < 1e-8));
    }

    #[test]
    fn missing_samples_are_not_zero_or_replaced_by_distant_quotes() {
        let event = 3600;
        let result = response(&[candle(3_599_999, 100.0)], event, event + 86400).unwrap();
        assert_eq!(result.readings, [Reading::Missing; 3]);
        assert!(response(&[candle(3_000_000, 100.0)], event, event + 86400).is_err());
        assert!(response(&[candle(3_599_999, 0.0)], event, event + 86400).is_err());
        assert!(response(&[candle(3_599_999, f64::NAN)], event, event + 86400).is_err());
    }
}

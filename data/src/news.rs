pub mod reaction;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub search: String,
    /// Comma separated symbols; an empty watchlist includes the entire market.
    pub assets: String,
    pub source: Option<String>,
    pub min_impact: u8,
    pub sort_by_impact: bool,
    pub category: Option<String>,
    pub lookback_hours: u16,
    pub exclude_macro: bool,
    pub comfortable: bool,
}

#[derive(Debug, Clone)]
pub struct Article {
    pub title: String,
    pub url: String,
    pub source: String,
    pub provider: String,
    pub published: i64,
    pub assets: Vec<String>,
    pub category: String,
    pub impact: u8,
    pub reason: String,
}

pub const ASSETS: &[&str] = &[
    "BTC", "ETH", "SOL", "XRP", "BNB", "DOGE", "ADA", "AVAX", "LINK", "SUI", "USDT", "USDC", "HYPE",
];

pub fn normalize_asset(value: &str) -> String {
    let upper = value.trim().to_uppercase();
    let upper = upper.split(['/', '-', ':']).next().unwrap_or("");
    let symbol = match upper {
        "BITCOIN" => "BTC",
        "ETHEREUM" | "ETHER" => "ETH",
        "SOLANA" => "SOL",
        "RIPPLE" => "XRP",
        "CHAINLINK" => "LINK",
        "CARDANO" => "ADA",
        "DOGECOIN" => "DOGE",
        "AVALANCHE" => "AVAX",
        "HYPERLIQUID" => "HYPE",
        _ => upper,
    };
    if ASSETS.contains(&symbol) {
        return symbol.into();
    }
    for quote in ["USDT", "USDC", "USD", "PERP"] {
        if let Some(base) = symbol.strip_suffix(quote)
            && !base.is_empty()
        {
            return base.into();
        }
    }
    symbol.into()
}

pub fn asset_filter(value: &str) -> Vec<String> {
    let mut assets: Vec<_> = value
        .split(|c: char| c == ',' || c.is_whitespace())
        .filter(|s| !s.is_empty())
        .map(normalize_asset)
        .collect();
    assets.sort();
    assets.dedup();
    assets
}

impl Article {
    pub fn matches(&self, config: &Config) -> bool {
        self.matches_at(config, chrono::Utc::now().timestamp())
    }

    pub fn matches_at(&self, config: &Config, now: i64) -> bool {
        let text = format!(
            "{} {} {} {}",
            self.title,
            self.source,
            self.category,
            self.assets.join(" ")
        )
        .to_lowercase();
        let assets = asset_filter(&config.assets);
        self.impact >= config.min_impact
            && config.source.as_ref().is_none_or(|s| s == &self.source)
            && config.category.as_ref().is_none_or(|s| s == &self.category)
            && (config.lookback_hours == 0
                || self.published >= now - i64::from(config.lookback_hours) * 3600)
            && config
                .search
                .to_lowercase()
                .split_whitespace()
                .all(|word| text.contains(word))
            && (assets.is_empty()
                || self.assets.iter().any(|symbol| assets.contains(symbol))
                || (!config.exclude_macro && self.category == "Macro"))
    }
}

fn negated(words: &[&str], index: usize) -> bool {
    words[index.saturating_sub(4)..index].iter().any(|w| {
        matches!(
            *w,
            "not"
                | "no"
                | "never"
                | "denies"
                | "denied"
                | "deny"
                | "debunks"
                | "debunked"
                | "false"
                | "without"
        )
    })
}

pub fn evidence(title: &str) -> &'static str {
    let lower = title.to_lowercase();
    let words: Vec<_> = lower.split(|c: char| !c.is_alphanumeric()).collect();
    if words.iter().any(|w| {
        matches!(
            *w,
            "rumor"
                | "rumors"
                | "rumour"
                | "rumours"
                | "alleged"
                | "allegedly"
                | "unconfirmed"
                | "reportedly"
        )
    }) {
        "Unconfirmed"
    } else {
        "Headline only"
    }
}

/// Editorial relevance based on headline terms, not a calibrated probability
/// or an estimate of price movement. No directional prediction is made.
pub fn classify(title: &str, supplied_assets: Vec<String>) -> (Vec<String>, String, u8, String) {
    let lower = title.to_lowercase();
    let words: Vec<_> = lower
        .split(|c: char| !c.is_alphanumeric())
        .filter(|v| !v.is_empty())
        .collect();
    let mut assets: Vec<String> = supplied_assets
        .into_iter()
        .map(|s| s.to_uppercase())
        .collect();
    for (symbol, names) in [
        ("BTC", &["bitcoin", "btc"][..]),
        ("ETH", &["ethereum", "ether", "eth"][..]),
        ("SOL", &["solana", "sol"][..]),
        ("XRP", &["ripple", "xrp"][..]),
        ("BNB", &["bnb"][..]),
        ("DOGE", &["dogecoin", "doge"][..]),
        ("ADA", &["cardano", "ada"][..]),
        ("AVAX", &["avalanche", "avax"][..]),
        ("LINK", &["chainlink"][..]),
        ("SUI", &["sui"][..]),
        ("USDT", &["tether", "usdt"][..]),
        ("USDC", &["usdc"][..]),
        ("HYPE", &["hyperliquid"][..]),
    ] {
        if names.iter().any(|name| words.contains(name))
            || title
                .split(|c: char| !c.is_alphanumeric())
                .any(|word| word == symbol)
        {
            assets.push(symbol.into());
        }
    }
    assets.sort();
    assets.dedup();
    let rules = [
        (
            "Security",
            90,
            &[
                "hack",
                "hacked",
                "exploit",
                "exploited",
                "breach",
                "breaches",
                "stolen",
                "phishing",
                "compromised",
                "unauthorized",
                "malware",
                "hacks",
                "exploits",
            ][..],
        ),
        (
            "Systemic risk",
            90,
            &[
                "bankruptcy",
                "insolvent",
                "depeg",
                "depegs",
                "collapse",
                "withdrawals halted",
            ][..],
        ),
        (
            "Macro",
            80,
            &[
                "federal reserve",
                "fed",
                "ecb",
                "tariffs",
                "interest rate",
                "rate cut",
                "rate hike",
                "inflation",
                "cpi",
                "fomc",
                "tariff",
                "payrolls",
            ][..],
        ),
        (
            "Regulation",
            75,
            &[
                "sec",
                "lawsuit",
                "regulation",
                "regulatory",
                "banned",
                "ban",
                "approval",
                "approved",
                "cftc",
                "rules",
                "tax",
                "taxes",
            ][..],
        ),
        (
            "Institutional",
            65,
            &[
                "etf",
                "etfs",
                "inflow",
                "inflows",
                "outflow",
                "outflows",
                "treasury",
                "institutional",
            ][..],
        ),
        (
            "Supply / listing",
            60,
            &[
                "unlock",
                "halving",
                "delisting",
                "listing",
                "listed",
                "burn",
            ][..],
        ),
        (
            "Network",
            55,
            &["upgrade", "outage", "mainnet", "hard fork"][..],
        ),
    ];
    let mut denial = None;
    for (category, base, terms) in rules {
        for term in terms {
            let term_words: Vec<_> = term.split_whitespace().collect();
            for (index, _) in words
                .windows(term_words.len())
                .enumerate()
                .filter(|(_, window)| *window == term_words.as_slice())
            {
                if negated(&words, index) {
                    denial = Some((category, *term));
                    continue;
                }
                let asset_bonus = if assets.is_empty() { 0 } else { 5 };
                let uncertainty_penalty = if evidence(title) == "Unconfirmed" {
                    20
                } else {
                    0
                };
                let score = base + asset_bonus - uncertainty_penalty;
                return (
                    assets,
                    category.into(),
                    score,
                    format!(
                        "Headline signal: '{term}' ({category}).{}{} Source claims have not been independently verified.",
                        if asset_bonus > 0 {
                            " An asset mention increases relevance."
                        } else {
                            ""
                        },
                        if uncertainty_penalty > 0 {
                            " Rumor or allegation wording lowers relevance."
                        } else {
                            ""
                        }
                    ),
                );
            }
        }
    }
    if let Some((category, term)) = denial {
        return (
            assets,
            category.into(),
            25,
            format!(
                "'{term}' appears in a negated or denied statement. Relevance reduced; read the source for context."
            ),
        );
    }
    let score = if assets.is_empty() { 20 } else { 35 };
    (assets, "Market".into(), score, "General market coverage; no high-impact event detected in the headline. Editorial heuristic.".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn classification_handles_denial_aliases_and_security_context() {
        assert_eq!(classify("Bitcoin was not hacked", vec![]).2, 25);
        assert_eq!(
            classify("Bitcoin was not hacked but Ethereum was hacked", vec![]).2,
            95
        );
        assert_eq!(classify("LINK price rallies", vec![]).0, ["LINK"]);
        assert_eq!(classify("Fed cuts rates", vec![]).1, "Macro");
        assert_eq!(
            classify("Ledger confirms unauthorized hardware implant", vec![]).1,
            "Security"
        );
        assert_eq!(classify("Bitcoin hack rumors circulate", vec![]).2, 75);
        assert!(classify("ETF hype and a new link", vec![]).0.is_empty());
        assert!(
            classify("Binance launches a new feature", vec![])
                .0
                .is_empty()
        );
    }

    #[test]
    fn filters_normalize_names_pairs_and_handle_time_and_macro() {
        assert_eq!(asset_filter("Ethereum BTCUSDT ETH/USDT"), ["BTC", "ETH"]);
        let article = Article {
            title: "Fed cuts rates today".into(),
            url: String::new(),
            source: "test".into(),
            provider: "test".into(),
            published: 10_000,
            assets: vec![],
            category: "Macro".into(),
            impact: 80,
            reason: String::new(),
        };
        let mut config = Config {
            assets: "Bitcoin".into(),
            search: "rates Fed".into(),
            ..Config::default()
        };
        assert!(article.matches_at(&config, 15_000));
        config.exclude_macro = true;
        assert!(!article.matches_at(&config, 15_000));
        config.exclude_macro = false;
        config.lookback_hours = 1;
        assert!(!article.matches_at(&config, 15_000));
        config.lookback_hours = 24;
        assert!(article.matches_at(&config, 15_000));
        config.category = Some("Security".into());
        assert!(!article.matches_at(&config, 15_000));
        let legacy: Config = serde_json::from_str(r#"{"assets":"BTC"}"#).unwrap();
        assert!(!legacy.exclude_macro);
        assert!(!legacy.comfortable);
    }

    #[test]
    fn matches_symbols_without_substring_false_positives() {
        assert!(classify("Solution links a new method", vec![]).0.is_empty());
        let (assets, category, score, _) = classify("Ethereum protocol hacked", vec!["ETH".into()]);
        assert_eq!(assets, ["ETH"]);
        assert_eq!(category, "Security");
        assert_eq!(score, 95);
    }
    #[test]
    fn macro_stays_visible_for_asset_watchlists() {
        let article = Article {
            title: "Federal Reserve rate cut".into(),
            url: String::new(),
            source: "test".into(),
            provider: "test".into(),
            published: 0,
            assets: vec![],
            category: "Macro".into(),
            impact: 80,
            reason: String::new(),
        };
        let mut config = Config {
            assets: "BTC,ETH".into(),
            ..Config::default()
        };
        assert!(article.matches(&config));
        config.min_impact = 90;
        assert!(!article.matches(&config));
    }
}

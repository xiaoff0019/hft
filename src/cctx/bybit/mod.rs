use std::collections::HashMap;

use anyhow::Result;
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use reqwest::Client;
use serde::Serialize;
use sha2::Sha256;

pub mod oneshot;
pub mod watch;

#[derive(Debug)]
pub struct Bybit {
    pub host: &'static str,
    pub api: Api,
    pub recv_window: i64,
    api_key: String,
    api_secret: String,
    http_client: reqwest::Client,
    pub option: BybitOptions,
}

// TODO: imply default
#[derive(Debug, Serialize)]
pub struct BybitOptions {
    pub account_by_type: HashMap<String, String>,
    pub account_by_id: HashMap<String, String>,
    pub networks: HashMap<String, String>,
    pub networks_by_id: HashMap<String, String>,
}

impl Default for BybitOptions {
    fn default() -> Self {
        let account_by_type = [
            ("spot", "SPOT"),
            ("margin", "SPOT"),
            ("future", "CONTRACT"),
            ("swap", "CONTRACT"),
            ("option", "OPTION"),
            ("investment", "INVESTMENT"),
            ("unified", "UNIFIED"),
            ("funding", "FUND"),
            ("fund", "FUND"),
            ("contract", "CONTRACT"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

        let account_by_id = [
            ("SPOT", "spot"),
            ("MARGIN", "spot"),
            ("CONTRACT", "contract"),
            ("OPTION", "option"),
            ("INVESTMENT", "investment"),
            ("UNIFIED", "unified"),
            ("FUND", "fund"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

        let networks = [
            ("ERC20", "ETH"),
            ("TRC20", "TRX"),
            ("BEP20", "BSC"),
            ("SOL", "SOL"),
            ("ACA", "ACA"),
            ("ADA", "ADA"),
            ("ALGO", "ALGO"),
            ("APT", "APTOS"),
            ("AR", "AR"),
            ("ARBONE", "ARBI"),
            ("AVAXC", "CAVAX"),
            ("AVAXX", "XAVAX"),
            ("ATOM", "ATOM"),
            ("BCH", "BCH"),
            ("BEP2", "BNB"),
            ("CHZ", "CHZ"),
            ("DCR", "DCR"),
            ("DGB", "DGB"),
            ("DOGE", "DOGE"),
            ("DOT", "DOT"),
            ("EGLD", "EGLD"),
            ("EOS", "EOS"),
            ("ETC", "ETC"),
            ("ETHF", "ETHF"),
            ("ETHW", "ETHW"),
            ("FIL", "FIL"),
            ("STEP", "FITFI"),
            ("FLOW", "FLOW"),
            ("FTM", "FTM"),
            ("GLMR", "GLMR"),
            ("HBAR", "HBAR"),
            ("HNT", "HNT"),
            ("ICP", "ICP"),
            ("ICX", "ICX"),
            ("KDA", "KDA"),
            ("KLAY", "KLAY"),
            ("KMA", "KMA"),
            ("KSM", "KSM"),
            ("LTC", "LTC"),
            ("MATIC", "MATIC"),
            ("MINA", "MINA"),
            ("MOVR", "MOVR"),
            ("NEAR", "NEAR"),
            ("NEM", "NEM"),
            ("OASYS", "OAS"),
            ("OASIS", "ROSE"),
            ("OMNI", "OMNI"),
            ("ONE", "ONE"),
            ("OPTIMISM", "OP"),
            ("POKT", "POKT"),
            ("QTUM", "QTUM"),
            ("RVN", "RVN"),
            ("SC", "SC"),
            ("SCRT", "SCRT"),
            ("STX", "STX"),
            ("THETA", "THETA"),
            ("TON", "TON"),
            ("WAVES", "WAVES"),
            ("WAX", "WAXP"),
            ("XDC", "XDC"),
            ("XEC", "XEC"),
            ("XLM", "XLM"),
            ("XRP", "XRP"),
            ("XTZ", "XTZ"),
            ("XYM", "XYM"),
            ("ZEN", "ZEN"),
            ("ZIL", "ZIL"),
            ("ZKSYNC", "ZKSYNC"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

        let networks_by_id = [
            ("ETH", "ERC20"),
            ("TRX", "TRC20"),
            ("BSC", "BEP20"),
            ("OMNI", "OMNI"),
            ("SPL", "SOL"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();

        Self {
            account_by_type,
            account_by_id,
            networks,
            networks_by_id,
        }
    }
}

impl Bybit {
    pub fn new(api_key: &str, api_secret: &str) -> Result<Self> {
        let http_client = Client::builder().tcp_nodelay(true).build()?;
        Ok(Self {
            host: "api.bybit.com",
            api: Api::default(),
            recv_window: 5000,
            api_key: api_key.to_string(),
            api_secret: api_secret.to_string(),
            option: BybitOptions::default(),
            http_client,
        })
    }
}

#[derive(Debug)]
pub struct Api {
    pub server_time: &'static str,
    pub coin_info: &'static str,
    pub market_info: &'static str,
}

impl Default for Api {
    fn default() -> Self {
        Self {
            server_time: "v5/market/time",
            coin_info: "v5/asset/coin/query-info",
            market_info: "v5/market/instruments-info",
        }
    }
}

fn timestamp() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis().try_into().unwrap()
}

fn hmax_sha256(key: &str, msg: &str) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(key.as_bytes()).unwrap();
    mac.update(msg.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

// timestamp: millisecond
fn iso_8601(timestamp: i64) -> Option<String> {
    if timestamp < 0 {
        return None;
    }
    Some(DateTime::<Utc>::from_timestamp_millis(timestamp)?.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
}

// timestamp: millisecond
fn yymmdd(timestamp: i64) -> Option<String> {
    Some(DateTime::<Utc>::from_timestamp_millis(timestamp)?.format("%y%m%d").to_string())
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn test_hmax_sha256() {
        let x = hmax_sha256(
            "XXXXXXXXXX",
            "1765272009594XXXXXXXXXX5000category=linear&settleCoin=USDT",
        );
        assert!(x == "3fe1cc838786f0fcd9af3ec01c12918a818d62ada8f9dd13c3e8f5b61dbb695f")
    }
}

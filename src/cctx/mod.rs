use std::collections::HashMap;

use serde::Serialize;
use serde_json::Value;

pub mod bybit;

trait AsOrParseJson {
    fn a_o_p_i64(&self) -> Option<i64>;
    fn a_o_p_f64(&self) -> Option<f64>;
}

impl AsOrParseJson for Value {
    fn a_o_p_i64(&self) -> Option<i64> {
        self.as_i64().or(self.as_str().and_then(|v| v.parse::<i64>().ok()))
    }

    fn a_o_p_f64(&self) -> Option<f64> {
        self.as_f64().or(self.as_str().and_then(|v| v.parse::<f64>().ok()))
    }
}

trait QueryExt<K, V>
where
    Self: AsRef<[(String, String)]>,
    K: AsRef<str>,
    V: AsRef<str>,
{
    fn append_q(&self, append: &(K, V)) -> Vec<(String, String)> {
        let mut query = self.as_ref().to_vec();
        query.push((append.0.as_ref().to_string(), append.1.as_ref().to_string()));
        query
    }

    fn extend_q(&self, extend: &[(K, V)]) -> Vec<(String, String)> {
        let mut query = self.as_ref().to_vec();
        query.extend(extend.iter().map(|(k, v)| (k.as_ref().to_string(), v.as_ref().to_string())));
        query
    }
}

impl<T, K, V> QueryExt<K, V> for T
where
    T: AsRef<[(String, String)]>,
    K: AsRef<str>,
    V: AsRef<str>,
{
}

#[derive(Debug, Serialize)]
pub struct Ticker {
    pub symbol: String,
    pub timestamp: i64,
    pub datetime: String,
    pub high: f64,
    pub low: f64,
    pub bid: f64,
    pub bid_volume: f64,
    pub ask: f64,
    pub ask_volume: f64,
    pub vwap: f64,
    pub open: f64,
    pub close: f64,
    pub last: f64,
    pub previous_close: f64,
    pub change: f64,
    pub percentage: f64,
    pub average: f64,
    pub base_volume: f64,
    pub quote_volume: f64,
    pub info: HashMap<String, serde_json::Value>,
}

#[derive(Debug, Serialize, Default)]
pub struct Limit {
    pub min: Option<f64>,
    pub max: Option<f64>,
}

#[derive(Debug, Serialize, Default)]
pub struct CurrenyLimits {
    amount: Limit,
    withdraw: Limit,
    deposit: Limit,
}

#[derive(Debug, Serialize, Default)]
pub struct Network {
    pub id: String,
    pub network: String,
    pub name: Option<String>,
    pub active: Option<bool>,
    pub fee: Option<f64>,
    pub precision: Option<i64>,
    pub deposit: Option<bool>,
    pub withdraw: Option<bool>,
    pub limits: CurrenyLimits,
    pub info: Value,
}

#[derive(Debug, Serialize, Default)]
pub struct Curreny {
    pub id: String,
    pub code: String,
    pub name: String,
    pub active: Option<bool>,
    pub fee: Option<f64>,
    pub precision: Option<i64>,
    pub deposit: Option<bool>,
    pub withdraw: Option<bool>,
    pub limits: CurrenyLimits,
    pub networks: HashMap<String, Network>,
    #[serde(rename = "type")]
    pub r#type: String,
    pub info: Value,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Market {
    pub id: String,
    pub symbol: String,
    pub base: String,
    pub quote: String,
    pub base_id: String,
    pub quote_id: String,
    pub active: bool,
    #[serde(rename = "type")]
    pub r#type: String,
    pub spot: bool,
    pub margin: Option<bool>,
    pub future: bool,
    pub swap: bool,
    pub option: bool,
    pub contract: bool,
    pub settle: Option<String>,
    pub settle_id: Option<String>,
    pub contract_size: Option<f64>,
    pub linear: Option<bool>,
    pub inverse: Option<bool>,
    pub expiry: Option<i64>,
    pub expiry_datetime: Option<String>,
    pub strike: Option<f64>,
    pub option_type: Option<String>,
    pub taker: Option<f64>,
    pub maker: Option<f64>,
    pub percentage: Option<bool>,
    pub tier_based: Option<bool>,
    pub fee_side: Option<String>,
    pub precision: MarketPrecision,
    pub limits: MarketLimits,
    pub margin_modes: Option<HashMap<String, bool>>,
    pub created: Option<bool>,
    pub info: Value,
}

#[derive(Debug, Serialize, Default)]
pub struct MarketPrecision {
    pub amount: Option<f64>,
    pub price: Option<f64>,
    pub cost: Option<f64>,
}

#[derive(Debug, Serialize, Default)]
pub struct MarketLimits {
    pub amount: Limit,
    pub price: Limit,
    pub cost: Limit,
    pub leverage: Limit,
}
#[derive(Debug, Serialize, Default)]
pub struct MarginModes {
    pub cross: bool,
    pub isolated: bool,
}

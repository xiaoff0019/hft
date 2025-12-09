use std::collections::HashMap;

use crate::cctx::*;

use super::Bybit;
use super::*;
use anyhow::{Context, Result};
use reqwest::{Method, RequestBuilder, Url};
use serde_json::Value;
#[cfg(test)]
mod test;
// use tokio_tungstenite::connect_async;
// use tungstenite::Bytes;
// use tungstenite::client::IntoClientRequest;

impl Bybit {
    pub async fn fetch_time(&self) -> Result<i64> {
        // https://bybit-exchange.github.io/docs/v5/market/time
        let url = self.new_url(self.api.server_time)?;
        let resp = self.http_client.get(url).send().await?.json::<Value>().await?;
        self.check_resp(&resp)?;
        let time = resp.get("time").and_then(|v| v.a_o_p_i64()).context("parse time fail")?;
        Ok(time)
    }

    pub async fn fetch_currencies(&self) -> Result<HashMap<String, Curreny>> {
        // https://bybit-exchange.github.io/docs/v5/asset/coin-info
        let url = self.new_url(self.api.coin_info)?;
        let resp = self.sign_requst(self.http_client.get(url)).send().await?.json::<Value>().await?;

        self.check_resp(&resp)?;

        let result = resp.get("result").context("no result")?;
        let rows = result.get("rows").context("no rows")?.as_array().context("rows not array")?;

        let mut res = HashMap::new();
        for row in rows {
            let Some(currenty_id) = row.get("coin").and_then(|v| v.as_str()) else {
                continue;
            };
            let code = currenty_id;
            let Some(name) = row.get("name").and_then(|v| v.as_str()) else {
                continue;
            };
            let mut networks = HashMap::new();
            _ = row.get("chains").and_then(|v| v.as_array()).inspect(|&chains| {
                chains.iter().for_each(|chain| {
                    let Some(network_id) = chain.get("chain").and_then(|v| v.as_str()) else {
                        return;
                    };
                    let network_code = self.option.networks_by_id.get(network_id).map_or(network_id, |v| v);
                    let network = Network {
                        info: chain.clone(),
                        id: network_id.to_string(),
                        network: network_code.to_string(),
                        active: None,
                        name: None,
                        fee: chain.get("withdrawFee").and_then(|v| v.a_o_p_f64()),
                        precision: chain.get("minAccuracy").and_then(|v| v.a_o_p_i64()),
                        limits: CurrenyLimits {
                            withdraw: Limit {
                                min: chain.get("withdrawMin").and_then(|v| v.a_o_p_f64()),
                                max: None,
                            },
                            deposit: Limit {
                                min: chain.get("depositMin").and_then(|v| v.a_o_p_f64()),
                                max: None,
                            },
                            amount: Limit { min: None, max: None },
                        },
                        deposit: None,
                        withdraw: None,
                    };
                    networks.insert(network_code.to_string(), network);
                });
            });

            let currency_item = Curreny {
                info: row.clone(),
                code: code.to_string(),
                id: currenty_id.to_string(),
                name: name.to_string(),
                active: None,
                deposit: None,
                withdraw: None,
                fee: None,
                precision: None,
                limits: CurrenyLimits::default(),
                r#type: "crypto".to_string(),
                networks,
            };
            res.insert(code.to_string(), currency_item);
        }
        Ok(res)
    }

    async fn fetch_spot_markets(&self, query: &[(String, String)]) -> Result<Vec<Market>> {
        let url = self.new_url(self.api.market_info)?;
        let query = query.append_q(&("category", "spot"));
        let mut resp = self.http_client.get(url.clone()).query(&query).send().await?.json::<Value>().await?;
        self.check_resp(&resp)?;
        let result = resp.get_mut("result").context("no result")?;
        let mut next_cursor =
            result.get("nextPageCursor").and_then(|v| v.as_str()).map_or(String::new(), |v| v.to_string());
        let markets = result.get_mut("list").context("no list")?.as_array_mut().context("list not array")?;
        while !next_cursor.is_empty() {
            let new_query = query.append_q(&("cursor", &next_cursor));
            let resp = self.http_client.get(url.clone()).query(&new_query).send().await?.json::<Value>().await?;
            self.check_resp(&resp)?;
            let result = resp.get("result").context("no result")?;
            next_cursor =
                result.get("nextPageCursor").and_then(|v| v.as_str()).map_or(String::new(), |v| v.to_string());
            let new_markets = result.get("list").context("no list")?.as_array().context("list not array")?;
            markets.extend_from_slice(new_markets);
        }

        let mut res = Vec::new();
        for market in markets {
            let Some(id) = market.get("symbol").and_then(|v| v.as_str()) else {
                continue;
            };
            let Some(base_id) = market.get("baseCoin").and_then(|v| v.as_str()) else {
                continue;
            };
            let Some(quote_id) = market.get("quoteCoin").and_then(|v| v.as_str()) else {
                continue;
            };
            let base = self.get_currency_code(base_id);
            let quote = self.get_currency_code(quote_id);
            let symbol = format!("{base}/{quote}");
            let active = market.get("status").and_then(|v| v.as_str()).is_some_and(|status| status == "Trading");
            let lot_size_filter = market.get("lotSizeFilter");
            let price_filter = market.get("priceFilter");
            let quote_precision = lot_size_filter.as_ref().and_then(|v| v.get("quotePrecision"));
            let allow_margin = market.get("marginTrading").and_then(|v| v.as_str()).is_none_or(|v| v != "none");
            let item = Market {
                id: id.to_string(),
                symbol,
                base,
                quote,
                base_id: base_id.to_string(),
                quote_id: quote_id.to_string(),
                active,
                r#type: "spot".to_string(),
                spot: true,
                margin: Some(allow_margin),
                future: false,
                swap: false,
                option: false,
                contract: false,
                settle: None,
                settle_id: None,
                contract_size: None,
                linear: None,
                inverse: None,
                expiry: None,
                expiry_datetime: None,
                strike: None,
                option_type: None,
                taker: None,
                maker: None,
                percentage: None,
                tier_based: None,
                fee_side: None,
                precision: MarketPrecision {
                    amount: lot_size_filter.as_ref().and_then(|v| v.get("basePrecision").and_then(|v| v.a_o_p_f64())),
                    price: price_filter
                        .and_then(|v| v.get("tickSize").and_then(|v| v.a_o_p_f64()))
                        .or(quote_precision.and_then(|v| v.a_o_p_f64())),
                    cost: None,
                },
                limits: MarketLimits {
                    amount: Limit {
                        min: lot_size_filter.as_ref().and_then(|v| v.get("minOrderQty").and_then(|v| v.a_o_p_f64())),
                        max: lot_size_filter.as_ref().and_then(|v| v.get("maxOrderQty").and_then(|v| v.a_o_p_f64())),
                    },
                    price: Limit { min: None, max: None },
                    cost: Limit {
                        min: lot_size_filter.as_ref().and_then(|v| v.get("minOrderAmt").and_then(|v| v.a_o_p_f64())),
                        max: lot_size_filter.as_ref().and_then(|v| v.get("maxOrderAmt").and_then(|v| v.a_o_p_f64())),
                    },
                    leverage: Limit {
                        min: Some(1.0),
                        max: None,
                    },
                },
                margin_modes: None,
                created: None,
                info: market.clone(),
            };
            res.push(item);
        }
        Ok(res)
    }

    async fn fetch_future_markets(&self, query: &[(String, String)]) -> Result<Vec<Market>> {
        let url = self.new_url(self.api.market_info)?;
        let query = query.append_q(&("limit", "1000"));
        let pre_query = query.append_q(&("status", "PreLaunch"));
        let (resp, pre_resp) = tokio::join!(
            self.http_client.get(url.clone()).query(&query).send().await?.json::<Value>(),
            self.http_client.get(url.clone()).query(&pre_query).send().await?.json::<Value>()
        );
        let mut resp = resp?;
        let mut pre_resp = pre_resp?;
        self.check_resp(&resp)?;
        self.check_resp(&pre_resp)?;
        let result = resp.get_mut("result").context("no result")?;
        let pre_result = pre_resp.get_mut("result").context("no result")?;
        let mut next_cursor =
            result.get("nextPageCursor").and_then(|v| v.as_str()).map_or(String::new(), |v| v.to_string());
        let markets = result.get_mut("list").context("no list")?.as_array_mut().context("list not array")?;
        if let Some(pre_markets) = pre_resp.get("list").and_then(|v| v.as_array()) {
            markets.extend_from_slice(pre_markets);
        }
        while !next_cursor.is_empty() {
            let new_query = query.append_q(&("cursor", &next_cursor));
            let new_resp = self.http_client.get(url.clone()).query(&new_query).send().await?.json::<Value>().await?;
            self.check_resp(&new_resp)?;
            let new_result = new_resp.get("result").context("no result")?;
            next_cursor =
                new_result.get("nextPageCursor").and_then(|v| v.as_str()).map_or(String::new(), |v| v.to_string());
            if let Some(new_markets) = new_result.get("list").and_then(|v| v.as_array()) {
                markets.extend_from_slice(new_markets);
            }
        }
        // let mut res = Vec::new();

        for market in markets {
            let Some(category) = market.get("category").and_then(|v| v.as_str()) else {
                continue;
            };

            // var linear interface{} = (IsEqual(category, "linear"))
            // var inverse interface{} = (IsEqual(category, "inverse"))
            // var contractType interface{} = this.SafeString(market, "contractType")
            // var inverseFutures interface{} = (IsEqual(contractType, "InverseFutures"))
            // var linearFutures interface{} = (IsEqual(contractType, "LinearFutures"))
            // var linearPerpetual interface{} = (IsEqual(contractType, "LinearPerpetual"))
            // var inversePerpetual interface{} = (IsEqual(contractType, "InversePerpetual"))
            // var id interface{} = this.SafeString(market, "symbol")
            // var baseId interface{} = this.SafeString(market, "baseCoin")
            // var quoteId interface{} = this.SafeString(market, "quoteCoin")
            // var defaultSettledId interface{} = Ternary(IsTrue(linear), quoteId, baseId)
            // var settleId interface{} = this.SafeString(market, "settleCoin", defaultSettledId)
            // var base interface{} = this.SafeCurrencyCode(baseId)
            // var quote interface{} = this.SafeCurrencyCode(quoteId)
            // var settle interface{} = nil
            // if IsTrue(IsTrue(linearPerpetual) && IsTrue((IsEqual(settleId, "USD")))) {
            // 	settle = "USDC"
            // } else {
            // 	settle = this.SafeCurrencyCode(settleId)
            // }
            // var symbol interface{} = Add(Add(base, "/"), quote)
            // var lotSizeFilter interface{} = this.SafeDict(market, "lotSizeFilter", map[string]interface{}{})
            // var priceFilter interface{} = this.SafeDict(market, "priceFilter", map[string]interface{}{})
            // var leverage interface{} = this.SafeDict(market, "leverageFilter", map[string]interface{}{})
            // var status interface{} = this.SafeString(market, "status")
            // var swap interface{} = IsTrue(linearPerpetual) || IsTrue(inversePerpetual)
            // var future interface{} = IsTrue(inverseFutures) || IsTrue(linearFutures)
            // var typeVar interface{} = nil
            // if IsTrue(swap) {
            // 	typeVar = "swap"
            // } else if IsTrue(future) {
            // 	typeVar = "future"
            // }
            // var expiry interface{} = nil
            // // some swaps have deliveryTime meaning delisting time
            // if !IsTrue(swap) {
            // 	expiry = this.OmitZero(this.SafeString(market, "deliveryTime"))
            // 	if IsTrue(!IsEqual(expiry, nil)) {
            // 		expiry = ParseInt(expiry)
            // 	}
            // }
            // var expiryDatetime interface{} = this.Iso8601(expiry)
            // symbol = Add(Add(symbol, ":"), settle)
            // if IsTrue(!IsEqual(expiry, nil)) {
            // 	symbol = Add(Add(symbol, "-"), this.Yymmdd(expiry))
            // }

            let linear = category == "linear";
            let inverse = category == "inverse";
            let contract_type = market.get("contractType").and_then(|v| v.as_str()).unwrap_or_default();
            let inverse_future = contract_type == "InverseFutures";
            let linear_future = contract_type == "LinearFutures";
            let linear_perpetual = contract_type == "LinearPerpetual";
            let inverse_perpetual = contract_type == "InversePerpetual";
            let Some(id) = market.get("symbol").and_then(|v| v.as_str()) else {
                continue;
            };
            let Some(base_id) = market.get("baseCoin").and_then(|v| v.as_str()) else {
                continue;
            };
            let Some(quote_id) = market.get("quoteCoin").and_then(|v| v.as_str()) else {
                continue;
            };
            let base = self.get_currency_code(base_id);
            let quote = self.get_currency_code(quote_id);
            let symbol = format!("{base}/{quote}");
            let default_settled_id = if linear { quote_id } else { base_id };
            let settle_id = market.get("settleCoin").and_then(|v| v.as_str()).unwrap_or(default_settled_id);
            let settle = if linear_perpetual && (settle_id == "USD") {
                "USDC".to_string()
            } else {
                self.get_currency_code(settle_id)
            };
            let symbol = format!("{base}/{quote}");
            let lot_size_filter = market.get("lotSizeFilter");
            let price_filter = market.get("priceFilter");
            let leverage = market.get("leverageFilter");
            let status = market.get("status").and_then(|v| v.as_str());
            let swap = linear_perpetual || inverse_perpetual;
            let future = inverse_future || linear_future;
            let type_var = if swap {
                "swap".to_string()
            } else if future {
                "future".to_string()
            } else {
                String::new()
            };
            let expiry = if !swap {
                market
                    .get("deliveryTime")
                    .and_then(|v| v.a_o_p_f64())
                    .and_then(|v| if v == 0.0 { None } else { Some(v) })
                    .map(|v| v as i64)
            } else {
                None
            };
            let mut symbol = format!("{symbol}:{settle}");
            if let Some(ts) = expiry {
                symbol = format!("{symbol}-{}", yymmdd(ts * 1000).unwrap_or_default());
            }
            let expiry_data_time = expiry.and_then(|v| iso_8601(v));
            let contract_size = if inverse { 
                // lot_size_filter.as_ref().get("minTradingQty")
             } else { Some(1.0f) };
        }

        todo!()
    }

    async fn fetch_option_markets(&self, query: &[(String, String)]) -> Result<Vec<Market>> {
        todo!()
    }

    async fn fetch_markets(&self, query: &[(String, String)]) -> Result<Vec<Market>> {
        // https://bybit-exchange.github.io/docs/v5/market/instrument

        let linear_query = query.append_q(&("category", "linear"));
        let inverse_query = query.append_q(&("category", "inverse"));
        let btc_option_query = query.append_q(&("baseCoin", "BTC"));
        let eth_option_query = query.append_q(&("baseCoin", "ETH"));
        let sol_option_query = query.append_q(&("baseCoin", "SOL"));
        let (spot_markets, linear_markets, inverse_markets, btc_option_markets, eth_option_markets, sol_option_markets) = tokio::join!(
            self.fetch_spot_markets(query),
            self.fetch_future_markets(&linear_query),
            self.fetch_future_markets(&inverse_query),
            self.fetch_option_markets(&btc_option_query),
            self.fetch_option_markets(&eth_option_query),
            self.fetch_option_markets(&sol_option_query),
        );

        let mut spot_markets = spot_markets.context("spot markets fail")?;
        let linear_markets = linear_markets.context("linear markets fail")?;
        let inverse_markets = inverse_markets.context("inverse markets fail")?;
        let btc_option_markets = btc_option_markets.context("btc option markets fail")?;
        let eth_option_markets = eth_option_markets.context("eth option markets fail")?;
        let sol_option_markets = sol_option_markets.context("sol option markets fail")?;
        spot_markets.extend(linear_markets);
        spot_markets.extend(inverse_markets);
        spot_markets.extend(btc_option_markets);
        spot_markets.extend(eth_option_markets);
        spot_markets.extend(sol_option_markets);

        Ok(spot_markets)
    }

    // TODO
    fn get_currency_code(&self, id: &str) -> String {
        id.to_string()
    }

    #[inline]
    fn new_url(&self, url: &str) -> Result<Url> {
        Ok(Url::parse(&format!("https://{}/", self.host))?.join(url)?)
    }

    #[inline]
    fn check_resp(&self, resp_value: &Value) -> Result<()> {
        let Some(ret_code) = resp_value.get("retCode").and_then(|v| v.a_o_p_i64()) else {
            return Ok(());
        };

        let Some(ret_msg) = resp_value.get("retMsg").and_then(|v| v.as_str()) else {
            return Ok(());
        };

        if ret_code != 0 {
            anyhow::bail!("retCode: {ret_code}, retMsg: {ret_msg}")
        } else {
            Ok(())
        }
    }

    fn sign_requst(&self, builder: RequestBuilder) -> RequestBuilder {
        let Some(build_ori) = builder.try_clone() else {
            return builder;
        };
        let Ok(mut req_ori) = build_ori.build() else {
            return builder;
        };

        let sign_item = match *req_ori.method() {
            Method::GET => req_ori.url().query().unwrap_or_default().to_string(),
            Method::POST => req_ori
                .body()
                .and_then(|body| body.as_bytes())
                .map(|body_bytes| String::from_utf8_lossy(body_bytes).to_string())
                .unwrap_or_default(),
            _ => return builder,
        };

        let timestamp = timestamp();
        let api_key = self.api_key.as_str();
        let recv_window = self.recv_window;
        let to_be_signed = format!("{timestamp}{api_key}{recv_window}{sign_item}");
        let sign_str = hmax_sha256(&self.api_secret, &to_be_signed);
        let header_ori = req_ori.headers_mut();
        let Ok(api_key) = api_key.parse() else {
            return builder;
        };
        let Ok(sign_str) = sign_str.parse() else {
            return builder;
        };
        let Ok(timestamp) = timestamp.to_string().parse() else {
            return builder;
        };

        let Ok(recv_window) = recv_window.to_string().parse() else {
            return builder;
        };

        header_ori.insert("X-BAPI-SIGN", sign_str);
        header_ori.insert("X-BAPI-API-KEY", api_key);
        header_ori.insert("X-BAPI-TIMESTAMP", timestamp);
        header_ori.insert("X-BAPI-RECV-WINDOW", recv_window);
        builder.headers(header_ori.clone())
    }
}

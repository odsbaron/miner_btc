use anyhow::{Context, Result};
use base64::Engine;
use reqwest::blocking::Client;
use serde::de::DeserializeOwned;
use serde_json::{json, Value};

use crate::template::BlockTemplate;

#[derive(Debug, Clone)]
pub struct RpcClient {
    url: String,
    user: String,
    pass: String,
    http: Client,
}

impl RpcClient {
    #[must_use]
    pub fn new(url: impl Into<String>, user: impl Into<String>, pass: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            user: user.into(),
            pass: pass.into(),
            http: Client::new(),
        }
    }

    pub fn get_block_template(&self) -> Result<BlockTemplate> {
        self.call("getblocktemplate", json!([{ "rules": ["segwit"] }]))
    }

    pub fn submit_block(&self, block_hex: &str) -> Result<Option<String>> {
        self.call("submitblock", json!([block_hex]))
    }

    pub fn call<T: DeserializeOwned>(&self, method: &str, params: Value) -> Result<T> {
        let auth = base64::engine::general_purpose::STANDARD
            .encode(format!("{}:{}", self.user, self.pass));
        let body = json!({
            "jsonrpc": "1.0",
            "id": "miner_btc",
            "method": method,
            "params": params,
        });

        let response: Value = self
            .http
            .post(&self.url)
            .header("Authorization", format!("Basic {auth}"))
            .json(&body)
            .send()
            .with_context(|| format!("RPC transport failed for {method}"))?
            .error_for_status()
            .with_context(|| format!("RPC HTTP error for {method}"))?
            .json()
            .with_context(|| format!("RPC JSON decode failed for {method}"))?;

        if !response["error"].is_null() {
            anyhow::bail!("RPC {method} returned error: {}", response["error"]);
        }
        serde_json::from_value(response["result"].clone())
            .with_context(|| format!("RPC result decode failed for {method}"))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn getblocktemplate_params_include_segwit_rule() {
        let params = json!([{ "rules": ["segwit"] }]);
        assert_eq!(params[0]["rules"][0], "segwit");
    }
}

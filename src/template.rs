use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct BlockTemplate {
    pub version: i32,
    pub previousblockhash: String,
    pub transactions: Vec<TemplateTransaction>,
    pub coinbasevalue: u64,
    pub target: String,
    pub mintime: Option<u64>,
    pub curtime: u64,
    pub bits: String,
    pub height: u64,
    pub default_witness_commitment: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TemplateTransaction {
    pub data: String,
    pub txid: String,
    #[serde(default)]
    pub hash: Option<String>,
    #[serde(default)]
    pub depends: Vec<u64>,
    #[serde(default)]
    pub fee: Option<i64>,
    #[serde(default)]
    pub weight: Option<u64>,
}

impl BlockTemplate {
    #[must_use]
    pub fn total_fees(&self) -> i64 {
        self.transactions.iter().filter_map(|tx| tx.fee).sum()
    }

    #[must_use]
    pub fn total_weight_hint(&self) -> u64 {
        self.transactions.iter().filter_map(|tx| tx.weight).sum()
    }
}

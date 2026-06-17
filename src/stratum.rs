use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpStream};

use anyhow::{Context, Result};
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct SubscribeResponse {
    pub extranonce1: String,
    pub extranonce2_size: usize,
}

impl SubscribeResponse {
    pub fn from_json(raw: &str) -> Result<Self> {
        let value: Value = serde_json::from_str(raw).context("invalid stratum subscribe JSON")?;
        let result = value
            .get("result")
            .and_then(Value::as_array)
            .context("subscribe response result must be an array")?;
        let extranonce1 = result
            .get(1)
            .and_then(Value::as_str)
            .context("subscribe response missing extranonce1")?
            .to_string();
        let extranonce2_size = result
            .get(2)
            .and_then(Value::as_u64)
            .context("subscribe response missing extranonce2_size")?
            as usize;
        Ok(Self {
            extranonce1,
            extranonce2_size,
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ServerMessage {
    SetDifficulty { difficulty: f64 },
    Notify(StratumJob),
    Other { method: Option<String> },
}

impl ServerMessage {
    pub fn from_json(raw: &str) -> Result<Self> {
        let value: Value = serde_json::from_str(raw).context("invalid stratum server JSON")?;
        let method = value.get("method").and_then(Value::as_str);
        match method {
            Some("mining.set_difficulty") => {
                let difficulty = value
                    .get("params")
                    .and_then(Value::as_array)
                    .and_then(|params| params.first())
                    .and_then(Value::as_f64)
                    .context("mining.set_difficulty missing numeric difficulty")?;
                Ok(Self::SetDifficulty { difficulty })
            }
            Some("mining.notify") => Ok(Self::Notify(StratumJob::from_params(
                value
                    .get("params")
                    .and_then(Value::as_array)
                    .context("mining.notify missing params array")?,
            )?)),
            other => Ok(Self::Other {
                method: other.map(str::to_string),
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StratumJob {
    pub job_id: String,
    pub prevhash: String,
    pub coinb1: String,
    pub coinb2: String,
    pub merkle_branch: Vec<String>,
    pub version: String,
    pub nbits: String,
    pub ntime: String,
    pub clean_jobs: bool,
}

impl StratumJob {
    fn from_params(params: &[Value]) -> Result<Self> {
        Ok(Self {
            job_id: string_param(params, 0, "job_id")?,
            prevhash: string_param(params, 1, "prevhash")?,
            coinb1: string_param(params, 2, "coinb1")?,
            coinb2: string_param(params, 3, "coinb2")?,
            merkle_branch: params
                .get(4)
                .and_then(Value::as_array)
                .context("mining.notify missing merkle_branch")?
                .iter()
                .map(|item| {
                    item.as_str()
                        .map(str::to_string)
                        .context("merkle branch item must be hex string")
                })
                .collect::<Result<Vec<_>>>()?,
            version: string_param(params, 5, "version")?,
            nbits: string_param(params, 6, "nbits")?,
            ntime: string_param(params, 7, "ntime")?,
            clean_jobs: params
                .get(8)
                .and_then(Value::as_bool)
                .context("mining.notify missing clean_jobs bool")?,
        })
    }
}

fn string_param(params: &[Value], index: usize, name: &str) -> Result<String> {
    params
        .get(index)
        .and_then(Value::as_str)
        .map(str::to_string)
        .with_context(|| format!("mining.notify missing {name}"))
}

#[derive(Debug, Clone, Serialize)]
pub struct ClientRequest<'a> {
    id: u64,
    method: &'a str,
    params: Vec<&'a str>,
}

impl<'a> ClientRequest<'a> {
    #[must_use]
    pub fn subscribe(id: u64) -> Self {
        Self {
            id,
            method: "mining.subscribe",
            params: vec![],
        }
    }

    #[must_use]
    pub fn authorize(id: u64, username: &'a str, password: &'a str) -> Self {
        Self {
            id,
            method: "mining.authorize",
            params: vec![username, password],
        }
    }

    #[must_use]
    pub fn submit(
        id: u64,
        username: &'a str,
        job_id: &'a str,
        extranonce2: &'a str,
        ntime: &'a str,
        nonce: &'a str,
    ) -> Self {
        Self {
            id,
            method: "mining.submit",
            params: vec![username, job_id, extranonce2, ntime, nonce],
        }
    }

    #[must_use]
    pub fn to_json_line(&self) -> String {
        let mut line = serde_json::to_string(self).expect("stratum client request serializes");
        line.push('\n');
        line
    }
}

#[derive(Debug)]
pub struct LineClient {
    stream: TcpStream,
    reader: BufReader<TcpStream>,
}

impl LineClient {
    pub fn connect(addr: SocketAddr) -> Result<Self> {
        let stream =
            TcpStream::connect(addr).with_context(|| format!("connect stratum pool {addr}"))?;
        let reader = BufReader::new(
            stream
                .try_clone()
                .context("clone stratum TCP stream for line reader")?,
        );
        Ok(Self { stream, reader })
    }

    pub fn send(&mut self, request: &ClientRequest<'_>) -> Result<()> {
        self.stream
            .write_all(request.to_json_line().as_bytes())
            .context("write stratum request")?;
        self.stream.flush().context("flush stratum request")
    }

    pub fn read_line(&mut self) -> Result<String> {
        let mut line = String::new();
        let bytes = self
            .reader
            .read_line(&mut line)
            .context("read stratum line")?;
        if bytes == 0 {
            anyhow::bail!("stratum connection closed");
        }
        Ok(line)
    }
}

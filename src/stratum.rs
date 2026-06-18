use std::io::{BufRead, BufReader, Write};
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use anyhow::{bail, Context, Result};
use num_bigint::BigUint;
use num_traits::{One, ToPrimitive};
use serde::Serialize;
use serde_json::Value;

const DIFF1_TARGET_HEX: &str = "00000000ffff000000000000000000000000000000000000000000000000000000";

pub fn difficulty_to_target_be_hex(difficulty: f64) -> Result<String> {
    if !difficulty.is_finite() || difficulty <= 0.0 {
        bail!("stratum difficulty must be positive and finite");
    }
    let diff1 = BigUint::parse_bytes(DIFF1_TARGET_HEX.as_bytes(), 16)
        .context("parse diff1 target constant")?;
    let scaled = if (difficulty.fract()).abs() < f64::EPSILON {
        let divisor = BigUint::from(difficulty.to_u64().context("difficulty too large")?);
        diff1 / divisor
    } else {
        let scale = 1_000_000u64;
        let numerator = diff1 * BigUint::from(scale);
        let divisor = BigUint::from((difficulty * scale as f64).round() as u64);
        numerator / divisor
    };
    Ok(format!("{scaled:0>64x}"))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Extranonce2Roller {
    width: usize,
    next: u128,
}

impl Extranonce2Roller {
    #[must_use]
    pub fn new(width: usize) -> Self {
        Self { width, next: 0 }
    }

    #[must_use]
    pub fn with_start(width: usize, start: u128) -> Self {
        Self { width, next: start }
    }

    pub fn next_hex(&mut self) -> Result<String> {
        if self.width == 0 || self.width > 16 {
            bail!("extranonce2 width must be between 1 and 16 bytes");
        }
        let limit = BigUint::one() << (self.width * 8);
        if BigUint::from(self.next) >= limit {
            bail!("extranonce2 exhausted for {} bytes", self.width);
        }
        let bytes = self.next.to_le_bytes();
        let hex = hex::encode(&bytes[..self.width]);
        self.next += 1;
        Ok(hex)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReconnectPolicy {
    base_delay_secs: u64,
    max_delay_secs: u64,
}

impl ReconnectPolicy {
    #[must_use]
    pub fn new(base_delay_secs: u64, max_delay_secs: u64) -> Self {
        Self {
            base_delay_secs: base_delay_secs.max(1),
            max_delay_secs: max_delay_secs.max(1),
        }
    }

    #[must_use]
    pub fn delay_secs(&self, attempt: u32) -> u64 {
        self.base_delay_secs
            .saturating_mul(2u64.saturating_pow(attempt))
            .min(self.max_delay_secs)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connecting { attempt: u32 },
    Connected,
}

#[derive(Debug, Clone)]
pub struct StratumRuntime {
    policy: ReconnectPolicy,
    state: ConnectionState,
    total_failures: u64,
}

impl StratumRuntime {
    #[must_use]
    pub fn new(policy: ReconnectPolicy) -> Self {
        Self {
            policy,
            state: ConnectionState::Disconnected,
            total_failures: 0,
        }
    }

    pub fn record_connect_failure(&mut self) -> u64 {
        self.total_failures += 1;
        let next_attempt = self.current_attempt() + 1;
        self.state = ConnectionState::Connecting {
            attempt: next_attempt,
        };
        self.policy.delay_secs(next_attempt.saturating_sub(1))
    }

    pub fn record_connected(&mut self) {
        self.state = ConnectionState::Connected;
    }

    #[must_use]
    pub fn total_failures(&self) -> u64 {
        self.total_failures
    }

    #[must_use]
    pub fn current_attempt(&self) -> u32 {
        match self.state {
            ConnectionState::Connecting { attempt } => attempt,
            ConnectionState::Disconnected | ConnectionState::Connected => 0,
        }
    }
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NtimeRoller {
    base: u32,
    max_delta: u32,
    next_delta: u32,
}

impl NtimeRoller {
    pub fn new(base_hex: &str, max_delta: u32) -> Result<Self> {
        let base = u32::from_str_radix(base_hex, 16).context("ntime must be 4-byte hex")?;
        Ok(Self {
            base,
            max_delta,
            next_delta: 0,
        })
    }

    pub fn next_hex(&mut self) -> Result<String> {
        if self.next_delta > self.max_delta {
            bail!("ntime rolling exhausted after {} seconds", self.max_delta);
        }
        let value = self
            .base
            .checked_add(self.next_delta)
            .context("ntime overflow")?;
        self.next_delta += 1;
        Ok(format!("{value:08x}"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionRollingMask {
    base: u32,
    mask: u32,
}

impl VersionRollingMask {
    pub fn new(base_hex: &str, mask_hex: &str) -> Result<Self> {
        Ok(Self {
            base: u32::from_str_radix(base_hex, 16).context("version must be 4-byte hex")?,
            mask: u32::from_str_radix(mask_hex, 16)
                .context("version rolling mask must be 4-byte hex")?,
        })
    }

    pub fn apply(&self, value: u32) -> Result<String> {
        if value & !self.mask != 0 {
            bail!("version rolling value uses bits outside negotiated mask");
        }
        Ok(format!("{:08x}", self.base | (value & self.mask)))
    }
}

#[derive(Debug, Clone)]
pub struct StratumLoopConfig {
    pub addr: SocketAddr,
    pub username: String,
    pub password: String,
    pub max_shares: usize,
    pub read_timeout: Duration,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StratumLoopSummary {
    pub submitted_shares: usize,
    pub reconnects: u64,
}

#[derive(Debug, Clone)]
pub struct StratumMinerLoop {
    config: StratumLoopConfig,
    runtime: StratumRuntime,
}

impl StratumMinerLoop {
    #[must_use]
    pub fn new(config: StratumLoopConfig) -> Self {
        Self {
            config,
            runtime: StratumRuntime::new(ReconnectPolicy::new(1, 30)),
        }
    }

    pub fn run_once(&mut self) -> Result<StratumLoopSummary> {
        let stream = TcpStream::connect(self.config.addr)
            .with_context(|| format!("connect stratum pool {}", self.config.addr))?;
        stream
            .set_read_timeout(Some(self.config.read_timeout))
            .context("set stratum read timeout")?;
        stream
            .set_write_timeout(Some(self.config.read_timeout))
            .context("set stratum write timeout")?;
        let mut client = LineClient {
            reader: BufReader::new(stream.try_clone().context("clone stratum stream")?),
            stream,
        };
        self.runtime.record_connected();
        client.send(&ClientRequest::subscribe(1))?;
        client.send(&ClientRequest::authorize(
            2,
            &self.config.username,
            &self.config.password,
        ))?;

        let mut extranonce2 = Extranonce2Roller::new(4);
        let mut submitted = 0usize;
        while submitted < self.config.max_shares {
            let line = client.read_line()?;
            if line.contains("\"id\":1") {
                let sub = SubscribeResponse::from_json(&line)?;
                extranonce2 = Extranonce2Roller::new(sub.extranonce2_size);
                continue;
            }
            let message = ServerMessage::from_json(&line)?;
            if let ServerMessage::Notify(job) = message {
                let extra = extranonce2.next_hex()?;
                let ntime = NtimeRoller::new(&job.ntime, 0)?.next_hex()?;
                let nonce = "00000000";
                client.send(&ClientRequest::submit(
                    3 + submitted as u64,
                    &self.config.username,
                    &job.job_id,
                    &extra,
                    &ntime,
                    nonce,
                ))?;
                submitted += 1;
            }
        }
        Ok(StratumLoopSummary {
            submitted_shares: submitted,
            reconnects: self.runtime.total_failures(),
        })
    }
}

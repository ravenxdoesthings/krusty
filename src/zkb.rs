/*
 * Copyright (C) 2025 Raven X Does Things
 */

use opentelemetry::trace::Status;
use serde::Deserialize;
use tracing::{Level, Span};
use tracing_opentelemetry::OpenTelemetrySpanExt;

#[derive(Debug, Deserialize)]
pub struct Response {
    #[serde(rename = "package")]
    pub killmail: Option<Killmail>,
}

#[derive(Debug, Deserialize)]
pub struct Zkb {
    pub href: String,
}

#[derive(Debug, Deserialize)]
pub struct Killmail {
    #[serde(rename = "killID")]
    pub kill_id: u64,
    pub zkb: Zkb,
    #[serde(skip)]
    pub killmail: Option<KillmailData>,
}

impl Killmail {
    pub async fn fetch_data(&mut self) -> Result<(), reqwest::Error> {
        if self.killmail.is_some() {
            return Ok(());
        }

        let resp = reqwest::get(&self.zkb.href).await;
        match resp {
            Ok(response) => {
                let json = response.json::<KillmailData>().await;
                match json {
                    Ok(data) => {
                        self.killmail = Some(data);
                    }
                    Err(e) => {
                        tracing::error!(error = e.to_string(), "failed to parse killmail data");
                        return Err(e);
                    }
                }
            }
            Err(e) => {
                tracing::error!(error = e.to_string(), "failed to fetch killmail data");
                return Err(e);
            }
        }

        Ok(())
    }

    pub fn skew(&self) -> chrono::Duration {
        match &self.killmail {
            None => chrono::Duration::zero(),
            Some(km) => {
                let now = chrono::Utc::now();
                now.signed_duration_since(km.timestamp)
            }
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct KillmailData {
    #[serde(rename = "killmail_time")]
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub attackers: Vec<Participant>,
    pub victim: Participant,
    #[serde(rename = "solar_system_id")]
    pub system_id: u64,
}

impl Default for KillmailData {
    fn default() -> Self {
        Self {
            timestamp: chrono::Utc::now(),
            attackers: vec![],
            victim: Participant::default(),
            system_id: 0,
        }
    }
}

#[derive(Debug, Default, Deserialize)]
pub struct Participant {
    pub character_id: Option<u64>,
    pub corporation_id: Option<u64>,
    pub alliance_id: Option<u64>,
    pub ship_type_id: Option<u64>,
}

impl Participant {
    pub fn is_npc(&self) -> bool {
        self.character_id.is_none()
    }
}

#[derive(Debug, Deserialize)]
pub struct SequenceTracker {
    pub sequence: u64,
    #[serde(skip)]
    pub last_status: Option<u16>,
}

#[derive(Debug)]
pub struct SequenceNotFoundError {
    pub sequence: u64,
}

impl std::fmt::Display for SequenceNotFoundError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "sequence {} not found", self.sequence)
    }
}

impl std::error::Error for SequenceNotFoundError {}

impl SequenceTracker {
    pub fn new() -> Self {
        Self {
            sequence: 0,
            last_status: None,
        }
    }

    pub async fn fetch_starting(&mut self, client: &reqwest::Client) -> Result<(), reqwest::Error> {
        let resp = client
            .get("https://r2z2.zkillboard.com/ephemeral/sequence.json")
            .send()
            .await?;
        let json = resp.json::<SequenceTracker>().await?;

        self.sequence = json.sequence;

        Ok(())
    }

    pub async fn fetch_next(
        &mut self,
        parent_span: Option<&Span>,
        client: &reqwest::Client,
    ) -> Result<KillmailV3, anyhow::Error> {
        let span = tracing::span!(Level::INFO, "fetching next killmail");
        if let Some(parent) = parent_span {
            let _ = span.set_parent(parent.context());
        }
        let _enter = span.enter();

        let url = format!(
            "https://r2z2.zkillboard.com/ephemeral/{}.json",
            self.sequence
        );
        let resp = client.get(&url).send().await?;
        self.last_status = Some(resp.status().as_u16());
        if resp.status() == reqwest::StatusCode::NOT_FOUND {
            span.set_status(Status::error("not found"));
            return Err(SequenceNotFoundError {
                sequence: self.sequence,
            }
            .into());
        }
        let raw = resp.text().await?;

        let killmail: KillmailV3 = match simd_json::from_slice(&mut raw.clone().into_bytes()) {
            Ok(km) => km,
            Err(e) => {
                span.set_status(Status::error(format!("failed to parse killmail: {e}")));
                tracing::error!(error = e.to_string(), "failed to parse killmail");
                return Err(e.into());
            }
        };

        self.sequence += 1;

        Ok(killmail)
    }
}

impl Default for SequenceTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
pub struct KillmailV3 {
    pub killmail_id: u64,
    pub hash: String,
    pub esi: KillmailData,
    pub uploaded_at: u64,
    pub sequence_id: u64,
}

impl KillmailV3 {
    pub fn skew(&self) -> chrono::Duration {
        let now = chrono::Utc::now();
        let kill_time = self.esi.timestamp;
        now.signed_duration_since(kill_time)
    }
}

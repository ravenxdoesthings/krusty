/*
 * Copyright (C) 2025 Raven X Does Things
 */

#[derive(Debug, serde::Deserialize)]
pub struct Response {
    #[serde(rename = "package")]
    pub killmail: Option<Killmail>,
}

#[derive(Debug, serde::Deserialize)]
pub struct Zkb {
    pub href: String,
}

#[derive(Debug, serde::Deserialize)]
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

#[derive(Debug, serde::Deserialize)]
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

#[derive(Debug, Default, serde::Deserialize)]
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

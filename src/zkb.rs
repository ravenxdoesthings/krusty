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
    pub hash: String,
    #[serde(rename = "fittedValue")]
    pub fitted_value: f64,
    #[serde(rename = "destroyedValue")]
    pub destroyed_value: f64,
    #[serde(rename = "droppedValue")]
    pub dropped_value: f64,
    #[serde(rename = "totalValue")]
    pub total_value: f64,
    #[serde(rename = "attackerCount")]
    pub attacker_count: u64,
}

impl Zkb {
    pub fn killmail_id(&self) -> Option<u64> {
        let re = regex::Regex::new(r"/killmails/(\d+)/").unwrap();
        if let Some(caps) = re.captures(&self.href) {
            if let Some(matched) = caps.get(1) {
                if let Ok(id) = matched.as_str().parse::<u64>() {
                    return Some(id);
                }
            }
        }
        None
    }
}

impl Default for Zkb {
    fn default() -> Self {
        Self {
            href: String::new(),
            hash: String::new(),
            fitted_value: 0.0,
            destroyed_value: 0.0,
            dropped_value: 0.0,
            total_value: 0.0,
            attacker_count: 0,
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zkb_killmail_id_extraction() {
        let zkb = Zkb {
            href: "https://esi.evetech.net/killmails/132171502/f92138513e5e6f1ff78151b810a7688ae577155f/".to_string(),
            ..Default::default()
        };

        assert_eq!(zkb.killmail_id(), Some(132171502));
    }
}

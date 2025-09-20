/*
 * Copyright (C) 2025 Raven X Does Things
 */

#[derive(Debug, serde::Deserialize)]
pub struct Filters {
    pub include_npc: bool,
    pub characters: Vec<u64>,
    pub corps: Vec<u64>,
    pub alliances: Vec<u64>,
}

#[derive(Debug, serde::Deserialize)]
pub struct Response {
    #[serde(rename = "package")]
    pub killmail: Option<Killmail>,
}

#[derive(Debug, serde::Deserialize)]
pub struct Killmail {
    #[serde(rename = "killID")]
    pub kill_id: u64,
    pub killmail: KillmailData,
    #[serde(skip)]
    pub ours: bool,
}

impl Killmail {
    pub fn filter(&mut self, filters: &Filters) -> bool {
        self.ours = false;
        if self.killmail.victim.filter(filters) {
            return true;
        }
        for attacker in &self.killmail.attackers {
            if attacker.filter(filters) {
                self.ours = true;
                return true;
            }
        }
        false
    }

    pub fn skew(&self) -> chrono::Duration {
        let now = chrono::Utc::now();
        now.signed_duration_since(self.killmail.timestamp)
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct KillmailData {
    #[serde(rename = "killmail_time")]
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub attackers: Vec<Participant>,
    pub victim: Participant,
}

#[derive(Debug, serde::Deserialize)]
pub struct Participant {
    pub character_id: Option<u64>,
    pub corporation_id: Option<u64>,
    pub alliance_id: Option<u64>,
}

impl Participant {
    pub fn is_npc(&self) -> bool {
        self.character_id.is_none()
    }

    fn filter(&self, filters: &Filters) -> bool {
        if !filters.include_npc && self.is_npc() {
            return false;
        }
        if let Some(char_id) = self.character_id
            && filters.characters.contains(&char_id)
        {
            return true;
        }
        if let Some(corp_id) = self.corporation_id
            && filters.corps.contains(&corp_id)
        {
            return true;
        }
        if let Some(alliance_id) = self.alliance_id
            && filters.alliances.contains(&alliance_id)
        {
            return true;
        }
        false
    }
}

/*
 * Copyright (C) 2025 Raven X Does Things
 */

#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct Filter {
    #[serde(default = "Vec::new")]
    pub includes: Vec<u64>,
    #[serde(default = "Vec::new")]
    pub excludes: Vec<u64>,
}

impl Filter {
    pub fn is_empty(&self) -> bool {
        self.includes.is_empty() && self.excludes.is_empty()
    }

    pub fn includes(&self, id: &u64) -> bool {
        *id > 0 && self.includes.contains(id)
    }

    pub fn excludes(&self, id: &u64) -> bool {
        *id > 0 && self.excludes.contains(id)
    }
}

#[derive(Debug, serde::Deserialize, Clone)]
pub struct ChannelConfig {
    #[serde(default = "Vec::new")]
    pub channel_ids: Vec<i64>,
    pub filters: Filters,
}

#[derive(Debug, serde::Deserialize, Clone)]
pub struct Filters {
    pub include_npc: bool,
    pub characters: Option<Filter>,
    pub corps: Option<Filter>,
    pub alliances: Option<Filter>,
}

impl Filters {
    pub fn is_empty(&self) -> bool {
        self.characters.as_ref().is_none_or(|f| f.is_empty())
            && self.corps.as_ref().is_none_or(|f| f.is_empty())
            && self.alliances.as_ref().is_none_or(|f| f.is_empty())
    }
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
}

impl Killmail {
    pub fn filter(&self, filters: &Vec<ChannelConfig>) -> Vec<(i64, bool)> {
        let mut result = vec![];

        for config in filters {
            if config.filters.is_empty() {
                config.channel_ids.iter().for_each(|id| {
                    result.push((*id, false));
                });
                continue;
            }

            if self.killmail.victim.filter(&config.filters) {
                config.channel_ids.iter().for_each(|id| {
                    result.push((*id, false));
                });
                continue;
            }
            for attacker in &self.killmail.attackers {
                if attacker.filter(&config.filters) {
                    config.channel_ids.iter().for_each(|id| {
                        result.push((*id, true));
                    });
                    break;
                }
            }
        }

        result
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

        let character_id = self.character_id.unwrap_or(0);
        let corp_id = self.corporation_id.unwrap_or(0);
        let alliance_id = self.alliance_id.unwrap_or(0);

        if character_id == 0 && corp_id == 0 && alliance_id == 0 {
            return false;
        }

        let filter_default = Filter::default();
        let character_filters = filters.characters.as_ref().unwrap_or(&filter_default);
        let corp_filters = filters.corps.as_ref().unwrap_or(&filter_default);
        let alliance_filters = filters.alliances.as_ref().unwrap_or(&filter_default);

        if character_filters.excludes(&character_id) {
            return false;
        }

        if corp_filters.excludes(&corp_id) {
            return false;
        }

        if alliance_filters.excludes(&alliance_id) {
            return false;
        }

        let character_filtered =
            character_filters.includes(&character_id) || (self.is_npc() && filters.include_npc);

        if character_filtered
            || corp_filters.includes(&corp_id)
            || alliance_filters.includes(&alliance_id)
        {
            return true;
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_npc_filter() {
        let filters = Filters {
            include_npc: false,
            characters: Some(Filter {
                includes: vec![1],
                excludes: vec![],
            }),
            corps: None,
            alliances: None,
        };

        let participant = Participant {
            character_id: None,
            corporation_id: Some(10),
            alliance_id: Some(100),
        };
        assert!(!participant.filter(&filters), "NPC should be filtered out");

        let participant = Participant {
            character_id: Some(1),
            corporation_id: Some(10),
            alliance_id: Some(100),
        };
        assert!(participant.filter(&filters), "Non-NPC should pass");

        let filters = Filters {
            include_npc: true,
            characters: None,
            corps: None,
            alliances: None,
        };

        let participant = Participant {
            character_id: None,
            corporation_id: Some(10),
            alliance_id: Some(100),
        };
        assert!(
            participant.filter(&filters),
            "NPC should pass when include_npc is true"
        );
    }

    #[tokio::test]
    async fn test_includes() {
        let filters = Filters {
            include_npc: false,
            characters: Some(Filter {
                includes: vec![1, 2, 3],
                excludes: vec![],
            }),
            corps: Some(Filter {
                includes: vec![10, 20, 30],
                excludes: vec![],
            }),
            alliances: Some(Filter {
                includes: vec![100, 200, 300],
                excludes: vec![],
            }),
        };

        let participant = Participant {
            character_id: Some(1),
            corporation_id: None,
            alliance_id: None,
        };
        assert!(participant.filter(&filters));

        let participant = Participant {
            character_id: Some(999),
            corporation_id: Some(20),
            alliance_id: None,
        };
        assert!(participant.filter(&filters));

        let participant = Participant {
            character_id: Some(999),
            corporation_id: None,
            alliance_id: Some(300),
        };
        assert!(participant.filter(&filters));
    }

    #[tokio::test]
    async fn test_excludes() {
        let filters = Filters {
            include_npc: false,
            characters: Some(Filter {
                includes: vec![],
                excludes: vec![1, 2, 3],
            }),
            corps: Some(Filter {
                includes: vec![],
                excludes: vec![10, 20, 30],
            }),
            alliances: Some(Filter {
                includes: vec![],
                excludes: vec![100, 200, 300],
            }),
        };

        let participant = Participant {
            character_id: Some(1),
            corporation_id: None,
            alliance_id: None,
        };
        assert!(!participant.filter(&filters));

        let participant = Participant {
            character_id: Some(999),
            corporation_id: Some(20),
            alliance_id: None,
        };
        assert!(!participant.filter(&filters));

        let participant = Participant {
            character_id: Some(999),
            corporation_id: None,
            alliance_id: Some(300),
        };
        assert!(!participant.filter(&filters));
    }

    #[tokio::test]
    async fn test_priority() {
        let filters = Filters {
            include_npc: false,
            characters: Some(Filter {
                includes: vec![1],
                excludes: vec![2],
            }),
            corps: Some(Filter {
                includes: vec![10],
                excludes: vec![20],
            }),
            alliances: Some(Filter {
                includes: vec![100],
                excludes: vec![200],
            }),
        };

        let participant = Participant {
            character_id: Some(2),
            corporation_id: Some(10),
            alliance_id: None,
        };
        assert!(!participant.filter(&filters));

        let participant = Participant {
            character_id: Some(1),
            corporation_id: Some(20),
            alliance_id: Some(100),
        };
        assert!(!participant.filter(&filters));

        let participant = Participant {
            character_id: Some(1),
            corporation_id: Some(10),
            alliance_id: Some(200),
        };
        assert!(!participant.filter(&filters));
    }

    #[tokio::test]
    async fn test_killmail_multi_filter() {
        let killmail = Killmail {
            kill_id: 12345,
            killmail: KillmailData {
                timestamp: chrono::Utc::now(),
                attackers: vec![
                    Participant {
                        character_id: Some(1),
                        corporation_id: Some(10),
                        alliance_id: None,
                    },
                    Participant {
                        character_id: Some(2),
                        corporation_id: Some(10),
                        alliance_id: None,
                    },
                ],
                victim: Participant {
                    character_id: Some(3),
                    corporation_id: Some(30),
                    alliance_id: None,
                },
            },
        };

        let filters = vec![
            ChannelConfig {
                channel_ids: vec![1, 3],
                filters: Filters {
                    include_npc: false,
                    characters: None,
                    corps: Some(Filter {
                        includes: vec![10],
                        excludes: vec![],
                    }),
                    alliances: None,
                },
            },
            ChannelConfig {
                channel_ids: vec![2],
                filters: Filters {
                    include_npc: false,
                    characters: None,
                    corps: Some(Filter {
                        includes: vec![30],
                        excludes: vec![],
                    }),
                    alliances: None,
                },
            },
        ];

        let result = killmail.filter(&filters);
        let expected = vec![(1, true), (3, true), (2, false)];
        assert_eq!(result, expected);
    }
}

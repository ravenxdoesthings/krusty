/*
 * Copyright (C) 2025 Raven X Does Things
 */

use crate::static_data;

#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct Filter {
    #[serde(default = "Vec::new")]
    pub includes: Vec<u64>,
    #[serde(default = "Vec::new")]
    pub excludes: Vec<u64>,
}

type IncludeFilter = Vec<u64>;

#[derive(Debug, Clone, serde::Deserialize)]
pub struct ShipFilter {
    pub ship_type_ids: Vec<u64>,
    pub losses_only: bool,
}

impl ShipFilter {
    pub fn is_empty(&self) -> bool {
        self.ship_type_ids.is_empty()
    }
}

#[derive(Debug, PartialEq)]
enum MatchKind {
    Character,
    Corporation,
    Alliance,
    Ship,
}

#[derive(Debug, PartialEq)]
pub enum KillmailKind {
    Green,
    Red,
    Neutral,
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
    pub regions: Option<IncludeFilter>,
    pub systems: Option<IncludeFilter>,
    pub ships: Option<ShipFilter>,
}

impl Filters {
    pub fn is_empty(&self) -> bool {
        self.characters.as_ref().is_none_or(|f| f.is_empty())
            && self.corps.as_ref().is_none_or(|f| f.is_empty())
            && self.alliances.as_ref().is_none_or(|f| f.is_empty())
            && self.regions.as_ref().is_none_or(|f| f.is_empty())
            && self.systems.as_ref().is_none_or(|f| f.is_empty())
            && self.ships.as_ref().is_none_or(|f| f.is_empty())
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
    pub fn filter(&self, filters: &Vec<ChannelConfig>) -> Vec<(i64, KillmailKind)> {
        let mut result = vec![];

        for config in filters {
            if config.filters.is_empty() {
                config.channel_ids.iter().for_each(|id| {
                    result.push((*id, KillmailKind::Neutral));
                });
                continue;
            }

            match self.killmail.victim.filter(&config.filters) {
                Some(MatchKind::Ship) => {
                    config.channel_ids.iter().for_each(|id| {
                        result.push((*id, KillmailKind::Neutral));
                    });
                    continue;
                }
                Some(_) => {
                    config.channel_ids.iter().for_each(|id| {
                        result.push((*id, KillmailKind::Red));
                    });
                    continue;
                }
                None => {}
            }
            for attacker in &self.killmail.attackers {
                match attacker.filter(&config.filters) {
                    Some(MatchKind::Ship) => {
                        if let Some(ship_filters) = &config.filters.ships
                            && !ship_filters.losses_only
                        {
                            config.channel_ids.iter().for_each(|id| {
                                result.push((*id, KillmailKind::Neutral));
                            });
                            break;
                        }
                    }
                    Some(_) => {
                        config.channel_ids.iter().for_each(|id| {
                            result.push((*id, KillmailKind::Green));
                        });
                        break;
                    }
                    None => {}
                }
            }

            if config.filters.regions.is_some()
                && let Some(region_id) =
                    static_data::get_region_by_system_id(self.killmail.system_id)
                && config
                    .filters
                    .regions
                    .as_ref()
                    .unwrap()
                    .contains(&region_id)
            {
                config.channel_ids.iter().for_each(|id| {
                    result.push((*id, KillmailKind::Neutral));
                });
            }

            if config.filters.systems.is_some()
                && config
                    .filters
                    .systems
                    .as_ref()
                    .unwrap()
                    .contains(&self.killmail.system_id)
            {
                config.channel_ids.iter().for_each(|id| {
                    result.push((*id, KillmailKind::Neutral));
                });
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

    fn filter(&self, filters: &Filters) -> Option<MatchKind> {
        if !filters.include_npc && self.is_npc() {
            return None;
        }

        let character_id = self.character_id.unwrap_or(0);
        let corp_id = self.corporation_id.unwrap_or(0);
        let alliance_id = self.alliance_id.unwrap_or(0);

        if character_id == 0 && corp_id == 0 && alliance_id == 0 {
            return None;
        }

        let filter_default = Filter::default();
        let character_filters = filters.characters.as_ref().unwrap_or(&filter_default);
        let corp_filters = filters.corps.as_ref().unwrap_or(&filter_default);
        let alliance_filters = filters.alliances.as_ref().unwrap_or(&filter_default);

        if character_filters.excludes(&character_id) {
            return None;
        }

        if corp_filters.excludes(&corp_id) {
            return None;
        }

        if alliance_filters.excludes(&alliance_id) {
            return None;
        }

        if character_filters.includes(&character_id) || (self.is_npc() && filters.include_npc) {
            return Some(MatchKind::Character);
        }

        if corp_filters.includes(&corp_id) {
            return Some(MatchKind::Corporation);
        }

        if alliance_filters.includes(&alliance_id) {
            return Some(MatchKind::Alliance);
        }

        if let Some(ship_filters) = &filters.ships
            && let Some(ship_id) = self.ship_type_id
            && ship_filters.ship_type_ids.contains(&ship_id)
        {
            return Some(MatchKind::Ship);
        }

        None
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
            regions: None,
            systems: None,
            ships: None,
        };

        let participant = Participant {
            character_id: None,
            corporation_id: Some(10),
            alliance_id: Some(100),
            ship_type_id: None,
        };
        assert_eq!(
            participant.filter(&filters),
            None,
            "NPC should be filtered out"
        );

        let participant = Participant {
            character_id: Some(1),
            corporation_id: Some(10),
            alliance_id: Some(100),
            ship_type_id: None,
        };
        assert_eq!(
            participant.filter(&filters),
            Some(MatchKind::Character),
            "Non-NPC should pass"
        );

        let filters = Filters {
            include_npc: true,
            characters: None,
            corps: None,
            alliances: None,
            regions: None,
            systems: None,
            ships: None,
        };

        let participant = Participant {
            character_id: None,
            corporation_id: Some(10),
            alliance_id: Some(100),
            ship_type_id: None,
        };
        assert_eq!(
            participant.filter(&filters),
            Some(MatchKind::Character),
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
            regions: None,
            systems: None,
            ships: None,
        };

        let participant = Participant {
            character_id: Some(1),
            corporation_id: None,
            alliance_id: None,
            ship_type_id: None,
        };
        assert_eq!(participant.filter(&filters), Some(MatchKind::Character));

        let participant = Participant {
            character_id: Some(999),
            corporation_id: Some(20),
            alliance_id: None,
            ship_type_id: None,
        };
        assert_eq!(participant.filter(&filters), Some(MatchKind::Corporation));

        let participant = Participant {
            character_id: Some(999),
            corporation_id: None,
            alliance_id: Some(300),
            ship_type_id: None,
        };
        assert_eq!(participant.filter(&filters), Some(MatchKind::Alliance));
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
            regions: None,
            systems: None,
            ships: None,
        };

        let participant = Participant {
            character_id: Some(1),
            corporation_id: None,
            alliance_id: None,
            ship_type_id: None,
        };
        assert_eq!(participant.filter(&filters), None);

        let participant = Participant {
            character_id: Some(999),
            corporation_id: Some(20),
            alliance_id: None,
            ship_type_id: None,
        };
        assert_eq!(participant.filter(&filters), None);

        let participant = Participant {
            character_id: Some(999),
            corporation_id: None,
            alliance_id: Some(300),
            ship_type_id: None,
        };
        assert_eq!(participant.filter(&filters), None);
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
            regions: None,
            systems: None,
            ships: None,
        };

        let participant = Participant {
            character_id: Some(2),
            corporation_id: Some(10),
            alliance_id: None,
            ship_type_id: None,
        };
        assert_eq!(participant.filter(&filters), None);

        let participant = Participant {
            character_id: Some(1),
            corporation_id: Some(20),
            alliance_id: Some(100),
            ship_type_id: None,
        };
        assert_eq!(participant.filter(&filters), None);

        let participant = Participant {
            character_id: Some(1),
            corporation_id: Some(10),
            alliance_id: Some(200),
            ship_type_id: None,
        };
        assert_eq!(participant.filter(&filters), None);
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
                        ship_type_id: None,
                    },
                    Participant {
                        character_id: Some(2),
                        corporation_id: Some(10),
                        alliance_id: None,
                        ship_type_id: None,
                    },
                ],
                victim: Participant {
                    character_id: Some(3),
                    corporation_id: Some(30),
                    alliance_id: None,
                    ship_type_id: None,
                },
                system_id: 30000142,
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
                    regions: None,
                    systems: None,
                    ships: None,
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
                    regions: None,
                    systems: None,
                    ships: None,
                },
            },
        ];

        let result = killmail.filter(&filters);
        let expected = vec![
            (1, KillmailKind::Green),
            (3, KillmailKind::Green),
            (2, KillmailKind::Red),
        ];
        assert_eq!(result, expected);
    }

    #[tokio::test]
    async fn test_killmail_region_filter() {
        let killmail = Killmail {
            kill_id: 12345,
            killmail: KillmailData {
                timestamp: chrono::Utc::now(),
                attackers: vec![],
                victim: Participant {
                    character_id: Some(3),
                    corporation_id: Some(30),
                    alliance_id: None,
                    ship_type_id: None,
                },
                system_id: 30000142, // This system is in region 10000002
            },
        };

        let filters = vec![ChannelConfig {
            channel_ids: vec![1],
            filters: Filters {
                include_npc: false,
                characters: None,
                corps: None,
                alliances: None,
                regions: Some(vec![10000002]),
                systems: None,
                ships: None,
            },
        }];

        let result = killmail.filter(&filters);
        let expected = vec![(1, KillmailKind::Neutral)];
        assert_eq!(result, expected);
    }

    #[tokio::test]
    async fn test_killmail_system_filter() {
        let killmail = Killmail {
            kill_id: 12345,
            killmail: KillmailData {
                timestamp: chrono::Utc::now(),
                attackers: vec![],
                victim: Participant {
                    character_id: Some(3),
                    corporation_id: Some(30),
                    alliance_id: None,
                    ship_type_id: None,
                },
                system_id: 30000142, // This system is in region 10000002
            },
        };

        let filters = vec![ChannelConfig {
            channel_ids: vec![1],
            filters: Filters {
                include_npc: false,
                characters: None,
                corps: None,
                alliances: None,
                regions: None,
                systems: Some(vec![30000142]),
                ships: None,
            },
        }];

        let result = killmail.filter(&filters);
        let expected = vec![(1, KillmailKind::Neutral)];
        assert_eq!(result, expected);
    }

    #[tokio::test]
    async fn test_killmail_ship_filter_victim() {
        let killmail = Killmail {
            kill_id: 12345,
            killmail: KillmailData {
                timestamp: chrono::Utc::now(),
                attackers: vec![],
                victim: Participant {
                    character_id: Some(3),
                    corporation_id: Some(30),
                    alliance_id: None,
                    ship_type_id: Some(4000),
                },
                system_id: 30000142,
            },
        };

        let filters = vec![ChannelConfig {
            channel_ids: vec![1],
            filters: Filters {
                include_npc: false,
                characters: None,
                corps: None,
                alliances: None,
                regions: None,
                systems: None,
                ships: Some(ShipFilter {
                    ship_type_ids: vec![4000, 5000],
                    losses_only: false,
                }),
            },
        }];

        let result = killmail.filter(&filters);
        let expected = vec![(1, KillmailKind::Neutral)];
        assert_eq!(result, expected);
    }

    #[tokio::test]
    async fn test_killmail_ship_filter_only_losses() {
        let killmail = Killmail {
            kill_id: 12345,
            killmail: KillmailData {
                timestamp: chrono::Utc::now(),
                attackers: vec![Participant {
                    character_id: Some(3),
                    corporation_id: Some(30),
                    alliance_id: None,
                    ship_type_id: Some(4000),
                }],
                victim: Participant {
                    character_id: Some(3),
                    corporation_id: Some(30),
                    alliance_id: None,
                    ship_type_id: Some(3000),
                },
                system_id: 30000142,
            },
        };

        let filters = vec![ChannelConfig {
            channel_ids: vec![1],
            filters: Filters {
                include_npc: false,
                characters: None,
                corps: None,
                alliances: None,
                regions: None,
                systems: None,
                ships: Some(ShipFilter {
                    ship_type_ids: vec![4000, 5000],
                    losses_only: true,
                }),
            },
        }];

        let result = killmail.filter(&filters);
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_killmail_ship_filter_all() {
        let killmail = Killmail {
            kill_id: 12345,
            killmail: KillmailData {
                timestamp: chrono::Utc::now(),
                attackers: vec![Participant {
                    character_id: Some(3),
                    corporation_id: Some(30),
                    alliance_id: None,
                    ship_type_id: Some(4000),
                }],
                victim: Participant {
                    character_id: Some(3),
                    corporation_id: Some(30),
                    alliance_id: None,
                    ship_type_id: Some(3000),
                },
                system_id: 30000142,
            },
        };

        let filters = vec![ChannelConfig {
            channel_ids: vec![1],
            filters: Filters {
                include_npc: false,
                characters: None,
                corps: None,
                alliances: None,
                regions: None,
                systems: None,
                ships: Some(ShipFilter {
                    ship_type_ids: vec![4000, 5000],
                    losses_only: false,
                }),
            },
        }];

        let result = killmail.filter(&filters);
        let expected = vec![(1, KillmailKind::Neutral)];
        assert_eq!(result, expected);
    }
}

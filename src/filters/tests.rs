mod parser_tests {
    use crate::filters::*;

    #[test]
    fn test_simple_filter() {
        let filter_str = String::from("region:10000002");
        let filter: Filter = filter_str.into();
        assert!(matches!(filter.kind, FilterKind::Region));
        assert_eq!(filter.ids[0], 10000002);
        assert_eq!(filter.properties.len(), 0);
    }

    #[test]
    fn test_filter_with_id_list() {
        let filter_str = String::from("ship:12747,33475,670");
        let filter: Filter = filter_str.into();
        assert!(matches!(filter.kind, FilterKind::Ship));
        assert_eq!(filter.ids.len(), 3);
        assert!(filter.ids.contains(&12747));
        assert!(filter.ids.contains(&33475));
        assert!(filter.ids.contains(&670));
    }

    #[test]
    fn test_filter_with_properties() {
        let filter_str = String::from("ship:12747,33475,670:loss,exclude");
        let filter: Filter = filter_str.into();
        assert!(matches!(filter.kind, FilterKind::Ship));
        assert_eq!(filter.ids.len(), 3);
        assert!(filter.ids.contains(&12747));
        assert!(filter.ids.contains(&33475));
        assert!(filter.ids.contains(&670));
        assert_eq!(filter.properties.len(), 2);
        assert!(filter.properties.contains(&FilterProperty::Losses));
        assert!(filter.properties.contains(&FilterProperty::Exclude));
    }
}

#[cfg(test)]
mod region_tests {
    use crate::filters::*;
    use crate::zkb::KillmailData;

    #[test]
    fn test_region_filter_include() {
        let filter_str = String::from("region:10000002");
        let filter: Filter = filter_str.into();

        let killmail = KillmailData {
            system_id: 30000142, // system in region 10000002
            ..Default::default()
        };

        let result = filter.filter(&killmail);
        assert!(matches!(result, FilterResult::Include(None)));
    }

    #[test]
    fn test_region_filter_exclude() {
        let filter_str = String::from("region:10000002:exclude");
        let filter: Filter = filter_str.into();

        let killmail = KillmailData {
            system_id: 30000142, // system in region 10000002
            ..Default::default()
        };

        let result = filter.filter(&killmail);
        assert!(matches!(result, FilterResult::Exclude));
    }

    #[test]
    fn test_region_filter_no_match() {
        let filter_str = String::from("region:10000003");
        let filter: Filter = filter_str.into();

        let killmail = KillmailData {
            system_id: 30000144, // system not in region 10000003
            ..Default::default()
        };

        let result = filter.filter(&killmail);
        assert!(matches!(result, FilterResult::NoMatch));
    }
}

#[cfg(test)]
mod system_tests {
    use crate::filters::*;
    use crate::zkb::KillmailData;

    #[test]
    fn test_system_filter_include() {
        let filter_str = String::from("system:30000142");
        let filter: Filter = filter_str.into();

        let killmail = KillmailData {
            system_id: 30000142, // system in region 10000002
            ..Default::default()
        };

        let result = filter.filter(&killmail);
        assert!(matches!(result, FilterResult::Include(None)));
    }

    #[test]
    fn test_system_filter_exclude() {
        let filter_str = String::from("system:30000142:exclude");
        let filter: Filter = filter_str.into();

        let killmail = KillmailData {
            system_id: 30000142, // system in region 10000002
            ..Default::default()
        };

        let result = filter.filter(&killmail);
        assert!(matches!(result, FilterResult::Exclude));
    }

    #[test]
    fn test_system_filter_no_match() {
        let filter_str = String::from("system:30000142");
        let filter: Filter = filter_str.into();

        let killmail = KillmailData {
            system_id: 30000144, // system not in region 10000003
            ..Default::default()
        };

        let result = filter.filter(&killmail);
        assert!(matches!(result, FilterResult::NoMatch));
    }
}

#[cfg(test)]
mod character_tests {
    use crate::filters::*;
    use crate::zkb::KillmailData;

    #[test]
    fn test_character_filter_include_attacker() {
        let filter_str = String::from("character:1");
        let filter: Filter = filter_str.into();

        let killmail = KillmailData {
            attackers: vec![crate::zkb::Participant {
                character_id: Some(1),
                ..Default::default()
            }],
            victim: crate::zkb::Participant {
                character_id: Some(2),
                ..Default::default()
            },
            ..Default::default()
        };

        let result = filter.filter(&killmail);
        assert!(matches!(
            result,
            FilterResult::Include(Some(KillmailSide::Attackers))
        ));
    }

    #[test]
    fn test_character_filter_exclude_attacker() {
        let filter_str = String::from("character:1:exclude");
        let filter: Filter = filter_str.into();

        let killmail = KillmailData {
            attackers: vec![crate::zkb::Participant {
                character_id: Some(1),
                ..Default::default()
            }],
            victim: crate::zkb::Participant {
                character_id: Some(2),
                ..Default::default()
            },
            ..Default::default()
        };

        let result = filter.filter(&killmail);
        assert!(matches!(result, FilterResult::Exclude));
    }

    #[test]
    fn test_character_filter_include_victim() {
        let filter_str = String::from("character:2");
        let filter: Filter = filter_str.into();

        let killmail = KillmailData {
            victim: crate::zkb::Participant {
                character_id: Some(2),
                ..Default::default()
            },
            ..Default::default()
        };

        let result = filter.filter(&killmail);
        assert!(matches!(
            result,
            FilterResult::Include(Some(KillmailSide::Victim))
        ));
    }

    #[test]
    fn test_character_filter_exclude_victim() {
        let filter_str = String::from("character:2:exclude");
        let filter: Filter = filter_str.into();

        let killmail = KillmailData {
            victim: crate::zkb::Participant {
                character_id: Some(2),
                ..Default::default()
            },
            ..Default::default()
        };

        let result = filter.filter(&killmail);
        assert!(matches!(result, FilterResult::Exclude));
    }

    #[test]
    fn test_character_filter_exclude_victim_kills_only() {
        let filter_str = String::from("character:2:exclude,kills");
        let filter: Filter = filter_str.into();

        let killmail = KillmailData {
            victim: crate::zkb::Participant {
                character_id: Some(2),
                ..Default::default()
            },
            ..Default::default()
        };

        let result = filter.filter(&killmail);
        println!("{:?}", result);
        assert!(matches!(result, FilterResult::NoMatch));
    }

    #[test]
    fn test_character_filter_exclude_victim_losses_only() {
        let filter_str = String::from("character:2:exclude,losses");
        let filter: Filter = filter_str.into();

        let killmail = KillmailData {
            victim: crate::zkb::Participant {
                character_id: Some(2),
                ..Default::default()
            },
            ..Default::default()
        };

        let result = filter.filter(&killmail);
        assert!(matches!(result, FilterResult::Exclude));
    }

    #[test]
    fn test_character_filter_no_match() {
        let filter_str = String::from("character:2");
        let filter: Filter = filter_str.into();

        let killmail = KillmailData {
            victim: crate::zkb::Participant {
                character_id: None,
                ..Default::default()
            },
            ..Default::default()
        };

        let result = filter.filter(&killmail);
        assert!(matches!(result, FilterResult::NoMatch));
    }
}

#[cfg(test)]
mod corp_tests {
    use crate::filters::*;
    use crate::zkb::KillmailData;

    #[test]
    fn test_corp_filter_include_attacker() {
        let filter_str = String::from("corp:1");
        let filter: Filter = filter_str.into();

        let killmail = KillmailData {
            attackers: vec![crate::zkb::Participant {
                corporation_id: Some(1),
                ..Default::default()
            }],
            victim: crate::zkb::Participant {
                corporation_id: Some(2),
                ..Default::default()
            },
            ..Default::default()
        };

        let result = filter.filter(&killmail);
        assert!(matches!(
            result,
            FilterResult::Include(Some(KillmailSide::Attackers))
        ));
    }

    #[test]
    fn test_corp_filter_exclude_attacker() {
        let filter_str = String::from("corp:1:exclude");
        let filter: Filter = filter_str.into();

        let killmail = KillmailData {
            attackers: vec![crate::zkb::Participant {
                corporation_id: Some(1),
                ..Default::default()
            }],
            victim: crate::zkb::Participant {
                corporation_id: Some(2),
                ..Default::default()
            },
            ..Default::default()
        };

        let result = filter.filter(&killmail);
        assert!(matches!(result, FilterResult::Exclude));
    }

    #[test]
    fn test_corp_filter_include_victim() {
        let filter_str = String::from("corp:2");
        let filter: Filter = filter_str.into();

        let killmail = KillmailData {
            victim: crate::zkb::Participant {
                corporation_id: Some(2),
                ..Default::default()
            },
            ..Default::default()
        };

        let result = filter.filter(&killmail);
        assert!(matches!(
            result,
            FilterResult::Include(Some(KillmailSide::Victim))
        ));
    }

    #[test]
    fn test_corp_filter_exclude_victim() {
        let filter_str = String::from("corp:2:exclude");
        let filter: Filter = filter_str.into();

        let killmail = KillmailData {
            victim: crate::zkb::Participant {
                corporation_id: Some(2),
                ..Default::default()
            },
            ..Default::default()
        };

        let result = filter.filter(&killmail);
        assert!(matches!(result, FilterResult::Exclude));
    }

    #[test]
    fn test_corp_filter_exclude_victim_kills_only() {
        let filter_str = String::from("corp:2:exclude,kills");
        let filter: Filter = filter_str.into();

        let killmail = KillmailData {
            victim: crate::zkb::Participant {
                corporation_id: Some(2),
                ..Default::default()
            },
            ..Default::default()
        };

        let result = filter.filter(&killmail);
        println!("{:?}", result);
        assert!(matches!(result, FilterResult::NoMatch));
    }

    #[test]
    fn test_corp_filter_exclude_victim_losses_only() {
        let filter_str = String::from("corp:2:exclude,losses");
        let filter: Filter = filter_str.into();

        let killmail = KillmailData {
            victim: crate::zkb::Participant {
                corporation_id: Some(2),
                ..Default::default()
            },
            ..Default::default()
        };

        let result = filter.filter(&killmail);
        assert!(matches!(result, FilterResult::Exclude));
    }

    #[test]
    fn test_corp_filter_no_match() {
        let filter_str = String::from("corp:2");
        let filter: Filter = filter_str.into();

        let killmail = KillmailData {
            victim: crate::zkb::Participant {
                corporation_id: None,
                ..Default::default()
            },
            ..Default::default()
        };

        let result = filter.filter(&killmail);
        assert!(matches!(result, FilterResult::NoMatch));
    }
}

#[cfg(test)]
mod alliance_tests {
    use crate::filters::*;
    use crate::zkb::KillmailData;

    #[test]
    fn test_alliance_filter_include_attacker() {
        let filter_str = String::from("alliance:1");
        let filter: Filter = filter_str.into();

        let killmail = KillmailData {
            attackers: vec![crate::zkb::Participant {
                alliance_id: Some(1),
                ..Default::default()
            }],
            victim: crate::zkb::Participant {
                alliance_id: Some(2),
                ..Default::default()
            },
            ..Default::default()
        };

        let result = filter.filter(&killmail);
        assert!(matches!(
            result,
            FilterResult::Include(Some(KillmailSide::Attackers))
        ));
    }

    #[test]
    fn test_alliance_filter_exclude_attacker() {
        let filter_str = String::from("alliance:1:exclude");
        let filter: Filter = filter_str.into();

        let killmail = KillmailData {
            attackers: vec![crate::zkb::Participant {
                alliance_id: Some(1),
                ..Default::default()
            }],
            victim: crate::zkb::Participant {
                alliance_id: Some(2),
                ..Default::default()
            },
            ..Default::default()
        };

        let result = filter.filter(&killmail);
        assert!(matches!(result, FilterResult::Exclude));
    }

    #[test]
    fn test_alliance_filter_include_victim() {
        let filter_str = String::from("alliance:2");
        let filter: Filter = filter_str.into();

        let killmail = KillmailData {
            victim: crate::zkb::Participant {
                alliance_id: Some(2),
                ..Default::default()
            },
            ..Default::default()
        };

        let result = filter.filter(&killmail);
        assert!(matches!(
            result,
            FilterResult::Include(Some(KillmailSide::Victim))
        ));
    }

    #[test]
    fn test_alliance_filter_exclude_victim() {
        let filter_str = String::from("alliance:2:exclude");
        let filter: Filter = filter_str.into();

        let killmail = KillmailData {
            victim: crate::zkb::Participant {
                alliance_id: Some(2),
                ..Default::default()
            },
            ..Default::default()
        };

        let result = filter.filter(&killmail);
        assert!(matches!(result, FilterResult::Exclude));
    }

    #[test]
    fn test_alliance_filter_exclude_victim_kills_only() {
        let filter_str = String::from("alliance:2:exclude,kills");
        let filter: Filter = filter_str.into();

        let killmail = KillmailData {
            victim: crate::zkb::Participant {
                alliance_id: Some(2),
                ..Default::default()
            },
            ..Default::default()
        };

        let result = filter.filter(&killmail);
        println!("{:?}", result);
        assert!(matches!(result, FilterResult::NoMatch));
    }

    #[test]
    fn test_alliance_filter_exclude_victim_losses_only() {
        let filter_str = String::from("alliance:2:exclude,losses");
        let filter: Filter = filter_str.into();

        let killmail = KillmailData {
            victim: crate::zkb::Participant {
                alliance_id: Some(2),
                ..Default::default()
            },
            ..Default::default()
        };

        let result = filter.filter(&killmail);
        assert!(matches!(result, FilterResult::Exclude));
    }

    #[test]
    fn test_alliance_filter_no_match() {
        let filter_str = String::from("alliance:2");
        let filter: Filter = filter_str.into();

        let killmail = KillmailData {
            victim: crate::zkb::Participant {
                alliance_id: None,
                ..Default::default()
            },
            ..Default::default()
        };

        let result = filter.filter(&killmail);
        assert!(matches!(result, FilterResult::NoMatch));
    }
}

#[cfg(test)]
mod ship_tests {
    use crate::filters::*;
    use crate::zkb::KillmailData;

    #[test]
    fn test_ship_filter_include_attacker_side_none() {
        let filter_str = String::from("ship:1");
        let filter: Filter = filter_str.into();

        let killmail = KillmailData {
            attackers: vec![crate::zkb::Participant {
                ship_type_id: Some(1),
                ..Default::default()
            }],
            victim: crate::zkb::Participant {
                ship_type_id: Some(2),
                ..Default::default()
            },
            ..Default::default()
        };

        let result = filter.filter(&killmail);
        assert!(matches!(result, FilterResult::Include(None)));
    }

    #[test]
    fn test_ship_filter_exclude_attacker() {
        let filter_str = String::from("ship:1:exclude");
        let filter: Filter = filter_str.into();

        let killmail = KillmailData {
            attackers: vec![crate::zkb::Participant {
                ship_type_id: Some(1),
                ..Default::default()
            }],
            victim: crate::zkb::Participant {
                ship_type_id: Some(2),
                ..Default::default()
            },
            ..Default::default()
        };

        let result = filter.filter(&killmail);
        assert!(matches!(result, FilterResult::Exclude));
    }

    #[test]
    fn test_ship_filter_include_victim_side_none() {
        let filter_str = String::from("ship:2");
        let filter: Filter = filter_str.into();

        let killmail = KillmailData {
            victim: crate::zkb::Participant {
                ship_type_id: Some(2),
                ..Default::default()
            },
            ..Default::default()
        };

        let result = filter.filter(&killmail);
        assert!(matches!(result, FilterResult::Include(None)));
    }

    #[test]
    fn test_ship_filter_exclude_victim() {
        let filter_str = String::from("ship:2:exclude");
        let filter: Filter = filter_str.into();

        let killmail = KillmailData {
            victim: crate::zkb::Participant {
                ship_type_id: Some(2),
                ..Default::default()
            },
            ..Default::default()
        };

        let result = filter.filter(&killmail);
        assert!(matches!(result, FilterResult::Exclude));
    }

    #[test]
    fn test_ship_filter_exclude_victim_kills_only() {
        let filter_str = String::from("ship:2:exclude,kills");
        let filter: Filter = filter_str.into();

        let killmail = KillmailData {
            victim: crate::zkb::Participant {
                ship_type_id: Some(2),
                ..Default::default()
            },
            ..Default::default()
        };

        let result = filter.filter(&killmail);
        println!("{:?}", result);
        assert!(matches!(result, FilterResult::NoMatch));
    }

    #[test]
    fn test_ship_filter_exclude_victim_losses_only() {
        let filter_str = String::from("ship:2:exclude,losses");
        let filter: Filter = filter_str.into();

        let killmail = KillmailData {
            victim: crate::zkb::Participant {
                ship_type_id: Some(2),
                ..Default::default()
            },
            ..Default::default()
        };

        let result = filter.filter(&killmail);
        assert!(matches!(result, FilterResult::Exclude));
    }

    #[test]
    fn test_ship_filter_no_match() {
        let filter_str = String::from("ship:2");
        let filter: Filter = filter_str.into();

        let killmail = KillmailData {
            victim: crate::zkb::Participant {
                ship_type_id: None,
                ..Default::default()
            },
            ..Default::default()
        };

        let result = filter.filter(&killmail);
        assert!(matches!(result, FilterResult::NoMatch));
    }
}

#[cfg(test)]
mod config_tests {
    use crate::{filters::*, zkb::*};

    #[test]
    fn test_complex_exclude_priority() {
        let filter_set = FilterSet {
            channel_id: 1,
            filters: vec![
                String::from("region:10000002"),
                String::from("corp:500000"),
                String::from("alliance:400000"),
                String::from("character:600000:exclude"),
                String::from("ship:12747"),
            ],
        };

        let mut config = Config {
            filter_sets: vec![filter_set],
            compiled_filters: vec![],
        };

        let killmail = Killmail {
            kill_id: 1,
            zkb: Zkb {
                href: "".to_string(),
            },
            killmail: Some(KillmailData {
                system_id: 30000142, // system in region 10000002
                attackers: vec![crate::zkb::Participant {
                    corporation_id: Some(500000),
                    alliance_id: Some(400000),
                    character_id: Some(600000),
                    ship_type_id: Some(12747),
                    ..Default::default()
                }],
                ..Default::default()
            }),
        };

        let result = config.filter(&killmail);
        assert_eq!(result, vec![]);
    }

    #[test]
    fn test_include_if_any() {
        fn create_filter_set(channel_id: u64) -> FilterSet {
            FilterSet {
                channel_id,
                filters: vec![
                    String::from("region:10000002"),
                    String::from("ship:12747"),
                    String::from("corp:600000"),
                ],
            }
        }

        let mut config = Config {
            filter_sets: vec![
                create_filter_set(1),
                create_filter_set(2),
                create_filter_set(3),
            ],
            compiled_filters: vec![],
        };

        let killmail_include = Killmail {
            kill_id: 1,
            zkb: Zkb {
                href: "".to_string(),
            },
            killmail: Some(KillmailData {
                system_id: 30000142, // system in region 10000002
                attackers: vec![crate::zkb::Participant {
                    corporation_id: Some(500000),
                    ..Default::default()
                }],
                ..Default::default()
            }),
        };

        let result_include = config.filter(&killmail_include);
        assert_eq!(result_include, vec![(1, None), (2, None), (3, None)]);
    }

    fn create_real_life_config() -> Config {
        let filter_sets = vec![
            FilterSet {
                channel_id: 10,
                filters: vec![String::from("corp:100000")],
            },
            FilterSet {
                channel_id: 20,
                filters: vec![String::from("ship:20002:losses")], // Titan losses
            },
            FilterSet {
                channel_id: 30,
                filters: vec![String::from("system:30000142")], // Jita kills
            },
            FilterSet {
                channel_id: 40,
                filters: vec![String::from("ship:670"), String::from("system:30000142")], // Pods in The Forge
            },
        ];

        Config {
            filter_sets,
            compiled_filters: vec![],
        }
    }

    #[test]
    fn test_real_life_corp_kill() {
        let mut config = create_real_life_config();

        let km = Killmail {
            kill_id: 1,
            zkb: Zkb {
                href: "".to_string(),
            },
            killmail: Some(KillmailData {
                attackers: vec![crate::zkb::Participant {
                    corporation_id: Some(100000),
                    ..Default::default()
                }],
                ..Default::default()
            }),
        };

        let result = config.filter(&km);
        assert_eq!(result, vec![(10, Some(KillmailSide::Attackers))]);
    }

    #[test]
    fn test_real_life_titan_losses() {
        let mut config = create_real_life_config();

        let km = Killmail {
            kill_id: 2,
            zkb: Zkb {
                href: "".to_string(),
            },
            killmail: Some(KillmailData {
                victim: crate::zkb::Participant {
                    ship_type_id: Some(20002),
                    ..Default::default()
                },
                ..Default::default()
            }),
        };

        let result = config.filter(&km);
        assert_eq!(result, vec![(20, None)]);
    }

    #[test]
    fn test_real_life_jita_kill() {
        let mut config = create_real_life_config();

        let km = Killmail {
            kill_id: 3,
            zkb: Zkb {
                href: "".to_string(),
            },
            killmail: Some(KillmailData {
                system_id: 30000142,
                ..Default::default()
            }),
        };

        let result = config.filter(&km);
        assert_eq!(result, vec![(30, None), (40, None)]);
    }

    #[test]
    fn test_real_life_pod_kill_in_forge() {
        let mut config = create_real_life_config();

        let km = Killmail {
            kill_id: 4,
            zkb: Zkb {
                href: "".to_string(),
            },
            killmail: Some(KillmailData {
                system_id: 30000142,
                attackers: vec![crate::zkb::Participant {
                    ship_type_id: Some(670),
                    ..Default::default()
                }],
                ..Default::default()
            }),
        };

        let result = config.filter(&km);
        assert_eq!(result, vec![(30, None), (40, None)]);
    }
}

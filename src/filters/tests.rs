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

use crate::static_data;

#[cfg(test)]
pub mod tests;

#[derive(Debug, Default, serde::Deserialize)]
pub struct Config {
    pub filter_sets: Vec<FilterSet>,

    #[serde(skip)]
    compiled_filters: Vec<CompiledFilters>,
}

#[derive(Clone, Debug)]
pub struct CompiledFilters {
    pub channel_ids: Vec<u64>,
    pub filters: Vec<Filter>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum KillmailSide {
    Victim,
    Attackers,
}

#[derive(Debug, PartialEq)]
pub enum FilterResult {
    Exclude,
    Include(Option<KillmailSide>),
    NoMatch,
}

impl Config {
    // Lazily compile filters from filter sets
    pub fn get_compiled_filters(&mut self) -> Vec<CompiledFilters> {
        if self.compiled_filters.is_empty() {
            for set in &self.filter_sets {
                let mut compiled: Vec<Filter> = vec![];
                for filter_str in &set.filters {
                    let filter: Filter = filter_str.clone().into();
                    compiled.push(filter);
                }

                self.compiled_filters.push(CompiledFilters {
                    filters: compiled,
                    channel_ids: set.channel_ids.clone(),
                });
            }
        }

        self.compiled_filters.clone()
    }

    pub fn filter(&mut self, killmail: &crate::zkb::Killmail) -> Vec<(u64, Option<KillmailSide>)> {
        let mut result = vec![];

        let killmail_data = match &killmail.killmail {
            Some(data) => data,
            None => {
                tracing::warn!(kill_id = killmail.kill_id, "killmail has no data to filter on");
                return result;
            }
        };

        let sets = self.get_compiled_filters();
        for compiled_set in sets {
            let mut include = false;
            let mut result_side: Option<KillmailSide> = None;
            for filter in &compiled_set.filters {
                match filter.filter(killmail_data) {
                    FilterResult::Exclude => {
                        include = false;
                        break;
                    }
                    FilterResult::Include(side) => {
                        include = true;
                        result_side = side;
                    }
                    FilterResult::NoMatch => continue,
                }
            }

            if include {
                for channel_id in &compiled_set.channel_ids {
                    result.push((*channel_id, result_side.clone()));
                }
            }
        }

        result
    }
}

#[derive(Debug, serde::Deserialize)]
pub struct FilterSet {
    pub channel_ids: Vec<u64>,
    pub filters: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FilterKind {
    Region,
    System,
    Ship,
    Character,
    Corporation,
    Alliance,
}

impl From<&str> for FilterKind {
    fn from(s: &str) -> Self {
        match s {
            "region" => FilterKind::Region,
            "system" => FilterKind::System,
            "ship" => FilterKind::Ship,
            "character" => FilterKind::Character,
            "corporation" | "corp" => FilterKind::Corporation,
            "alliance" => FilterKind::Alliance,
            _ => panic!("Unknown filter kind: {}", s),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum FilterProperty {
    WithNPC,
    Exclude,
    Losses,
    Kills,
    Unknown,
}

impl From<&str> for FilterProperty {
    fn from(s: &str) -> Self {
        match s {
            "with_npc" => FilterProperty::WithNPC,
            "exclude" => FilterProperty::Exclude,
            "loss" | "losses" => FilterProperty::Losses,
            "kill" | "kills" => FilterProperty::Kills,
            _ => FilterProperty::Unknown,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Filter {
    kind: FilterKind,
    ids: Vec<u64>,
    properties: Vec<FilterProperty>,
}

impl From<String> for Filter {
    fn from(s: String) -> Self {
        let parts: Vec<&str> = s.split(':').collect();
        let kind = FilterKind::from(parts[0]);
        let ids_str = parts[1].split(',').collect::<Vec<&str>>();
        let props: Vec<&str> = if parts.len() > 2 {
            parts[2].split(',').collect()
        } else {
            vec![]
        };

        let mut properties = vec![];
        for item in props {
            let property = FilterProperty::from(item);
            match property {
                FilterProperty::Unknown => {
                    tracing::warn!(property = item, "unknown filter property");
                }
                _ => {
                    properties.push(property);
                }
            }
        }

        let mut ids: Vec<u64> = vec![];
        for id_str in ids_str {
            match id_str.parse::<u64>() {
                Ok(id) => ids.push(id),
                Err(e) => {
                    tracing::warn!(id_str=id_str, error=%e, "failed to parse subject id");
                }
            }
        }

        Filter {
            kind,
            ids,
            properties,
        }
    }
}

impl Filter {
    fn filter(&self, killmail: &crate::zkb::KillmailData) -> FilterResult {
        match self.kind {
            FilterKind::Region => self.filter_region(killmail),
            FilterKind::System => self.filter_system(killmail),
            FilterKind::Character => self.filter_character(killmail),
            FilterKind::Corporation => self.filter_corp(killmail),
            FilterKind::Alliance => self.filter_alliance(killmail),
            FilterKind::Ship => self.filter_ship_type(killmail),
        }
    }

    fn filter_system(&self, killmail: &crate::zkb::KillmailData) -> FilterResult {
        if self.ids.contains(&killmail.system_id) {
            if self.properties.contains(&FilterProperty::Exclude) {
                return FilterResult::Exclude;
            } else {
                return FilterResult::Include(None);
            }
        }

        FilterResult::NoMatch
    }

    fn filter_region(&self, killmail: &crate::zkb::KillmailData) -> FilterResult {
        if let Some(region_id) = static_data::get_region_by_system_id(killmail.system_id)
            && self.ids.contains(&region_id)
        {
            if self.properties.contains(&FilterProperty::Exclude) {
                return FilterResult::Exclude;
            }

            return FilterResult::Include(None);
        }

        FilterResult::NoMatch
    }

    fn filter_character(&self, killmail: &crate::zkb::KillmailData) -> FilterResult {
        let mut attacker_character_ids: Vec<u64> = vec![];
        killmail.attackers.iter().for_each(|attacker| {
            if let Some(id) = attacker.character_id {
                attacker_character_ids.push(id);
            }
        });

        self.filter_participant_data(killmail.victim.character_id, &attacker_character_ids)
    }

    fn filter_corp(&self, killmail: &crate::zkb::KillmailData) -> FilterResult {
        let mut attacker_corp_ids: Vec<u64> = vec![];
        killmail.attackers.iter().for_each(|attacker| {
            if let Some(id) = attacker.corporation_id {
                attacker_corp_ids.push(id);
            }
        });

        self.filter_participant_data(killmail.victim.corporation_id, &attacker_corp_ids)
    }

    fn filter_alliance(&self, killmail: &crate::zkb::KillmailData) -> FilterResult {
        let mut attacker_alliance_ids: Vec<u64> = vec![];
        killmail.attackers.iter().for_each(|attacker| {
            if let Some(id) = attacker.alliance_id {
                attacker_alliance_ids.push(id);
            }
        });

        self.filter_participant_data(killmail.victim.alliance_id, &attacker_alliance_ids)
    }

    fn filter_ship_type(&self, killmail: &crate::zkb::KillmailData) -> FilterResult {
        // If the victim has no ship type id, we can't match
        let victim_ship_type_id = killmail.victim.ship_type_id;

        let mut attacker_ship_type_ids: Vec<u64> = vec![];
        killmail.attackers.iter().for_each(|attacker| {
            if let Some(id) = attacker.ship_type_id {
                attacker_ship_type_ids.push(id);
            }
        });

        let filter_result =
            self.filter_participant_data(victim_ship_type_id, &attacker_ship_type_ids);

        match filter_result {
            FilterResult::Exclude => FilterResult::Exclude,
            FilterResult::Include(_) => FilterResult::Include(None),
            FilterResult::NoMatch => FilterResult::NoMatch,
        }
    }

    fn filter_participant_data(
        &self,
        victim_id: Option<u64>,
        attacker_ids: &Vec<u64>,
    ) -> FilterResult {
        tracing::trace!(
            victim_id,
            attacker_ids = format!("{attacker_ids:?}"),
            ids = format!("{:?}", self.ids),
            properties = format!("{:?}", self.properties),
            "filtering participant data"
        );
        if self.properties.contains(&FilterProperty::Exclude) {
            if let Some(victim_id) = victim_id
                && self.ids.contains(&victim_id)
                && !self.properties.contains(&FilterProperty::Kills)
            {
                return FilterResult::Exclude;
            }

            for attacker_id in attacker_ids {
                if self.ids.contains(attacker_id)
                    && !self.properties.contains(&FilterProperty::Losses)
                {
                    return FilterResult::Exclude;
                }
            }

            return FilterResult::NoMatch;
        }

        if let Some(victim_id) = victim_id
            && self.ids.contains(&victim_id)
            && !self.properties.contains(&FilterProperty::Kills)
        {
            return FilterResult::Include(Some(KillmailSide::Victim));
        }

        for attacker_id in attacker_ids {
            if self.ids.contains(attacker_id) && !self.properties.contains(&FilterProperty::Losses)
            {
                return FilterResult::Include(Some(KillmailSide::Attackers));
            }
        }

        FilterResult::NoMatch
    }
}

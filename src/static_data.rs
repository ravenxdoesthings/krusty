use lazy_static::lazy_static;
use rust_embed::Embed;
use std::collections::HashMap;

#[derive(Embed)]
#[folder = "static/"]
pub struct Data;

type SystemRow = (u64, u64, u64, String);

pub struct System {
    pub region_id: u64,
    pub _constellation_id: u64,
    pub system_id: u64,
    pub _name: String,
}

impl From<SystemRow> for System {
    fn from(row: SystemRow) -> Self {
        System {
            region_id: row.0,
            _constellation_id: row.1,
            system_id: row.2,
            _name: row.3,
        }
    }
}

lazy_static! {
    pub static ref SYSTEMS_DATA: HashMap<u64, System> = {
        let system_rows: Vec<SystemRow> = csv::Reader::from_reader(
            Data::get("mapSolarSystemsTrimmed.csv")
                .expect("Failed to load systems.csv")
                .data
                .as_ref(),
        )
        .deserialize()
        .map(|result| result.expect("Failed to parse CSV row"))
        .collect();

        let mut systems = HashMap::new();

        for row in system_rows {
            let system: System = row.into();
            systems.insert(system.system_id, system);
        }

        systems
    };
}

pub fn get_region_by_system_id(system_id: u64) -> Option<u64> {
    SYSTEMS_DATA.get(&system_id).map(|s| s.region_id)
}

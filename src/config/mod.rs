use std::collections::HashMap;

use uuid::Uuid;

#[derive(Debug, serde::Deserialize, Clone)]
pub enum CommandsEnabled {
    None,
    All,
    Some(Vec<String>),
}

#[derive(Debug, serde::Deserialize)]
pub struct GuildConfig {
    pub commands: CommandsEnabled,
}

#[derive(Debug, serde::Deserialize)]
pub struct Config {
    pub queue_id: Option<String>,
    pub redis_url: Option<String>,
    pub postgres_url: Option<String>,
    pub guilds: Option<HashMap<u64, GuildConfig>>,
}

impl Config {
    pub fn load(path: String) -> Self {
        let content = std::fs::read_to_string(path).expect("Failed to read config file");
        serde_yaml::from_str(&content).expect("Failed to parse config file")
    }

    pub fn queue_id(&self) -> String {
        let id = self
            .queue_id
            .clone()
            .unwrap_or_else(|| Uuid::new_v4().to_string());

        format!("krusty-{id}")
    }

    pub fn redis_url(&self) -> String {
        self.redis_url
            .clone()
            .unwrap_or_else(|| "redis://localhost:6379".to_string())
    }

    pub fn postgres_url(&self) -> String {
        self.postgres_url
            .clone()
            .unwrap_or_else(|| "postgres://postgres:postgres@localhost:5432/postgres".to_string())
    }

    pub fn guild_commands(&self, guild_id: u64) -> CommandsEnabled {
        if let Some(guilds) = &self.guilds
            && let Some(guild_config) = guilds.get(&guild_id)
        {
            return guild_config.commands.clone();
        }

        CommandsEnabled::None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_guild_config() {
        let yaml = r#"
101:
    commands: All
102:
    commands: None
103:
    commands: !Some
        - ping
        - stats
"#;

        let guild_configs: HashMap<u64, GuildConfig> =
            serde_yaml::from_str(yaml).expect("Failed to parse guild config");

        assert_eq!(guild_configs.len(), 3);

        match &guild_configs.get(&101).unwrap().commands {
            CommandsEnabled::All => {}
            _ => panic!("Expected CommandsEnabled::All for guild 101"),
        }

        match &guild_configs.get(&102).unwrap().commands {
            CommandsEnabled::None => {}
            _ => panic!("Expected CommandsEnabled::None for guild 102"),
        }

        match &guild_configs.get(&103).unwrap().commands {
            CommandsEnabled::Some(cmds) => {
                assert_eq!(cmds.len(), 2);
                assert_eq!(cmds[0], "ping");
                assert_eq!(cmds[1], "stats");
            }
            _ => panic!("Expected CommandsEnabled::Some for guild 103"),
        }
    }
}

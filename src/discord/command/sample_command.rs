use twilight_model::application::command::{CommandOption, CommandType};

use super::{CommandParams, CommandTrait, DEV_GUILD_ID};

pub struct TestCmd {}

impl TestCmd {
    pub fn new() -> Self {
        Self {}
    }
}

impl CommandTrait for TestCmd {
    fn name(&self) -> String {
        "testing-krusty".to_string()
    }

    fn description(&self) -> String {
        "this is a test".to_string()
    }

    fn guilds_enabled(&self) -> Vec<u64> {
        vec![DEV_GUILD_ID]
    }

    fn kind(&self) -> CommandType {
        CommandType::ChatInput
    }

    fn options(&self) -> Option<Vec<CommandOption>> {
        None
    }

    fn callback(
        &self,
        store: &dyn crate::persistence::StoreTrait,
        _interaction: &CommandParams,
    ) -> Result<String, anyhow::Error> {
        tracing::info!("testing-krusty command executed");

        store.add_filter_to_set(1, &"test".to_string())?;

        Ok("purr! 🐾".to_string())
    }
}

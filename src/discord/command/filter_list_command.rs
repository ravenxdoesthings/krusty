use twilight_model::{
    application::command::{CommandOption, CommandType},
    channel::ChannelType,
};
use twilight_util::builder::command::ChannelBuilder;

use super::{CommandParams, CommandTrait, DEV_GUILD_ID};

pub struct FilterListCmd {}

impl FilterListCmd {
    pub fn new() -> Self {
        Self {}
    }
}

impl CommandTrait for FilterListCmd {
    fn name(&self) -> String {
        "filter-list".to_string()
    }

    fn description(&self) -> String {
        "List filters configured for a channel".to_string()
    }

    fn guilds_enabled(&self) -> Vec<u64> {
        vec![DEV_GUILD_ID]
    }

    fn kind(&self) -> CommandType {
        CommandType::ChatInput
    }

    fn options(&self) -> Option<Vec<CommandOption>> {
        let channel = ChannelBuilder::new("channel", "Channel to see filters for")
            .channel_types(vec![ChannelType::GuildText])
            .required(true)
            .build();

        Some(vec![channel])
    }

    fn callback(
        &self,
        store: &dyn crate::persistence::StoreTrait,
        interaction: &CommandParams,
    ) -> Result<String, anyhow::Error> {
        let channel_id = interaction.get_option_channel_id("channel");

        let channel_id = match channel_id {
            Some(id) => id,
            None => {
                return Err(anyhow::anyhow!(
                    "Missing required option channel".to_string()
                ));
            }
        };

        tracing::info!(channel_id, "listing filters for channel");

        let filters = store.get_channel_filter_set(channel_id)?;

        Ok(format!("Filters for channel: {:?}", filters))
    }
}

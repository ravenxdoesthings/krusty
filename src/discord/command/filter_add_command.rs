use twilight_model::{
    application::command::{CommandOption, CommandType},
    channel::ChannelType,
};
use twilight_util::builder::command::{ChannelBuilder, StringBuilder};

use super::{CommandParams, CommandTrait, DEV_GUILD_ID};

pub struct FilterAddCmd {}

impl FilterAddCmd {
    pub fn new() -> Self {
        Self {}
    }
}

impl CommandTrait for FilterAddCmd {
    fn name(&self) -> String {
        "filter-add".to_string()
    }

    fn description(&self) -> String {
        "Add a new filter".to_string()
    }

    fn guilds_enabled(&self) -> Vec<u64> {
        vec![DEV_GUILD_ID]
    }

    fn kind(&self) -> CommandType {
        CommandType::ChatInput
    }

    fn options(&self) -> Option<Vec<CommandOption>> {
        let filter = StringBuilder::new("filter", "Filter to add")
            .required(true)
            .build();

        let channel = ChannelBuilder::new("channel", "Channel to add filter to")
            .channel_types(vec![ChannelType::GuildText])
            .required(true)
            .build();

        Some(vec![channel, filter])
    }

    fn callback(
        &self,
        store: &dyn crate::persistence::StoreTrait,
        interaction: &CommandParams,
    ) -> Result<String, anyhow::Error> {
        tracing::debug!(?interaction, "executing filter_add command");

        let channel_id = interaction.get_option_channel_id("channel");
        let filter = interaction.get_option_string("filter");

        if channel_id.is_none() {
            return Ok("Missing required option channel".to_string());
        }
        if filter.is_none() {
            return Ok("Missing required option filter".to_string());
        }

        store.add_filter_to_set(channel_id.unwrap(), filter.unwrap())?;

        Ok("Filter added successfully".to_string())
    }
}

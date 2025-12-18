use twilight_model::{
    application::command::{CommandOption, CommandType},
    channel::ChannelType,
};
use twilight_util::builder::command::{ChannelBuilder, StringBuilder};

use super::{CommandParams, CommandTrait};

pub struct FilterRemoveCmd {}

impl FilterRemoveCmd {
    pub fn new() -> Self {
        Self {}
    }
}

impl CommandTrait for FilterRemoveCmd {
    fn name(&self) -> String {
        "filter-remove".to_string()
    }

    fn description(&self) -> String {
        "Remove a filter from the channel".to_string()
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

    fn permissions(&self) -> Option<twilight_model::guild::Permissions> {
        Some(
            twilight_model::guild::Permissions::ADMINISTRATOR
                | twilight_model::guild::Permissions::MANAGE_GUILD
                | twilight_model::guild::Permissions::MANAGE_CHANNELS,
        )
    }

    fn callback(
        &self,
        store: &dyn crate::persistence::Store,
        interaction: &CommandParams,
    ) -> Result<String, anyhow::Error> {
        let channel_id = match interaction.get_option_channel_id("channel") {
            None => return Ok("Missing required option channel".to_string()),
            Some(id) => id,
        };
        let filter = match interaction.get_option_string("filter") {
            None => return Ok("Missing required option filter".to_string()),
            Some(f) => f,
        };

        tracing::info!(channel_id, filter, "removing filter from channel");

        store.remove_filter_from_set(channel_id, &filter)?;

        Ok(format!(
            "Filter `{filter}` removed successfully from channel <#{channel_id}>"
        ))
    }
}

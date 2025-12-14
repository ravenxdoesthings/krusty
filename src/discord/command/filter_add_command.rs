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
            None => interaction.channel.id,
            Some(id) => id,
        };
        let filter = match interaction.get_option_string("filter") {
            None => return Ok("Missing required option filter".to_string()),
            Some(f) => f,
        };

        tracing::info!(channel_id, filter, "adding filter to channel");

        store.add_filter_to_set(interaction.guild_id.get(), channel_id, &filter)?;

        Ok(format!(
            "Filter `{}` added successfully to channel {}",
            filter, interaction.channel.name
        ))
    }
}

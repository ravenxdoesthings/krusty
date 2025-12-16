use std::{collections::HashMap, sync::Arc};
use twilight_http::Client;
use twilight_model::{
    application::{
        command::{Command, CommandOption, CommandType},
        interaction::application_command::{CommandData, CommandDataOption},
    },
    gateway::payload::incoming::InteractionCreate,
    guild::Permissions,
    id::{Id, marker::GuildMarker},
};
use twilight_util::builder::command::CommandBuilder;

use crate::config;

mod filter_add_command;
mod filter_clear_command;
mod filter_list_command;
mod filter_remove_command;

const DEV_GUILD_ID: u64 = 1149091527600132167;

#[derive(Clone)]
pub struct Handler {
    // store is the persistence store used by commands
    store: Arc<dyn crate::persistence::Store>,

    // commands is a map of command name to command implementation
    commands: Arc<HashMap<String, Arc<dyn CommandTrait>>>,

    // validator is a map of guild ID to list of valid commands
    validator: HashMap<String, Vec<u64>>,
}

#[derive(Debug)]
pub struct Channel {
    pub id: u64,
    pub name: String,
}

#[derive(Debug)]
pub struct CommandParams {
    guild_id: Id<GuildMarker>,
    channel: Channel,
    name: String,
    options: HashMap<String, CommandDataOption>,
}

impl CommandParams {
    pub fn get_option_string(&self, name: &str) -> Option<String> {
        match self.options.get(name) {
            Some(val) => match &val.value {
                twilight_model::application::interaction::application_command::CommandOptionValue::String(s) => Some(s.clone()),
                _ => None,
            },
            None => None,
        }
    }

    pub fn get_option_channel_id(&self, name: &str) -> Option<u64> {
        match self.options.get(name) {
            Some(val) => match &val.value {
                twilight_model::application::interaction::application_command::CommandOptionValue::Channel(channel_id) => Some(channel_id.get()),
                _ => None,
            },
            None => None,
        }
    }

    pub fn parse_interaction(event: &CommandData) -> Result<CommandParams, anyhow::Error> {
        let guild_id = match event.guild_id {
            Some(gid) => gid,
            None => {
                return Err(anyhow::format_err!("command not issued in a guild"));
            }
        };

        let resolutions = match &event.resolved {
            Some(res) => res,
            None => {
                tracing::error!(?event, "missing resolved data in interaction");
                return Err(anyhow::format_err!("missing resolved data in interaction"));
            }
        };

        let channel = resolutions.channels.iter().next().ok_or_else(|| {
            anyhow::format_err!("missing channel data in resolved interaction data")
        })?;

        let options = event
            .options
            .iter()
            .map(|opt| (opt.name.clone(), opt.clone()))
            .collect::<HashMap<String, CommandDataOption>>();

        Ok(CommandParams {
            guild_id,
            channel: Channel {
                id: channel.0.get(),
                name: channel.1.name.clone(),
            },
            name: event.name.clone(),
            options,
        })
    }
}

impl Handler {
    pub fn build(
        config: &config::Config,
        store: Arc<dyn crate::persistence::Store>,
        guild_ids: Vec<Id<GuildMarker>>,
    ) -> Result<Self, anyhow::Error> {
        let mut handler = Self {
            store,
            commands: Arc::new(HashMap::new()),
            validator: HashMap::new(),
        };

        build_commands(config, &mut handler, guild_ids)?;

        Ok(handler)
    }

    pub async fn handle(&self, event: &InteractionCreate) -> Result<String, anyhow::Error> {
        let params: CommandParams = match &event.data {
            Some(data) => match data {
                twilight_model::application::interaction::InteractionData::ApplicationCommand(
                    cmd,
                ) => CommandParams::parse_interaction(cmd)?,
                _ => {
                    return Err(anyhow::format_err!("unexpected interaction data"));
                }
            },
            None => {
                return Err(anyhow::format_err!("missing interaction data"));
            }
        };

        tracing::trace!(
            guild_id = params.guild_id.get(),
            command_name = params.name.as_str(),
            command_names = ?self.commands.keys().collect::<Vec<&String>>(),
            "handling command"
        );

        let command = match self.commands.get(&params.name) {
            Some(cmd) => cmd,
            None => {
                return Err(anyhow::format_err!("command not found: {}", params.name));
            }
        };

        if !command.guilds_enabled().is_empty()
            && !command.guilds_enabled().contains(&params.guild_id.get())
        {
            return Err(anyhow::format_err!(
                "command {} not enabled for guild {}",
                params.name,
                params.guild_id.get()
            ));
        }

        command.callback(self.store.as_ref(), &params)
    }

    pub async fn shutdown(&self, client: &Client) -> Result<(), anyhow::Error> {
        deregister_commands(self, client).await?;
        tracing::info!("shutting down command handler");
        Ok(())
    }
}

pub async fn deregister_commands(handler: &Handler, client: &Client) -> Result<(), anyhow::Error> {
    let current_application = client.current_user_application().await?.model().await?;
    let mut output: Vec<(String, Vec<u64>)> = Vec::new();
    for command in handler.commands.values() {
        let guilds = command.guilds_enabled();
        for guild_id in &guilds {
            let id = Id::<GuildMarker>::new(*guild_id);
            let commands = client
                .interaction(current_application.id)
                .guild_commands(id)
                .await?
                .models()
                .await?;

            for cmd in commands {
                if cmd.name == command.name() {
                    tracing::trace!(
                        guild_id = guild_id,
                        command_name = cmd.name.as_str(),
                        "deleting command from guild"
                    );

                    let cmd_id = match cmd.id {
                        Some(cid) => cid,
                        None => {
                            tracing::warn!(
                                guild_id = guild_id,
                                command_name = cmd.name.as_str(),
                                "command has no ID, skipping deletion"
                            );
                            continue;
                        }
                    };

                    client
                        .interaction(current_application.id)
                        .delete_guild_command(id, cmd_id)
                        .await?;

                    tracing::trace!(
                        guild_id = guild_id,
                        command_name = cmd.name.as_str(),
                        "deleted command from guild"
                    );
                }
            }
        }

        output.push((command.name(), guilds));
    }

    tracing::debug!(?output, "deregistered commands");

    Ok(())
}

pub async fn register_commands(handler: &Handler, client: &Client) -> Result<(), anyhow::Error> {
    let current_application = client.current_user_application().await?.model().await?;

    for command in handler.commands.values() {
        let guilds = command.guilds_enabled();
        for guild_id in guilds {
            let id = Id::<GuildMarker>::new(guild_id);
            let options = command.options().unwrap_or_default();
            client
                .interaction(current_application.id)
                .create_guild_command(id)
                .chat_input(command.name().as_str(), command.description().as_str())
                .command_options(&options)
                .await?;
        }
    }
    Ok(())
}

fn build_commands(
    config: &config::Config,
    handler: &mut Handler,
    guild_ids: Vec<Id<GuildMarker>>,
) -> Result<(), anyhow::Error> {
    let mut built_commands: HashMap<String, Arc<dyn CommandTrait>> = HashMap::new();

    let command_list: Vec<Arc<dyn CommandTrait>> = vec![
        Arc::new(filter_add_command::FilterAddCmd::new()),
        Arc::new(filter_list_command::FilterListCmd::new()),
        Arc::new(filter_remove_command::FilterRemoveCmd::new()),
        Arc::new(filter_clear_command::FilterClearCmd::new()),
    ];

    for cmd in command_list {
        for guild_id in guild_ids.clone() {
            match config.guild_commands(guild_id.get()) {
                config::CommandsEnabled::None => {
                    tracing::debug!(
                        guild_id = guild_id.get(),
                        command_name = cmd.name().as_str(),
                        "commands disabled for guild, skipping command build"
                    );
                    continue;
                }
                config::CommandsEnabled::Some(allowed) => {
                    if !allowed.contains(&cmd.name()) {
                        tracing::debug!(
                            guild_id = guild_id.get(),
                            command_name = cmd.name().as_str(),
                            "command not enabled for guild, skipping command build"
                        );
                        continue;
                    }
                }
                config::CommandsEnabled::All => {}
            }

            tracing::trace!(
                guild_id = guild_id.get(),
                guilds_enabled = ?cmd.guilds_enabled(),
                command_name = cmd.name().as_str(),
                "building command for guild"
            );
            match build_command(cmd.as_ref(), guild_id) {
                Err(e) => {
                    if e.to_string().contains("not enabled for guild") {
                        tracing::trace!(
                            guild_id = guild_id.get(),
                            command_name = cmd.name().as_str(),
                            "command not enabled for guild, skipping"
                        );
                        continue;
                    }
                    tracing::warn!(
                        error = e.to_string(),
                        guild_id = guild_id.get(),
                        command_name = cmd.name().as_str(),
                        "failed to build command for guild"
                    );
                    continue;
                }
                Ok(command) => {
                    tracing::trace!(
                        guild_id = guild_id.get(),
                        command_name = command.name.as_str(),
                        "created command in guild"
                    );
                    built_commands.insert(command.name.clone(), cmd.clone());
                    handler
                        .validator
                        .entry(cmd.name())
                        .or_default()
                        .push(guild_id.get());
                }
            }
        }
    }

    handler.commands = Arc::new(built_commands.clone());

    Ok(())
}

pub trait CommandTrait: Send + Sync {
    fn name(&self) -> String;
    fn description(&self) -> String;
    fn kind(&self) -> CommandType;
    fn guilds_enabled(&self) -> Vec<u64>;
    fn options(&self) -> Option<Vec<CommandOption>>;
    fn permissions(&self) -> Option<Permissions>;
    fn callback(
        &self,
        store: &dyn crate::persistence::Store,
        interaction: &CommandParams,
    ) -> Result<String, anyhow::Error>;
}

pub fn build_command(
    cmd: &dyn CommandTrait,
    guild_id: Id<GuildMarker>,
) -> Result<Command, anyhow::Error> {
    if !cmd.guilds_enabled().is_empty() && !cmd.guilds_enabled().contains(&guild_id.get()) {
        return Err(anyhow::anyhow!(
            "command not enabled for guild {}",
            guild_id.get()
        ));
    }

    let mut builder =
        CommandBuilder::new(cmd.name().as_str(), cmd.description().as_str(), cmd.kind());

    if let Some(permissions) = cmd.permissions() {
        builder = builder.default_member_permissions(permissions);
    }

    let built_cmd = builder
        .default_member_permissions(
            Permissions::ADMINISTRATOR | Permissions::MANAGE_GUILD | Permissions::MANAGE_CHANNELS,
        )
        .guild_id(guild_id)
        .validate()?
        .build();

    Ok(built_cmd)
}

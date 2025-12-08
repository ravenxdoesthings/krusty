use std::{collections::HashMap, sync::Arc};
use twilight_http::Client;
use twilight_model::{
    application::{
        command::{Command, CommandType},
        interaction::application_command::CommandDataOption,
    },
    gateway::payload::incoming::InteractionCreate,
    id::{Id, marker::GuildMarker},
};
use twilight_util::builder::command::CommandBuilder;

const DEV_GUILD_ID: u64 = 1149091527600132167;

#[derive(Clone)]
pub struct Handler {
    // store is the persistence store used by commands
    store: Arc<dyn crate::persistence::StoreTrait>,

    // commands is a map of command name to command implementation
    commands: Arc<HashMap<String, Arc<dyn CommandTrait>>>,

    // validator is a map of guild ID to list of valid commands
    validator: HashMap<String, Vec<u64>>,
}

pub struct CommandParams {
    guild_id: Id<GuildMarker>,
    name: String,
    _options: Vec<CommandDataOption>,
}

impl Handler {
    pub fn build(
        store: Arc<dyn crate::persistence::StoreTrait>,
        guild_ids: Vec<Id<GuildMarker>>,
    ) -> Result<Self, anyhow::Error> {
        let mut handler = Self {
            store,
            commands: Arc::new(HashMap::new()),
            validator: HashMap::new(),
        };

        handler.build_commands(guild_ids)?;

        Ok(handler)
    }

    pub async fn handle(&self, event: &InteractionCreate) -> Result<(), anyhow::Error> {
        let params: CommandParams = match &event.data {
            Some(data) => match data {
                twilight_model::application::interaction::InteractionData::ApplicationCommand(
                    cmd,
                ) => {
                    let guild_id = match cmd.guild_id {
                        Some(gid) => gid,
                        None => {
                            return Err(anyhow::format_err!("command not issued in a guild"));
                        }
                    };

                    CommandParams {
                        guild_id,
                        name: cmd.name.clone(),
                        _options: cmd.options.clone(),
                    }
                }
                _ => {
                    return Err(anyhow::format_err!("unexpected interaction data"));
                }
            },
            None => {
                return Err(anyhow::format_err!("missing interaction data"));
            }
        };

        tracing::debug!(
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

        if !command.guilds_enabled().contains(&params.guild_id.get()) {
            return Err(anyhow::format_err!(
                "command {} not enabled for guild {}",
                params.name,
                params.guild_id.get()
            ));
        }

        command.callback(self.store.as_ref(), &params)?;

        Ok(())
    }

    pub async fn register_commands(&self, client: &Client) -> Result<(), anyhow::Error> {
        let current_application = client.current_user_application().await?.model().await?;

        for command in self.commands.values() {
            let guilds = command.guilds_enabled();
            for guild_id in guilds {
                let id = Id::<GuildMarker>::new(guild_id);
                client
                    .interaction(current_application.id)
                    .create_guild_command(id)
                    .chat_input("testing-krusty", "this is a test")
                    .await?;
            }
        }
        Ok(())
    }

    fn build_commands(&mut self, guild_ids: Vec<Id<GuildMarker>>) -> Result<(), anyhow::Error> {
        let mut built_commands: HashMap<String, Arc<dyn CommandTrait>> = HashMap::new();

        let command_list: Vec<Arc<dyn CommandTrait>> =
            vec![Arc::new(FilterAddCmd::new()), Arc::new(TestCmd::new())];

        tracing::debug!("building commands for guilds: {:?}", guild_ids);

        for cmd in command_list {
            for guild_id in guild_ids.clone() {
                tracing::debug!(
                    guild_id = guild_id.get(),
                    guilds_enabled = ?cmd.guilds_enabled(),
                    command_name = cmd.name().as_str(),
                    "building command for guild"
                );
                if cmd.guilds_enabled().contains(&guild_id.get()) {
                    match build_command(cmd.as_ref(), guild_id) {
                        Err(e) => {
                            tracing::warn!(
                                error = e.to_string(),
                                guild_id = guild_id.get(),
                                command_name = cmd.name().as_str(),
                                "failed to build command for guild"
                            );
                            continue;
                        }
                        Ok(command) => {
                            tracing::debug!(
                                guild_id = guild_id.get(),
                                command_name = command.name.as_str(),
                                "created command in guild"
                            );
                            built_commands.insert(command.name.clone(), cmd.clone());
                            self.validator
                                .entry(cmd.name())
                                .or_default()
                                .push(guild_id.get());
                        }
                    }
                }
            }
        }

        self.commands = Arc::new(built_commands.clone());

        let command_debug: Vec<(String, Vec<u64>)> = self
            .commands
            .iter()
            .map(|(name, cmd)| (name.clone(), cmd.guilds_enabled()))
            .collect();

        tracing::debug!(?command_debug, "built commands");
        tracing::debug!(?self.validator, "command validator");

        Ok(())
    }
}

pub enum CallbackSuccess {
    FilterAdd,
}

pub trait CommandTrait: Send + Sync {
    fn name(&self) -> String;
    fn description(&self) -> String;
    fn kind(&self) -> CommandType;
    fn guilds_enabled(&self) -> Vec<u64>;
    fn callback(
        &self,
        store: &dyn crate::persistence::StoreTrait,
        interaction: &CommandParams,
    ) -> Result<CallbackSuccess, anyhow::Error>;
}

pub fn build_command(
    cmd: &dyn CommandTrait,
    guild_id: Id<GuildMarker>,
) -> Result<Command, anyhow::Error> {
    if !cmd.guilds_enabled().contains(&guild_id.get()) {
        return Err(anyhow::anyhow!(
            "command not enabled for guild {}",
            guild_id.get()
        ));
    }

    let cmd = CommandBuilder::new(cmd.name().as_str(), cmd.description().as_str(), cmd.kind())
        .guild_id(guild_id)
        .validate()?
        .build();

    Ok(cmd)
}

struct FilterAddCmd {}

impl FilterAddCmd {
    pub fn new() -> Self {
        Self {}
    }
}

impl CommandTrait for FilterAddCmd {
    fn name(&self) -> String {
        "filter_add".to_string()
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

    fn callback(
        &self,
        store: &dyn crate::persistence::StoreTrait,
        _interaction: &CommandParams,
    ) -> Result<CallbackSuccess, anyhow::Error> {
        store.add_filter_to_set(1, "test".into())?;

        Ok(CallbackSuccess::FilterAdd)
    }
}

struct TestCmd {}

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

    fn callback(
        &self,
        store: &dyn crate::persistence::StoreTrait,
        _interaction: &CommandParams,
    ) -> Result<CallbackSuccess, anyhow::Error> {
        tracing::info!("testing-krusty command executed");

        store.add_filter_to_set(1, "test".into())?;

        Ok(CallbackSuccess::FilterAdd)
    }
}

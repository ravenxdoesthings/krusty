use opentelemetry::trace::Status;
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use tokio::task::JoinHandle;
use twilight_gateway::{Config, Event, EventTypeFlags, Intents, MessageSender, Shard, StreamExt};

use tracing::{Level, Span};
use tracing_opentelemetry::OpenTelemetrySpanExt;
use twilight_http::Client;
use twilight_model::{
    channel::message::{Embed, MessageFlags, embed::EmbedThumbnail},
    id::{Id, marker::GuildMarker},
    user::CurrentUser,
};

mod command;
use crate::{filters, zkb};

static SHUTDOWN: AtomicBool = AtomicBool::new(false);

struct Listener {
    _senders: Vec<MessageSender>,
    _tasks: Vec<JoinHandle<()>>,
}

impl Listener {
    async fn listener(mut shard: Shard, client: Arc<Client>, handler: command::Handler) {
        let wanted_events = EventTypeFlags::MESSAGE_CREATE
            | EventTypeFlags::DIRECT_MESSAGES
            | EventTypeFlags::GUILD_MESSAGES
            | EventTypeFlags::INTERACTION_CREATE;

        let current_user: CurrentUser = client.current_user().await.unwrap().model().await.unwrap();
        let current_application = client
            .current_user_application()
            .await
            .unwrap()
            .model()
            .await
            .unwrap();

        while let Some(item) = shard.next_event(wanted_events).await {
            let event = match item {
                Ok(Event::GatewayClose(_)) if SHUTDOWN.load(Ordering::Relaxed) => break,
                Ok(event) => event,
                Err(source) => {
                    tracing::warn!(?source, "error receiving event");

                    continue;
                }
            };

            let handler = handler.clone();
            let client = client.clone();
            // You'd normally want to spawn a new tokio task for each event and
            // handle the event there to not block the shard.
            tokio::spawn(async move {
                match event {
                    Event::MessageCreate(msg) => {
                        if msg.author.bot {
                            return;
                        }

                        if msg.author.id == current_user.id {
                            return;
                        }

                        tracing::info!(author = msg.author.name, "received message");
                    }
                    Event::InteractionCreate(msg) => {
                        tracing::trace!(
                            interaction_id = msg.id.get(),
                            interaction_type = ?msg.kind,
                            data = ?msg.data,
                            "received command"
                        );

                        let result = handler.handle(&msg).await;

                        let response_text = match result {
                            Ok(response) => response,
                            Err(e) => format!("Error handling command: {}", e),
                        };

                        client
                            .interaction(current_application.id)
                            .create_response(
                                msg.id,
                                &msg.token,
                                &twilight_model::http::interaction::InteractionResponse {
                                    kind: twilight_model::http::interaction::InteractionResponseType::ChannelMessageWithSource,
                                    data: Some(twilight_model::http::interaction::InteractionResponseData {
                                        content: Some(response_text),
                                        flags: Some(MessageFlags::EPHEMERAL),
                                        ..Default::default()
                                    }),
                                }
                            )
                            .await
                            .unwrap();
                    }
                    _ => {
                        tracing::debug!(?event, "received unhandled event");
                    }
                }
            });
        }
    }
}

#[derive(Clone)]
pub struct Gateway {
    client: Arc<Client>,
    command_handler: command::Handler,
    _listener: Arc<Listener>,
}

impl Gateway {
    pub async fn build(
        store: Arc<dyn crate::persistence::StoreTrait>,
        token: String,
    ) -> Result<Self, anyhow::Error> {
        let client = Arc::new(Client::new(token.clone()));
        let config = Config::new(
            token.clone(),
            Intents::GUILD_MESSAGES | Intents::MESSAGE_CONTENT | Intents::DIRECT_MESSAGES,
        );

        let guild_ids = client
            .current_user_guilds()
            .await?
            .model()
            .await?
            .iter()
            .map(|g| g.id)
            .collect::<Vec<Id<GuildMarker>>>();

        let command_handler = command::Handler::build(store, guild_ids)?;
        command::register_commands(&command_handler, &client).await?;

        let shards =
            twilight_gateway::create_recommended(&client, config, |_, builder| builder.build())
                .await?;

        let mut senders = Vec::with_capacity(shards.len());
        let mut tasks = Vec::with_capacity(shards.len());

        for shard in shards {
            senders.push(shard.sender());
            tasks.push(tokio::spawn(Listener::listener(
                shard,
                client.clone(),
                command_handler.clone(),
            )));
        }

        let listener = Listener {
            _senders: senders,
            _tasks: tasks,
        };

        Ok(Self {
            client,
            command_handler,
            _listener: Arc::new(listener),
        })
    }

    pub async fn shutdown(&self) {
        let _ = self.command_handler.shutdown(&self.client).await;
        SHUTDOWN.store(true, Ordering::Relaxed);
    }

    // #[tracing::instrument(skip(self, parent), parent = parent)]
    pub async fn embed(
        &self,
        parent: &Span,
        killmail: &zkb::Killmail,
        channel_id: u64,
        kind: Option<filters::KillmailSide>,
    ) -> Result<(), anyhow::Error> {
        let span = tracing::span!(Level::INFO, "embedding killmail");
        let _ = span.set_parent(parent.context());
        let _enter = span.enter();

        let url = format!("https://zkillboard.com/kill/{}/", killmail.kill_id);
        let meta = Meta::from_url(url)?;

        let client = Arc::clone(&self.client);

        let channel_id = Id::new(channel_id);
        let embed = Self::new_embed(&meta, kind);

        match client.create_message(channel_id).embeds(&[embed]).await {
            Ok(_) => {
                tracing::info!(url = meta.url, "embedded killmail");
            }
            Err(e) => {
                span.set_status(Status::error(format!("failed to send message: {e}")));
                tracing::error!(error = e.to_string(), "failed to send message");
            }
        }
        Ok(())
    }

    fn color(kind: Option<filters::KillmailSide>) -> Option<u32> {
        match kind {
            Some(filters::KillmailSide::Attackers) => Some(0x93c47d),
            Some(filters::KillmailSide::Victim) => Some(0x990000),
            None => Some(0xd3d3d3),
        }
    }

    fn new_embed(meta: &Meta, kind: Option<filters::KillmailSide>) -> Embed {
        Embed {
            author: None,
            color: Self::color(kind),
            description: Some(meta.description.clone()),
            fields: vec![],
            footer: None,
            image: None,
            kind: "link".to_owned(),
            provider: None,
            thumbnail: Some(EmbedThumbnail {
                height: Some(meta.thumbnail.height as u64),
                proxy_url: None,
                url: meta.thumbnail.url.clone(),
                width: Some(meta.thumbnail.width as u64),
            }),
            timestamp: None,
            title: Some(meta.title.clone()),
            url: Some(meta.url.clone()),
            video: None,
        }
    }
}

struct Thumbnail {
    url: String,
    width: u32,
    height: u32,
}

struct Meta {
    url: String,
    title: String,
    description: String,
    thumbnail: Thumbnail,
}

impl Meta {
    fn from_url(value: String) -> Result<Self, anyhow::Error> {
        let Ok(meta) = opengraph::scrape(value.clone().as_str(), Default::default()) else {
            return Err(anyhow::anyhow!("error occured"));
        };

        Ok(Self {
            url: value,
            title: meta.title,
            description: meta.description.unwrap_or("".to_string()),
            thumbnail: Thumbnail {
                url: meta
                    .images
                    .first()
                    .map_or("".to_string(), |img| img.url.clone()),
                width: 64,
                height: 64,
            },
        })
    }
}

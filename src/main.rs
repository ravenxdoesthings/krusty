use std::{env, sync::Arc, time::Duration, vec};
use tokio::sync::Mutex;
use tracing_subscriber::{EnvFilter, fmt, prelude::*};
use twilight_http::Client;
use twilight_model::{
    channel::message::{Embed, embed::EmbedThumbnail},
    id::{Id, marker::ChannelMarker},
};
use uuid::Uuid;

use crate::{cache::Entry, zkb::Killmail};

mod cache;
mod config;
mod zkb;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let discord_token = env::var("DISCORD_TOKEN")?;
    let config_path = env::var("CONFIG_PATH").unwrap_or("./config.json".to_string());

    let config = config::Config::load(config_path);

    // Initialize the tracing subscriber.
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(
            EnvFilter::from_default_env()
                .add_directive("warn".parse()?)
                .add_directive("krusty=debug".parse()?),
        )
        .init();

    tracing::debug!(
        filters = format!("{:?}", config.filters),
        "applying filters"
    );

    let cache = cache::Cache::new();
    let _expiration_task = tokio::spawn({
        let cache = cache.clone();
        async move {
            cache.start_expiration_task().await;
        }
    });
    let client = Client::new(discord_token);
    let channel_id = config
        .channels
        .first()
        .ok_or(anyhow::format_err!("no channels specified"))?;

    let sender = Sender::new(client, channel_id.to_owned());

    let client = reqwest::Client::builder()
        .user_agent("krusty/0.1")
        .build()?;

    let queue_id = format!("krusty-{}", Uuid::new_v4());
    loop {
        let response: zkb::Response = client
            .clone()
            .get(format!(
                "https://zkillredisq.stream/listen.php?queueID={queue_id}&"
            ))
            .send()
            .await?
            .json()
            .await?;

        let Some(mut killmail) = response.killmail else {
            tracing::debug!("dropped empty killmail");
            continue;
        };

        let time_divergence = chrono::Utc::now().signed_duration_since(killmail.killmail.timestamp);

        if !killmail.filter(&config.filters) {
            tracing::debug!(
                time_divergence_s = format!("{}s", time_divergence.num_seconds()),
                time_divergence_ms = format!("{}ms", time_divergence.num_milliseconds()),
                time_divergence_m = format!("{}m", time_divergence.num_minutes()),
                "filtered out killmail"
            );
            continue;
        }

        if cache
            .get(format!("kill:{}", killmail.kill_id).as_str())
            .is_some()
        {
            continue;
        }
        cache.set(
            format!("kill:{}", killmail.kill_id),
            Entry::new(Some(Duration::from_secs(10800))),
        );

        match sender.embed(&killmail).await {
            Ok(_) => {}
            Err(e) => {
                eprintln!("error: {e}");
            }
        }

        let _ = tokio::time::sleep(Duration::from_secs(1)).await;
    }
    // Ok(())
}

#[derive(Clone)]
struct Sender {
    client: Arc<Mutex<Client>>,
    channel_id: Id<ChannelMarker>,
}

impl Sender {
    fn new(client: Client, channel_id: u64) -> Self {
        tracing::debug!(channel_id, "creating Discord client");
        Self {
            client: Arc::new(Mutex::new(client)),
            channel_id: Id::new(channel_id),
        }
    }

    async fn embed(&self, killmail: &Killmail) -> Result<(), anyhow::Error> {
        let url = format!("https://zkillboard.com/kill/{}/", killmail.kill_id);
        tracing::debug!(url, "embedding killmail");
        let meta = Meta::from_url(url)?;

        let color: Option<u32> = if killmail.ours {
            Some(0x93c47d)
        } else {
            Some(0x990000)
        };

        let client = Arc::clone(&self.client);
        let client = client.lock().await;
        client
            .create_message(self.channel_id)
            .embeds(&[Embed {
                author: None,
                color,
                description: Some(meta.description),
                fields: vec![],
                footer: None,
                image: None,
                kind: "link".to_owned(),
                provider: None,
                thumbnail: Some(EmbedThumbnail {
                    height: Some(meta.thumbnail.height as u64),
                    proxy_url: None,
                    url: meta.thumbnail.url,
                    width: Some(meta.thumbnail.width as u64),
                }),
                timestamp: None,
                title: Some(meta.title),
                url: Some(meta.url),
                video: None,
            }])
            .await?;
        Ok(())
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

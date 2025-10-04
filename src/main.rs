use opentelemetry::trace::Status;
use std::{env, sync::Arc, time::Duration, vec};
use tokio::sync::Mutex;
use tracing::{Level, Span};
use tracing_opentelemetry::OpenTelemetrySpanExt;
use twilight_http::Client;
use twilight_model::{
    channel::message::{Embed, embed::EmbedThumbnail},
    id::{Id, marker::ChannelMarker},
};

use crate::zkb::Killmail;

mod cache;
mod config;
mod otel;
mod zkb;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let discord_token = env::var("DISCORD_TOKEN")?;
    let config_path = env::var("CONFIG_PATH").unwrap_or("./config.json".to_string());

    let config = config::Config::load(config_path);
    let queue_id = config.queue_id();

    let _guard = otel::init_tracing_subscriber();

    tracing::debug!(
        filters = format!("{:?}", config.filters),
        "applying filters"
    );

    let cache = Arc::new(cache::Cache::new(config.redis_url()));
    let client = Client::new(discord_token);
    let sender = Sender::new(client, config.channels);

    let client = reqwest::Client::builder()
        .user_agent("krusty/0.1")
        .build()?;

    let mut running = false;
    loop {
        if running {
            let _ = tokio::time::sleep(Duration::from_secs(2)).await;
        }
        running = true;

        let request_span: Span = tracing::span!(Level::INFO, "sending request");
        let _enter = request_span.enter();
        let response = match client
            .clone()
            .get(format!(
                "https://zkillredisq.stream/listen.php?queueID={queue_id}&"
            ))
            .send()
            .await
        {
            Ok(resp) => match resp.text().await {
                Ok(raw) => match serde_json::from_str::<zkb::Response>(&raw) {
                    Ok(parsed) => parsed,
                    Err(e) => {
                        request_span.set_status(Status::error(format!(
                            "failed to parse response JSON: {e}"
                        )));
                        tracing::error!(
                            raw,
                            error = e.to_string(),
                            "Failed to parse response JSON"
                        );
                        continue;
                    }
                },
                Err(e) => {
                    request_span
                        .set_status(Status::error(format!("failed to parse response JSON: {e}")));
                    tracing::error!(error = e.to_string(), "Failed to parse response JSON");
                    continue;
                }
            },
            Err(e) => {
                request_span.set_status(Status::error(format!("failed to send request: {e}")));
                tracing::error!(error = e.to_string(), "Failed to send request");
                continue;
            }
        };

        let Some(mut killmail) = response.killmail else {
            request_span.set_status(Status::Ok);
            request_span.add_event("dropped empty killmail".to_string(), vec![]);
            tracing::debug!("dropped empty killmail");
            continue;
        };

        let time_divergence = killmail.skew();
        let keep_killmail = killmail.filter(&config.filters);

        tracing::info!(
            keep_killmail,
            time_divergence_s = format!("{}", time_divergence.num_seconds()),
            time_divergence_ms = format!("{}", time_divergence.num_milliseconds()),
            time_divergence_m = format!("{}", time_divergence.num_minutes()),
            "ran killmail through filters"
        );

        if keep_killmail {
            let cache_key = format!("kill:{}", killmail.kill_id);
            if let Ok(hit) = cache.check(&cache_key)
                && hit
            {
                continue;
            }

            if let Err(e) = cache.store(&cache_key, Some(Duration::from_secs(10800))) {
                tracing::error!(error = e.to_string(), "failed to store killmail in cache");
            }

            match sender.embed(&request_span, &killmail).await {
                Ok(_) => {}
                Err(e) => {
                    tracing::error!(error = e.to_string(), "failed to embed killmail");
                    request_span
                        .set_status(Status::error(format!("failed to embed killmail: {e}")));
                }
            }

            request_span.set_status(Status::Ok);
        }
    }
    // Ok(())
}

#[derive(Clone)]
struct Sender {
    client: Arc<Mutex<Client>>,
    channel_ids: Vec<Id<ChannelMarker>>,
}

impl Sender {
    fn new(client: Client, channel_ids: Vec<u64>) -> Self {
        tracing::debug!(
            channel_ids = format!("{:?}", channel_ids),
            "creating Discord client"
        );

        let ids = channel_ids.into_iter().map(Id::new).collect();

        Self {
            client: Arc::new(Mutex::new(client)),
            channel_ids: ids,
        }
    }

    // #[tracing::instrument(skip(self, parent), parent = parent)]
    async fn embed(&self, parent: &Span, killmail: &Killmail) -> Result<(), anyhow::Error> {
        let span = tracing::span!(Level::INFO, "embedding killmail");
        span.set_parent(parent.context());
        let _enter = span.enter();

        let url = format!("https://zkillboard.com/kill/{}/", killmail.kill_id);
        let meta = Meta::from_url(url)?;

        let color: Option<u32> = if killmail.ours {
            Some(0x93c47d)
        } else {
            Some(0x990000)
        };

        let client = Arc::clone(&self.client);
        let client = client.lock().await;
        for channel_id in &self.channel_ids {
            let description = meta.description.clone();
            let thumb_url = meta.thumbnail.url.clone();
            let title = meta.title.clone();
            match client
                .create_message(*channel_id)
                .embeds(&[Embed {
                    author: None,
                    color,
                    description: Some(description),
                    fields: vec![],
                    footer: None,
                    image: None,
                    kind: "link".to_owned(),
                    provider: None,
                    thumbnail: Some(EmbedThumbnail {
                        height: Some(meta.thumbnail.height as u64),
                        proxy_url: None,
                        url: thumb_url,
                        width: Some(meta.thumbnail.width as u64),
                    }),
                    timestamp: None,
                    title: Some(title),
                    url: Some(meta.url.clone()),
                    video: None,
                }])
                .await
            {
                Ok(_) => {
                    tracing::info!(url = meta.url, "embedded killmail");
                }
                Err(e) => {
                    span.set_status(Status::error(format!("failed to send message: {e}")));
                    tracing::error!(error = e.to_string(), "failed to send message");
                }
            }
        }
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

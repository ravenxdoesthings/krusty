use std::{
    env,
    sync::{Arc, Mutex},
    time::Duration,
    vec,
};
use twilight_http::Client;
use twilight_model::{
    channel::message::{Embed, embed::EmbedThumbnail},
    id::{Id, marker::ChannelMarker},
};
use uuid::Uuid;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize the tracing subscriber.
    tracing_subscriber::fmt::init();

    let client = Client::new(env::var("DISCORD_TOKEN")?);
    let channel_id = 141_875_592_304_800_9859;

    let sender = Sender::new(client, channel_id);

    let client = reqwest::Client::builder()
        .user_agent("krusty/0.1")
        .build()?;

    let queue_id = format!("krusty-{}", Uuid::new_v4());
    loop {
        let response: serde_json::Value = client
            .clone()
            .get(format!(
                "https://zkillredisq.stream/listen.php?queueID={queue_id}&"
            ))
            .send()
            .await?
            .json()
            .await?;

        // Add caching here later

        // Figure out if kill is relevant (corp ids, etc.)

        let url = format!(
            "https://zkillboard.com/kill/{}",
            response["package"]["killID"]
        );

        match sender.embed(url).await {
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
        Self {
            client: Arc::new(Mutex::new(client)),
            channel_id: Id::new(channel_id),
        }
    }

    async fn embed(&self, url: String) -> Result<(), anyhow::Error> {
        let client = self.client.lock().unwrap();

        let meta = Meta::from_url(url)?;

        client
            .create_message(self.channel_id)
            .embeds(&[Embed {
                author: None,
                color: Some(0x93c47d),
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

    async fn send(&self, content: &str) -> Result<(), twilight_http::Error> {
        let client = self.client.lock().unwrap();
        client
            .create_message(self.channel_id.clone())
            .content(&content)
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
                    .get(0)
                    .map_or("".to_string(), |img| img.url.clone()),
                width: 64,
                height: 64,
            },
        })
    }
}

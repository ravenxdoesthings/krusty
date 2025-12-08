use opentelemetry::trace::Status;
use std::{env, sync::Arc, time::Duration};
use tracing::{Level, Span};
use tracing_opentelemetry::OpenTelemetrySpanExt;

use krusty::{config, discord, filters, otel, persistence, zkb};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let discord_token = env::var("DISCORD_TOKEN")?;
    let config_path = env::var("CONFIG_PATH").unwrap_or("./config.yaml".to_string());

    let mut config = config::Config::load(config_path);
    let queue_id = config.queue_id();

    let _guard = otel::init_tracing_subscriber(queue_id.as_str());

    match &config.filters {
        Some(exp) => {
            tracing::info!(filter_sets = exp.filter_sets.len(), "loaded filters");
        }
        None => {
            tracing::info!("no filters loaded");
        }
    }

    let cache = persistence::cache::Cache::build(config.redis_url())?;
    let persistence = Arc::new(persistence::memory::Store::new());

    let discord = match discord::Gateway::build(persistence, discord_token).await {
        Ok(gateway) => gateway,
        Err(e) => {
            tracing::error!(error = e.to_string(), "failed to build Discord gateway");
            return Err(e);
        }
    };

    let version = env!("CARGO_PKG_VERSION");
    let client = reqwest::Client::builder()
        .user_agent(format!("krusty/{version}"))
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
            tracing::debug!("dropped empty killmail");
            continue;
        };

        if let Err(e) = killmail.fetch_data().await {
            request_span.set_status(Status::error(format!("failed to fetch killmail data: {e}")));
            tracing::error!(error = e.to_string(), "failed to fetch killmail data");
            continue;
        }

        if killmail.killmail.is_none() {
            request_span.set_status(Status::Ok);
            tracing::debug!("dropped null killmail");
            continue;
        }

        let time_divergence = killmail.skew();

        let filter_config: &mut filters::Config = match &mut config.filters {
            Some(exp) => exp,
            None => {
                tracing::warn!("no filter config found, skipping killmail");
                continue;
            }
        };

        let channels = filter_config.filter(&killmail);

        tracing::info!(
            channel_len = channels.len(),
            time_divergence_s = format!("{}", time_divergence.num_seconds()),
            time_divergence_ms = format!("{}", time_divergence.num_milliseconds()),
            time_divergence_m = format!("{}", time_divergence.num_minutes()),
            "ran killmail through filters"
        );

        // We don't have to send anything
        if channels.is_empty() {
            continue;
        }

        for (channel_id, side) in channels {
            tracing::info!(channel_id, "matched filter");
            let cache_key = format!("kill:{channel_id}:{}", killmail.kill_id);
            if let Ok(hit) = cache.check(&cache_key)
                && hit
            {
                continue;
            }

            if let Err(e) = cache.store(&cache_key, Some(Duration::from_secs(10800))) {
                tracing::error!(error = e.to_string(), "failed to store killmail in cache");
            }

            match discord
                .embed(&request_span, &killmail, channel_id, side)
                .await
            {
                Ok(_) => {}
                Err(e) => {
                    tracing::error!(error = e.to_string(), "failed to embed killmail");
                    request_span
                        .set_status(Status::error(format!("failed to embed killmail: {e}")));
                }
            }
        }

        request_span.set_status(Status::Ok);
    }
    // Ok(())
}

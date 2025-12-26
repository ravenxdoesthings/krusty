use opentelemetry::trace::Status;
use std::{env, sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;
use tracing::{Level, Span};
use tracing_opentelemetry::OpenTelemetrySpanExt;

use krusty::{
    config, discord,
    filters::{self, FilterSet},
    otel,
    persistence::{self, Store},
    zkb,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let discord_token = env::var("DISCORD_TOKEN")?;
    let config_path = env::var("CONFIG_PATH").unwrap_or("./config.yaml".to_string());

    rustls::crypto::ring::default_provider()
        .install_default()
        .unwrap();

    let config = config::Config::load(config_path);

    let queue_id = config.queue_id();

    let _guard = otel::init_tracing_subscriber(queue_id.as_str());

    let cache = persistence::cache::Cache::build(config.redis_url())?;
    let persistence = Arc::new(persistence::provider::postgres::Store::new(
        config.postgres_url().as_str(),
    )?);

    let discord = match discord::Gateway::build(&config, persistence.clone(), discord_token).await {
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

    let cancel_token = CancellationToken::new();
    let cancel_token_clone = cancel_token.clone();

    let inner_discord = discord.clone();
    let main_loop = tokio::spawn(async move {
        let mut running = false;
        let discord = inner_discord;
        let persistence = Arc::clone(&persistence);
        loop {
            if running {
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_secs(2)) => {}
                    _ = cancel_token_clone.cancelled() => {
                        tracing::info!("shutdown signal received, exiting main loop");
                        break;
                    }
                }
            }
            running = true;

            if cancel_token_clone.is_cancelled() {
                break;
            }

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
                    Ok(raw) => {
                        match simd_json::from_slice::<zkb::Response>(&mut raw.clone().into_bytes())
                        {
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
                        }
                    }
                    Err(e) => {
                        request_span.set_status(Status::error(format!(
                            "failed to parse response JSON: {e}"
                        )));
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
                request_span
                    .set_status(Status::error(format!("failed to fetch killmail data: {e}")));
                tracing::error!(error = e.to_string(), "failed to fetch killmail data");
                continue;
            }

            if killmail.killmail.is_none() {
                request_span.set_status(Status::Ok);
                tracing::debug!("dropped null killmail");
                continue;
            }

            let time_divergence = killmail.skew();

            let filter_sets = match persistence.list_filter_sets() {
                Ok(sets) => sets,
                Err(e) => {
                    request_span
                        .set_status(Status::error(format!("failed to get filter sets: {e}")));
                    tracing::error!(error = e.to_string(), "failed to get filter sets");
                    continue;
                }
            }
            .into_iter()
            .collect::<Vec<FilterSet>>();

            if filter_sets.is_empty() {
                request_span.set_status(Status::Ok);
                tracing::debug!("no filter sets found, skipping killmail");
                continue;
            }

            let mut filter_config = filters::Config {
                filter_sets,
                ..Default::default()
            };

            let channels = match filter_config.filter(&killmail) {
                Ok(channels) => channels,
                Err(e) => {
                    request_span
                        .set_status(Status::error(format!("failed to filter killmail: {e}")));
                    tracing::error!(error = e.to_string(), "failed to filter killmail");
                    continue;
                }
            };

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

            let cache_key = format!("kill:global:{}", killmail.kill_id);
            let cache_hit = cache.check(&cache_key).unwrap_or_default();

            if !cache_hit {
                if let Err(e) = persistence.add_analytics_data(&killmail.zkb) {
                    tracing::error!(error = e.to_string(), "failed to add analytics data");
                };

                if let Err(e) = cache.store(&cache_key, Some(Duration::from_secs(10800))) {
                    tracing::error!(error = e.to_string(), "failed to store killmail in cache");
                }
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
    });

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("received Ctrl+C, initiating graceful shutdown");
            cancel_token.cancel();
        }
        result = main_loop => {
            if let Err(e) = result {
                tracing::error!(error = %e, "main loop task panicked");
            }
        }
    }

    let _ = discord.shutdown().await;

    tracing::info!("shutdown complete");
    Ok(())
}

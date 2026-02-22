use opentelemetry::trace::Status;
use std::{collections::HashMap, env, sync::Arc, time::Duration};
use tokio_util::sync::CancellationToken;
use tracing::{Level, Span};
use tracing_opentelemetry::OpenTelemetrySpanExt;

use krusty::{
    config, discord,
    filters::{self, FilterSet},
    otel,
    persistence::{self, Store},
    zkb::{SequenceNotFoundError, SequenceTracker},
};

const NORMAL_WAIT_TIME_MS: u64 = 100;
const ERROR_WAIT_TIME_MS: u64 = 6_000;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let discord_token = env::var("DISCORD_TOKEN")?;
    let config_path = env::var("CONFIG_PATH").unwrap_or("./config.yaml".to_string());

    rustls::crypto::ring::default_provider()
        .install_default()
        .unwrap();

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
    let persistence = Arc::new(persistence::provider::redis::Store::new(
        config.redis_url().as_str(),
    )?);

    import_filters_from_config(&mut config, persistence.clone()).await;

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
    let sequence_tracker = Arc::new(tokio::sync::Mutex::new(SequenceTracker::new()));
    match sequence_tracker.lock().await.fetch_starting(&client).await {
        Ok(_) => {}
        Err(e) => {
            tracing::error!(error = e.to_string(), "failed to get starting sequence");
            return Err(e.into());
        }
    };

    tracing::debug!(
        sequence = sequence_tracker.lock().await.sequence,
        "got starting sequence"
    );

    let cancel_token = CancellationToken::new();
    let cancel_token_clone = cancel_token.clone();

    let inner_discord = discord.clone();

    let mut r2z2_loop = tokio::spawn(async move {
        let mut running = false;
        let mut wait_ms = NORMAL_WAIT_TIME_MS;
        let discord = inner_discord;
        loop {
            if running {
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_millis(wait_ms)) => {}
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

            let request_span: Span = tracing::span!(Level::INFO, "fetching next killmail");
            let _enter = request_span.enter();

            let killmail = match sequence_tracker
                .lock()
                .await
                .fetch_next(Some(&request_span), &client)
                .await
            {
                Ok(killmail) => killmail,
                Err(e) => {
                    request_span
                        .set_status(Status::error(format!("fetch request returned error: {e}")));

                    // If there's no killmail that's not really an error.
                    if let Some(not_found) = e.downcast_ref::<SequenceNotFoundError>() {
                        tracing::warn!(
                            sequence_id = not_found.sequence,
                            "no new killmail found for sequence"
                        );
                    } else {
                        tracing::error!(error = e.to_string(), "failed to fetch next killmail");
                    }

                    // In case of an error bump the wait time.
                    wait_ms = ERROR_WAIT_TIME_MS;
                    continue;
                }
            };
            // If there was no error set the wait time to normal.
            wait_ms = NORMAL_WAIT_TIME_MS;

            let time_divergence = killmail.skew();
            tracing::info!(
                killmail_id = killmail.killmail_id,
                sequence_id = killmail.sequence_id,
                time_divergence_s = format!("{}", time_divergence.num_seconds()),
                time_divergence_ms = format!("{}", time_divergence.num_milliseconds()),
                time_divergence_m = format!("{}", time_divergence.num_minutes()),
                "fetched killmail from zkb"
            );

            let filter_sets = match persistence.list_filter_sets() {
                Ok(sets) => sets,
                Err(e) => {
                    request_span
                        .set_status(Status::error(format!("failed to get filter sets: {e}")));
                    tracing::error!(error = e.to_string(), "failed to get filter sets");
                    continue;
                }
            };

            if filter_sets.is_empty() {
                request_span.set_status(Status::Ok);
                tracing::debug!("no filter sets found, skipping killmail");
                continue;
            }

            let mut filter_config = filters::Config {
                filter_sets,
                ..Default::default()
            };

            let channels = match filter_config.filter_v3(&killmail) {
                Ok(channels) => channels,
                Err(e) => {
                    request_span
                        .set_status(Status::error(format!("failed to filter killmail: {e}")));
                    tracing::error!(error = e.to_string(), "failed to filter killmail");
                    continue;
                }
            };

            tracing::info!(
                killmail_id = killmail.killmail_id,
                sequence_id = killmail.sequence_id,
                channel_len = channels.len(),
                time_divergence_s = format!("{}", time_divergence.num_seconds()),
                time_divergence_ms = format!("{}", time_divergence.num_milliseconds()),
                time_divergence_m = format!("{}", time_divergence.num_minutes()),
                "ran killmail through filters"
            );

            if channels.is_empty() {
                request_span.set_status(Status::Ok);
                tracing::debug!("no filters matched for killmail, skipping");
                continue;
            }

            let mut cache_hits: HashMap<String, bool> = HashMap::new();

            for (channel_id, side) in channels {
                tracing::info!(channel_id, "matched filter");
                let cache_key = format!("kill:{channel_id}:{}", killmail.killmail_id);
                if let Ok(hit) = cache.check(&cache_key)
                    && hit
                {
                    cache_hits.insert(channel_id.to_string(), true);
                    continue;
                }

                cache_hits.insert(channel_id.to_string(), true);
                if let Err(e) = cache.store(&cache_key, Some(Duration::from_secs(10800))) {
                    tracing::error!(error = e.to_string(), "failed to store killmail in cache");
                }

                match discord.embed_v3(None, &killmail, channel_id, side).await {
                    Ok(_) => {}
                    Err(e) => {
                        tracing::error!(error = e.to_string(), "failed to embed killmail");
                    }
                }
            }

            request_span.set_attribute("cache_hits", format!("{:?}", cache_hits));
        }
    });

    tokio::select! {
        _ = tokio::signal::ctrl_c() => {
            tracing::info!("received Ctrl+C, initiating graceful shutdown");
            cancel_token.cancel();
        }
        result = &mut r2z2_loop => {
            if let Err(e) = result {
                tracing::error!(error = %e, "main loop task panicked");
            }
        }
    }

    // Wait for the task to complete if it hasn't already
    if !r2z2_loop.is_finished() {
        match r2z2_loop.await {
            Ok(_) => {}
            Err(e) => {
                tracing::error!(error = %e, "main loop task panicked");
            }
        }
    }

    let _ = discord.shutdown().await;

    tracing::info!("shutdown complete");
    Ok(())
}

async fn import_filters_from_config(
    config: &mut config::Config,
    persistence: Arc<dyn persistence::Store>,
) {
    for config_filters in config.filters.iter_mut() {
        for filter_set in &config_filters.filter_sets {
            let persistence = &persistence;

            let set = FilterSet {
                guild_id: filter_set.guild_id,
                channel_id: filter_set.channel_id,
                filters: filter_set.filters.clone(),
            };

            if let Err(e) = &persistence.set_filter_set(set) {
                tracing::error!(error = e.to_string(), "failed to add filter set");
            }
        }
    }
}

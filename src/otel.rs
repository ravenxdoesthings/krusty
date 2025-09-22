use opentelemetry::{KeyValue, trace::TracerProvider as _};
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{
    Resource,
    logs::{LoggerProviderBuilder, SdkLoggerProvider},
    trace::{RandomIdGenerator, SdkTracerProvider},
};
use opentelemetry_semantic_conventions::{SCHEMA_URL, attribute::SERVICE_VERSION};
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::{EnvFilter, Layer, layer::SubscriberExt, util::SubscriberInitExt};

fn resource() -> Resource {
    Resource::builder()
        .with_service_name("krusty")
        .with_schema_url(
            vec![KeyValue::new(SERVICE_VERSION, env!("CARGO_PKG_VERSION"))],
            SCHEMA_URL,
        )
        .build()
}

fn init_logger_provider() -> SdkLoggerProvider {
    let exporter = opentelemetry_otlp::LogExporter::builder()
        .with_http()
        .build()
        .unwrap();

    LoggerProviderBuilder::default()
        .with_resource(resource())
        .with_batch_exporter(exporter)
        .build()
}

fn init_tracer_provider() -> SdkTracerProvider {
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_http()
        .with_protocol(opentelemetry_otlp::Protocol::Grpc)
        .build()
        .unwrap();

    SdkTracerProvider::builder()
        .with_id_generator(RandomIdGenerator::default())
        .with_resource(resource())
        .with_batch_exporter(exporter)
        .build()
}

pub fn init_tracing_subscriber() -> OtelGuard {
    let tracer_provider = init_tracer_provider();
    let logger_provider = init_logger_provider();

    let tracer = tracer_provider.tracer("tracing-otel-subscriber");

    let otel_layer = OpenTelemetryTracingBridge::new(&logger_provider);

    let filter_otel = EnvFilter::new("warn")
        .add_directive("hyper=off".parse().unwrap())
        .add_directive("tonic=off".parse().unwrap())
        .add_directive("h2=off".parse().unwrap())
        .add_directive("reqwest=off".parse().unwrap())
        .add_directive("krusty=debug".parse().unwrap());
    let otel_layer = otel_layer.with_filter(filter_otel);

    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .with(OpenTelemetryLayer::new(tracer))
        .with(otel_layer)
        .with(
            EnvFilter::from_default_env()
                .add_directive("warn".parse().unwrap())
                .add_directive("krusty=debug".parse().unwrap()),
        )
        .init();

    OtelGuard { tracer_provider }
}

pub struct OtelGuard {
    tracer_provider: SdkTracerProvider,
}

impl Drop for OtelGuard {
    fn drop(&mut self) {
        if let Err(err) = self.tracer_provider.shutdown() {
            eprintln!("{err:?}");
        }
    }
}

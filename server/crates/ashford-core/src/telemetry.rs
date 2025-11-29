use std::fmt as StdFmt;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

use chrono::Utc;
use opentelemetry::KeyValue;
use opentelemetry::global;
use opentelemetry::trace::{TraceContextExt, TracerProvider};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::trace::{BatchSpanProcessor, SdkTracerProvider};
use opentelemetry_sdk::{Resource, trace};
use serde_json::json;
use thiserror::Error;
use tracing::Subscriber;
use tracing_opentelemetry::{OpenTelemetrySpanExt, OtelData};
use tracing_subscriber::fmt::format::{FormatEvent, FormatFields, Writer};
use tracing_subscriber::fmt::{self, FmtContext};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::reload;
use tracing_subscriber::{EnvFilter, Registry};

use crate::config::{AppConfig, TelemetryConfig};

struct TelemetryHandles {
    install_otel: Box<dyn Fn(trace::Tracer) -> Result<(), TelemetryError> + Send + Sync + 'static>,
}

impl TelemetryHandles {
    fn install_tracer(&self, tracer: trace::Tracer) -> Result<(), TelemetryError> {
        (self.install_otel)(tracer)
    }
}

static TELEMETRY_HANDLES: OnceLock<TelemetryHandles> = OnceLock::new();
static GLOBAL_GUARD: OnceLock<Mutex<Option<TelemetryGuard>>> = OnceLock::new();

/// Guard that owns the tracer provider so spans are flushed on drop.
#[derive(Clone)]
pub struct TelemetryGuard {
    provider: Option<Arc<SdkTracerProvider>>,
    shutdown_on_drop: bool,
}

impl Drop for TelemetryGuard {
    fn drop(&mut self) {
        if self.shutdown_on_drop {
            if let Some(provider) = self.provider.take() {
                // Only shut down when this is the last guard holding the provider.
                if Arc::strong_count(&provider) == 1 {
                    let _ = provider.shutdown();
                }
            }
        }
    }
}

#[derive(Debug, Error)]
pub enum TelemetryError {
    #[error("failed to set tracing subscriber: {0}")]
    SubscriberInit(String),
    #[error("failed to build OTLP exporter: {0}")]
    ExporterBuild(String),
}

/// Initialize structured logging (RUST_LOG driven) and optional OpenTelemetry tracing.
/// JSON output is used for production; pretty output for dev.
pub fn init_telemetry(
    app: &AppConfig,
    telemetry: &TelemetryConfig,
) -> Result<TelemetryGuard, TelemetryError> {
    let env_filter = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .map_err(|err| TelemetryError::SubscriberInit(err.to_string()))?;

    let (tracer, provider) = build_tracer(app, telemetry)?;

    let json_format = !app.env.eq_ignore_ascii_case("dev");
    if json_format {
        let fmt_layer = fmt::layer().event_format(JsonTraceFormatter::default());
        install_subscriber(fmt_layer, env_filter, tracer)?;
    } else {
        let fmt_layer = fmt::layer()
            .with_target(true)
            .pretty()
            .with_writer(std::io::stderr);
        install_subscriber(fmt_layer, env_filter, tracer)?;
    }

    let guard = provider
        .as_ref()
        .map(|provider| retain_provider(provider.clone()))
        .unwrap_or_else(|| TelemetryGuard {
            provider: None,
            shutdown_on_drop: false,
        });

    Ok(guard)
}

fn build_tracer(
    app: &AppConfig,
    telemetry: &TelemetryConfig,
) -> Result<(Option<trace::Tracer>, Option<Arc<SdkTracerProvider>>), TelemetryError> {
    if !telemetry.export_traces {
        return Ok((None, None));
    }

    let endpoint = match telemetry.otlp_endpoint.as_deref() {
        Some(endpoint) if !endpoint.is_empty() => endpoint,
        _ => return Ok((None, None)),
    };

    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_http()
        .with_timeout(Duration::from_secs(3))
        .with_endpoint(endpoint)
        .build()
        .map_err(|err| TelemetryError::ExporterBuild(err.to_string()))?;

    let resource = Resource::builder()
        .with_attributes([
            KeyValue::new("service.name", app.service_name.clone()),
            KeyValue::new("service.version", env!("CARGO_PKG_VERSION")),
            KeyValue::new("deployment.environment", app.env.clone()),
            KeyValue::new("host.arch", std::env::consts::ARCH.to_string()),
            KeyValue::new("os.type", std::env::consts::OS.to_string()),
        ])
        .build();
    let provider = SdkTracerProvider::builder()
        .with_span_processor(BatchSpanProcessor::builder(exporter).build())
        .with_resource(resource)
        .build();

    let provider = Arc::new(provider);

    global::set_tracer_provider(provider.as_ref().clone());
    global::set_text_map_propagator(opentelemetry_sdk::propagation::TraceContextPropagator::new());

    let tracer = provider.tracer(app.service_name.clone());

    Ok((Some(tracer), Some(provider)))
}

fn retain_provider(provider: Arc<SdkTracerProvider>) -> TelemetryGuard {
    let local_guard = TelemetryGuard {
        provider: Some(provider.clone()),
        shutdown_on_drop: false,
    };

    let global_guard = TelemetryGuard {
        provider: Some(provider),
        shutdown_on_drop: true,
    };

    let slot = GLOBAL_GUARD.get_or_init(|| Mutex::new(None));
    *slot.lock().expect("lock global guard") = Some(global_guard);

    local_guard
}

fn install_subscriber(
    fmt_layer: impl tracing_subscriber::Layer<Registry> + Send + Sync + 'static,
    env_filter: EnvFilter,
    tracer: Option<trace::Tracer>,
) -> Result<(), TelemetryError> {
    if let Some(handles) = TELEMETRY_HANDLES.get() {
        if let Some(tracer) = tracer {
            handles.install_tracer(tracer)?;
        }
        return Ok(());
    }

    let (otel_layer, otel_reload) =
        reload::Layer::new(tracer.map(|tracer| tracing_opentelemetry::layer().with_tracer(tracer)));

    let subscriber = Registry::default()
        .with(fmt_layer)
        .with(otel_layer)
        .with(env_filter);

    tracing::subscriber::set_global_default(subscriber)
        .map_err(|err| TelemetryError::SubscriberInit(err.to_string()))?;

    let handles = TelemetryHandles {
        install_otel: Box::new(move |tracer: trace::Tracer| {
            otel_reload
                .modify(|layer| {
                    *layer = Some(tracing_opentelemetry::layer().with_tracer(tracer));
                })
                .map_err(|err| TelemetryError::SubscriberInit(err.to_string()))
        }),
    };
    let _ = TELEMETRY_HANDLES.set(handles);

    Ok(())
}

/// Basic logging initializer for early binaries/tests that do not yet wire full config.
pub fn init_logging(env: &str) -> Result<(), TelemetryError> {
    let dummy_app = AppConfig {
        service_name: "ashford".to_string(),
        port: 0,
        env: env.to_string(),
    };
    let telemetry = TelemetryConfig {
        otlp_endpoint: None,
        export_traces: false,
    };
    init_telemetry(&dummy_app, &telemetry).map(|_guard| ())
}

#[derive(Default)]
struct JsonTraceFormatter;

impl<S, N> FormatEvent<S, N> for JsonTraceFormatter
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'writer> FormatFields<'writer> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: Writer<'_>,
        event: &tracing::Event<'_>,
    ) -> StdFmt::Result {
        let mut visitor = JsonVisitor::default();
        event.record(&mut visitor);

        let trace_id = ctx
            .lookup_current()
            .and_then(|span| {
                span.extensions()
                    .get::<OtelData>()
                    .and_then(|data| data.trace_id())
            })
            .or_else(|| {
                let ctx = tracing::Span::current().context();
                let span_ctx = ctx.span().span_context().clone();
                if span_ctx.is_valid() {
                    Some(span_ctx.trace_id())
                } else {
                    None
                }
            })
            .map(|id| id.to_string());

        let span_name = ctx.lookup_current().map(|span| span.name().to_string());

        let payload = json!({
            "timestamp": Utc::now().to_rfc3339(),
            "level": event.metadata().level().as_str().to_lowercase(),
            "target": event.metadata().target(),
            "span": span_name,
            "trace_id": trace_id,
            "fields": visitor.fields,
        });

        let serialized = serde_json::to_string(&payload).map_err(|_| StdFmt::Error)?;
        writer.write_str(&serialized)?;
        writer.write_str("\n")
    }
}

#[derive(Default)]
struct JsonVisitor {
    fields: serde_json::Map<String, serde_json::Value>,
}

impl tracing_subscriber::field::Visit for JsonVisitor {
    fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
        self.fields.insert(field.name().to_string(), value.into());
    }

    fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
        self.fields.insert(field.name().to_string(), value.into());
    }

    fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
        self.fields.insert(field.name().to_string(), value.into());
    }

    fn record_i128(&mut self, field: &tracing::field::Field, value: i128) {
        self.fields
            .insert(field.name().to_string(), value.to_string().into());
    }

    fn record_u128(&mut self, field: &tracing::field::Field, value: u128) {
        self.fields
            .insert(field.name().to_string(), value.to_string().into());
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        self.fields
            .insert(field.name().to_string(), value.to_string().into());
    }

    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn StdFmt::Debug) {
        self.fields
            .insert(field.name().to_string(), format!("{value:?}").into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use once_cell::sync::Lazy;
    use std::io::{self, Write};
    use std::sync::{Arc, Mutex};

    static INIT_GUARD: Lazy<Mutex<()>> = Lazy::new(|| Mutex::new(()));

    #[test]
    fn telemetry_init_is_idempotent_and_handles_missing_endpoint() {
        let _guard = INIT_GUARD.lock().expect("lock init");
        let app = AppConfig {
            service_name: "ashford".into(),
            port: 0,
            env: "prod".into(),
        };
        let telemetry = TelemetryConfig {
            otlp_endpoint: None,
            export_traces: true,
        };

        init_telemetry(&app, &telemetry).expect("telemetry initializes without endpoint");
        init_telemetry(&app, &telemetry).expect("second init is a no-op");
    }

    #[test]
    fn json_formatter_includes_trace_id_and_fields() {
        #[derive(Clone)]
        struct BufferWriter {
            buf: Arc<Mutex<Vec<u8>>>,
        }

        impl Write for BufferWriter {
            fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
                let mut locked = self.buf.lock().expect("lock buffer");
                locked.extend_from_slice(buf);
                Ok(buf.len())
            }

            fn flush(&mut self) -> io::Result<()> {
                Ok(())
            }
        }

        let buffer = Arc::new(Mutex::new(Vec::new()));
        let writer = {
            let buffer = buffer.clone();
            move || BufferWriter {
                buf: buffer.clone(),
            }
        };

        let provider = opentelemetry_sdk::trace::SdkTracerProvider::builder().build();
        let tracer = provider.tracer("test");

        let fmt_layer = fmt::layer()
            .event_format(JsonTraceFormatter::default())
            .with_writer(writer);

        let subscriber = Registry::default()
            .with(fmt_layer)
            .with(tracing_opentelemetry::layer().with_tracer(tracer));

        let recorded_trace_id: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

        tracing::subscriber::with_default(subscriber, || {
            let span = tracing::info_span!("sample_span");
            let trace_id = span.context().span().span_context().trace_id();
            *recorded_trace_id.lock().expect("lock trace id") = Some(trace_id.to_string());

            let _guard = span.enter();
            let current_ctx = tracing::Span::current().context();
            assert!(
                current_ctx.span().span_context().is_valid(),
                "current span should carry an OpenTelemetry span context"
            );
            tracing::info!(foo = 42, message = "hello world");
        });

        let output =
            String::from_utf8(buffer.lock().expect("lock buffer").clone()).expect("utf8 output");
        let line = output.lines().next().expect("log line");
        let payload: serde_json::Value = serde_json::from_str(line).expect("json line");

        assert_eq!(payload["level"], "info");
        assert_eq!(payload["span"], "sample_span");
        assert_eq!(payload["fields"]["foo"], 42);
        assert_eq!(payload["fields"]["message"], "hello world");

        let trace_id = recorded_trace_id
            .lock()
            .expect("lock trace id")
            .clone()
            .expect("trace id present");
        assert_eq!(payload["trace_id"], trace_id);

        let ts = payload["timestamp"]
            .as_str()
            .expect("timestamp string present");
        let looks_rfc3339 = ts.contains('T') && (ts.ends_with('Z') || ts.ends_with("+00:00"));
        assert!(looks_rfc3339, "timestamp should be RFC3339, got {ts}");
    }
}

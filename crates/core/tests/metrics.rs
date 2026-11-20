//! The metrics a registry records, read back through the OpenTelemetry SDK. This test has a
//! binary of its own: the instruments are made once per process, from the meter provider installed
//! at the first poll.

use opentelemetry::{KeyValue, global};
use opentelemetry_sdk::metrics::{
    InMemoryMetricExporter, PeriodicReader, SdkMeterProvider,
    data::{AggregatedMetrics, MetricData},
};
use sisa_health_checks_core::{HealthRegistry, HealthResult, Status};

/// One exported point as `name{key=value,...} value`, with the attributes in order.
fn line<'a>(
    name: &str,
    attributes: impl Iterator<Item = &'a KeyValue>,
    value: impl std::fmt::Display,
) -> String {
    let mut attributes: Vec<String> = attributes
        .map(|attribute| format!("{}={}", attribute.key, attribute.value))
        .collect();

    attributes.sort();

    format!("{name}{{{}}} {value}", attributes.join(","))
}

/// Every exported point, in order. A histogram's value is how many runs it counted.
fn points(exporter: &InMemoryMetricExporter) -> Vec<String> {
    let exported = exporter
        .get_finished_metrics()
        .expect("the exporter keeps its metrics in memory");
    let mut points = Vec::new();

    for metric in exported
        .iter()
        .flat_map(|resource| resource.scope_metrics())
        .flat_map(|scope| scope.metrics())
    {
        match metric.data() {
            AggregatedMetrics::F64(MetricData::Gauge(gauge)) => {
                points.extend(
                    gauge
                        .data_points()
                        .map(|point| line(metric.name(), point.attributes(), point.value())),
                );
            }
            AggregatedMetrics::F64(MetricData::Histogram(histogram)) => {
                points.extend(
                    histogram
                        .data_points()
                        .map(|point| line(metric.name(), point.attributes(), point.count())),
                );
            }
            other => panic!("the registry records only a gauge and a histogram, not {other:?}"),
        }
    }

    points.sort();
    points
}

#[tokio::test]
async fn a_run_records_each_checks_status_and_duration() {
    let exporter = InMemoryMetricExporter::default();
    let provider = SdkMeterProvider::builder()
        .with_reader(PeriodicReader::builder(exporter.clone()).build())
        .build();

    global::set_meter_provider(provider.clone());

    let health = HealthRegistry::builder()
        .check_fn("postgres", async || HealthResult::healthy())
        .check_fn("cache", async || HealthResult::unhealthy("down"))
        .failure(Status::Degraded)
        .build()
        .expect("the test builds a valid registry");

    health.run().await;
    provider
        .force_flush()
        .expect("the exporter keeps its metrics in memory");

    assert_eq!(
        points(&exporter),
        [
            "sisa.health.check.duration{check=cache,status=degraded} 1",
            "sisa.health.check.duration{check=postgres,status=healthy} 1",
            "sisa.health.status{check=cache} 0.5",
            "sisa.health.status{check=postgres} 1",
        ]
    );
}

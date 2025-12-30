use axum::{
    body::Body,
    extract::MatchedPath,
    http::{header, HeaderValue, Request, StatusCode},
    middleware::Next,
    response::Response,
};
use prometheus::{
    Encoder, HistogramOpts, HistogramVec, IntCounterVec, IntGaugeVec, Opts, Registry, TextEncoder,
};
use std::time::Instant;

pub struct Metrics {
    registry: Registry,
    req_total: IntCounterVec,
    req_duration: HistogramVec,
    in_flight: IntGaugeVec,
}

impl Metrics {
    pub fn new() -> Self {
        let registry = Registry::new();

        let req_total = IntCounterVec::new(
            Opts::new(
                "http_requests_received_total",
                "Total HTTP requests received",
            ),
            &["method", "path", "code"],
        )
        .expect("counter");

        let req_duration = {
            let opts = HistogramOpts::new(
                "http_request_duration_seconds",
                "HTTP request duration in seconds",
            )
            .buckets(vec![
                0.0005, 0.001, 0.002, 0.005, 0.01, 0.02, 0.05, 0.1, 0.2, 0.5, 1.0,
            ]);

            HistogramVec::new(opts, &["method", "path", "code"]).expect("histogram")
        };

        let in_flight = IntGaugeVec::new(
            Opts::new(
                "http_requests_in_progress",
                "HTTP requests currently in progress",
            ),
            &["path"],
        )
        .expect("gauge");

        registry.register(Box::new(req_total.clone())).unwrap();
        registry.register(Box::new(req_duration.clone())).unwrap();
        registry.register(Box::new(in_flight.clone())).unwrap();

        Self {
            registry,
            req_total,
            req_duration,
            in_flight,
        }
    }

    fn path_label(req: &Request<Body>) -> String {
        
        if let Some(matched) = req.extensions().get::<MatchedPath>() {
            matched.as_str().to_string()
        } else {
            req.uri().path().to_string()
        }
    }

    pub fn middleware(&self, method: &str, path: &str, code: &str, seconds: f64) {
        self.req_total
            .with_label_values(&[method, path, code])
            .inc();
        self.req_duration
            .with_label_values(&[method, path, code])
            .observe(seconds);
    }

    pub fn inc_in_flight(&self, path: &str) {
        self.in_flight.with_label_values(&[path]).inc();
    }

    pub fn dec_in_flight(&self, path: &str) {
        self.in_flight.with_label_values(&[path]).dec();
    }

    pub fn render(&self) -> (String, Vec<u8>) {
        let metric_families = self.registry.gather();
        let encoder = TextEncoder::new();
        let mut buf = Vec::new();
        encoder.encode(&metric_families, &mut buf).unwrap();
        (encoder.format_type().to_string(), buf)
    }

    pub fn response(&self) -> Response {
        let (content_type, bytes) = self.render();

        let mut res = Response::new(Body::from(bytes));
        *res.status_mut() = StatusCode::OK;
        res.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_str(&content_type).unwrap(),
        );
        res
    }
}


pub async fn metrics_middleware(
    axum::extract::State(metrics): axum::extract::State<std::sync::Arc<Metrics>>,
    req: Request<Body>,
    next: Next,
) -> Response {
    let method = req.method().to_string();
    let path = Metrics::path_label(&req);

    // optional: do not measure /metrics or /health to avoid contaminating RPS/latency
    if path == "/metrics" || path == "/health" {
        return next.run(req).await;
    }

    metrics.inc_in_flight(&path);
    let start = Instant::now();
    let mut res = next.run(req).await;
    let elapsed = start.elapsed().as_secs_f64();

    let code = res.status().as_u16().to_string();
    metrics.middleware(&method, &path, &code, elapsed);

    metrics.dec_in_flight(&path);

    res.headers_mut()
        .insert("x-service", HeaderValue::from_static("rust-axum"));

    res
}

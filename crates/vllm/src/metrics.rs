//! Metrics API for vLLM (Prometheus format).

use crate::client::VllmClient;
use crate::constants::endpoints;
use crate::error::Result;

/// Client for metrics.
#[derive(Debug)]
pub struct Metrics<'a> {
    client: &'a VllmClient,
}

impl<'a> Metrics<'a> {
    /// Create a new metrics client.
    pub fn new(client: &'a VllmClient) -> Self {
        Self { client }
    }

    /// Get raw Prometheus metrics.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use vllm::VllmClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = VllmClient::new("http://localhost:8000")?;
    ///
    /// let metrics = client.metrics().get().await?;
    /// println!("{}", metrics);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get(&self) -> Result<String> {
        self.client
            .execute_with_retry(|| {
                let client = self.client.clone();
                Box::pin(async move {
                    let response = client.get(endpoints::METRICS).await?;
                    let body = client.handle_text_response(response).await?;
                    Ok(body)
                })
            })
            .await
    }

    /// Get parsed metrics.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use vllm::VllmClient;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let client = VllmClient::new("http://localhost:8000")?;
    ///
    /// let info = client.metrics().get_parsed().await?;
    /// println!("GPU cache usage: {:?}", info.gpu_cache_usage_perc);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn get_parsed(&self) -> Result<MetricsInfo> {
        let raw = self.get().await?;
        Ok(MetricsInfo::parse(&raw))
    }
}

/// Parsed metrics information from vLLM.
#[derive(Debug, Clone, Default)]
pub struct MetricsInfo {
    /// Raw metrics text.
    pub raw: String,
    /// GPU cache usage percentage.
    pub gpu_cache_usage_perc: Option<f64>,
    /// CPU cache usage percentage.
    pub cpu_cache_usage_perc: Option<f64>,
    /// Number of prefill tokens total.
    pub num_preemption_total: Option<u64>,
    /// Number of generation tokens total.
    pub generation_tokens_total: Option<u64>,
    /// Number of prompt tokens total.
    pub prompt_tokens_total: Option<u64>,
    /// Time to first token histogram sum.
    pub time_to_first_token_seconds_sum: Option<f64>,
    /// Time to first token histogram count.
    pub time_to_first_token_seconds_count: Option<u64>,
    /// Time per output token histogram sum.
    pub time_per_output_token_seconds_sum: Option<f64>,
    /// Time per output token histogram count.
    pub time_per_output_token_seconds_count: Option<u64>,
    /// Inference latency histogram sum.
    pub inference_latency_seconds_sum: Option<f64>,
    /// Inference latency histogram count.
    pub inference_latency_seconds_count: Option<u64>,
    /// Time to first token histogram buckets.
    pub time_to_first_token_seconds_bucket: Vec<(f64, u64)>,
    /// Time per output token histogram buckets.
    pub time_per_output_token_seconds_bucket: Vec<(f64, u64)>,
    /// All gauge metrics.
    pub gauges: std::collections::HashMap<String, f64>,
    /// All counter metrics.
    pub counters: std::collections::HashMap<String, u64>,
}

impl MetricsInfo {
    /// Parse raw Prometheus metrics.
    pub fn parse(raw: &str) -> Self {
        let mut info = Self {
            raw: raw.to_string(),
            ..Default::default()
        };

        for line in raw.lines() {
            let line = line.trim();

            // Skip comments and empty lines
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            // Parse gauge metrics
            if line.starts_with("vllm:gpu_cache_usage_perc") {
                info.gpu_cache_usage_perc = parse_gauge_value(line);
            } else if line.starts_with("vllm:cpu_cache_usage_perc") {
                info.cpu_cache_usage_perc = parse_gauge_value(line);
            }
            // Parse counter metrics
            else if line.starts_with("vllm:num_preemption_total") && !line.contains("_created") {
                info.num_preemption_total = parse_counter_value(line);
            } else if line.starts_with("vllm:generation_tokens_total") && !line.contains("_created")
            {
                info.generation_tokens_total = parse_counter_value(line);
            } else if line.starts_with("vllm:prompt_tokens_total") && !line.contains("_created") {
                info.prompt_tokens_total = parse_counter_value(line);
            }
            // Parse histogram sums
            else if line.starts_with("vllm:time_to_first_token_seconds_sum") {
                info.time_to_first_token_seconds_sum = parse_gauge_value(line);
            } else if line.starts_with("vllm:time_to_first_token_seconds_count") {
                info.time_to_first_token_seconds_count = parse_counter_value(line);
            } else if line.starts_with("vllm:time_per_output_token_seconds_sum") {
                info.time_per_output_token_seconds_sum = parse_gauge_value(line);
            } else if line.starts_with("vllm:time_per_output_token_seconds_count") {
                info.time_per_output_token_seconds_count = parse_counter_value(line);
            } else if line.starts_with("vllm:inference_latency_seconds_sum") {
                info.inference_latency_seconds_sum = parse_gauge_value(line);
            } else if line.starts_with("vllm:inference_latency_seconds_count") {
                info.inference_latency_seconds_count = parse_counter_value(line);
            }
            // Parse histogram buckets
            else if line.starts_with("vllm:time_to_first_token_seconds_bucket") {
                if let Some(bucket) = parse_histogram_bucket(line) {
                    info.time_to_first_token_seconds_bucket.push(bucket);
                }
            } else if line.starts_with("vllm:time_per_output_token_seconds_bucket") {
                if let Some(bucket) = parse_histogram_bucket(line) {
                    info.time_per_output_token_seconds_bucket.push(bucket);
                }
            }
        }

        info
    }

    /// Calculate average time to first token (TTFT).
    pub fn avg_time_to_first_token(&self) -> Option<f64> {
        if let (Some(sum), Some(count)) = (
            self.time_to_first_token_seconds_sum,
            self.time_to_first_token_seconds_count,
        ) {
            if count > 0 {
                return Some(sum / count as f64);
            }
        }
        None
    }

    /// Calculate average time per output token (TPOT).
    pub fn avg_time_per_output_token(&self) -> Option<f64> {
        if let (Some(sum), Some(count)) = (
            self.time_per_output_token_seconds_sum,
            self.time_per_output_token_seconds_count,
        ) {
            if count > 0 {
                return Some(sum / count as f64);
            }
        }
        None
    }

    /// Calculate average inference latency.
    pub fn avg_inference_latency(&self) -> Option<f64> {
        if let (Some(sum), Some(count)) = (
            self.inference_latency_seconds_sum,
            self.inference_latency_seconds_count,
        ) {
            if count > 0 {
                return Some(sum / count as f64);
            }
        }
        None
    }

    /// Get total tokens processed.
    pub fn total_tokens(&self) -> Option<u64> {
        if let (Some(prompt), Some(generation)) =
            (self.prompt_tokens_total, self.generation_tokens_total)
        {
            Some(prompt + generation)
        } else {
            self.prompt_tokens_total.or(self.generation_tokens_total)
        }
    }
}

impl std::fmt::Display for MetricsInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "vLLM Metrics:")?;
        writeln!(f, "=============")?;

        if let Some(usage) = self.gpu_cache_usage_perc {
            writeln!(f, "GPU Cache Usage: {:.2}%", usage * 100.0)?;
        }

        if let Some(usage) = self.cpu_cache_usage_perc {
            writeln!(f, "CPU Cache Usage: {:.2}%", usage * 100.0)?;
        }

        if let Some(tokens) = self.prompt_tokens_total {
            writeln!(f, "Prompt Tokens: {}", tokens)?;
        }

        if let Some(tokens) = self.generation_tokens_total {
            writeln!(f, "Generation Tokens: {}", tokens)?;
        }

        if let Some(tokens) = self.total_tokens() {
            writeln!(f, "Total Tokens: {}", tokens)?;
        }

        if let Some(ttft) = self.avg_time_to_first_token() {
            writeln!(f, "Avg Time to First Token: {:.3}s", ttft)?;
        }

        if let Some(tpot) = self.avg_time_per_output_token() {
            writeln!(f, "Avg Time per Output Token: {:.3}s", tpot)?;
        }

        if let Some(latency) = self.avg_inference_latency() {
            writeln!(f, "Avg Inference Latency: {:.3}s", latency)?;
        }

        if let Some(preemption) = self.num_preemption_total {
            writeln!(f, "Total Preemptions: {}", preemption)?;
        }

        Ok(())
    }
}

/// Parse a gauge metric value from a Prometheus line.
fn parse_gauge_value(line: &str) -> Option<f64> {
    // Format: metric_name{labels} value
    let parts: Vec<&str> = line.split_whitespace().collect();
    parts.last().and_then(|v| v.parse().ok())
}

/// Parse a counter metric value from a Prometheus line.
fn parse_counter_value(line: &str) -> Option<u64> {
    parse_gauge_value(line).map(|v| v as u64)
}

/// Parse a histogram bucket from a Prometheus line.
fn parse_histogram_bucket(line: &str) -> Option<(f64, u64)> {
    // Format: metric_name_bucket{le="0.005"} 123
    if let Some(le_start) = line.find("le=\"") {
        let le_end = line[le_start + 4..].find('"')? + le_start + 4;
        let le: f64 = line[le_start + 4..le_end].parse().ok()?;

        let parts: Vec<&str> = line.split_whitespace().collect();
        let count: u64 = parts.last()?.parse().ok()?;

        Some((le, count))
    } else {
        None
    }
}

/// Common vLLM metric names.
pub mod metric_names {
    /// GPU cache usage percentage.
    pub const GPU_CACHE_USAGE_PERC: &str = "vllm:gpu_cache_usage_perc";

    /// CPU cache usage percentage.
    pub const CPU_CACHE_USAGE_PERC: &str = "vllm:cpu_cache_usage_perc";

    /// Number of preemptions total.
    pub const NUM_PREEMPTION_TOTAL: &str = "vllm:num_preemption_total";

    /// Generation tokens total.
    pub const GENERATION_TOKENS_TOTAL: &str = "vllm:generation_tokens_total";

    /// Prompt tokens total.
    pub const PROMPT_TOKENS_TOTAL: &str = "vllm:prompt_tokens_total";

    /// Time to first token seconds.
    pub const TIME_TO_FIRST_TOKEN_SECONDS: &str = "vllm:time_to_first_token_seconds";

    /// Time per output token seconds.
    pub const TIME_PER_OUTPUT_TOKEN_SECONDS: &str = "vllm:time_per_output_token_seconds";

    /// Inference latency seconds.
    pub const INFERENCE_LATENCY_SECONDS: &str = "vllm:inference_latency_seconds";

    /// Number of requests running.
    pub const NUM_REQUESTS_RUNNING: &str = "vllm:num_requests_running";

    /// Number of requests waiting.
    pub const NUM_REQUESTS_WAITING: &str = "vllm:num_requests_waiting";

    /// Number of requests swapped.
    pub const NUM_REQUESTS_SWAPPED: &str = "vllm:num_requests_swapped";
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_gauge_value() {
        let line = "vllm:gpu_cache_usage_perc 0.75";
        assert_eq!(parse_gauge_value(line), Some(0.75));

        let line = "vllm:gpu_cache_usage_perc{model=\"test\"} 0.5";
        assert_eq!(parse_gauge_value(line), Some(0.5));
    }

    #[test]
    fn test_parse_counter_value() {
        let line = "vllm:generation_tokens_total 12345";
        assert_eq!(parse_counter_value(line), Some(12345));
    }

    #[test]
    fn test_parse_histogram_bucket() {
        let line = r#"vllm:time_to_first_token_seconds_bucket{le="0.005"} 10"#;
        assert_eq!(parse_histogram_bucket(line), Some((0.005, 10)));

        let line = r#"vllm:time_to_first_token_seconds_bucket{le="+Inf"} 100"#;
        assert_eq!(parse_histogram_bucket(line), Some((f64::INFINITY, 100)));
    }

    #[test]
    fn test_metrics_info_parse() {
        let raw = r#"
# HELP vllm:gpu_cache_usage_perc GPU cache usage percentage
vllm:gpu_cache_usage_perc 0.75
# HELP vllm:generation_tokens_total Total generation tokens
vllm:generation_tokens_total 12345
# HELP vllm:prompt_tokens_total Total prompt tokens
vllm:prompt_tokens_total 5678
vllm:time_to_first_token_seconds_sum 150.0
vllm:time_to_first_token_seconds_count 100
vllm:time_per_output_token_seconds_sum 50.0
vllm:time_per_output_token_seconds_count 1000
"#;

        let info = MetricsInfo::parse(raw);
        assert_eq!(info.gpu_cache_usage_perc, Some(0.75));
        assert_eq!(info.generation_tokens_total, Some(12345));
        assert_eq!(info.prompt_tokens_total, Some(5678));
        assert_eq!(info.total_tokens(), Some(18023));
        assert_eq!(info.avg_time_to_first_token(), Some(1.5));
        assert_eq!(info.avg_time_per_output_token(), Some(0.05));
    }
}

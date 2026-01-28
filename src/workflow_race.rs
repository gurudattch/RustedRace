use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::Barrier;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowRequest {
    pub id: String,
    pub name: String,
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub cookies: HashMap<String, String>,
    pub auth_token: String,
    pub enabled: bool,
    pub request_count: usize,
}

impl Default for WorkflowRequest {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            name: "New Request".to_string(),
            method: "GET".to_string(),
            url: String::new(),
            headers: HashMap::new(),
            body: String::new(),
            cookies: HashMap::new(),
            auth_token: String::new(),
            enabled: true,
            request_count: 1,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ExecutionMode {
    Burst,      // All requests at once
    Wave,       // Requests in waves with delay
    Random,     // Random timing
}

#[derive(Debug, Clone)]
pub struct WorkflowConfig {
    pub requests: Vec<WorkflowRequest>,
    pub concurrency: usize,
    pub execution_mode: ExecutionMode,
    pub synchronize: bool,
    pub delay_ms: u64,
    pub shared_session: bool,
    pub csrf_refresh: bool,
}

impl Default for WorkflowConfig {
    fn default() -> Self {
        Self {
            requests: vec![WorkflowRequest::default(), WorkflowRequest::default()],
            concurrency: 10,
            execution_mode: ExecutionMode::Burst,
            synchronize: true,
            delay_ms: 100,
            shared_session: true,
            csrf_refresh: false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct WorkflowResponse {
    pub request_id: String,
    pub request_name: String,
    pub status_code: u16,
    pub body: String,
    pub headers: HashMap<String, String>,
    pub duration: Duration,
    pub timestamp: Instant,
    pub thread_id: usize,
}

#[derive(Debug)]
pub struct WorkflowResult {
    pub total_requests: usize,
    pub success_count: usize,
    pub failure_count: usize,
    pub error_count: usize,
    pub responses: Vec<WorkflowResponse>,
    pub total_duration: Duration,
    pub anomalies: Vec<String>,
    pub timeline: Vec<(Instant, String)>,
}

pub struct WorkflowEngine {
    config: WorkflowConfig,
    session_store: Arc<tokio::sync::Mutex<HashMap<String, String>>>,
}

impl WorkflowEngine {
    pub fn new(config: WorkflowConfig) -> Self {
        Self {
            config,
            session_store: Arc::new(tokio::sync::Mutex::new(HashMap::new())),
        }
    }

    pub async fn execute(&self) -> WorkflowResult {
        let start_time = Instant::now();
        let mut responses = Vec::new();
        let mut timeline = Vec::new();
        
        let enabled_requests: Vec<_> = self.config.requests.iter()
            .filter(|req| req.enabled)
            .cloned()
            .collect();
        
        if enabled_requests.is_empty() {
            return WorkflowResult {
                total_requests: 0,
                success_count: 0,
                failure_count: 0,
                error_count: 0,
                responses,
                total_duration: start_time.elapsed(),
                anomalies: vec!["No enabled requests".to_string()],
                timeline,
            };
        }

        let barrier = Arc::new(Barrier::new(self.config.concurrency));
        let mut handles = Vec::new();
        let config = self.config.clone();
        let session_store = Arc::clone(&self.session_store);

        for thread_id in 0..config.concurrency {
            let requests = enabled_requests.clone();
            let barrier = Arc::clone(&barrier);
            let session_store = Arc::clone(&session_store);
            let config = config.clone();
            
            let handle = tokio::spawn(async move {
                let mut thread_responses = Vec::new();
                let thread_timeline = Vec::new();
                
                // Wait for synchronization if enabled
                if config.synchronize {
                    barrier.wait().await;
                }
                
                match config.execution_mode {
                    ExecutionMode::Burst => {
                        // Execute all requests simultaneously
                        let mut request_handles = Vec::new();
                        
                        for request in requests {
                            let session_store = Arc::clone(&session_store);
                            
                            let handle = tokio::spawn(async move {
                                Self::execute_single_request(request, thread_id, session_store).await
                            });
                            request_handles.push(handle);
                        }
                        
                        for handle in request_handles {
                            if let Ok(response) = handle.await {
                                thread_responses.push(response);
                            }
                        }
                    }
                    ExecutionMode::Wave => {
                        // Execute requests in sequence with delay
                        for request in requests {
                            let response = Self::execute_single_request(
                                request, 
                                thread_id, 
                                Arc::clone(&session_store)
                            ).await;
                            thread_responses.push(response);
                            
                            if config.delay_ms > 0 {
                                tokio::time::sleep(Duration::from_millis(config.delay_ms)).await;
                            }
                        }
                    }
                    ExecutionMode::Random => {
                        // Execute with random delays
                        for request in requests {
                            let random_delay = rand::random::<u64>() % (config.delay_ms + 1);
                            tokio::time::sleep(Duration::from_millis(random_delay)).await;
                            
                            let response = Self::execute_single_request(
                                request, 
                                thread_id, 
                                Arc::clone(&session_store)
                            ).await;
                            thread_responses.push(response);
                        }
                    }
                }
                
                (thread_responses, thread_timeline)
            });
            
            handles.push(handle);
        }

        // Collect all responses
        for handle in handles {
            if let Ok((thread_responses, thread_timeline)) = handle.await {
                responses.extend(thread_responses);
                timeline.extend(thread_timeline);
            }
        }

        // Analyze results
        let success_count = responses.iter().filter(|r| r.status_code >= 200 && r.status_code < 300).count();
        let failure_count = responses.iter().filter(|r| r.status_code >= 400).count();
        let error_count = responses.iter().filter(|r| r.status_code == 0).count();
        
        let anomalies = self.detect_anomalies(&responses);
        
        WorkflowResult {
            total_requests: responses.len(),
            success_count,
            failure_count,
            error_count,
            responses,
            total_duration: start_time.elapsed(),
            anomalies,
            timeline,
        }
    }

    async fn execute_single_request(
        request: WorkflowRequest,
        thread_id: usize,
        session_store: Arc<tokio::sync::Mutex<HashMap<String, String>>>,
    ) -> WorkflowResponse {
        let start_time = Instant::now();
        
        let client = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_default();

        // Build request with session data
        let mut headers = reqwest::header::HeaderMap::new();
        for (key, value) in &request.headers {
            if let (Ok(name), Ok(val)) = (
                reqwest::header::HeaderName::from_bytes(key.as_bytes()),
                reqwest::header::HeaderValue::from_str(value),
            ) {
                headers.insert(name, val);
            }
        }

        // Add auth token if present
        if !request.auth_token.is_empty() {
            if let Ok(auth_header) = reqwest::header::HeaderValue::from_str(&format!("Bearer {}", request.auth_token)) {
                headers.insert(reqwest::header::AUTHORIZATION, auth_header);
            }
        }

        let method = match request.method.as_str() {
            "GET" => reqwest::Method::GET,
            "POST" => reqwest::Method::POST,
            "PUT" => reqwest::Method::PUT,
            "DELETE" => reqwest::Method::DELETE,
            "PATCH" => reqwest::Method::PATCH,
            _ => reqwest::Method::GET,
        };

        let mut req_builder = client.request(method, &request.url).headers(headers);

        if !request.body.is_empty() {
            req_builder = req_builder.body(request.body.clone());
        }

        // Add cookies
        for (name, value) in &request.cookies {
            req_builder = req_builder.header("Cookie", format!("{}={}", name, value));
        }

        match req_builder.send().await {
            Ok(response) => {
                let status = response.status().as_u16();
                let response_headers: HashMap<String, String> = response.headers()
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                    .collect();
                
                let body = response.text().await.unwrap_or_else(|_| "Error reading response".to_string());
                
                // Update session store if needed
                if let Some(set_cookie) = response_headers.get("set-cookie") {
                    let mut store = session_store.lock().await;
                    store.insert("session_cookie".to_string(), set_cookie.clone());
                }

                WorkflowResponse {
                    request_id: request.id,
                    request_name: request.name,
                    status_code: status,
                    body,
                    headers: response_headers,
                    duration: start_time.elapsed(),
                    timestamp: start_time,
                    thread_id,
                }
            }
            Err(e) => WorkflowResponse {
                request_id: request.id,
                request_name: request.name,
                status_code: 0,
                body: format!("Error: {}", e),
                headers: HashMap::new(),
                duration: start_time.elapsed(),
                timestamp: start_time,
                thread_id,
            },
        }
    }

    fn detect_anomalies(&self, responses: &[WorkflowResponse]) -> Vec<String> {
        let mut anomalies = Vec::new();
        
        // Group responses by request type
        let mut response_groups: HashMap<String, Vec<&WorkflowResponse>> = HashMap::new();
        for response in responses {
            response_groups.entry(response.request_name.clone())
                .or_insert_with(Vec::new)
                .push(response);
        }
        
        // Detect anomalies
        for (request_name, group_responses) in response_groups {
            let success_responses: Vec<_> = group_responses.iter()
                .filter(|r| r.status_code >= 200 && r.status_code < 300)
                .collect();
            
            // Check for duplicate successes (potential race condition)
            if success_responses.len() > 1 {
                anomalies.push(format!(
                    "Multiple successful responses for '{}': {} successes detected",
                    request_name, success_responses.len()
                ));
            }
            
            // Check for status code variations
            let unique_statuses: std::collections::HashSet<_> = group_responses.iter()
                .map(|r| r.status_code)
                .collect();
            
            if unique_statuses.len() > 1 {
                anomalies.push(format!(
                    "Status code variations in '{}': {:?}",
                    request_name, unique_statuses
                ));
            }
            
            // Check for timing anomalies
            let durations: Vec<_> = group_responses.iter()
                .map(|r| r.duration.as_millis())
                .collect();
            
            if let (Some(&min), Some(&max)) = (durations.iter().min(), durations.iter().max()) {
                if max > min * 3 {  // 3x timing difference threshold
                    anomalies.push(format!(
                        "Timing anomaly in '{}': {}ms - {}ms range",
                        request_name, min, max
                    ));
                }
            }
        }
        
        anomalies
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_workflow_engine_creation() {
        let config = WorkflowConfig::default();
        let engine = WorkflowEngine::new(config);
        let result = engine.execute().await;
        
        assert_eq!(result.total_requests, 0); // No valid URLs in default config
    }
}

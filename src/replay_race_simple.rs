use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::sync::Arc;
use tokio::sync::Barrier;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayRequest {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: String,
    pub variables: HashMap<String, String>,
}

impl Default for ReplayRequest {
    fn default() -> Self {
        Self {
            method: "GET".to_string(),
            url: String::new(),
            headers: HashMap::new(),
            body: String::new(),
            variables: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ExecutionMode {
    Burst,
    Wave,
    Random,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RaceType {
    QuotaRace,
    DoubleSpend,
    ResourceRace,
    LostUpdate,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayConfig {
    pub request: ReplayRequest,
    pub thread_count: usize,
    pub total_requests: usize,
    pub execution_mode: ExecutionMode,
    pub micro_delay_ms: u64,
}

impl Default for ReplayConfig {
    fn default() -> Self {
        Self {
            request: ReplayRequest::default(),
            thread_count: 10,
            total_requests: 100,
            execution_mode: ExecutionMode::Burst,
            micro_delay_ms: 0,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReplayResponse {
    pub request_id: usize,
    pub status_code: u16,
    pub body: String,
    pub headers: HashMap<String, String>,
    pub duration: Duration,
    pub timestamp: Instant,
    pub thread_id: usize,
}

#[derive(Debug)]
pub struct ReplayResult {
    pub total_requests: usize,
    pub success_count: usize,
    pub failure_count: usize,
    pub error_count: usize,
    pub responses: Vec<ReplayResponse>,
    pub total_duration: Duration,
    pub race_type: RaceType,
    pub anomalies: Vec<String>,
    pub before_state: Option<String>,
    pub after_state: Option<String>,
}

pub struct ReplayEngine {
    config: ReplayConfig,
    wordlists: Vec<Vec<String>>,
}

impl ReplayEngine {
    pub fn new(config: ReplayConfig) -> Self {
        Self {
            config,
            wordlists: Vec::new(),
        }
    }

    pub fn set_wordlists(&mut self, wordlists: Vec<Vec<String>>) {
        self.wordlists = wordlists;
    }

    pub async fn execute(&self) -> ReplayResult {
        let start_time = Instant::now();
        
        if self.config.request.url.is_empty() {
            return ReplayResult {
                total_requests: 0,
                success_count: 0,
                failure_count: 0,
                error_count: 1,
                responses: vec![],
                total_duration: start_time.elapsed(),
                race_type: RaceType::Unknown,
                anomalies: vec!["No URL provided".to_string()],
                before_state: None,
                after_state: None,
            };
        }

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .danger_accept_invalid_certs(true)
            .build()
            .unwrap();

        match self.config.execution_mode {
            ExecutionMode::Burst => self.execute_burst(client, start_time).await,
            ExecutionMode::Wave => self.execute_wave(client, start_time).await,
            ExecutionMode::Random => self.execute_random(client, start_time).await,
        }
    }

    async fn execute_burst(&self, client: reqwest::Client, start_time: Instant) -> ReplayResult {
        let barrier = Arc::new(Barrier::new(self.config.thread_count));
        let mut handles = Vec::new();
        let requests_per_thread = self.config.total_requests / self.config.thread_count;
        let remaining_requests = self.config.total_requests % self.config.thread_count;

        for thread_id in 0..self.config.thread_count {
            let client = client.clone();
            let config = self.config.clone();
            let wordlists = self.wordlists.clone();
            let barrier = barrier.clone();
            
            // Distribute remaining requests to first threads
            let thread_requests = if thread_id < remaining_requests {
                requests_per_thread + 1
            } else {
                requests_per_thread
            };
            
            let handle = tokio::spawn(async move {
                let mut thread_responses = Vec::new();
                
                // Wait for all threads to be ready
                barrier.wait().await;
                
                // Execute all requests for this thread immediately after barrier
                for i in 0..thread_requests {
                    let request_id = thread_id * requests_per_thread + i;
                    let response = Self::execute_single_request(
                        &client, &config, &wordlists, request_id, thread_id
                    ).await;
                    thread_responses.push(response);
                }
                
                thread_responses
            });
            
            handles.push(handle);
        }
        
        self.collect_results(handles, start_time).await
    }

    async fn execute_wave(&self, client: reqwest::Client, start_time: Instant) -> ReplayResult {
        let mut all_responses = Vec::new();
        let wave_size = self.config.thread_count;
        let total_waves = (self.config.total_requests + wave_size - 1) / wave_size;
        
        for wave in 0..total_waves {
            let requests_in_wave = wave_size.min(self.config.total_requests - wave * wave_size);
            let barrier = Arc::new(Barrier::new(requests_in_wave));
            let mut handles = Vec::new();
            
            for i in 0..requests_in_wave {
                let request_id = wave * wave_size + i;
                
                let client = client.clone();
                let config = self.config.clone();
                let wordlists = self.wordlists.clone();
                let barrier = barrier.clone();
                
                let handle = tokio::spawn(async move {
                    barrier.wait().await;
                    Self::execute_single_request(&client, &config, &wordlists, request_id, i).await
                });
                
                handles.push(handle);
            }
            
            for handle in handles {
                if let Ok(response) = handle.await {
                    all_responses.push(response);
                }
            }
            
            // Wave delay
            if self.config.micro_delay_ms > 0 && wave < total_waves - 1 {
                tokio::time::sleep(Duration::from_millis(self.config.micro_delay_ms)).await;
            }
        }
        
        self.build_result(all_responses, start_time)
    }

    async fn execute_random(&self, client: reqwest::Client, start_time: Instant) -> ReplayResult {
        let mut handles = Vec::new();
        
        for request_id in 0..self.config.total_requests {
            let client = client.clone();
            let config = self.config.clone();
            let wordlists = self.wordlists.clone();
            
            let handle = tokio::spawn(async move {
                // Random delay before execution
                if config.micro_delay_ms > 0 {
                    let random_delay = rand::random::<u64>() % config.micro_delay_ms;
                    tokio::time::sleep(Duration::from_millis(random_delay)).await;
                }
                
                Self::execute_single_request(&client, &config, &wordlists, request_id, request_id % config.thread_count).await
            });
            
            handles.push(handle);
        }
        
        let mut all_responses = Vec::new();
        for handle in handles {
            if let Ok(response) = handle.await {
                all_responses.push(response);
            }
        }
        
        self.build_result(all_responses, start_time)
    }

    async fn execute_single_request(
        client: &reqwest::Client,
        config: &ReplayConfig,
        wordlists: &[Vec<String>],
        request_id: usize,
        thread_id: usize,
    ) -> ReplayResponse {
        let request_start = Instant::now();
        
        // Build request with unique values
        let mut url = config.request.url.clone();
        let mut body = config.request.body.clone();
        
        // Replace variables with wordlist values or unique IDs
        if !wordlists.is_empty() {
            for (j, wordlist) in wordlists.iter().enumerate() {
                let value = if !wordlist.is_empty() {
                    wordlist[request_id % wordlist.len()].clone()
                } else {
                    format!("unique{}-{}", j + 1, request_id)
                };
                let placeholder = format!("{{WORDLIST{}}}", j + 1);
                url = url.replace(&placeholder, &value);
                body = body.replace(&placeholder, &value);
            }
        }
        
        // Replace common variables
        url = url.replace("{UNIQUE_ID}", &format!("id_{}", request_id));
        body = body.replace("{UNIQUE_ID}", &format!("id_{}", request_id));
        
        // Build and send request
        let mut request_builder = match config.request.method.as_str() {
            "GET" => client.get(&url),
            "POST" => client.post(&url),
            "PUT" => client.put(&url),
            "DELETE" => client.delete(&url),
            "PATCH" => client.patch(&url),
            _ => client.get(&url),
        };
        
        // Add headers
        for (key, value) in &config.request.headers {
            request_builder = request_builder.header(key, value);
        }
        
        // Add body for POST/PUT/PATCH
        if !body.is_empty() && matches!(config.request.method.as_str(), "POST" | "PUT" | "PATCH") {
            request_builder = request_builder.body(body);
        }
        
        // Execute request
        match request_builder.send().await {
            Ok(resp) => {
                let status = resp.status().as_u16();
                let headers: HashMap<String, String> = resp.headers()
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                    .collect();
                let body = resp.text().await.unwrap_or_default();
                
                ReplayResponse {
                    request_id,
                    status_code: status,
                    body,
                    headers,
                    duration: request_start.elapsed(),
                    timestamp: request_start,
                    thread_id,
                }
            }
            Err(e) => {
                ReplayResponse {
                    request_id,
                    status_code: 0,
                    body: format!("Error: {}", e),
                    headers: HashMap::new(),
                    duration: request_start.elapsed(),
                    timestamp: request_start,
                    thread_id,
                }
            }
        }
    }

    async fn collect_results(&self, handles: Vec<tokio::task::JoinHandle<Vec<ReplayResponse>>>, start_time: Instant) -> ReplayResult {
        let mut all_responses = Vec::new();
        
        for handle in handles {
            if let Ok(responses) = handle.await {
                all_responses.extend(responses);
            }
        }
        
        self.build_result(all_responses, start_time)
    }

    fn build_result(&self, mut all_responses: Vec<ReplayResponse>, start_time: Instant) -> ReplayResult {
        // Sort by request_id for consistent ordering
        all_responses.sort_by_key(|r| r.request_id);
        
        let success_count = all_responses.iter().filter(|r| r.status_code >= 200 && r.status_code < 300).count();
        let failure_count = all_responses.iter().filter(|r| r.status_code >= 400).count();
        let error_count = all_responses.iter().filter(|r| r.status_code == 0).count();
        
        // Detect race condition type
        let race_type = self.detect_race_type(&all_responses);
        let anomalies = self.detect_anomalies(&all_responses);

        ReplayResult {
            total_requests: all_responses.len(),
            success_count,
            failure_count,
            error_count,
            responses: all_responses,
            total_duration: start_time.elapsed(),
            race_type,
            anomalies,
            before_state: None,
            after_state: None,
        }
    }
    
    fn detect_race_type(&self, responses: &[ReplayResponse]) -> RaceType {
        let success_responses: Vec<_> = responses.iter().filter(|r| r.status_code >= 200 && r.status_code < 300).collect();
        
        // Check for quota/limit bypass (multiple successes when only one expected)
        if success_responses.len() > 1 {
            // Look for patterns indicating quota bypass
            let unique_bodies: std::collections::HashSet<_> = success_responses.iter().map(|r| &r.body).collect();
            if unique_bodies.len() == 1 && success_responses.len() > 2 {
                return RaceType::QuotaRace;
            }
        }
        
        // Check for double spend (successful resource consumption)
        if success_responses.iter().any(|r| r.body.contains("balance") || r.body.contains("credit") || r.body.contains("purchase")) {
            if success_responses.len() > 1 {
                return RaceType::DoubleSpend;
            }
        }
        
        // Check for resource race (conflicting resource access)
        if responses.iter().any(|r| r.status_code == 409 || r.body.contains("conflict")) {
            return RaceType::ResourceRace;
        }
        
        // Check for lost update (inconsistent final state)
        let status_codes: std::collections::HashSet<_> = responses.iter().map(|r| r.status_code).collect();
        if status_codes.len() > 2 {
            return RaceType::LostUpdate;
        }
        
        RaceType::Unknown
    }
    
    fn detect_anomalies(&self, responses: &[ReplayResponse]) -> Vec<String> {
        let mut anomalies = Vec::new();
        
        // Check for unexpected multiple successes
        let success_count = responses.iter().filter(|r| r.status_code >= 200 && r.status_code < 300).count();
        if success_count > 1 {
            anomalies.push(format!("Multiple successful responses: {} (potential race condition)", success_count));
        }
        
        // Check for timing anomalies
        let avg_duration: f64 = responses.iter().map(|r| r.duration.as_millis() as f64).sum::<f64>() / responses.len() as f64;
        let outliers: Vec<_> = responses.iter().filter(|r| {
            let duration_ms = r.duration.as_millis() as f64;
            (duration_ms - avg_duration).abs() > avg_duration * 2.0
        }).collect();
        
        if !outliers.is_empty() {
            anomalies.push(format!("Timing outliers detected: {} requests", outliers.len()));
        }
        
        // Check for different response sizes (potential state changes)
        let response_sizes: std::collections::HashSet<_> = responses.iter().map(|r| r.body.len()).collect();
        if response_sizes.len() > 2 {
            anomalies.push("Varying response sizes detected (potential state inconsistency)".to_string());
        }
        
        anomalies
    }
}

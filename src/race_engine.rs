use crate::http_parser::ParsedRequest;
use crate::request_builder::RequestBuilder;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct ResponseData {
    pub status_code: u16,
    pub body: String,
    pub duration: Duration,
}

#[derive(Debug)]
pub struct RaceResult {
    pub total_requests: usize,
    pub success_count: usize,
    pub failure_count: usize,
    pub error_count: usize,
    pub status_codes: HashMap<u16, usize>,
    pub responses: Vec<ResponseData>,
    pub total_duration: Duration,
}

pub struct RaceEngine {
    parsed_request: ParsedRequest,
    concurrency: usize,
    use_unique_values: bool,
    placeholder: String,
    wordlist1: Vec<String>,
    wordlist2: Vec<String>,
    wordlist3: Vec<String>,
}

impl RaceEngine {
    pub fn new(
        parsed_request: ParsedRequest,
        concurrency: usize,
        use_unique_values: bool,
        placeholder: String,
    ) -> Self {
        Self {
            parsed_request,
            concurrency,
            use_unique_values,
            placeholder,
            wordlist1: Vec::new(),
            wordlist2: Vec::new(),
            wordlist3: Vec::new(),
        }
    }
    
    pub fn with_wordlists(
        mut self,
        wordlist1: Vec<String>,
        wordlist2: Vec<String>,
        wordlist3: Vec<String>,
    ) -> Self {
        self.wordlist1 = wordlist1;
        self.wordlist2 = wordlist2;
        self.wordlist3 = wordlist3;
        self
    }

    pub fn execute(&self, success_keyword: &str, failure_keyword: &str) -> RaceResult {
        let start_time = Instant::now();
        
        let responses = Arc::new(Mutex::new(Vec::new()));
        let status_codes = Arc::new(Mutex::new(HashMap::new()));
        
        let mut handles = vec![];

        // Create a barrier to synchronize the start of all threads
        let barrier = Arc::new(std::sync::Barrier::new(self.concurrency));

        for i in 0..self.concurrency {
            let parsed_req = self.parsed_request.clone();
            let use_unique = self.use_unique_values;
            let placeholder = self.placeholder.clone();
            let wordlist1 = self.wordlist1.clone();
            let wordlist2 = self.wordlist2.clone();
            let wordlist3 = self.wordlist3.clone();
            let responses_clone = Arc::clone(&responses);
            let status_codes_clone = Arc::clone(&status_codes);
            let barrier_clone = Arc::clone(&barrier);

            let handle = thread::spawn(move || {
                let request_builder = RequestBuilder::new(parsed_req, use_unique, placeholder)
                    .with_wordlists(wordlist1.clone(), wordlist2.clone(), wordlist3.clone());

                // Wait at the barrier for all threads to be ready
                barrier_clone.wait();

                // Execute request immediately after barrier
                let req_start = Instant::now();
                
                match request_builder.build(i) {
                    Ok(request) => {
                        let client = reqwest::blocking::Client::builder()
                            .danger_accept_invalid_certs(true)
                            .timeout(Duration::from_secs(30))
                            .build()
                            .unwrap();

                        match client.execute(request) {
                            Ok(response) => {
                                let status = response.status().as_u16();
                                let body = response.text().unwrap_or_else(|_| String::from("Error reading response"));
                                let duration = req_start.elapsed();

                                let response_data = ResponseData {
                                    status_code: status,
                                    body,
                                    duration,
                                };

                                // Store response
                                responses_clone.lock().unwrap().push(response_data);

                                // Update status code count
                                let mut codes = status_codes_clone.lock().unwrap();
                                *codes.entry(status).or_insert(0) += 1;
                            }
                            Err(e) => {
                                let response_data = ResponseData {
                                    status_code: 0,
                                    body: format!("Error: {}", e),
                                    duration: req_start.elapsed(),
                                };
                                responses_clone.lock().unwrap().push(response_data);
                            }
                        }
                    }
                    Err(e) => {
                        let response_data = ResponseData {
                            status_code: 0,
                            body: format!("Build error: {}", e),
                            duration: Duration::from_secs(0),
                        };
                        responses_clone.lock().unwrap().push(response_data);
                    }
                }
            });

            handles.push(handle);
        }

        // Wait for all threads to complete
        for handle in handles {
            let _ = handle.join();
        }

        let total_duration = start_time.elapsed();

        // Analyze results
        let responses_vec = responses.lock().unwrap().clone();
        let status_codes_map = status_codes.lock().unwrap().clone();

        let mut success_count = 0;
        let mut failure_count = 0;
        let mut error_count = 0;

        for response in &responses_vec {
            if response.status_code == 0 {
                error_count += 1;
            } else if !success_keyword.is_empty()
                && response.body.contains(success_keyword)
            {
                success_count += 1;
            } else if !failure_keyword.is_empty()
                && response.body.contains(failure_keyword)
            {
                failure_count += 1;
            } else {
                // If no keywords are set, classify by status code
                if response.status_code >= 200 && response.status_code < 300 {
                    success_count += 1;
                } else if response.status_code >= 400 {
                    failure_count += 1;
                }
            }
        }

        RaceResult {
            total_requests: self.concurrency,
            success_count,
            failure_count,
            error_count,
            status_codes: status_codes_map,
            responses: responses_vec,
            total_duration,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_race_engine_creation() {
        let mut headers = HashMap::new();
        headers.insert("Host".to_string(), "example.com".to_string());

        let parsed = ParsedRequest {
            method: "GET".to_string(),
            path: "/test".to_string(),
            url: "http://example.com/test".to_string(),
            headers,
            body: String::new(),
        };

        let engine = RaceEngine::new(parsed, 5, false, String::new());
        assert_eq!(engine.concurrency, 5);
    }
}

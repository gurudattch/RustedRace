use crate::http_parser::ParsedRequest;
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};
use std::str::FromStr;
use uuid::Uuid;

pub struct RequestBuilder {
    parsed_request: ParsedRequest,
    use_unique_values: bool,
    placeholder: String,
    wordlist1: Vec<String>,
    wordlist2: Vec<String>,
    wordlist3: Vec<String>,
}

impl RequestBuilder {
    pub fn new(
        parsed_request: ParsedRequest,
        use_unique_values: bool,
        placeholder: String,
    ) -> Self {
        Self {
            parsed_request,
            use_unique_values,
            placeholder: if placeholder.is_empty() {
                "{{UNIQUE}}".to_string()
            } else {
                placeholder
            },
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

    pub fn build(&self, request_id: usize) -> Result<reqwest::blocking::Request, String> {
        let client = reqwest::blocking::Client::builder()
            .danger_accept_invalid_certs(true)
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| format!("Failed to build client: {}", e))?;

        // Generate unique values
        let unique_value = if self.use_unique_values {
            format!("{}-{}", Uuid::new_v4(), request_id)
        } else {
            String::new()
        };
        
        // Get wordlist values (cycle through if request_id exceeds wordlist length)
        let unique1 = if !self.wordlist1.is_empty() {
            self.wordlist1[request_id % self.wordlist1.len()].clone()
        } else {
            format!("unique1-{}", request_id)
        };
        
        let unique2 = if !self.wordlist2.is_empty() {
            self.wordlist2[request_id % self.wordlist2.len()].clone()
        } else {
            format!("unique2-{}", request_id)
        };
        
        let unique3 = if !self.wordlist3.is_empty() {
            self.wordlist3[request_id % self.wordlist3.len()].clone()
        } else {
            format!("unique3-{}", request_id)
        };

        // Replace placeholders in URL
        let mut url = self.parsed_request.url.clone();
        if self.use_unique_values {
            url = url.replace(&self.placeholder, &unique_value);
        }
        url = url.replace("{{UNIQUE1}}", &unique1);
        url = url.replace("{{UNIQUE2}}", &unique2);
        url = url.replace("{{UNIQUE3}}", &unique3);

        // Build headers with replacements
        let mut headers = HeaderMap::new();
        for (key, value) in &self.parsed_request.headers {
            let mut header_value = value.clone();
            if self.use_unique_values {
                header_value = header_value.replace(&self.placeholder, &unique_value);
            }
            header_value = header_value.replace("{{UNIQUE1}}", &unique1);
            header_value = header_value.replace("{{UNIQUE2}}", &unique2);
            header_value = header_value.replace("{{UNIQUE3}}", &unique3);

            if let (Ok(name), Ok(val)) = (
                HeaderName::from_str(key),
                HeaderValue::from_str(&header_value),
            ) {
                headers.insert(name, val);
            }
        }

        // Build body with replacements
        let mut body = self.parsed_request.body.clone();
        if self.use_unique_values {
            body = body.replace(&self.placeholder, &unique_value);
        }
        body = body.replace("{{UNIQUE1}}", &unique1);
        body = body.replace("{{UNIQUE2}}", &unique2);
        body = body.replace("{{UNIQUE3}}", &unique3);

        // Create request based on method
        let method = self.parsed_request.method.as_str();
        let request = match method {
            "GET" => client.get(&url),
            "POST" => client.post(&url),
            "PUT" => client.put(&url),
            "DELETE" => client.delete(&url),
            "PATCH" => client.patch(&url),
            "HEAD" => client.head(&url),
            "OPTIONS" => client.request(reqwest::Method::OPTIONS, &url),
            _ => return Err(format!("Unsupported HTTP method: {}", method)),
        };

        let request = request.headers(headers);

        let request = if !body.is_empty() {
            request.body(body)
        } else {
            request
        };

        request
            .build()
            .map_err(|e| format!("Failed to build request: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_unique_value_replacement() {
        let mut headers = HashMap::new();
        headers.insert("Host".to_string(), "example.com".to_string());

        let parsed = ParsedRequest {
            method: "POST".to_string(),
            path: "/api/test".to_string(),
            url: "http://example.com/api/test".to_string(),
            headers,
            body: "{\"id\":\"{{UNIQUE}}\"}".to_string(),
        };

        let builder = RequestBuilder::new(parsed, true, "{{UNIQUE}}".to_string());
        let request = builder.build(1);
        
        assert!(request.is_ok());
    }
}

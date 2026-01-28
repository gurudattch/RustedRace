use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ParsedRequest {
    pub method: String,
    pub path: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: String,
}

pub fn parse_burp_request(raw: &str) -> Result<ParsedRequest, String> {
    let lines: Vec<&str> = raw.lines().collect();
    
    if lines.is_empty() {
        return Err("Empty request".to_string());
    }

    // Parse request line (GET /path HTTP/1.1)
    let request_line = lines[0];
    let parts: Vec<&str> = request_line.split_whitespace().collect();
    
    if parts.len() < 2 {
        return Err("Invalid request line".to_string());
    }

    let method = parts[0].to_string();
    let path = parts[1].to_string();

    // Parse headers
    let mut headers = HashMap::new();
    let mut body_start = 0;
    let mut host = String::new();

    for (i, line) in lines.iter().enumerate().skip(1) {
        if line.trim().is_empty() {
            body_start = i + 1;
            break;
        }

        if let Some(colon_pos) = line.find(':') {
            let key = line[..colon_pos].trim().to_string();
            let value = line[colon_pos + 1..].trim().to_string();
            
            if key.to_lowercase() == "host" {
                host = value.clone();
            }
            
            headers.insert(key, value);
        }
    }

    // Parse body
    let body = if body_start < lines.len() {
        lines[body_start..].join("\n")
    } else {
        String::new()
    };

    // Construct full URL
    let scheme = if headers.contains_key("X-Forwarded-Proto") 
        && headers.get("X-Forwarded-Proto").unwrap() == "https" {
        "https"
    } else if host.contains(":443") {
        "https"
    } else {
        "http"
    };

    let url = if host.is_empty() {
        return Err("Host header is required".to_string());
    } else {
        format!("{}://{}{}", scheme, host, path)
    };

    Ok(ParsedRequest {
        method,
        path,
        url,
        headers,
        body,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_get_request() {
        let raw = "GET /api/test HTTP/1.1\nHost: example.com\nUser-Agent: Test\n\n";
        let result = parse_burp_request(raw);
        assert!(result.is_ok());
        
        let parsed = result.unwrap();
        assert_eq!(parsed.method, "GET");
        assert_eq!(parsed.path, "/api/test");
        assert_eq!(parsed.headers.get("Host").unwrap(), "example.com");
    }

    #[test]
    fn test_parse_post_request() {
        let raw = "POST /api/create HTTP/1.1\nHost: example.com\nContent-Type: application/json\n\n{\"test\":\"data\"}";
        let result = parse_burp_request(raw);
        assert!(result.is_ok());
        
        let parsed = result.unwrap();
        assert_eq!(parsed.method, "POST");
        assert_eq!(parsed.body, "{\"test\":\"data\"}");
    }
}

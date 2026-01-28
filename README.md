# RustedRace

<p align="center"><img src="https://github.com/gurudattch/RustedRace/blob/main/src/rustedrace.png"></p>
**RustedRace** A GUI-based race condition vulnerability testing tool built in Rust. RustedRace allows security researchers and developers to explore and test for race condition vulnerabilities in web applications through two main testing modes: Replay Race and Workflow Race.

## Features

- **Replay Race**: Test simple race conditions with HTTP request replay
- **Workflow Race**: Test complex multi-request scenarios
- **Multiple Execution Modes**: Burst, Wave, and Random timing
- **Wordlist Support**: Load custom wordlists for parameter fuzzing
- **Burp Suite Integration**: Parse raw HTTP requests directly
- **Real-time Results**: Live monitoring of responses and timing

## Installation

### Prerequisites
- Rust 1.70+

### Quick Start
```bash
git clone <repository-url>
cd RustedRace
cargo build --release
./target/release/rustedrace
```

### Linux Desktop Integration
```bash
chmod +x launch.sh
./launch.sh
```

## Usage

1. **Launch** the application
2. **Choose mode**: Replay Race or Workflow Race
3. **Paste HTTP request** from Burp Suite or write manually
4. **Set concurrency** (number of parallel requests)
5. **Click "Start Race"** and monitor results

### Request Format
```
GET /api/endpoint HTTP/1.1
Host: example.com
Authorization: Bearer <token>

{"data": "value"}
```

### Wordlists
- Load text files with one value per line
- Use `{{UNIQUE}}` placeholder in requests
- Values get substituted automatically

## Configuration

- **Burst**: All requests at once
- **Wave**: Requests in timed batches
- **Random**: Randomized timing
- **Thread Count**: 1-1000 concurrent requests

## Security Notice

Only test applications you own or have permission to test. This tool is for legitimate security research only.

## Author
<a href="https://www.linkedin.com/in/gurudatt-choudhary">Gurudatt Choudhary</a>

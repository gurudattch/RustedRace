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
git clone https://github.com/gurudattch/RustedRace
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

## How to use
1. Launch the tool
2. Paste Request from burp
3. Edit configuration according to your use case
4. Click on **Parse the request**
5. **Start Race Attack**
Analyze response report in sidebar

Tip: For better Result make number of Threads & Total Requests equal 

---

**Replay Race:**

<img width="1920" height="1080" alt="Screenshot_2026-01-28_15_59_19" src="https://github.com/user-attachments/assets/e7bf924d-4a96-490a-b8bd-cc41bc000c4d" />

---

**Workflow Race:**

<img width="1920" height="1080" alt="Screenshot_2026-01-28_15_59_22" src="https://github.com/user-attachments/assets/42c015d3-f5f2-4797-a143-79e699b0b4f9" />

---
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

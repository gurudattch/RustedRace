# RustedRace

<p align="center"><img src="https://github.com/gurudattch/RustedRace/blob/main/src/rustedrace.png"></p>
A GUI-based race condition vulnerability testing tool built in Rust. RustedRace allows security researchers and developers to explore and test for race condition vulnerabilities in web applications through two main testing modes: Replay Race and Workflow Race.

## Features

- **Dual Testing Modes**
  - **Replay Race**: Simple race condition testing with HTTP request replay
  - **Workflow Race**: Complex multi-request workflow testing with synchronization
- **Multiple Execution Modes**: Burst, Wave, and Random timing patterns
- **Dynamic Wordlist Support**: Load and use custom wordlists for parameter fuzzing
- **HTTP Request Parsing**: Parse raw HTTP requests from Burp Suite or other tools
- **Real-time Results**: Live monitoring of response codes, timing, and success rates
- **Cross-platform GUI**: Native desktop application using egui framework
- **Concurrent Testing**: Configurable thread count for parallel request execution

## Installation

### Prerequisites

- Rust 1.70 or later
- Git

### Building from Source

1. Clone the repository:
```bash
git clone https://github.com/gurudattch/RustedRace
cd RustedRace
```

2. Build the application:
```bash
cargo build --release
```

3. Run the application:
```bash
./target/release/rustedrace
```

### Linux Desktop Integration

For Linux users, use the provided launcher script for desktop integration:

```bash
chmod +x launch.sh
./launch.sh
```

This will:
- Build the application if needed
- Create a desktop entry
- Install it to your applications menu
- Launch the application

## Usage

### Basic Workflow

1. **Launch RustedRace**
   - Run the executable or use the desktop launcher

2. **Choose Testing Mode**
   - **Replay Race**: For simple race condition testing
   - **Workflow Race**: For complex multi-request scenarios

3. **Configure Request**
   - Paste raw HTTP request (Burp Suite format supported)
   - Set concurrency level (number of parallel threads)
   - Configure execution parameters

4. **Execute Test**
   - Click "Start Race" to begin testing
   - Monitor real-time results and response analysis

### Replay Race Mode

Ideal for testing simple race conditions like:
- Quota bypasses
- Double spending vulnerabilities
- Resource race conditions
- Lost update problems

**Configuration Options:**
- Thread count (1-1000)
- Total requests to send
- Execution mode (Burst/Wave/Random)
- Micro-delay timing
- Synchronization barriers

### Workflow Race Mode

Designed for complex testing scenarios involving:
- Multi-step authentication bypasses
- Sequential request dependencies
- Cross-request state manipulation
- Complex business logic races

**Configuration Options:**
- Multiple request definitions
- Per-request concurrency settings
- Request synchronization
- Conditional execution
- Variable substitution

### HTTP Request Format

RustedRace accepts raw HTTP requests in standard format:

```
GET /api/endpoint HTTP/1.1
Host: example.com
Authorization: Bearer <token>
Content-Type: application/json

{"parameter": "value"}
```

### Wordlist Integration

Load custom wordlists for parameter fuzzing:
1. Click "Load Wordlist" in the interface
2. Select text files containing one value per line
3. Use placeholder tokens in requests (e.g., `{{UNIQUE}}`)
4. RustedRace will substitute values during execution

## Configuration

### Execution Modes

- **Burst**: All requests sent simultaneously
- **Wave**: Requests sent in timed waves with delays
- **Random**: Randomized timing between requests

### Concurrency Settings

- Adjust thread count based on target capacity
- Higher concurrency increases race condition likelihood
- Monitor system resources during high-concurrency tests

### Timing Configuration

- Micro-delays: Fine-tune request timing (microseconds)
- Synchronization: Use barriers to coordinate request timing
- Wave delays: Configure intervals between request waves

## Results Analysis

RustedRace provides comprehensive result analysis:

- **Response Codes**: Distribution of HTTP status codes
- **Timing Analysis**: Request duration statistics
- **Success Rates**: Percentage of successful requests
- **Error Tracking**: Failed request categorization
- **Response Comparison**: Identify anomalous responses

## Security Considerations

### Responsible Testing

- Only test applications you own or have explicit permission to test
- Be mindful of rate limiting and server capacity
- Monitor target system impact during testing
- Follow responsible disclosure practices for discovered vulnerabilities

### Legal Compliance

- Ensure testing activities comply with local laws and regulations
- Obtain proper authorization before testing third-party systems
- Document testing scope and permissions
- Respect terms of service and usage policies

## Development

### Project Structure

```
src/
├── main.rs              # Application entry point and GUI
├── workflow_race.rs     # Workflow race engine
├── replay_race_simple.rs # Simple replay race engine
├── race_engine.rs       # Core race condition testing logic
├── http_parser.rs       # HTTP request parsing utilities
├── request_builder.rs   # HTTP request construction
├── loading_screen.rs    # Application loading interface
├── rustedrace.ico       # Windows icon
└── rustedrace.png       # Application logo
```

### Dependencies

- **eframe/egui**: Cross-platform GUI framework
- **reqwest**: HTTP client library
- **tokio**: Asynchronous runtime
- **serde**: Serialization framework
- **uuid**: Unique identifier generation
- **chrono**: Date and time utilities

### Building for Different Platforms

**Linux:**
```bash
cargo build --release
```

**Windows:**
```bash
cargo build --release --target x86_64-pc-windows-gnu
```

**macOS:**
```bash
cargo build --release --target x86_64-apple-darwin
```

### Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Add tests if applicable
5. Submit a pull request

## Troubleshooting

### Common Issues

**Application won't start:**
- Ensure Rust toolchain is properly installed
- Check for missing system dependencies
- Verify executable permissions on Linux/macOS

**HTTP requests failing:**
- Verify target server accessibility
- Check SSL/TLS certificate validation settings
- Confirm request format and headers

**Performance issues:**
- Reduce concurrency for resource-constrained systems
- Adjust timeout values for slow networks
- Monitor system memory usage during large tests

### Debug Mode

Run with debug logging:
```bash
RUST_LOG=debug ./target/release/rustedrace
```

## License

This project is licensed under the MIT License. See the LICENSE file for details.

## Disclaimer

RustedRace is intended for legitimate security testing and research purposes only. Users are responsible for ensuring their testing activities are authorized and comply with applicable laws and regulations. The developers assume no liability for misuse of this tool.

## Support

For issues, feature requests, or questions:
- Open an issue on the project repository
- Review existing documentation and troubleshooting guides
- Check for updates and new releases

---

**Version**: 1.0.0  
**Author**: Security Researcher  
**Built with**: Rust, egui, reqwest

# Solana Data Aggregator

A Rust application that aggregates and processes Solana blockchain data, providing a real-time data ingestion service with a REST API and WebSocket interface.

## Features

- Real-time transaction ingestion from Solana blockchain
- REST API endpoints for querying transactions and accounts
- WebSocket support for live transaction updates
- In-memory storage with transaction indexing
- Health monitoring and metrics
- Configurable polling and batch processing
- Error handling and automatic retry mechanisms

## Prerequisites

- Rust (latest stable version)
- Cargo
- Solana CLI (for interacting with Solana networks)
- A Solana RPC endpoint (mainnet, testnet, or devnet)

## Installation

1. Clone the repository:
```bash
git clone https://github.com/yourusername/solana-data-aggregator.git
cd solana-data-aggregator
```

2. Build the project:
```bash
cargo build --release
```

## Configuration

The application can be configured through:
- Environment variables
- Configuration file (config.toml)
- Command line arguments

### Configuration File (config.toml)
```toml
[blockchain]
rpc_url = "https://api.devnet.solana.com"
max_batch_size = 100
max_retry_attempts = 3

[server]
port = 3000
host = "127.0.0.1"
api_key = "your-api-key"

[polling]
interval = "5s"
max_retry_attempts = 3
retry_delay_secs = 5
batch_size = 100
```

### Environment Variables
- `APP__BLOCKCHAIN__RPC_URL`: Solana RPC endpoint URL
- `APP__SERVER__API_KEY`: API key for authentication
- `APP__SERVER__PORT`: Server port number
- `APP__SERVER__HOST`: Server host address

## Running the Application

1. Start the application:
```bash
cargo run --release
```

2. With custom config file:
```bash
cargo run --release -- --config path/to/config.toml
```

## API Endpoints

### REST API

- `GET /health` - Health check endpoint
- `GET /accounts?id={pubkey}` - Get account information
- `GET /transactions?id={signature}` - Get transaction details
- `GET /transactions/by-date?day=DD/MM/YYYY` - Get transactions by date

### WebSocket

- `/subscribe` - WebSocket endpoint for real-time transaction updates
  - Optional query parameter: `chunk_size` for batch size control

## Authentication

All API endpoints require an API key to be sent in the `X-API-KEY` header:
```bash
curl -H "X-API-KEY: your-api-key" http://localhost:3000/health
```

## Development

### Running Tests
```bash
cargo test
```

### Running with Logging
```bash
RUST_LOG=info cargo run
```

## Error Handling

The application handles various types of errors:
- Blockchain connection errors
- Invalid transactions
- Storage errors
- API errors
- Configuration errors

Each error type is properly logged and reported through the appropriate channels.

## Monitoring

The application provides health monitoring through:
- Health check endpoint
- Connection health tracking
- Error rate monitoring
- Transaction processing statistics

## Architecture

The application follows a clean architecture pattern with:
- Domain models in `core/domain`
- Port interfaces in `core/ports`
- Adapter implementations in `adapters/`
- Services in `core/services`
- API handlers in `adapters/api/handlers`

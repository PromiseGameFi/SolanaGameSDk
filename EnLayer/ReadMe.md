# Multi-Party Computation & Threshold Signature Scheme

A robust implementation of a Multi-Party Computation (MPC) and Threshold Signature Scheme in Rust, designed for use with Ethereum (Sepolia), other EVM-compatible networks and other networks.

## Features

- **Threshold Signature Scheme**: Implementation of t-of-n threshold signatures
- **Secure Multi-Party Computation**: Distributed key generation and signature creation
- **Ethereum Integration**: Native support for Sepolia testnet
- **Secure Communication**: Authenticated encryption for inter-party communication
- **Cryptographic Proofs**: Verifiable secret sharing and zero-knowledge proofs
- **Async Support**: Built with Tokio for high-performance async operations

## Prerequisites

- Rust 1.70.0 or higher
- Cargo package manager
- Access to Ethereum Sepolia RPC endpoint
- Network connectivity for inter-party communication

## Installation

1. Clone the repository:

2. Install dependencies:

cargo build

## Configuration

Create a `.env` file in the project root:

    - ETHEREUM_RPC_URL=https://sepolia.infura.io/v3/YOUR-PROJECT-ID
    - PRIVATE_KEY=your_private_key_here
    - THRESHOLD=3
    - TOTAL_PARTIES=5

## Usage

1. Start the MPC protocol:

cargo run -- --party-id 1

2. Start additional parties:

cargo run -- --party-id 2

cargo run -- --party-id 3           

3. Generate a threshold signature:

cargo run -- --generate-signature

4. Submit a transaction:

cargo run -- --submit-transaction





## Architecture

### Components

1. **Threshold Module** (`src/threshold.rs`)
   - Implements Shamir's Secret Sharing
   - Handles share generation and verification

2. **MPC Protocol** (`src/mpc.rs`)
   - Coordinates multi-party computation
   - Manages protocol state and rounds

3. **Communication** (`src/communication.rs`)
   - Secure party-to-party communication
   - Network protocol implementation

4. **Ethereum Interface** (`src/ethereum.rs`)
   - Blockchain interaction
   - Transaction management

## Security Considerations

- All parties must be authenticated
- Secure channel required for communication
- Private keys should never be transmitted
- Use proper entropy for random number generation
- Implement timeout mechanisms
- Validate all incoming messages

## Protocol Flow

1. **Setup Phase**
   - Party initialization
   - Network connection establishment
   - Key share distribution

2. **Signature Generation**
   - Share combination
   - Threshold verification
   - Signature aggregation

3. **Transaction Submission**
   - Signature verification
   - Ethereum transaction creation
   - Network broadcast
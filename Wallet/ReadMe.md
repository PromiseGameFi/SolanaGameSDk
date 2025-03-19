# MPC-TSS Ethereum Wallet

This is a multi-party computation threshold signature scheme (MPC-TSS) wallet implementation for the Sepolia Ethereum network, written in Rust.

## Features

- Create multiple key shares from a single private key using MPC-TSS
- Define the threshold number of shares required for signing
- Generate partial signatures using individual shares
- Combine partial signatures to create a valid Ethereum transaction signature
- Send signed transactions to the Sepolia Ethereum network

## Installation

1. Make sure you have Rust and Cargo installed
2. Clone this repository
3. Build the project:

```bash
cargo build --release
```

## Usage

### Generate Key Shares

```bash
./mpc-tss-wallet generate-shares --total 3 --threshold 2 --output-dir ./shares
```

This will create 3 key shares, where any 2 can be used to sign a transaction.

### Sign a Transaction with a Share

```bash
./mpc-tss-wallet sign-transaction --share-path ./shares/share_1.json --to 0x1234567890abcdef1234567890abcdef12345678 --amount 0.1 --output ./partial_sig_1.json
```

Repeat this process with different shares to get multiple partial signatures.

### Combine Partial Signatures

```bash
./mpc-tss-wallet combine-signatures --partial-signatures ./partial_sig_1.json ./partial_sig_2.json --output ./full_sig.json
```

### Send Transaction to Sepolia

```bash
./mpc-tss-wallet send-transaction --signature-path ./full_sig.json --rpc-url https://sepolia.infura.io/v3/your-infura-project-id
```

## Security Considerations

- Store key shares securely and separately
- Never share your key shares with untrusted parties
- This is a demonstration project and should be audited before production use
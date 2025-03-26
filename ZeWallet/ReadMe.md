# ZeWallet - Threshold Signature Wallet

ZeWallet is a secure Ethereum wallet implementation using threshold signatures based on ZenGo's Multi-Party ECDSA implementation. This approach provides enhanced security through distributed key management.

## Security Features

- **No Single Point of Failure**: The private key is split into shares - compromising one share doesn't reveal the private key
- **Threshold Security**: Multiple parties (threshold) must cooperate to sign transactions
- **Compromise Resistance**: The complete private key never exists in one place (except briefly in memory during signing)

## Prerequisites

- Rust and Cargo (latest stable version)
- An Infura Project ID (for Sepolia testnet access)

## Installation

1. Clone the repository:
```bash
clone the SDK 
cd ZeWallet
```

2. Build the project:
```bash
cargo build --release
```

## Configuration

Before running the wallet, you need to:

1. Create a `.env` file in the project root (or set environment variables):
```env
INFURA_PROJECT_ID=your_infura_project_id
```

2. Make sure you have enough test ETH in your Sepolia account for transactions

## Usage

### Generate Key Shares

Generate a set of key shares with a specified threshold:

```bash
cargo run -- generate-keys --threshold 2 --shares 3
```

This creates 3 shares where any 2 can sign transactions. Files will be saved as `share_1.json`, `share_2.json`, etc.

### Sign a Transaction

Sign a transaction using one share:

```bash
cargo run -- sign --share-id 1 --to 0x123...456 --amount 0.01
```

This creates a partial signature file: `sig_1_[id].json`

### Combine and Send Transaction

Combine multiple signature shares and send the transaction:

```bash
cargo run -- combine --signature-files sig_1_abcd.json sig_2_abcd.json
```

## Security Considerations

1. **Share Storage**: Keep key shares securely stored and separated
2. **Threshold Selection**: Choose appropriate threshold values for your security needs
3. **Network Security**: Use secure channels when coordinating between share holders
4. **Backup**: Maintain secure backups of shares while ensuring they remain separated

## Development Status

This is a proof-of-concept implementation. For production use, additional security measures and features would be needed:

- [ ] Secure share storage
- [ ] Network communication encryption
- [ ] Transaction nonce management
- [ ] Gas price estimation
- [ ] Error handling improvements
- [ ] Testing suite
- [ ] Audit of cryptographic implementations

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Dependencies

- [multi-party-ecdsa](https://github.com/ZenGo-X/multi-party-ecdsa) - Core threshold signature implementation
- web3.rs - Ethereum interaction
- Other dependencies listed in `Cargo.toml`

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Acknowledgments

- ZenGo-X for their multi-party-ecdsa implementation
- The Ethereum community for web3 tools and documentation

## Contact

For questions or support, please open an issue in the GitHub repository.

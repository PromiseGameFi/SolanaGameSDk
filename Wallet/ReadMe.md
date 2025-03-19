## Clone

 - cd to directory

 - cargo build

# Generate 3 shares with a threshold of 2
cargo run -- generate-shares --total 3 --threshold 2 --output-dir ./shares

# Create partial signatures from two different shares
cargo run -- partial-sign --share-path ./shares/share_1.json --to 0x1234... --amount 0.01 --output-path ./sigs/sig_1.json
cargo run -- partial-sign --share-path ./shares/share_2.json --to 0x1234... --amount 0.01 --output-path ./sigs/sig_2.json

# Combine the signatures
cargo run -- combine-signatures --signature-paths ./sigs/sig_1.json ./sigs/sig_2.json --share-path ./shares/share_1.json --output-path ./combined.json

# Send the transaction to Sepolia
cargo run -- send-transaction --signature-path ./combined.json --infura-api-key YOUR_INFURA_KEY
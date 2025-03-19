
cargo run -- generate-keys --threshold 2 --shares 3

    cargo run -- sign --share-id 1 --to 0x123...456 --amount 0.01
   cargo run -- sign --share-id 2 --to 0x123...456 --amount 0.01

      cargo run -- sign-round2 --share-id 1 --commitment-files commit_1_abcd.json commit_2_abcd.json
   cargo run -- sign-round2 --share-id 2 --commitment-files commit_1_abcd.json commit_2_abcd.json

      cargo run -- combine --signature-files sig_1_abcd.json sig_2_abcd.json

      cargo run -- get-wallet-info --share-id 1
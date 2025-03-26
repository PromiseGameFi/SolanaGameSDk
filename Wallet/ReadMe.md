### Generating Key Shares

To generate key shares for your wallet, run the following command:
```
cargo run -- generate-keys --threshold 2 --shares 3
```
This will generate 3 key shares with a threshold of 2, meaning at least 2 shares are required to perform any wallet operations.

### Signing a Transaction (Round 1)

To sign a transaction using a key share, run the following command:
```
cargo run -- sign --share-id 1 --to 0x868263F7D8B5339655E7B3C9097c7b5149099AAf --amount 0.001

cargo run -- sign --share-id 2 --to 0x868263F7D8B5339655E7B3C9097c7b5149099AAf --amount 0.001
```
Replace `1` with the ID of the key share you want to use, `0x123...456` with the recipient's Ethereum address, and `0.01` with the amount of ETH you want to send.

### Signing a Transaction (Round 2)

After collecting commitments from all signers, run the following command to create a partial signature:
```
cargo run -- sign-round2 --share-id 1 --commitment-files commit_1_abcd.json commit_2_abcd.json 
   cargo run -- sign-round2 --share-id 1 --commitment-files commit_1_tx8678638dab764ba1.json commit_2_tx8678638dab764ba1.json
   cargo run -- sign-round2 --share-id 2 --commitment-files commit_1_tx8678638dab764ba1.json commit_2_tx8678638dab764ba1.json  
```
Replace `1` with the ID of the key share you want to use, and `commit_1_abcd.json` and `commit_2_abcd.json` with the paths to the commitment files from Round 1.

### Combining Signatures

Once you have all the partial signatures, run the following command to combine them:
```
cargo run -- combine --signature-files sig_1_abcd.json sig_2_abcd.json

cargo run -- combine --signature-files sig_1_tx8678638dab764ba1.json sig_2_tx8678638dab764ba1.json


sig_1_ec346bf9.json
```
Replace `sig_1_abcd.json` and `sig_2_abcd.json` with the paths to the signature files generated in Round 2.

### Getting Wallet Information


To get information about your wallet, including the Ethereum address and balance, run the following command:
```
cargo run -- get-wallet-info --share-id 1
```
Replace `1` with the ID of the key share you want to use.
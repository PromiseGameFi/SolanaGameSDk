import json
from web3 import Web3

# Use Alchemy Sepolia RPC URL
ALCHEMY_URL = "https://eth-sepolia.alchemyapi.io/v2/YOUR_ALCHEMY_API_KEY"
web3 = Web3(Web3.HTTPProvider(ALCHEMY_URL))

def send_transaction(full_signature, to_address, value):
    """Send Ethereum transaction using reconstructed signature."""
    if not web3.is_connected():
        print("❌ Failed to connect to Ethereum Sepolia")
        return

    tx = {
        "to": to_address,
        "value": web3.to_wei(value, "ether"),
        "gas": 21000,
        "gasPrice": web3.eth.gas_price,
        "nonce": web3.eth.get_transaction_count(web3.eth.default_account),
        "chainId": 11155111,  # Sepolia Chain ID
    }

    signed_tx = web3.eth.account.sign_transaction(tx, full_signature)
    tx_hash = web3.eth.send_raw_transaction(signed_tx.rawTransaction)
    print(f"🚀 Transaction sent! Hash: {web3.to_hex(tx_hash)}")

if __name__ == "__main__":
    signature = input("Enter reconstructed signature: ")
    recipient = input("Enter recipient address: ")
    amount = float(input("Enter amount in ETH: "))

    send_transaction(signature, recipient, amount)

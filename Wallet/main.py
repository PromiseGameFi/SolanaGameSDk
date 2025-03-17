import argparse
import json
import os
import random
import hashlib
from typing import List, Dict, Any, Tuple
import base64

import secp256k1
from web3 import Web3
from web3.middleware import geth_poa_middleware
from eth_account import Account
from eth_account.messages import encode_defunct
from Crypto.Protocol.SecretSharing import Shamir

# Constants
SHARES_DIR = "key_shares"
SEPOLIA_RPC_URL = "https://sepolia.infura.io/v3/713582309bac484c98256ff6a93e9bac"  # Replace with your Alchemy API key
CURVE_ORDER = 0xFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFEBAAEDCE6AF48A03BBFD25E8CD0364141

# Initialize Web3
w3 = Web3(Web3.HTTPProvider(SEPOLIA_RPC_URL))
w3.middleware_onion.inject(geth_poa_middleware, layer=0)  # Required for Sepolia

def setup_directory():
    """Create directory for storing key shares if it doesn't exist."""
    if not os.path.exists(SHARES_DIR):
        os.makedirs(SHARES_DIR)

def generate_shares(total_shares: int, threshold: int) -> Dict[str, Any]:
    """
    Generate a private key and split it into shares using Shamir's Secret Sharing.
    
    Args:
        total_shares: Total number of shares to create
        threshold: Minimum number of shares required to reconstruct the secret
        
    Returns:
        Dict containing wallet info including public key and address
    """
    setup_directory()
    
    # Generate private key
    private_key_bytes = os.urandom(32)
    private_key_int = int.from_bytes(private_key_bytes, byteorder='big')
    private_key_hex = private_key_bytes.hex()
    
    # Generate shares using Shamir's Secret Sharing
    shares = Shamir.split(threshold, total_shares, private_key_int, CURVE_ORDER)
    
    # Derive public key and address
    account = Account.from_key(private_key_hex)
    public_key = account.key.public_key
    address = account.address
    
    # Save shares to files
    for idx, share in shares:
        share_data = {
            "index": idx,
            "value": share,
            "total_shares": total_shares,
            "threshold": threshold,
            "public_key": public_key.to_hex(),
            "address": address
        }
        
        with open(os.path.join(SHARES_DIR, f"share_{idx}.json"), 'w') as f:
            json.dump(share_data, f, indent=4)
    
    # Return wallet info
    wallet_info = {
        "public_key": public_key.to_hex(),
        "address": address,
        "total_shares": total_shares,
        "threshold": threshold,
        "shares_location": SHARES_DIR
    }
    
    # Save wallet info
    with open("wallet_info.json", 'w') as f:
        json.dump(wallet_info, f, indent=4)
    
    return wallet_info

def get_wallet_info() -> Dict[str, Any]:
    """
    Get wallet information including address.
    Returns the content of wallet_info.json or extracts info from any share file.
    """
    # Try to load from wallet_info.json first
    if os.path.exists("wallet_info.json"):
        with open("wallet_info.json", 'r') as f:
            return json.load(f)
    
    # If wallet_info.json doesn't exist, try to find any share file
    if not os.path.exists(SHARES_DIR):
        raise FileNotFoundError("No wallet has been created yet. Use the 'generate' command first.")
    
    share_files = [f for f in os.listdir(SHARES_DIR) if f.startswith("share_") and f.endswith(".json")]
    if not share_files:
        raise FileNotFoundError("No share files found. Use the 'generate' command first.")
    
    # Load the first share file found
    with open(os.path.join(SHARES_DIR, share_files[0]), 'r') as f:
        share_data = json.load(f)
    
    # Extract wallet info from share data
    return {
        "address": share_data["address"],
        "public_key": share_data["public_key"],
        "total_shares": share_data["total_shares"],
        "threshold": share_data["threshold"]
    }

def check_wallet_balance() -> float:
    """Check the balance of the wallet in ETH."""
    wallet_info = get_wallet_info()
    balance_wei = w3.eth.get_balance(wallet_info["address"])
    balance_eth = w3.from_wei(balance_wei, 'ether')
    return balance_eth

def load_share(share_idx: int) -> Dict[str, Any]:
    """Load a share from file."""
    share_path = os.path.join(SHARES_DIR, f"share_{share_idx}.json")
    if not os.path.exists(share_path):
        raise ValueError(f"Share {share_idx} does not exist")
    
    with open(share_path, 'r') as f:
        return json.load(f)

def get_current_gas_price() -> int:
    """Get current gas price from the network in Wei."""
    try:
        # Get gas price from the network
        gas_price = w3.eth.gas_price
        
        # Add a small buffer (10%) to ensure transaction goes through
        gas_price_with_buffer = int(gas_price * 1.1)
        
        print(f"Current gas price: {w3.from_wei(gas_price, 'gwei'):.2f} Gwei")
        print(f"Using gas price with buffer: {w3.from_wei(gas_price_with_buffer, 'gwei'):.2f} Gwei")
        
        return gas_price_with_buffer
    except Exception as e:
        print(f"Error getting gas price from network: {e}")
        print("Using default gas price of 20 Gwei")
        return w3.to_wei(20, 'gwei')  # Fallback to 20 Gwei

def create_transaction(from_address: str, to_address: str, value_wei: int, 
                      gas_limit: int, gas_price: int = None, nonce: int = None) -> Dict[str, Any]:
    """Create an Ethereum transaction dictionary."""
    if nonce is None:
        nonce = w3.eth.get_transaction_count(from_address)
    
    if gas_price is None:
        gas_price = get_current_gas_price()
    
    return {
        "from": from_address,
        "to": to_address,
        "value": value_wei,
        "gas": gas_limit,
        "gasPrice": gas_price,
        "nonce": nonce,
        "chainId": 11155111  # Sepolia chain ID
    }

def hash_transaction(transaction: Dict[str, Any]) -> bytes:
    """Hash a transaction for signing."""
    tx_dict = {
        "nonce": transaction["nonce"],
        "gasPrice": transaction["gasPrice"],
        "gas": transaction["gas"],
        "to": transaction["to"],
        "value": transaction["value"],
        "chainId": transaction["chainId"]
    }
    
    # Use keccak256 hash for Ethereum transactions
    tx_bytes = json.dumps(tx_dict, sort_keys=True).encode()
    return w3.keccak(tx_bytes)

def create_partial_signature(share_idx: int, transaction: Dict[str, Any]) -> Dict[str, Any]:
    """
    Create a partial signature for a transaction using a key share.
    
    Args:
        share_idx: Index of the share to use
        transaction: Transaction details to sign
        
    Returns:
        Dict containing the partial signature information
    """
    # Load share
    share_data = load_share(share_idx)
    share_value = share_data["value"]
    
    # Hash the transaction
    tx_hash = hash_transaction(transaction)
    
    # Generate a random nonce (k) for ECDSA signature
    k = random.randint(1, CURVE_ORDER - 1)
    
    # Calculate R = k*G (this would normally be done with a proper EC multiplication)
    # This is a simplified example; in practice we would use proper EC operations
    privkey = secp256k1.PrivateKey()
    pubkey = privkey.pubkey  # This is just a placeholder for demonstration
    
    # In a real implementation, we would do proper Lagrange interpolation
    # For now, we'll store the share value as our partial signature component
    # along with transaction information
    
    partial_sig = {
        "share_idx": share_idx,
        "transaction": transaction,
        "tx_hash": tx_hash.hex(),
        "partial_signature_data": str(share_value),
        "address": share_data["address"]
    }
    
    # Save partial signature
    partial_sig_path = os.path.join(SHARES_DIR, f"partial_sig_{share_idx}.json")
    with open(partial_sig_path, 'w') as f:
        json.dump(partial_sig, f, indent=4)
    
    return partial_sig

def reconstruct_signature(partial_sig_indices: List[int]) -> Dict[str, Any]:
    """
    Reconstruct a full signature from partial signatures.
    
    Args:
        partial_sig_indices: List of share indices with partial signatures
        
    Returns:
        Dict containing the reconstructed signature and transaction
    """
    # Load partial signatures
    partial_sigs = []
    for idx in partial_sig_indices:
        sig_path = os.path.join(SHARES_DIR, f"partial_sig_{idx}.json")
        if not os.path.exists(sig_path):
            raise ValueError(f"Partial signature for share {idx} does not exist")
        
        with open(sig_path, 'r') as f:
            partial_sigs.append(json.load(f))
    
    # Verify that all partial signatures are for the same transaction
    first_tx = partial_sigs[0]["transaction"]
    first_hash = partial_sigs[0]["tx_hash"]
    
    for sig in partial_sigs[1:]:
        if sig["tx_hash"] != first_hash:
            raise ValueError("Partial signatures are for different transactions")
    
    # Load a share to get threshold info
    share_data = load_share(partial_sig_indices[0])
    threshold = share_data["threshold"]
    
    if len(partial_sig_indices) < threshold:
        raise ValueError(f"Need at least {threshold} partial signatures, but only {len(partial_sig_indices)} provided")
    
    # Collect shares for reconstruction
    shares = []
    for sig in partial_sigs:
        shares.append((sig["share_idx"], int(sig["partial_signature_data"])))
    
    # Reconstruct the private key using Shamir's Secret Sharing
    private_key_int = Shamir.combine(shares, CURVE_ORDER)
    private_key_bytes = private_key_int.to_bytes(32, byteorder='big')
    private_key_hex = private_key_bytes.hex()
    
    # Sign the transaction with the reconstructed private key
    account = Account.from_key(private_key_hex)
    signed_tx = w3.eth.account.sign_transaction(first_tx, private_key_hex)
    
    # Save the signed transaction
    result = {
        "transaction": first_tx,
        "signed_transaction": signed_tx.rawTransaction.hex(),
        "tx_hash": signed_tx.hash.hex(),
        "reconstructed_from_shares": partial_sig_indices
    }
    
    with open("signed_transaction.json", 'w') as f:
        json.dump(result, f, indent=4)
    
    return result

def send_transaction(signed_tx_path: str = "signed_transaction.json") -> str:
    """
    Send a signed transaction to the Ethereum network.
    
    Args:
        signed_tx_path: Path to the signed transaction JSON file
        
    Returns:
        Transaction hash
    """
    # Load signed transaction
    with open(signed_tx_path, 'r') as f:
        signed_tx_data = json.load(f)
    
    raw_tx = signed_tx_data["signed_transaction"]
    
    # Convert hex string to bytes
    if raw_tx.startswith('0x'):
        raw_tx = raw_tx[2:]
    raw_tx_bytes = bytes.fromhex(raw_tx)
    
    # Send the transaction
    tx_hash = w3.eth.send_raw_transaction(raw_tx_bytes)
    
    print(f"Transaction sent: {tx_hash.hex()}")
    print(f"View on Etherscan: https://sepolia.etherscan.io/tx/{tx_hash.hex()}")
    
    return tx_hash.hex()

def main():
    parser = argparse.ArgumentParser(description="MPC-TSS Ethereum Wallet for Sepolia")
    subparsers = parser.add_subparsers(dest="command", help="Commands")
    
    # Generate shares command
    gen_parser = subparsers.add_parser("generate", help="Generate key shares")
    gen_parser.add_argument("--total", type=int, required=True, help="Total number of shares")
    gen_parser.add_argument("--threshold", type=int, required=True, help="Threshold number of shares required")
    
    # Info command
    info_parser = subparsers.add_parser("info", help="Display wallet information and balance")
    
    # Create partial signature command
    sig_parser = subparsers.add_parser("sign", help="Create partial signature")
    sig_parser.add_argument("--share", type=int, required=True, help="Share index to use")
    sig_parser.add_argument("--to", type=str, required=True, help="Recipient address")
    sig_parser.add_argument("--value", type=float, required=True, help="Amount in ETH")
    sig_parser.add_argument("--gas-limit", type=int, default=21000, help="Gas limit (default: 21000)")
    sig_parser.add_argument("--gas-price", type=int, help="Optional manual gas price in Gwei (if not specified, will be fetched from network)")
    
    # Reconstruct signature command
    rec_parser = subparsers.add_parser("reconstruct", help="Reconstruct full signature")
    rec_parser.add_argument("--shares", type=int, nargs="+", required=True, help="Share indices to use")
    
    # Send transaction command
    send_parser = subparsers.add_parser("send", help="Send transaction")
    
    args = parser.parse_args()
    
    if args.command == "generate":
        wallet_info = generate_shares(args.total, args.threshold)
        print(f"Generated {args.total} shares with threshold {args.threshold}")
        print(f"Wallet address: {wallet_info['address']}")
        print(f"Shares saved in {SHARES_DIR} directory")
        print("\nTo get test ETH, visit a Sepolia faucet and send to this address:")
        print(f"https://sepoliafaucet.com/ or https://sepolia-faucet.pk910.de/")
    
    elif args.command == "info":
        try:
            wallet_info = get_wallet_info()
            balance = check_wallet_balance()
            
            print("\n=== MPC-TSS Wallet Information ===")
            print(f"Wallet Address: {wallet_info['address']}")
            print(f"Current Balance: {balance:.6f} ETH")
            print(f"Total Shares: {wallet_info.get('total_shares', 'Unknown')}")
            print(f"Threshold: {wallet_info.get('threshold', 'Unknown')}")
            print(f"Etherscan: https://sepolia.etherscan.io/address/{wallet_info['address']}")
            print("\nTo get test ETH, visit a Sepolia faucet:")
            print(f"https://sepoliafaucet.com/ or https://sepolia-faucet.pk910.de/")
        except Exception as e:
            print(f"Error retrieving wallet information: {e}")
    
    elif args.command == "sign":
        # Load wallet info to get the from address
        wallet_info = get_wallet_info()
        
        # Convert optional gas price from Gwei to Wei if specified
        gas_price = None
        if args.gas_price:
            gas_price = w3.to_wei(args.gas_price, 'gwei')
        
        # Create transaction dict
        tx = create_transaction(
            from_address=wallet_info["address"],
            to_address=args.to,
            value_wei=w3.to_wei(args.value, 'ether'),
            gas_limit=args.gas_limit,
            gas_price=gas_price
        )
        
        # Create partial signature
        partial_sig = create_partial_signature(args.share, tx)
        print(f"Created partial signature for share {args.share}")
        print(f"Transaction: {args.value} ETH to {args.to}")
        print(f"Partial signature saved in {SHARES_DIR}/partial_sig_{args.share}.json")
    
    elif args.command == "reconstruct":
        result = reconstruct_signature(args.shares)
        print(f"Reconstructed signature from shares {args.shares}")
        print(f"Transaction ready to send")
        print(f"Signed transaction saved in signed_transaction.json")
    
    elif args.command == "send":
        tx_hash = send_transaction()
        print(f"Transaction sent with hash: {tx_hash}")
    
    else:
        parser.print_help()

if __name__ == "__main__":
    main()
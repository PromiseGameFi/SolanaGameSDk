import os

def main():
    print("\n🚀 MPC-TSS Wallet CLI 🚀\n")
    print("1. Generate Key Shares")
    print("2. Sign Message Partially")
    print("3. Reconstruct Signature")
    print("4. Send Transaction")
    print("5. Exit")

    choice = input("\nSelect an option: ")

    if choice == "1":
        os.system("python src/keygen.py")
    elif choice == "2":
        os.system("python src/sign.py")
    elif choice == "3":
        os.system("python src/reconstruct.py")
    elif choice == "4":
        os.system("python src/send_tx.py")
    elif choice == "5":
        exit()
    else:
        print("❌ Invalid option!")

if __name__ == "__main__":
    while True:
        main()

// SolanaWallet.cpp

#include "SolanaWallet.h"
#include "Misc/Base64.h"
#include "Misc/SecureHash.h"
#include "GenericPlatform/GenericPlatformCrypto.h"

FWalletInfo USolanaWallet::CurrentWallet;
FString USolanaWallet::SolanaRPCUrl = "https://api.mainnet-beta.solana.com";

bool USolanaWallet::CreateWallet(FWalletInfo& OutWalletInfo)
{
    // Generate a random 32-byte private key
    uint8 PrivateKeyBytes[32];
    FGenericPlatformCrypto::GetRandomBytes(PrivateKeyBytes, 32);

    // Convert private key to base58
    FString PrivateKey = FBase64::Encode(PrivateKeyBytes, 32);

    // Derive public key from private key (in a real implementation, this would use ed25519 derivation)
    FString PublicKey = FMD5::HashAnsiString(*PrivateKey);

    CurrentWallet.PrivateKey = PrivateKey;
    CurrentWallet.PublicKey = PublicKey;

    OutWalletInfo = CurrentWallet;
    return true;
}

FString USolanaWallet::GetWalletAddress()
{
    return CurrentWallet.PublicKey;
}

float USolanaWallet::GetWalletBalance()
{
    TArray<TSharedPtr<FJsonValue>> Params;
    Params.Add(MakeShared<FJsonValueString>(CurrentWallet.PublicKey));
    Params.Add(MakeShared<FJsonValueObject>(MakeShared<FJsonObject>()));

    TSharedPtr<FJsonObject> Result = SendRPCRequest("getBalance", Params);

    if (Result.IsValid())
    {
        double Balance;
        if (Result->TryGetNumberField("value", Balance))
        {
            return Balance / 1e9; // Convert lamports to SOL
        }
    }

    return 0.0f;
}

bool USolanaWallet::ImportWallet(const FString& PrivateKey, FWalletInfo& OutWalletInfo)
{
    // Validate private key format (this is a simplified check)
    if (PrivateKey.Len() != 44 || !PrivateKey.StartsWith("
Assistant: "))
    {
        return false;
    }

    // Derive public key from private key (in a real implementation, this would use ed25519 derivation)
    FString PublicKey = FMD5::HashAnsiString(*PrivateKey);

    CurrentWallet.PrivateKey = PrivateKey;
    CurrentWallet.PublicKey = PublicKey;

    OutWalletInfo = CurrentWallet;
    return true;
}

bool USolanaWallet::ExportWallet(FString& OutPrivateKey)
{
    if (CurrentWallet.PrivateKey.IsEmpty())
    {
        return false;
    }

    OutPrivateKey = CurrentWallet.PrivateKey;
    return true;
}

TSharedPtr<FJsonObject> USolanaWallet::SendRPCRequest(const FString& Method, const TArray<TSharedPtr<FJsonValue>>& Params)
{
    TSharedPtr<FJsonObject> RequestObj = MakeShared<FJsonObject>();
    RequestObj->SetStringField("jsonrpc", "2.0");
    RequestObj->SetStringField("id", "1");
    RequestObj->SetStringField("method", Method);
    RequestObj->SetArrayField("params", Params);

    FString RequestBody;
    TSharedRef<TJsonWriter<>> Writer = TJsonWriterFactory<>::Create(&RequestBody);
    FJsonSerializer::Serialize(RequestObj.ToSharedRef(), Writer);

    TSharedRef<IHttpRequest, ESPMode::ThreadSafe> HttpRequest = FHttpModule::Get().CreateRequest();
    HttpRequest->SetURL(SolanaRPCUrl);
    HttpRequest->SetVerb("POST");
    HttpRequest->SetHeader("Content-Type", "application/json");
    HttpRequest->SetContentAsString(RequestBody);

    HttpRequest->ProcessRequest();

    // Wait for the request to complete
    while (HttpRequest->GetStatus() == EHttpRequestStatus::Processing)
    {
        FPlatformProcess::Sleep(0.01);
    }

    if (HttpRequest->GetStatus() == EHttpRequestStatus::Succeeded)
    {
        TSharedPtr<FJsonObject> JsonObject;
        TSharedRef<TJsonReader<>> Reader = TJsonReaderFactory<>::Create(HttpRequest->GetContentAsString());
        if (FJsonSerializer::Deserialize(Reader, JsonObject))
        {
            TSharedPtr<FJsonObject> Result;
            if (JsonObject->TryGetObjectField("result", Result))
            {
                return Result;
            }
        }
    }

    return nullptr;
}
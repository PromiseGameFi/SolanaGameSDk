// SolanaWallet.h

#pragma once

#include "CoreMinimal.h"
#include "Kismet/BlueprintFunctionLibrary.h"
#include "Http.h"
#include "Json.h"
#include "SolanaWallet.generated.h"

USTRUCT(BlueprintType)
struct FWalletInfo
{
    GENERATED_BODY()

    UPROPERTY(BlueprintReadOnly, Category = "Solana|Wallet")
    FString PublicKey;

    UPROPERTY(BlueprintReadOnly, Category = "Solana|Wallet")
    FString PrivateKey;
};

UCLASS()
class SOLANAGAMESDK_API USolanaWallet : public UBlueprintFunctionLibrary
{
    GENERATED_BODY()

public:
    UFUNCTION(BlueprintCallable, Category = "Solana|Wallet")
    static bool CreateWallet(FWalletInfo& OutWalletInfo);

    UFUNCTION(BlueprintCallable, Category = "Solana|Wallet")
    static FString GetWalletAddress();

    UFUNCTION(BlueprintCallable, Category = "Solana|Wallet")
    static float GetWalletBalance();

    UFUNCTION(BlueprintCallable, Category = "Solana|Wallet")
    static bool ImportWallet(const FString& PrivateKey, FWalletInfo& OutWalletInfo);

    UFUNCTION(BlueprintCallable, Category = "Solana|Wallet")
    static bool ExportWallet(FString& OutPrivateKey);


private:
    static FWalletInfo CurrentWallet;
    static FString SolanaRPCUrl;

    static TSharedPtr<FJsonObject> SendRPCRequest(const FString& Method, const TArray<TSharedPtr<FJsonValue>>& Params);

};
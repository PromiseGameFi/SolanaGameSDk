#pragma once

#include "CoreMinimal.h"
#include "Kismet/BlueprintFunctionLibrary.h"
#include "SolanaTransactions.generated.h"

UCLASS()
class SOLANAGAMESDK_API USolanaTransactions : public UBlueprintFunctionLibrary
{
    GENERATED_BODY()

public:
    UFUNCTION(BlueprintCallable, Category = "Solana|Transactions")
    static void SendSOL(FString RecipientAddress, float Amount, const FString& Callback);

    UFUNCTION(BlueprintCallable, Category = "Solana|Transactions")
    static void SendToken(FString TokenMint, FString RecipientAddress, int32 Amount, const FString& Callback);

private:
    static void OnResponseReceived(FHttpRequestPtr Request, FHttpResponsePtr Response, bool bWasSuccessful, FString Callback);
};
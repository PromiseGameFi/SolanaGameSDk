#pragma once

#include "CoreMinimal.h"
#include "Kismet/BlueprintFunctionLibrary.h"
#include "SolanaMarketplace.generated.h"

UCLASS()
class SOLANAGAMESDK_API USolanaMarketplace : public UBlueprintFunctionLibrary
{
    GENERATED_BODY()

public:
    UFUNCTION(BlueprintCallable, Category = "Solana|Marketplace")
    static void ListItemForSale(FString ItemID, float Price, const FString& Callback);

    UFUNCTION(BlueprintCallable, Category = "Solana|Marketplace")
    static void BuyItem(FString ItemID, const FString& Callback);

private:
    static void OnResponseReceived(FHttpRequestPtr Request, FHttpResponsePtr Response, bool bWasSuccessful, FString Callback);
};
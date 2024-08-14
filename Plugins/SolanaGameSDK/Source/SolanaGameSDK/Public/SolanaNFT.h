#pragma once

#include "CoreMinimal.h"
#include "Kismet/BlueprintFunctionLibrary.h"
#include "SolanaNFT.generated.h"

UCLASS()
class SOLANAGAMESDK_API USolanaNFT : public UBlueprintFunctionLibrary
{
    GENERATED_BODY()

public:
    UFUNCTION(BlueprintCallable, Category = "Solana|NFT")
    static void MintNFT(FString MetadataURI, const FString& Callback);

    UFUNCTION(BlueprintCallable, Category = "Solana|NFT")
    static void TransferNFT(FString NFTMint, FString RecipientAddress, const FString& Callback);

private:
    static void OnResponseReceived(FHttpRequestPtr Request, FHttpResponsePtr Response, bool bWasSuccessful, FString Callback);
};
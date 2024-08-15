#pragma once

#include "CoreMinimal.h"
#include "Kismet/BlueprintFunctionLibrary.h"
#include "SolanaWallet.generated.h"

UCLASS()
class SOLANAGAMESDK_API USolanaWallet : public UBlueprintFunctionLibrary
{
    GENERATED_BODY()

public:
    UFUNCTION(BlueprintCallable, Category = "Solana|Wallet")
    static bool CreateWallet();

    UFUNCTION(BlueprintCallable, Category = "Solana|Wallet")
    static FString GetWalletAddress();
};

//gameName
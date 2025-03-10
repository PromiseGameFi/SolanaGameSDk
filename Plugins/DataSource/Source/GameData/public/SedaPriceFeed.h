#pragma once

#include "CoreMinimal.h"
#include "Components/ActorComponent.h"
#include "SedaSDK.h" // SEDA SDK header
#include "SedaPriceFeed.generated.h"

UCLASS(ClassGroup=(Custom), meta=(BlueprintSpawnableComponent))
class YOURGAME_API USedaPriceFeed : public UActorComponent
{
    GENERATED_BODY()

public:    
    USedaPriceFeed();

    UFUNCTION(BlueprintCallable, Category = "SEDA")
    float RequestPriceData();

    // Blueprint-exposed event for when price data is received
    UPROPERTY(BlueprintAssignable, Category = "SEDA")
    FOnPriceDataReceived OnPriceDataReceived;

protected:
    virtual void BeginPlay() override;

private:
    UPROPERTY()
    class USedaClient* SedaClient;
};
#include "SedaPriceFeed.h"
#include "SedaSDK.h"

USedaPriceFeed::USedaPriceFeed()
{
    PrimaryComponentTick.bCanEverTick = false;
}

void USedaPriceFeed::BeginPlay()
{
    Super::BeginPlay();
    
    // Initialize SEDA client
    SedaClient = NewObject<USedaClient>();
    SedaClient->Initialize();
}

float USedaPriceFeed::RequestPriceData()
{
    if (!SedaClient)
    {
        UE_LOG(LogTemp, Warning, TEXT("SEDA Client not initialized"));
        return 0.0f;
    }

    // Create request parameters
    FSedaRequestParams Params;
    Params.OracleId = TEXT("price_feed"); 
    Params.TimeoutSeconds = 30;

    // Make the async request
    SedaClient->MakeRequest(Params, [this](const FSedaResponse& Response)
    {
        if (Response.IsSuccessful())
        {
            float Price = FCString::Atof(*Response.Data);
            // Broadcast the received price to any blueprint listeners
            OnPriceDataReceived.Broadcast(Price);
            return Price;
        }
        else
        {
            UE_LOG(LogTemp, Warning, TEXT("Failed to get price data: %s"), 
                   *Response.ErrorMessage);
            return 0.0f;
        }
    });
}
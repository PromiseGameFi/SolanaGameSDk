#include "SolanaTransactions.h"
#include "Http.h"

void USolanaTransactions::SendSOL(FString RecipientAddress, float Amount, const FString& Callback)
{
    TSharedRef<IHttpRequest, ESPMode::ThreadSafe> Request = FHttpModule::Get().CreateRequest();
    Request->OnProcessRequestComplete().BindStatic(&USolanaTransactions::OnResponseReceived, Callback);
    Request->SetURL("https://api.mainnet-beta.solana.com");
    Request->SetVerb("POST");
    Request->SetHeader("Content-Type", "application/json");

    FString JsonString = FString::Printf(TEXT("{\"method\":\"sendTransaction\",\"params\":[\"%s\",%f]}"), *RecipientAddress, Amount);
    Request->SetContentAsString(JsonString);

    Request->ProcessRequest();
}

void USolanaTransactions::SendToken(FString TokenMint, FString RecipientAddress, int32 Amount, const FString& Callback)
{
    // Implement token sending logic here
    // This would involve creating a token transfer instruction, signing it, and sending it to the network
    // For now, we'll just simulate an HTTP request like in SendSOL
}

void USolanaTransactions::OnResponseReceived(FHttpRequestPtr Request, FHttpResponsePtr Response, bool bWasSuccessful, FString Callback)
{
    if (bWasSuccessful && Response.IsValid())
    {
        // Process the response
        FString ResponseString = Response->GetContentAsString();
        // Call the Callback function in Blueprints with the response
        // You'll need to implement a way to call Blueprint functions from C++
    }
    else
    {
        // Handle error
    }
}
using UnrealBuildTool;
using System.IO;

public class SolanaGameSDK : ModuleRules
{
    public SolanaGameSDK(ReadOnlyTargetRules Target) : base(Target)
    {
        PCHUsage = ModuleRules.PCHUsageMode.UseExplicitOrSharedPCHs;

        PublicIncludePaths.AddRange(
            new string[] {
                // ... add public include paths required here ...
            }
        );

        PrivateIncludePaths.AddRange(
            new string[] {
                // ... add other private include paths required here ...
            }
        );

      
        PrivateDependencyModuleNames.AddRange(
            new string[]
            {
                "Slate",
                "SlateCore"
            }
        );

        DynamicallyLoadedModuleNames.AddRange(
            new string[]
            {
                // ... add any modules that your module loads dynamically here ...
            }
        );

        // Uncomment if you are using Slate UI
        // PrivateDependencyModuleNames.AddRange(new string[] { "Slate", "SlateCore" });

        // Uncomment if you are using online features
        // PrivateDependencyModuleNames.Add("OnlineSubsystem");

        // To include OnlineSubsystemSteam, add it to the plugins section in your uproject file with the Enabled attribute set to true

        // If you're using a third-party Solana SDK, you might need to add it here
        // string SolanaSDKPath = Path.GetFullPath(Path.Combine(ModuleDirectory, "../../ThirdParty/SolanaSDK/"));
        // PublicIncludePaths.Add(Path.Combine(SolanaSDKPath, "include"));
        // PublicAdditionalLibraries.Add(Path.Combine(SolanaSDKPath, "lib", "SolanaSDK.lib"));
    }
}
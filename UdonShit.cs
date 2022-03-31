using System.Diagnostics;
using System.IO;
using System.Reflection;
using HarmonyLib;
using Il2CppSystem.Collections.Generic;
using MelonLoader;
using VRC.Udon.ClientBindings;
using VRC.Udon.Common;
using VRC.Udon.Common.Interfaces;
using VRC.Udon.ProgramSources;
using VRC.Udon.Security;
using VRC.Udon.Security.Interfaces;
using VRC.Udon.Serialization.OdinSerializer;
using VRC.Udon.VM;
using VRC.Udon.Wrapper;
using static UnhollowerRuntimeLib.ClassInjector;

[assembly: MelonInfo(typeof(vrc_udon_shit.UdonShit), "Udon Shit", "0.0.1", "Kitlith & Behemoth")]
[assembly: MelonGame("VRChat", "VRChat")]

namespace vrc_udon_shit {
    public class UdonShit : MelonMod {

        public static MelonLogger.Instance logger;

        public override void OnApplicationStart() {
            logger = LoggerInstance;

            var dllName = "native.dll";
            var dstPath = "VRChat_Data/Plugins/" + dllName;

            try
            {
                using var resourceStream = Assembly.GetExecutingAssembly().GetManifestResourceStream(dllName);
                using var fileStream = File.Open(dstPath, FileMode.Create, FileAccess.Write);
                resourceStream.CopyTo(fileStream);
            }
            catch (IOException ex)
            {
                LoggerInstance.Error("Failed to copy native library: " + ex.Message);
            }

            if (!NativeInterpreterApi.Internals.Initialize(dstPath))
            {
                LoggerInstance.Error("Failed to load native library");
                return;
            }

            RegisterTypeInIl2CppWithInterfaces<UdonVMDynarec>(true, typeof(IUdonVM));
            if (Stopwatch.IsHighResolution)
            {
                LoggerInstance.Msg("Using High Resolution Stopwatch! :)");
            }

            logger.Msg("Initialized native library");

            var blacklist = new UnityEngineObjectSecurityBlacklist();

            logger.Msg("a");

            var wrapperFactory = new UdonDefaultWrapperFactory(blacklist.Cast<IUdonSecurityBlacklist<UnityEngine.Object>>());

            logger.Msg("Created WrapperFactory");

            var wrapper = wrapperFactory.GetWrapper();

            logger.Msg("c");

            var timeSource = new UdonVMTimeSource();

            logger.Msg("d");

            var dynarec = new UdonVMDynarec(wrapper, timeSource.Cast<IUdonVMTimeSource>());

            logger.Msg("constructed dynarec");

            var stream = Assembly.GetExecutingAssembly().GetManifestResourceStream("vrc_udon_shit.recursive_test.txt");
            var contents = new StreamReader(stream).ReadToEnd();

            logger.Msg("Loaded serialized program");

            var serializedProgramAsset = new SerializedUdonProgramAsset
            {
                serializedProgramBytesString = contents,
                serializationDataFormat = DataFormat.Binary,
                programUnityEngineObjects = new List<UnityEngine.Object>(),
                name = "recursive_test"
            };

            logger.Msg("Constructed");

            var program = serializedProgramAsset.RetrieveProgram();
            foreach (var entryPoint in program.EntryPoints.GetSymbols()) {
                logger.Msg("Entry point: " + entryPoint + " " + program.EntryPoints.GetAddressFromSymbol(entryPoint));
            }
            foreach (var symbol in program.SymbolTable.GetSymbols()) {
                logger.Msg("Symbol: " + symbol + " " + program.SymbolTable.GetAddressFromSymbol(symbol));
            }
            var entry = program.EntryPoints.GetAddressFromSymbol("ExecuteTests");

            logger.Msg("Deserialized!");

            dynarec.LoadProgram(program);
            dynarec.SetProgramCounter(entry);
            dynarec.Interpret();

            logger.Msg("Done!");

            Process.GetCurrentProcess().Kill();
        }

        public override void OnSceneWasUnloaded(int buildIndex, string sceneName) {
            if (buildIndex == -1)
            {
                UdonVMFactoryPatch.Objects.Clear();
                LoggerInstance.Msg("Clearing VM Objects.");
            }
        }
    }

    [HarmonyPatch(typeof(VRC.Udon.VM.UdonVMFactory), "ConstructUdonVM")]
    class UdonVMFactoryPatch {
        static bool Prefix(VRC.Udon.VM.UdonVMFactory __instance, ref IUdonVM __result) {
            var dynarec = new UdonVMDynarec(__instance._wrapperFactory.GetWrapper(), __instance._udonVMTimeSource);
            Objects.Add(dynarec);
            __result = new IUdonVM(dynarec.Pointer);
            return false; // we're completely overriding the method, sorry.
        }

        internal readonly static Il2CppSystem.Collections.Generic.List<Il2CppSystem.Object> Objects = new Il2CppSystem.Collections.Generic.List<Il2CppSystem.Object>();
    }

    // [HarmonyPatch(typeof(VRC.Udon.UdonBehaviour), "ProcessEntryPoints")]
    // class UdonBehaviourPassNamePatch {
    //     static void Prefix(VRC.Udon.UdonBehaviour __instance) {
    //         var dynarec = __instance._udonVM.TryCast<UdonVMDynarec>();
    //         dynarec.SetName($"{__instance.name} ({__instance.serializedProgramAsset?.name})");
    //     }
    // }

    [HarmonyPatch(typeof(VRC.Udon.UdonBehaviour), "RunProgram", typeof(uint))]
    class TimeRunProgramPatch {
        static void Prefix(ref Stopwatch __state) {
            __state = new Stopwatch();
            __state.Start();
        }

        static void Postfix(VRC.Udon.UdonBehaviour __instance, ref Stopwatch __state) {
            __state.Stop();
            // 
            if (__instance.serializedProgramAsset.name == "48df62f63db4c32438044816f153d3f3")
            {
                UdonShit.logger.Msg($"{__instance.name} ({__instance.serializedProgramAsset.name}) took {__state.Elapsed.TotalSeconds} seconds");
            }
        }
    }
}

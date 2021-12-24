using System;
using System.Diagnostics;
using MelonLoader;

using HarmonyLib;

using VRC.Udon.Common.Interfaces;

using static UnhollowerRuntimeLib.ClassInjector;

[assembly: MelonInfo(typeof(vrc_udon_shit.UdonShit), "Udon Shit", "0.0.1", "Kitlith")]
[assembly: MelonGame("VRChat", "VRChat")]

namespace vrc_udon_shit
{
    public class UdonShit: MelonMod {
        public static MelonLogger.Instance logger;

        public readonly static Il2CppSystem.Collections.Generic.List<Il2CppSystem.Object> Objects = new Il2CppSystem.Collections.Generic.List<Il2CppSystem.Object>();

        public override void OnApplicationStart() {
            logger = LoggerInstance;
            RegisterTypeInIl2CppWithInterfaces<UdonVMDynarec>(true, typeof(IUdonVM));
            RegisterTypeInIl2CppWithInterfaces<UdonHeapReimpl>(true, typeof(IUdonHeap));
            if (Stopwatch.IsHighResolution) {
                LoggerInstance.Msg("Using High Resolution Stopwatch! :)");
            }

            logger.Msg($"GetIl2CppClass: {UnhollowerBaseLib.IL2CPP.GetIl2CppClass("UnityEngine.CoreModule.dll", "UnityEngine", "Vector3")}");
            logger.Msg($"ClassPointerStore: {UnhollowerBaseLib.Il2CppClassPointerStore<UnityEngine.Vector3>.NativeClassPtr}");
            logger.Msg($"CreatedTypeRedirect: {UnhollowerBaseLib.Il2CppClassPointerStore<UnityEngine.Vector3>.CreatedTypeRedirect}");
            System.Runtime.CompilerServices.RuntimeHelpers.RunClassConstructor(typeof(UnityEngine.Vector3).TypeHandle);
            logger.Msg($"ClassPointerStore: {UnhollowerBaseLib.Il2CppClassPointerStore<UnityEngine.Vector3>.NativeClassPtr}");
            logger.Msg($"CreatedTypeRedirect: {UnhollowerBaseLib.Il2CppClassPointerStore<UnityEngine.Vector3>.CreatedTypeRedirect}");

            // HarmonyInstance.Patch(
            //     typeof(VRC.Udon.Common.Factories.UdonHeapFactory)
            //         .GetMethod(nameof(VRC.Udon.Common.Factories.UdonHeapFactory.ConstructUdonHeap), new Type[] {typeof(uint)}),
            //     prefix: new HarmonyMethod(typeof(UdonShit).GetMethod(nameof(UdonShit.PatchConstructUdonHeapSize)))
            // );
            
            // HarmonyInstance.Patch(
            //     typeof(VRC.Udon.Common.Factories.UdonHeapFactory)
            //         .GetMethod(nameof(VRC.Udon.Common.Factories.UdonHeapFactory.ConstructUdonHeap), new Type[0]),
            //     prefix: new HarmonyMethod(typeof(UdonShit).GetMethod(nameof(UdonShit.PatchConstructUdonHeap)))
            // );
        }

        // public static bool PatchConstructUdonHeapSize(uint heapSize, ref IUdonHeap __result) {
        //     UdonShit.logger.Msg("Log uint heap construction!");
        //     var heap = new UdonHeapReimpl(heapSize);
        //     Objects.Add(heap);
        //     __result = new IUdonHeap(heap.Pointer);
        //     return false; // we're completely overriding the method, sorry.
        // }

        // public static bool PatchConstructUdonHeap(ref IUdonHeap __result) {
        //     UdonShit.logger.Msg("Log default heap construction!");
        //     var heap = new UdonHeapReimpl();
        //     Objects.Add(heap);
        //     __result = new IUdonHeap(heap.Pointer);
        //     return false; // we're completely overriding the method, sorry.
        // }

        public override void OnSceneWasUnloaded(int buildIndex, string sceneName) {
            if (buildIndex == -1) {
                Objects.Clear();
                LoggerInstance.Msg("Clearing VM Objects.");
            }
        }
    }

    [HarmonyPatch(typeof(VRC.Udon.VM.UdonVMFactory), nameof(VRC.Udon.VM.UdonVMFactory.ConstructUdonVM))]
    class UdonVMFactoryPatch {
        static bool Prefix(VRC.Udon.VM.UdonVMFactory __instance, ref IUdonVM __result) {
            var dynarec = new UdonVMDynarec(__instance._wrapperFactory.GetWrapper(), __instance._udonVMTimeSource);
            UdonShit.Objects.Add(dynarec);
            __result = new IUdonVM(dynarec.Pointer);
            return false; // we're completely overriding the method, sorry.
        }
    }

    [HarmonyPatch(typeof(VRC.Udon.UdonBehaviour), nameof(VRC.Udon.UdonBehaviour.ProcessEntryPoints))]
    class UdonBehaviourPassNamePatch {
        static void Prefix(VRC.Udon.UdonBehaviour __instance) {
            var dynarec = __instance._udonVM.TryCast<UdonVMDynarec>();
            dynarec.SetName($"{__instance.name} ({__instance.serializedProgramAsset?.name})");
        }
    }

    [HarmonyPatch(typeof(VRC.Udon.UdonBehaviour), nameof(VRC.Udon.UdonBehaviour.RunProgram), typeof(uint))]
    class TimeRunProgramPatch {
        static void Prefix(ref Stopwatch __state) {
            __state = new Stopwatch();
            __state.Start();
        }

        static void Postfix(VRC.Udon.UdonBehaviour __instance, ref Stopwatch __state) {
            __state.Stop();
            // 
            if (__instance.serializedProgramAsset.name == "48df62f63db4c32438044816f153d3f3") {
                UdonShit.logger.Msg($"{__instance.name} ({__instance.serializedProgramAsset.name}) took {__state.Elapsed.TotalSeconds} seconds");
            }
        }
    }

    [HarmonyPatch(typeof(VRC.Udon.Common.Factories.UdonHeapFactory), nameof(VRC.Udon.Common.Factories.UdonHeapFactory.ConstructUdonHeap), typeof(uint))]
    class UdonHeapFactoryPatch {
        static bool Prefix(uint heapSize, ref IUdonHeap __result) {
            var heap = new UdonHeapReimpl(heapSize);
            UdonShit.Objects.Add(heap);
            __result = new IUdonHeap(heap.Pointer);
            return false; // we're completely overriding the method, sorry.
        }
    }
}

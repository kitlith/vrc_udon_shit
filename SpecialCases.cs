using System;
using System.Linq;
using System.Collections.Generic;
using UnhollowerBaseLib;

using Il2CppRef = Il2CppSystem.Reflection;

using Il2Type = UnhollowerRuntimeLib.Il2CppType;

using static vrc_udon_shit.CodegenInfo;

namespace vrc_udon_shit {
    public static class Hardcoded {
        public static Dictionary<string, CodegenInfo> SpecialCases = new Dictionary<string, CodegenInfo>() {
            // There is no method that takes 4 objects, but there is a method that takes an object array, and that's what the 4 objects case in udon uses.
            {
                "SystemString.__Concat__SystemObject_SystemObject_SystemObject_SystemObject__SystemString",
                new ArrayCall () {
                    method = Il2Type.Of<string>()
                        .GetMethod("Concat", new Il2CppSystem.Type[] {
                            Il2Type.Of<Il2CppSystem.Object>().MakeArrayType(),
                        }),
                    type = Il2Type.Of<Il2CppSystem.Object>(),
                    count = 4
                }
            },

            // UnaryPlus is no longer present on System.Decimal because it's a no-op and got inlined.
            // The Udon extern still acts as a copy, though.
            {
                "SystemDecimal.__op_UnaryPlus__SystemDecimal__SystemDecimal",
                CopyVariable.Of<Il2CppSystem.Decimal>()
            },

            // VRChat moved these methods from IUdonEventReceiver to IUdonProgramVariableAccessTarget
            {
                "VRCUdonCommonInterfacesIUdonEventReceiver.__GetProgramVariableType__SystemString__SystemType",
                Il2Type.Of<VRC.Udon.Common.Interfaces.IUdonProgramVariableAccessTarget>()
                    .GetMethod("GetProgramVariableType")
            },
            {
                "VRCUdonCommonInterfacesIUdonEventReceiver.__GetProgramVariable__SystemString__SystemObject",
                Il2Type.Of<VRC.Udon.Common.Interfaces.IUdonProgramVariableAccessTarget>()
                    .GetMethods()
                    .Where(m => m.Name == "GetProgramVariable" && m.GetGenericArguments().Length == 0)
                    .First()
            },
            {
                "VRCUdonCommonInterfacesIUdonEventReceiver.__SetProgramVariable__SystemString_SystemObject__SystemVoid",
                Il2Type.Of<VRC.Udon.Common.Interfaces.IUdonProgramVariableAccessTarget>()
                    .GetMethods()
                    .Where(m => m.Name == "SetProgramVariable" && m.GetGenericArguments().Length == 0)
                    .First()
            },

            // generics muck things up, this is the one case that isn't automatically handled... assuming that the other cases actually work properly...
            {
                "UnityEngineTexture2D.__SetPixelData__TArray_SystemInt32_SystemInt32__SystemVoid",
                Il2Type.Of<UnityEngine.Texture2D>()
                    .GetMethod("SetPixelData", new Il2CppSystem.Type[] {
                        Il2Type.Of<UnityEngine.Object>().MakeArrayType(),
                        Il2Type.Of<Int32>(),
                        Il2Type.Of<Int32>(),
                    })
            },

            #region Inlined Constants
            {
                "UnityEngineAINavMesh.__get_AllAreas__SystemInt32",
                Constant.Of<Int32>(new Il2CppSystem.Int32() {
                    m_value = -1
                }.BoxIl2CppObject())
            },
            {
                "UnityEngineContactFilter2D.__get_NormalAngleUpperLimit__SystemSingle",
                Constant.Of<Single>(new Il2CppSystem.Single() {
                    m_value = 359.9999f
                }.BoxIl2CppObject())
            },
            {
                "UnityEngineMathf.__get_Deg2Rad__SystemSingle",
                Constant.Of<Single>(new Il2CppSystem.Single() {
                    m_value = (float)Math.PI / 180f
                }.BoxIl2CppObject())
            },
            {
                "UnityEngineMathf.__get_Infinity__SystemSingle",
                Constant.Of<Single>(new Il2CppSystem.Single() {
                    m_value = float.PositiveInfinity
                }.BoxIl2CppObject())
            },
            {
                "UnityEngineMathf.__get_NegativeInfinity__SystemSingle",
                Constant.Of<Single>(new Il2CppSystem.Single() {
                    m_value = float.NegativeInfinity
                }.BoxIl2CppObject())
            },
            {
                "UnityEngineMathf.__get_PI__SystemSingle",
                Constant.Of<Single>(new Il2CppSystem.Single() {
                    m_value = (float)Math.PI
                }.BoxIl2CppObject())
            },
            {
                "UnityEngineMathf.__get_Rad2Deg__SystemSingle",
                Constant.Of<Single>(new Il2CppSystem.Single() {
                    m_value = 180f / (float)Math.PI
                }.BoxIl2CppObject())
            },
            {
                "UnityEnginePhysics.__get_AllLayers__SystemInt32",
                Constant.Of<Int32>(new Il2CppSystem.Int32() {
                    m_value = -1
                }.BoxIl2CppObject())
            },
            {
                "UnityEnginePhysics.__get_DefaultRaycastLayers__SystemInt32",
                Constant.Of<Int32>(new Il2CppSystem.Int32() {
                    m_value = -5
                }.BoxIl2CppObject())
            },
            {
                "UnityEnginePhysics.__get_IgnoreRaycastLayer__SystemInt32",
                Constant.Of<Int32>(new Il2CppSystem.Int32() {
                    m_value = 4
                }.BoxIl2CppObject())
            },
            {
                "UnityEnginePhysics2D.__get_AllLayers__SystemInt32",
                Constant.Of<Int32>(new Il2CppSystem.Int32() {
                    m_value = -1
                }.BoxIl2CppObject())
            },
            {
                "UnityEnginePhysics2D.__get_DefaultRaycastLayers__SystemInt32",
                Constant.Of<Int32>(new Il2CppSystem.Int32() {
                    m_value = -5
                }.BoxIl2CppObject())
            },
            {
                "UnityEnginePhysics2D.__get_IgnoreRaycastLayer__SystemInt32",
                Constant.Of<Int32>(new Il2CppSystem.Int32() {
                    m_value = 4
                }.BoxIl2CppObject())
            },
            #endregion
            
            #region Assert No-Ops
            {
                "UnityEngineDebug.__AssertFormat__SystemBoolean_SystemString_SystemObjectArray__SystemVoid",
                NoOp.Of<bool, string, Il2CppReferenceArray<Il2CppSystem.Object>>()
            },
            {
                "UnityEngineDebug.__AssertFormat__SystemBoolean_UnityEngineObject_SystemString_SystemObjectArray__SystemVoid",
                NoOp.Of<bool, UnityEngine.Object, string, Il2CppReferenceArray<Il2CppSystem.Object>>()
            },
            {
                "UnityEngineDebug.__Assert__SystemBoolean_SystemObject_UnityEngineObject__SystemVoid",
                NoOp.Of<bool, Il2CppSystem.Object, UnityEngine.Object>()
            },
            {
                "UnityEngineDebug.__Assert__SystemBoolean_SystemObject__SystemVoid",
                NoOp.Of<bool, Il2CppSystem.Object>()
            },
            {
                "UnityEngineDebug.__Assert__SystemBoolean_SystemString_UnityEngineObject__SystemVoid",
                NoOp.Of<bool, string, UnityEngine.Object>()
            },
            {
                "UnityEngineDebug.__Assert__SystemBoolean_UnityEngineObject__SystemVoid",
                NoOp.Of<bool, UnityEngine.Object>()
            },
            {
                "UnityEngineDebug.__LogAssertionFormat__SystemString_SystemObjectArray__SystemVoid",
                NoOp.Of<string, Il2CppReferenceArray<Il2CppSystem.Object>>()
            },
            {
                "UnityEngineDebug.__LogAssertionFormat__UnityEngineObject_SystemString_SystemObjectArray__SystemVoid",
                NoOp.Of<string, UnityEngine.Object, Il2CppReferenceArray<Il2CppSystem.Object>>()
            },
            {
                "UnityEngineDebug.__LogAssertion__SystemObject_UnityEngineObject__SystemVoid",
                NoOp.Of<Il2CppSystem.Object, UnityEngine.Object>()
            },
            #endregion
        };
    }
}
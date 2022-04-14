using System;
using OneOf;

using Il2CppRef = Il2CppSystem.Reflection;

using Il2Type = UnhollowerRuntimeLib.Il2CppType;

using static vrc_udon_shit.CodegenInfo;

namespace vrc_udon_shit {
    public class CodegenInfo: OneOfBase<Il2CppRef.MethodInfo, Il2CppRef.ConstructorInfo, FieldGetSet, PrimitiveOp, Constant, NoOp, CopyVariable, ArrayCall> {
        // idea: use this just to mark as constant, then pull the value at runtime?
        public struct Constant {
            public Il2CppSystem.Type ty;
            public Il2CppSystem.Object value;

            public static Constant Of<T>(Il2CppSystem.Object value) => new Constant() {
                ty = Il2Type.Of<T>(),
                value = value,
            };
        }

        public struct NoOp {
            public Il2CppSystem.Type[] args;
            public static NoOp Of() => new NoOp() { args = Array.Empty<Il2CppSystem.Type>() };
            public static NoOp Of<T0>() => new NoOp() { args = new Il2CppSystem.Type[] {
                Il2Type.Of<T0>()
            } };
            public static NoOp Of<T0, T1>() => new NoOp() { args = new Il2CppSystem.Type[] {
                Il2Type.Of<T0>(),
                Il2Type.Of<T1>()
            } };
            public static NoOp Of<T0, T1, T2>() => new NoOp() { args = new Il2CppSystem.Type[] {
                Il2Type.Of<T0>(),
                Il2Type.Of<T1>(),
                Il2Type.Of<T2>()
            } };
            public static NoOp Of<T0, T1, T2, T3>() => new NoOp() { args = new Il2CppSystem.Type[] {
                Il2Type.Of<T0>(),
                Il2Type.Of<T1>(),
                Il2Type.Of<T2>(),
                Il2Type.Of<T3>()
            } };
        }

        public struct CopyVariable {
            public Il2CppSystem.Type ty;

            public static CopyVariable Of<T>() => new CopyVariable() { ty = Il2Type.Of<T>() };
        }

        public struct ArrayCall {
            public Il2CppRef.MethodInfo method;
            public Il2CppSystem.Type type; // is this necessary? obtain from method?
            public int count;
        }

        public struct FieldGetSet {
            public Il2CppRef.FieldInfo field;
            public bool isSet;
        }

        public struct PrimitiveOp {
            public Il2CppSystem.Type type;
            public string op;
        }

        CodegenInfo(OneOf<Il2CppRef.MethodInfo, Il2CppRef.ConstructorInfo, FieldGetSet, PrimitiveOp, Constant, NoOp, CopyVariable, ArrayCall> _): base(_) {}

        public static implicit operator CodegenInfo(Il2CppRef.MethodInfo _) => new CodegenInfo(_);
        public static implicit operator CodegenInfo(Il2CppRef.ConstructorInfo _) => new CodegenInfo(_);
        public static implicit operator CodegenInfo(FieldGetSet _) => new CodegenInfo(_);
        public static implicit operator CodegenInfo(PrimitiveOp _) => new CodegenInfo(_);
        public static implicit operator CodegenInfo(Constant _) => new CodegenInfo(_);
        public static implicit operator CodegenInfo(NoOp _) => new CodegenInfo(_);
        public static implicit operator CodegenInfo(CopyVariable _) => new CodegenInfo(_);
        public static implicit operator CodegenInfo(ArrayCall _) => new CodegenInfo(_);
    }
}
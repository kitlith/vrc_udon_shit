using VRC.Udon.Common.Interfaces;
using VRC.Udon.Common;
using System;
using System.Reflection;
using UnhollowerBaseLib;

namespace vrc_udon_shit {
    sealed class UdonHeapWrapper {
        private UdonHeap inner;

        public readonly System.IntPtr Pointer;

        public UdonHeapWrapper(IUdonHeap heap) {
            inner = heap.TryCast<UdonHeap>();
            Pointer = UnhollowerBaseLib.IL2CPP.Il2CppObjectBaseToPtrNotNull(inner);
        }

        public void CopyHeapVariable(uint src, uint dest){
            inner.CopyHeapVariable(src, dest);
        }

        public Il2CppSystem.Type GetHeapVariableType(uint address) {
            return inner.GetHeapVariableType(address);
        }

        public Il2CppSystem.Object GetHeapVariable(uint address) {
            return inner.GetHeapVariable(address);
        }

        private static Type StoreType_GetHeapVariable;
        private sealed class MethodInfoStoreGeneric_GetHeapVariable<T> {
            internal static System.IntPtr Pointer;

            static MethodInfoStoreGeneric_GetHeapVariable() {
                var genericInfoStore = StoreType_GetHeapVariable.MakeGenericType(typeof(T));
                System.Runtime.CompilerServices.RuntimeHelpers.RunClassConstructor(genericInfoStore.TypeHandle);
                Pointer = (System.IntPtr)genericInfoStore
                    .GetField("Pointer", BindingFlags.NonPublic | BindingFlags.Static)
                    .GetValue(null);
            }
        }

        static UdonHeapWrapper() {
            StoreType_GetHeapVariable = Array.Find(
                typeof(UdonHeap).GetNestedTypes(BindingFlags.NonPublic),
                (Type t) => t.Name.Contains("_GetHeapVariable_")
            );
            CastMethod = typeof(Il2CppObjectBase).GetMethod(nameof(Il2CppObjectBase.Cast));
            UnboxMethod = typeof(Il2CppObjectBase).GetMethod(nameof(Il2CppObjectBase.Unbox));

            // make sure these are pre-cached, because we're going to use them.
            System.Runtime.CompilerServices.RuntimeHelpers.RunClassConstructor(typeof(MethodInfoStoreGeneric_GetHeapVariable<bool>).TypeHandle);
            System.Runtime.CompilerServices.RuntimeHelpers.RunClassConstructor(typeof(MethodInfoStoreGeneric_GetHeapVariable<uint>).TypeHandle);

            System.Runtime.CompilerServices.RuntimeHelpers.RunClassConstructor(typeof(DelegateStore<bool>).TypeHandle);
            System.Runtime.CompilerServices.RuntimeHelpers.RunClassConstructor(typeof(DelegateStore<uint>).TypeHandle);
        }

        internal delegate T CastDelegate<T>(Il2CppObjectBase obj);
        private static MethodInfo CastMethod;
        private static MethodInfo UnboxMethod;
        internal sealed class DelegateStore<T> {
            internal static CastDelegate<T> Cast; 

            static DelegateStore() {
                if (typeof(T).IsValueType) {
                    Cast = Delegate.CreateDelegate(
                        typeof(CastDelegate<T>),
                        UnboxMethod.MakeGenericMethod(typeof(T))
                    ) as CastDelegate<T>;
                } else if (typeof(T).IsSubclassOf(typeof(Il2CppObjectBase))) {
                    Cast = Delegate.CreateDelegate(
                        typeof(CastDelegate<T>),
                        CastMethod.MakeGenericMethod(typeof(T))
                    ) as CastDelegate<T>;
                }
            }
        }

        public unsafe T GetHeapVariable<T>(uint address) /* where T: unmanaged */ {
            System.IntPtr* args = stackalloc System.IntPtr[1];
            args[0] = (System.IntPtr)(&address);
            Unsafe.SkipInit(out System.IntPtr exc);
            System.IntPtr result = IL2CPP.il2cpp_runtime_invoke(MethodInfoStoreGeneric_GetHeapVariable<T>.Pointer, Pointer, (void**)args, ref exc);
            Il2CppException.RaiseExceptionIfNecessary(exc);

            // return IL2CPP.PointerToValueGeneric<T>(result, isFieldPointer: false, valueTypeWouldBeBoxed: true)

            if (typeof(T) == typeof(string)) {
                return (T)(object)IL2CPP.Il2CppStringToManaged(result);
            }

            if (result == System.IntPtr.Zero) {
                return default;
            }

            var nativeObject = new Il2CppObjectBase(result);

            // if (typeof(T).IsValueType) {
            //     return (T)UnboxMethod.MakeGenericMethod(typeof(T)).Invoke(nativeObject, null);
            // }
            // return (T)CastMethod.MakeGenericMethod(typeof(T)).Invoke(nativeObject, null);

            return DelegateStore<T>.Cast(new Il2CppObjectBase(result));

            //return new Il2CppObjectBase(result).Unbox<T>();
        }

    }
}
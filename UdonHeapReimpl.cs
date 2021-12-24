using VRC.Udon.Common.Interfaces;
using static VRC.Udon.Common.UdonHeap;

using System;
using System.Runtime.CompilerServices;
using System.Collections.Generic;
using System.Linq;
using UnhollowerRuntimeLib;

using Il2CppCS = Il2CppSystem.Runtime.CompilerServices;

namespace vrc_udon_shit {
    class UdonHeapReimpl: Il2CppSystem.Object /*IUdonHeap */ {
        public UdonHeapReimpl(IntPtr handle): base(handle) {}

        public UdonHeapReimpl(): this(DEFAULT_CAPACITY) {}

        public UdonHeapReimpl(uint capacity): base(ClassInjector.DerivedConstructorPointer<UdonHeapReimpl>()) {
            heap = new IExStrongBox[Math.Min(capacity, MAXIMUM_CAPACITY)];
        }

        private static System.Reflection.MethodInfo CastMethod = typeof(UnhollowerBaseLib.Il2CppObjectBase).GetMethod(nameof(UnhollowerBaseLib.Il2CppObjectBase.Cast));
        private static System.Reflection.MethodInfo UnboxMethod = typeof(UnhollowerBaseLib.Il2CppObjectBase).GetMethod(nameof(UnhollowerBaseLib.Il2CppObjectBase.Unbox));

        public UdonHeapReimpl(VRC.Udon.Common.UdonHeap orig): base(ClassInjector.DerivedConstructorPointer<UdonHeapReimpl>()) {
            heap = new IExStrongBox[orig._heap.Length];
            for (int idx = 0; idx < heap.Length; idx += 1) {
                var nativeBox = orig._heap[idx];
                if (nativeBox is null) {
                    continue;
                }

                var nativeType = UnhollowerRuntimeLib.Il2CppType.TypeFromPointer(UnhollowerBaseLib.IL2CPP.il2cpp_object_get_class(nativeBox.Pointer)).GenericTypeArguments[0];

                UdonShit.logger.Msg(nativeType.FullName);

                var managedType = nativeType.SystemType();
                var managedBoxType = typeof(ExStrongBox<>).MakeGenericType(managedType);
                var managedBox = Activator.CreateInstance(managedBoxType) as IExStrongBox;

                if (nativeBox.Value is null) {
                    continue;
                }

                // so that the NativeClassPtr gets set, so that unbox won't complain.
                System.Runtime.CompilerServices.RuntimeHelpers.RunClassConstructor(managedType.TypeHandle);

                UdonShit.logger.Msg(managedType.FullName);

                if (managedType.IsValueType) {
                    // FUCK: but at least it's not a hot path.
                    managedBox.Value = UnboxMethod.MakeGenericMethod(managedType).Invoke(nativeBox.Value, null);
                } else if (managedType == typeof(string)) {
                    managedBox.Value = UnhollowerBaseLib.IL2CPP.Il2CppStringToManaged(nativeBox.Value.Pointer);
                } else {
                    managedBox.Value = CastMethod.MakeGenericMethod(managedType).Invoke(nativeBox.Value, null);
                }
            }
        }

        private IExStrongBox[] heap;

        [MethodImpl(MethodImplOptions.AggressiveInlining)]
        private void CheckHeapBounds(uint address) {
            if (address >= heap.Length) {
                throw new IndexOutOfRangeException($"Adddress 0x{address:X} is larger than the heap size of 0x{heap.Length:X}");
            }
        }

        // NOTE: Currently banking on this not being in the hot path.
        public void InitializeHeapVariable(uint address, Il2CppSystem.Type type) {
            CheckHeapBounds(address);
            if (type == null) {
                throw new ArgumentNullException("type");
            }
            object value = null;
            var systemType = type.SystemType();
            if (systemType.IsValueType) {
                value = Activator.CreateInstance(systemType);
            }
            SetHeapVariableInternal(address, value, systemType);
        }

        public void InitializeHeapVariable<T>(uint address) {
            CheckHeapBounds(address);
            heap[address] = new ExStrongBox<T>(default(T));
        }

        public void CopyHeapVariable(uint src, uint dest) {
            CheckHeapBounds(src);
            CheckHeapBounds(dest);
            heap[src].CopyTo(ref heap[dest]);
        }

        // NOTE: Currently banking on this not being in the hot path.
        public void SetHeapVariable(uint address, Il2CppSystem.Object value, Il2CppSystem.Type type) {
            CheckHeapBounds(address);
            if (type == null) {
                throw new ArgumentNullException("type");
            }
            Type systemType = type.SystemType();
            SetHeapVariableInternal(address, value, systemType);
        }

        private void SetHeapVariableInternal(uint address, object value, Type type) {
            var box = Activator.CreateInstance(typeof(ExStrongBox<>).MakeGenericType(type)) as IExStrongBox;
            box.Value = value;
            heap[address] = box;
        }

        public void SetHeapVariable<T>(uint address, T value) {
            CheckHeapBounds(address);
            ExStrongBox<T>.Set(ref heap[address], value);
        }

        public object GetHeapVariable(uint address) {
            return GetHeapVariable<object>(address);
        }

        public T GetHeapVariable<T>(uint address) {
            CheckHeapBounds(address);
            var box = heap[address];

            if (box is StrongBox<T> strongBox) {
                return strongBox.Value;
            }

            object val = box.Value;
            if (val != null) {
                if (val is T result) {
                    return result;
                } else {
                    throw new Exception($"Cannot retrieve {val.GetType().Name} as {typeof(T).Name}");
                }
            } else {
                return default;
            }
        }

        public bool TryGetHeapVariable(uint address, out Il2CppSystem.Object value) {
            return TryGetHeapVariable<Il2CppSystem.Object>(address, out value);
        }

        public bool TryGetHeapVariable<T>(uint address, out T value) {
            var box = heap[address];

            if (box is null) {
                value = default;
                return false;
            }

            if (box is StrongBox<T> strongBox) {
                value = strongBox.Value;
                return true; // we're good
            }

            object val = box.Value;
            if (val != null) {
                if (val is T result) {
                    value = result;
                    return true; // we're good
                } else {
                    value = default;
                    return false; // type mismatch...
                }
            } else {
                value = default;
                return false; // no real value
            }
        }

        public bool IsHeapVariableInitialized(uint address) {
            CheckHeapBounds(address);
            return heap[address] != null;
        }

        public Il2CppSystem.Type GetHeapVariableType(uint address) {
            CheckHeapBounds(address);
            if (heap[address] == null) {
                throw new NullReferenceException("Tried to access uninitialized heap address!");
            }
            
            // wish it was this easy to go the other way...
            return UnhollowerRuntimeLib.Il2CppType.From(heap[address].GetType().GenericTypeArguments[0]);
        }

        public uint GetHeapCapacity() {
            return (uint)heap.Length;
        }

        public void DumpHeapObjects(Il2CppSystem.Collections.Generic.List<Il2CppSystem.ValueTuple<uint, Il2CppCS.IStrongBox, Il2CppSystem.Type>> destination) {
            if (destination == null) {
                return;
            }
            destination.Clear();
            for (uint num = 0; num < heap.Length; num++) {
                IExStrongBox box = heap[num];
                if (box is null) {
                    continue;
                }

                var managedType = heap[num].GetType().GenericTypeArguments[0];
                var nativeType = UnhollowerRuntimeLib.Il2CppType.From(managedType);
                var nativeBoxType = typeof(Il2CppCS.StrongBox<>)
                    .MakeGenericType(managedType);
                var nativeBox = Activator.CreateInstance(nativeBoxType) as Il2CppCS.IStrongBox;
                
                // if (managedType.IsValueType) {
                //     throw new NotImplementedException("Box Value Types");
                // }
                nativeBox.Value = (Il2CppSystem.Object)box.Value;
                
                var tuple = new Il2CppSystem.ValueTuple<uint, Il2CppCS.IStrongBox, Il2CppSystem.Type>(num, nativeBox, nativeType);
                destination.Add(tuple);
            }
        }

        interface IExStrongBox: IStrongBox {
            void CopyTo(ref IExStrongBox box);
        }

        class ExStrongBox<T>: StrongBox<T>, IExStrongBox {
            public ExStrongBox(): base() {}
            public ExStrongBox(T value): base(value) {}

            [MethodImpl(MethodImplOptions.AggressiveInlining)]
            public void CopyTo(ref IExStrongBox box) {
                Set(ref box, Value);
            }

            [MethodImpl(MethodImplOptions.AggressiveInlining)]
            public static void Set(ref IExStrongBox box, T value) {
                if (box is ExStrongBox<T> strongBox) {
                    strongBox.Value = value;
                } else {
                    box = new ExStrongBox<T>(value);
                }
            }
        }
    }

    internal static class TypeExt {
        static Dictionary<Il2CppSystem.Type, Type> lookup = new Dictionary<Il2CppSystem.Type, Type>();
        
        // based on Il2CppAssemblyUnhollower.ClassInjector.SystemTypeFromIl2CppType
        // this is likely slow! but at least i'm caching the result?
        public static Type SystemType(this Il2CppSystem.Type type) {
            if (lookup.TryGetValue(type, out Type val)) {
                return val;
            }

            // TODO: lock? meh.
            Type result = SystemTypeHelper(type);
            lookup.Add(type, result);
            return result;
        }

        private static Type SystemTypeHelper(Il2CppSystem.Type type) {
            var fullName = type.FullName;
            if (type.IsPrimitive) {
                return Type.GetType(fullName);
            } else if (type.IsArray) {
                var elementType = type.GetElementType().SystemType();
                if (elementType.IsValueType) {
                    return typeof(UnhollowerBaseLib.Il2CppStructArray<>).MakeGenericType(elementType);
                } else if (elementType == typeof(string)) {
                    return typeof(UnhollowerBaseLib.Il2CppStringArray);
                } else {
                    return typeof(UnhollowerBaseLib.Il2CppReferenceArray<>).MakeGenericType(elementType);
                }
            }
            if (fullName == "System.String") {
                return typeof(string);
            }
            if (fullName.StartsWith("System")) {
                fullName = "Il2Cpp" + fullName;
            }
            return AppDomain.CurrentDomain.GetAssemblies()
                .SelectMany(a => a.GetTypes())
                .First(t => t.FullName == fullName);
        }
    }
}
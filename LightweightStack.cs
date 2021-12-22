// based on VRC.Udon.VM.Common.LightweightStack<T>
using System.Runtime.InteropServices;
using System.Runtime.CompilerServices;
using UnhollowerBaseLib;
using System.Reflection;

namespace vrc_udon_shit {
    internal class LightweightStack<T> where T: unmanaged {
        private T[] contents;
        private GCHandle contentsPin;
        internal Il2CppSystem.Span<T> contentsSpan;
        //private UnhollowerBaseLib.Il2CppArrayBase<T> contents;

        private static uint _byteOffset_SpanOffset = GetFieldOffset("_byteOffset");
        private static uint _length_SpanOffset = GetFieldOffset("_length");

        private int count = 0;

        public LightweightStack(int capacity) {
            if (capacity == 0) {
                capacity = 4;
            }
            contents = new T[capacity];

            contentsPin = GCHandle.Alloc(contents, GCHandleType.Pinned);

            contentsSpan = new Il2CppSystem.Span<T>();
            contentsSpan._pinnable = null;
        }

        [MethodImpl(MethodImplOptions.AggressiveInlining)]
        public void Push(T entry) {
            if (count == contents.Length) {
                var resized = new T[contents.Length * 2];
                contents.CopyTo(resized, 0);
                contentsPin.Free();
                
                contents = resized;
                contentsPin = GCHandle.Alloc(contents, GCHandleType.Pinned);
            }

            contents[count] = entry;
            count += 1;
        }

        [MethodImpl(MethodImplOptions.AggressiveInlining)]
        public bool TryPop(out T result) {
            if (count == 0) {
                result = default(T);
                return false;
            } else {
                result = contents[--count];
                return true;
            }
        }

        [MethodImpl(MethodImplOptions.AggressiveInlining)]
        public T Pop() {
            if (!TryPop(out var ret)) {
                throw new System.Exception("Attempted to POP empty stack!");
            }

            return ret;
        }

        // TODO: work on this interface, since we're now doing extremely stateful things with this class.
        [MethodImpl(MethodImplOptions.AggressiveInlining)]
        public unsafe Il2CppSystem.Span<T> PeekSlice(int items) {
            if (count < items) {
                throw new System.Exception("Attempted to peek more items than present!");
            }
            
            ulong spanPtr = (ulong)IL2CPP.Il2CppObjectBaseToPtr(contentsSpan);
            *(System.IntPtr*)(spanPtr + _byteOffset_SpanOffset) = contentsPin.AddrOfPinnedObject().Add<T>(count - items);
            *(int*)(spanPtr + _length_SpanOffset) = items;

            //contentsSpan._length = items;
            //contentsSpan._byteOffset = contentsPin.AddrOfPinnedObject().Add<T>(count - items);
            return contentsSpan;
        }
        
        // TODO: work on this interface, since we're now doing extremely stateful things with this class.
        [MethodImpl(MethodImplOptions.AggressiveInlining)]
        public void PopSlice(int items) {
            if (count < items) {
                throw new System.Exception("Attempted to pop more items than present!");
            }

            count -= items;
        }

        private static uint GetFieldOffset(string field) {
            var fieldInfoPtr = (System.IntPtr)typeof(Il2CppSystem.Span<T>)
                .GetField($"NativeFieldInfoPtr_{field}", BindingFlags.NonPublic | BindingFlags.Static)
                .GetValue(null);

            return IL2CPP.il2cpp_field_get_offset(fieldInfoPtr);
        }
    }
}
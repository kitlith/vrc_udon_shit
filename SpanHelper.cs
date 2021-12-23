using System;
using System.Runtime.CompilerServices;

namespace vrc_udon_shit {
    // based on SpanHelpers from System.Memory
    static class SpanHelper {
        [MethodImpl(MethodImplOptions.AggressiveInlining)]
        internal unsafe static IntPtr Add<T>(this IntPtr start, int index) {
            // just use the 64bit variant.
            ulong offset = (ulong)index * (ulong)Unsafe.SizeOf<T>();
            return (IntPtr)((byte*)start + offset);
        }
    }
}
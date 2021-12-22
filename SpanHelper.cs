using System;
using System.Runtime.CompilerServices;

namespace vrc_udon_shit {
    // based on SpanHelpers from System.Memory
    static class SpanHelper {
        internal unsafe static IntPtr Add<T>(this IntPtr start, int index) where T: unmanaged {
            // just use the 64bit variant.
            ulong offset = (ulong)index * (ulong)sizeof(T);
            return (IntPtr)((byte*)start + offset);
        }
    }
}
using System.Runtime.CompilerServices;
using InlineIL;
using static InlineIL.IL.Emit;

// System.Runtime.CompilerServices.Unsafe does not have nullable reference types annotations
#nullable disable

namespace vrc_udon_shit {
    // Avoiding reference to System.Runtime.CompilerServices.Unsafe since ilrepack causes issues on my machine.
    // basically portions of https://github.com/ltrzesniewski/InlineIL.Fody/blob/master/src/InlineIL.Examples/Unsafe.cs
    // original license MIT (assuming that applies to the examples)
    // if i can figure out how the unhollower generated classes are referencing Unsafe, I'll probably use that instead.
    internal static unsafe class Unsafe {
        [MethodImpl(MethodImplOptions.AggressiveInlining)]
        public static int SizeOf<T>()
        {
            Sizeof(typeof(T));
            return IL.Return<int>();
        }

        [MethodImpl(MethodImplOptions.AggressiveInlining)]
        public static void SkipInit<T>(out T value)
        {
            Ret();
            throw IL.Unreachable();
        }
    }
}
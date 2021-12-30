using System;
using System.Reflection;
using System.Runtime.InteropServices;
using VRC.Udon.Common;
using VRC.Udon.Wrapper;
using vrc_udon_shit;
using static NativeInterpreterApi.Internals;

internal static class NativeInterpreterApi {
    public static SetFunctionPointersDelegate SetFunctionPointers;
    public static SetWrapperAndTimeSourceDelegate SetWrapperAndTimeSource;
    public static LoadProgramDelegate LoadProgram;
    public static InterpretDelegate Interpret;
    public static SetProgramCounterDelegate SetProgramCounter;
    public static GetProgramCounterDelegate GetProgramCounter;
    public static DisposeDelegate Dispose;

    public static class Internals {
        public static bool Initialize(string path) {
            var lib = LoadLibraryA(path);

            if (lib == IntPtr.Zero)
            {
                var error = Marshal.GetLastWin32Error();
                UdonShit.logger.Error("Native library load failed, mod won't work: {0}", error);
                return false;
            }

            SetFunctionPointers = GetPointer<SetFunctionPointersDelegate>(lib, "set_function_pointers");
            SetWrapperAndTimeSource = GetPointer<SetWrapperAndTimeSourceDelegate>(lib, "set_wrapper_and_time_source");
            LoadProgram = GetPointer<LoadProgramDelegate>(lib, "dynarec_load_program");
            Interpret = GetPointer<InterpretDelegate>(lib, "dynarec_interpret");
            SetProgramCounter = GetPointer<SetProgramCounterDelegate>(lib, "dynarec_set_program_counter");
            GetProgramCounter = GetPointer<GetProgramCounterDelegate>(lib, "dynarec_get_program_counter");
            Dispose = GetPointer<DisposeDelegate>(lib, "dynarec_dispose");

            static IntPtr GetFunctionPointer(Type type, string name) {
                var methodInfoPtr = (IntPtr)type.GetField(name, BindingFlags.Static | BindingFlags.NonPublic).GetValue(null);
                if (methodInfoPtr == IntPtr.Zero)
                {
                    UdonShit.logger.Error("Couldn't find method info pointer for {0}::{1}", type.Name, name);
                    return IntPtr.Zero;
                }
                unsafe
                {
                    return *(IntPtr*)methodInfoPtr;
                }
            }

            /* Ensure static constructors ran */
            new UdonHeap();
            new UdonWrapper();
            functions = new Functions
            {
                UdonHeapGetHeapVariable = GetFunctionPointer(typeof(UdonHeap), "NativeMethodInfoPtr_GetHeapVariable_Public_Virtual_Final_New_Object_UInt32_0"),
                UdonHeapCopyHeapVariable = GetFunctionPointer(typeof(UdonHeap), "NativeMethodInfoPtr_CopyHeapVariable_Public_Virtual_Final_New_Void_UInt32_UInt32_0"),
                UdonWrapperGetExternFunctionDelegate = GetFunctionPointer(typeof(UdonWrapper), "NativeMethodInfoPtr_GetExternFunctionDelegate_Public_Virtual_Final_New_UdonExternDelegate_String_0"),
                UdonWrapperGetExternFunctionParameterCount = GetFunctionPointer(typeof(UdonWrapper), "NativeMethodInfoPtr_GetExternFunctionParameterCount_Public_Virtual_Final_New_Int32_String_0"),
            };
            SetFunctionPointers(ref functions);

            return true;
        }

        public struct Functions {
            public IntPtr UdonHeapGetHeapVariable;
            public IntPtr UdonHeapCopyHeapVariable;
            public IntPtr UdonWrapperGetExternFunctionDelegate;
            public IntPtr UdonWrapperGetExternFunctionParameterCount;
        }
        private static Functions functions;

        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate bool SetFunctionPointersDelegate(ref Functions functions);
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate bool SetWrapperAndTimeSourceDelegate(IntPtr wrapper, IntPtr timeSource);
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate IntPtr LoadProgramDelegate(IntPtr bytecode, IntPtr heap);
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate bool InterpretDelegate(IntPtr interpreter);
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate bool SetProgramCounterDelegate(IntPtr interpreter, uint pc);
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate uint GetProgramCounterDelegate(IntPtr interpreter);
        [UnmanagedFunctionPointer(CallingConvention.Cdecl)]
        public delegate void DisposeDelegate(IntPtr interpreter);

        private static T GetPointer<T>(IntPtr lib, string name) where T : MulticastDelegate {
            var result = Marshal.GetDelegateForFunctionPointer<T>(GetProcAddress(lib, name));
            if (result == null) UdonShit.logger.Error($"Delegate for {name} not found! Bug?");

            return result;
        }

        [DllImport("kernel32", CharSet = CharSet.Ansi, ExactSpelling = true, SetLastError = true)]
        public static extern IntPtr GetProcAddress(IntPtr hModule, string procName);

        [DllImport("kernel32", CharSet = CharSet.Ansi, ExactSpelling = true, SetLastError = true)]
        internal static extern IntPtr LoadLibraryA(string libName);
    }
}

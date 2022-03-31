using System;
using System.Diagnostics;
using UnhollowerRuntimeLib;
using VRC.Udon.Common.Interfaces;

namespace vrc_udon_shit {
    public class UdonVMDynarec : Il2CppSystem.Object /*, IUdonVM */ {
        public UdonVMDynarec(IntPtr handle) : base(handle) { }
        public UdonVMDynarec(IUdonWrapper wrapper, VRC.Udon.VM.IUdonVMTimeSource timeSource) : base(ClassInjector.DerivedConstructorPointer<UdonVMDynarec>()) {
            ClassInjector.DerivedConstructorBody(this);
            if (!SetWrapperAndTimeSource)
            {
                NativeInterpreterApi.SetWrapperAndTimeSource(wrapper.Pointer, timeSource.Pointer);
                SetWrapperAndTimeSource = true;
            }
        }
        private static bool SetWrapperAndTimeSource = false;

        ~UdonVMDynarec() {
            NativeInterpreterApi.Dispose(_interpreter);
        }

        private IUdonProgram _program;
        private IntPtr _interpreter;

        public bool LoadProgram(IUdonProgram program) {
            if (program.InstructionSetIdentifier != "UDON" || program.InstructionSetVersion > 1)
            {
                return false;
            }

            var timer = new Stopwatch();
            timer.Start();

            _program = program;
            _interpreter = NativeInterpreterApi.LoadProgram(program.ByteCode.Pointer, program.Heap.Pointer);

            var elapsed = timer.ElapsedMilliseconds;
            UdonShit.logger.Msg("Loaded program in " + elapsed + "ms");

            return _interpreter != IntPtr.Zero;
        }

        public IUdonProgram RetrieveProgram() {
            return _program;
        }

        public void SetProgramCounter(uint pc) {
            if (!NativeInterpreterApi.SetProgramCounter(_interpreter, pc))
            {
                throw new Exception($"Unaligned PC 0x{pc:X}!");
            }
        }

        public uint GetProgramCounter() {
            return NativeInterpreterApi.GetProgramCounter(_interpreter);
        }

        bool _halted = false;

        public uint Interpret() {
            if (_halted)
            {
                return VRC.Udon.VM.UdonVM.RESULT_FAILURE;
            }

            if (!NativeInterpreterApi.Interpret(_interpreter))
            {
                _halted = true;
                UdonShit.logger.Error($"Udon VM halted!");
                return VRC.Udon.VM.UdonVM.RESULT_FAILURE;
            }

            return VRC.Udon.VM.UdonVM.RESULT_SUCCESS;
        }

        public IUdonHeap InspectHeap() {
            return _program.Heap;
        }

        public bool DebugLogging { get; set; }
    }
}
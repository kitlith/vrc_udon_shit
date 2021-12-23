using VRC.Udon.VM.Common;
using VRC.Udon.Common.Interfaces;

using static VRC.Udon.VM.UdonVM;

using System.Collections.Generic;

using UnhollowerRuntimeLib;

using System;
using System.Runtime.CompilerServices;
using System.Diagnostics;

namespace vrc_udon_shit {
    public class UdonVMDynarec: Il2CppSystem.Object /*, IUdonVM */ {
        public UdonVMDynarec(System.IntPtr handle) : base(handle) {}
        public UdonVMDynarec(IUdonWrapper wrapper, VRC.Udon.VM.IUdonVMTimeSource timeSource) : base(ClassInjector.DerivedConstructorPointer<UdonVMDynarec>()) {
            ClassInjector.DerivedConstructorBody(this);
            _wrapper = wrapper;
            _timeSource = timeSource;
        }

        public bool DebugLogging { get; set; }

        private IUdonProgram _program;
        private string _name;

        public void SetName(string name) {
            _name = name;
        }
        public bool LoadProgram(IUdonProgram program) {
            if (program.InstructionSetIdentifier != "UDON" || program.InstructionSetVersion > 1) {
                return false;
            }
            _program = program;
            heap = new UdonHeapWrapper(program.Heap);

            bytecode = new uint[_program.ByteCode.Length / 4];

            uint pc = 0;
            for (int i = 0; i < bytecode.Length; ++i) {
                // considered using BitConverter, but that doesn't specify endian, so i'd have to byteswap. meh.
                bytecode[i] =
                    ((uint)_program.ByteCode[(int)(pc + 0)] << 24) |
                    ((uint)_program.ByteCode[(int)(pc + 1)] << 16) |
                    ((uint)_program.ByteCode[(int)(pc + 2)] <<  8) |
                    ((uint)_program.ByteCode[(int)(pc + 3)] <<  0);
                pc += 4;
            }

            return true;
        }

        public IUdonProgram RetrieveProgram() {
            return _program;
        }

        public void SetProgramCounter(uint newPc) {
            pc = CheckProgramCounter(newPc);
        }

        private uint CheckProgramCounter(uint newPc) {
            if ((newPc & 3) != 0 && (newPc >> 2) < bytecode.Length) {
                throw new Exception($"Unaligned PC 0x{pc:X}!");
            }
            return newPc;
        }

        public uint GetProgramCounter() {
            return pc;
        }

        public IUdonHeap InspectHeap() {
            return _program.Heap;
        }

        private uint[] bytecode;

        private UdonHeapWrapper heap;
        //private IUdonHeap heap { get => _program.Heap; }
        private IUdonWrapper _wrapper;
        private VRC.Udon.VM.IUdonVMTimeSource _timeSource;

        // todo: consider making global, and sharing between identical UdonBehaviours?
        //private Dictionary<uint, Block> BlockCache = new Dictionary<uint, Block>();
        private LightweightStack<uint> stack = new LightweightStack<uint>(VRC.Udon.VM.UdonVM.INITIAL_STACK_SIZE);
        //private Il2CppSystem.Collections.Generic.List<uint> stack = new Il2CppSystem.Collections.Generic.List<uint>();

        private Dictionary<uint, CachedUdonExternDelegate> ExternCache = new Dictionary<uint, CachedUdonExternDelegate>();

        uint pc;

        bool _halted = false;

        long startTime;

        public uint Interpret() {
            if (_halted) {
                return VRC.Udon.VM.UdonVM.RESULT_FAILURE;
            }
            // pushTime.Reset();
            // popTime.Reset();
            // jumpIfFalseTime.Reset();
            // jumpTime.Reset();
            // externFetchTime.Reset();
            // externArgsTime.Reset();
            externTime.Reset();
            // jumpIndirectTime.Reset();
            // copyTime.Reset();
            // decodeTime.Reset();
            startTime = _timeSource.CurrentTime;
            try {
                while (pc / 4 < bytecode.Length) {
                    Block block;
                    // if (!BlockCache.TryGetValue(pc, out block)) {
                    //     // not in the cache, time to translate
                    //     // TODO: replace with actual translation
                        block = (x) => InterpretBlock(pc, x);
                    //     BlockCache.Add(pc, block);
                    // }

                    pc = block(stack);
                }
            } catch (Exception inner) {
                _halted = true;
                UdonShit.logger.Error($"An exception occurred during Udon execution of {_name}:\n{inner}");
                return RESULT_FAILURE;
            }

            var duration = _timeSource.CurrentTime - startTime;
            if (duration > MAX_VM_TIME_MS/10) {
                UdonShit.logger.Msg($"{_name} took a long time ({duration/1000.0f} seconds). Details:");
            //     UdonShit.logger.Msg($"push: {pushTime.Elapsed.TotalSeconds}, pop: {popTime.Elapsed.TotalSeconds}, copy: {copyTime.Elapsed.TotalSeconds}");
            //     UdonShit.logger.Msg($"jumpIfFalse: {jumpIfFalseTime.Elapsed.TotalSeconds}, jump: {jumpTime.Elapsed.TotalSeconds}, jumpIndirect: {jumpIndirectTime.Elapsed.TotalSeconds}");
            //     UdonShit.logger.Msg($"externFetch: {externFetchTime.Elapsed.TotalSeconds}, externArgs: {externArgsTime.Elapsed.TotalSeconds}, externCall: {externTime.Elapsed.TotalSeconds}, decode: {decodeTime.Elapsed.TotalSeconds}");
                UdonShit.logger.Msg($"externCall: {externTime.Elapsed.TotalSeconds}");
            }

            return VRC.Udon.VM.UdonVM.RESULT_SUCCESS;
        }

        [MethodImpl(MethodImplOptions.AggressiveInlining)]
        uint ReadBE(ref uint pc) {
            var ret = bytecode[pc >> 2];
            pc += 4;
            return ret;
        }

        private void CheckTimeLimit() {
            // Kinda a relic of when my test was taking 20s-2minutes
            // but i'll leave this commented until we stop worrying about maximizing performance.
            // if (_timeSource.CurrentTime - startTime > MAX_VM_TIME_MS) {
            //     _halted = true;
            //     throw new Exception($"Udon Execution time exceeded max of {MAX_VM_TIME_MS/1000.0f} seconds. PC was at {pc:X}");
            // }
        }

        // Stopwatch pushTime = new Stopwatch();
        // Stopwatch popTime = new Stopwatch();
        // Stopwatch jumpIfFalseTime = new Stopwatch();
        // Stopwatch jumpTime = new Stopwatch();
        // Stopwatch externFetchTime = new Stopwatch();
        // Stopwatch externArgsTime = new Stopwatch();
        Stopwatch externTime = new Stopwatch();
        // Stopwatch jumpIndirectTime = new Stopwatch();
        // Stopwatch copyTime = new Stopwatch();
        // Stopwatch decodeTime = new Stopwatch();

        // Trampolines on (indirect) branch -- goal of the dynarec is to generate functions that trampoline flow control.
        // At least that was the goal, but I took that out as soon as I realized it was performing slowly, so this just returns when it's done interpreting.
        private unsafe uint InterpretBlock(uint pc, LightweightStack<uint> stack) {
            // TODO? allocate as part of the VM and pin it -- this can last as long as the heap + stack exists.
            IntPtr* ptr = stackalloc System.IntPtr[2];
            ptr[0] = heap.Pointer;
            ptr[1] = stack.Pointer;
            void** externArgs = (void**)ptr;
    
            while ((pc >> 2) < bytecode.Length) {
                if ((pc & 3) != 0) {
                    throw new Exception($"Unaligned PC 0x{pc:X}!");
                }
                // decodeTime.Start();
                uint op = ReadBE(ref pc);
                // decodeTime.Stop();

                var instTime = _timeSource.CurrentTime;
                switch (op) {
                    case (uint)OpCode.NOP:
                        break;
                    case (uint)OpCode.PUSH:
                        // pushTime.Start();
                        stack.Push(ReadBE(ref pc));
                        // pushTime.Stop();
                        break;
                    case (uint)OpCode.POP:
                        // popTime.Start();
                        stack.Pop();
                        // popTime.Stop();
                        break;
                    case (uint)OpCode.JUMP_IF_FALSE:
                        // jumpIfFalseTime.Start();
                        CheckTimeLimit();
                        var jump_target = ReadBE(ref pc);
                        var cond_slot = stack.Pop();
                        // getting object and then unboxing is apparently faster than the generic version??
                        if (heap.GetHeapVariable<bool>(cond_slot) == false) {
                            // return jump_target;
                            //pc = CheckProgramCounter(jump_target);
                            pc = jump_target;
                        }
                        // jumpIfFalseTime.Stop();
                        break;
                    case (uint)OpCode.JUMP:
                        // jumpTime.Start();
                        CheckTimeLimit();
                        //return ReadBE(ref pc);
                        //pc = CheckProgramCounter(ReadBE(ref pc));
                        pc = ReadBE(ref pc);
                        // jumpTime.Stop();
                        break;
                    case (uint)OpCode.EXTERN:
                        // externFetchTime.Start();
                        CheckTimeLimit();
                        var function_slot = ReadBE(ref pc);

                        CachedUdonExternDelegate externDelegate;
                        if (!ExternCache.TryGetValue(function_slot, out externDelegate)) {
                            var obj_type = heap.GetHeapVariableType(function_slot);

                            if (obj_type != UnhollowerRuntimeLib.Il2CppType.Of<string>()) {
                                throw new Exception($"Extern operand expected type 'System.String' but got '{obj_type.FullName}' instead.");
                            }

                            var function_signature = UnhollowerBaseLib.IL2CPP.Il2CppStringToManaged(heap.GetHeapVariable(function_slot).Pointer);

                            var del = _wrapper.GetExternFunctionDelegate(function_signature);
                            var param_count = _wrapper.GetExternFunctionParameterCount(function_signature);

                            externDelegate = new CachedUdonExternDelegate(function_signature, del, param_count);
                            ExternCache.Add(function_slot, externDelegate);
                        }
                        // externFetchTime.Stop();

                        // externArgsTime.Start();
                        // NOTE: Invoke relies on the side-effect of PopSlice that sets up the slice referenced in externArgs
                        stack.PopSlice(externDelegate.parameterCount);
                        //externArgsTime.Stop();

                        externTime.Start();
                        externDelegate.Invoke(externArgs);
                        externTime.Stop();

                        break;
                    case (uint)OpCode.ANNOTATION:
                        // var annotation_slot = ReadBE(ref pc); // idea: debug print annotations? meh.
                        pc += 4; // skip the annotation, we're an extended nop
                        break;
                    case (uint)OpCode.JUMP_INDIRECT:
                        // jumpIndirectTime.Start();
                        CheckTimeLimit();
                        var destination_slot = ReadBE(ref pc);
                        // getting object and then unboxing is apparently faster than the generic version??
                        var destination = heap.GetHeapVariable<uint>(destination_slot);
                        //return destination;
                        //pc = CheckProgramCounter(destination);
                        pc = destination;
                        // jumpIndirectTime.Stop();
                        break;
                    case (uint)OpCode.COPY:
                        // copyTime.Start();
                        var dest = stack.Pop();
                        var src = stack.Pop();
                        heap.CopyHeapVariable(src, dest);
                        // copyTime.Stop();
                        break;
                }
            }

            return pc;
        }

        private delegate uint Block(LightweightStack<uint> stack);

        public struct CachedUdonExternDelegate {
            public string externSignature;
            //public VRC.Udon.Common.Delegates.UdonExternDelegate externDelegate;
            public int parameterCount;

            public IntPtr method;
            public IntPtr target;

            public CachedUdonExternDelegate(string sig, VRC.Udon.Common.Delegates.UdonExternDelegate del, int count) {
                externSignature = sig;
                //externDelegate = del;
                parameterCount = count;
                method = del.method;
                target = UnhollowerBaseLib.IL2CPP.Il2CppObjectBaseToPtrNotNull(del.m_target);
            }

            [MethodImpl(MethodImplOptions.AggressiveInlining)]
            public unsafe void Invoke(void **args) {
                Unsafe.SkipInit(out System.IntPtr exc);
                UnhollowerBaseLib.IL2CPP.il2cpp_runtime_invoke(method, target, args, ref exc);

                if (exc != System.IntPtr.Zero) {
                    ThrowException(externSignature);
                }
                void ThrowException(string name) => throw new Exception($"An exception occurred during udon call to '{name}'", new UnhollowerBaseLib.Il2CppException(exc));
            }
        }
    }
}
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
            if (newPc % 4 != 0 && newPc / 4 < bytecode.Length) {
                throw new Exception($"Unaligned PC 0x{pc:X}!");
            }
            pc = newPc;
        }

        public uint GetProgramCounter() {
            return pc;
        }

        public IUdonHeap InspectHeap() {
            return _program.Heap;
        }

        private uint[] bytecode;

        private IUdonHeap heap { get => _program.Heap; }
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
            // externTime.Reset();
            // jumpIndirectTime.Reset();
            // copyTime.Reset();
            // decodeTime.Reset();
            startTime = _timeSource.CurrentTime;
            try {
                while (pc / 4 < bytecode.Length) {
                    if (pc % 4 != 0) {
                        throw new Exception($"Unaligned PC 0x{pc:X}!");
                    }

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
            // if (duration > MAX_VM_TIME_MS/10) {
            //     UdonShit.logger.Msg($"{_name} took a long time ({duration/1000.0f} seconds). Details:");
            //     UdonShit.logger.Msg($"push: {pushTime.Elapsed.TotalSeconds}, pop: {popTime.Elapsed.TotalSeconds}, copy: {copyTime.Elapsed.TotalSeconds}");
            //     UdonShit.logger.Msg($"jumpIfFalse: {jumpIfFalseTime.Elapsed.TotalSeconds}, jump: {jumpTime.Elapsed.TotalSeconds}, jumpIndirect: {jumpIndirectTime.Elapsed.TotalSeconds}");
            //     UdonShit.logger.Msg($"externFetch: {externFetchTime.Elapsed.TotalSeconds}, externArgs: {externArgsTime.Elapsed.TotalSeconds}, externCall: {externTime.Elapsed.TotalSeconds}, decode: {decodeTime.Elapsed.TotalSeconds}");
            //     UdonShit.logger.Msg($"externCall: {externTime.Elapsed.TotalSeconds}");
            // }

            return VRC.Udon.VM.UdonVM.RESULT_SUCCESS;
        }

        [MethodImpl(MethodImplOptions.AggressiveInlining)]
        uint ReadBE(ref uint pc) {
            var ret = bytecode[pc/4];
            pc += 4;
            return ret;
        }

        uint InstructionSize(OpCode op) => op switch {
            OpCode.NOP => 4,
            OpCode.PUSH => 8,
            OpCode.POP => 4,
            OpCode.JUMP_IF_FALSE => 8,
            OpCode.JUMP => 8, // this should never be reached, but meh.
            OpCode.EXTERN => 8,
            OpCode.ANNOTATION => 8, // another NOP, basically a comment.
            OpCode.JUMP_INDIRECT => 8,
            OpCode.COPY => 4,
            _ => throw new System.Exception("Tried to obtain length of unknown instruction!")
        };

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
        // Stopwatch externTime = new Stopwatch();
        // Stopwatch jumpIndirectTime = new Stopwatch();
        // Stopwatch copyTime = new Stopwatch();
        // Stopwatch decodeTime = new Stopwatch();

        // Trampolines on (indirect) branch -- goal of the dynarec is to generate functions that trampoline flow control.
        // At least that was the goal, but I took that out as soon as I realized it was performing slowly, so this just returns when it's done interpreting.
        private unsafe uint InterpretBlock(uint pc, LightweightStack<uint> stack) {
            IntPtr* ptr = stackalloc System.IntPtr[2];
            ptr[0] = UnhollowerBaseLib.IL2CPP.Il2CppObjectBaseToPtrNotNull(heap);
            ptr[1] = UnhollowerBaseLib.IL2CPP.il2cpp_object_unbox(UnhollowerBaseLib.IL2CPP.Il2CppObjectBaseToPtrNotNull(stack.contentsSpan));
            void** externArgs = (void**)ptr;
    
            while (pc / 4 < bytecode.Length) {
                if (pc % 4 != 0) {
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
                        if (heap.GetHeapVariable(cond_slot).Unbox<bool>() == false) {
                            // return jump_target;
                            pc = jump_target;
                        }
                        // jumpIfFalseTime.Stop();
                        break;
                    case (uint)OpCode.JUMP:
                        // jumpTime.Start();
                        CheckTimeLimit();
                        //return ReadBE(ref pc);
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
                        stack.PeekSlice(externDelegate.parameterCount);
                        //externArgsTime.Stop();

                        // externTime.Start();
                        CallExtern(externArgs, externDelegate);
                        // externTime.Stop();

                        stack.PopSlice(externDelegate.parameterCount);

                        break;
                    case (uint)OpCode.ANNOTATION:
                        // var annotation_slot = ReadBE(ref pc); // idea: debug print annotations? meh.
                        pc += 4; // skip the annotation, we're an extended nop
                        break;
                    case (uint)OpCode.JUMP_INDIRECT:
                        // jumpIndirectTime.Start();
                        CheckTimeLimit();
                        var destination_slot = ReadBE(ref pc);
                        var destination = heap.GetHeapVariable(destination_slot).Unbox<uint>();
                        //return destination;
                        pc = destination;
                        // jumpIndirectTime.Stop();
                        break;
                    case (uint)OpCode.COPY:
                        // copyTime.Start();
                        // if (stack._size < 2) {
                        //     throw new Exception("Attempted to pop more items than present on stack for call to extern!");
                        // }
                        var dest = stack.Pop();
                        var src = stack.Pop();
                        // var src = stack.Contents[^2];
                        // var dest = stack.Contents[^1];
                        // stack.PopSlice(2);
                        heap.CopyHeapVariable(src, dest);
                        // copyTime.Stop();
                        break;
                }
            }

            return pc;
        }

        private unsafe void CallExtern(void** args, CachedUdonExternDelegate ex) {
            System.IntPtr exc = System.IntPtr.Zero;
            UnhollowerBaseLib.IL2CPP.il2cpp_runtime_invoke(ex.method, ex.target, (void**)args, ref exc);

            if (exc != System.IntPtr.Zero) {
                throw new Exception($"An exception occurred during udon call to '{ex.externSignature}'", new UnhollowerBaseLib.Il2CppException(exc));
            }
        }

        private delegate uint Block(LightweightStack<uint> stack);

        public class CachedUdonExternDelegate {
            public string externSignature;
            public VRC.Udon.Common.Delegates.UdonExternDelegate externDelegate;
            public int parameterCount;

            public IntPtr method;
            public IntPtr target;
            //public IntPtr[] args = new IntPtr[2];
            

            public CachedUdonExternDelegate(string sig, VRC.Udon.Common.Delegates.UdonExternDelegate del, int count) {
                externSignature = sig;
                externDelegate = del;
                parameterCount = count;
                method = del.method;
                target = UnhollowerBaseLib.IL2CPP.Il2CppObjectBaseToPtrNotNull(del.m_target);
            }
        }
    }
}
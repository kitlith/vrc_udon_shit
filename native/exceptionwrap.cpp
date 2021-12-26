#include <cstdint>
#include <exception>

extern "C" uint64_t call_vm_code(void * code_ptr, void *state);

// TODO: could we cause il2cpp_runtime_invoke to call our own code?
//  we could use the existing exception return mechanism then, and I don't think it'd be in a very hot loop.
//  also could get rid of the requirement for us to compile our own c++ code.
extern "C" uint64_t wrap_vm_exception_unknown(void *code_ptr, void *state) {
    try {
        return call_vm_code(code_ptr, state);
    } catch (...) {
        // top u32 used, there was an exception, we don't know what it was.
        // TODO: provide a second function for where we know the type of the exception at compile-time
        // then, at runtime, we can determine if we know the true exception type by checking unity version, and switch between the two functions.
        return 0xFFFFFFFFFFFFFFFF;
    }
}

// I started working by using C++'s inline assembly feature,
// but I find it much easier to use Rust's inline assembly.
// however, technically unwinding C++ exceptions across rust code is undefined behaviour
// so just in case, I've left what little i've done down here.

// #if defined(_MSC_VER)
//     #define NOINLINE __declspec(noinline)
// #elif defined(__GNUC__)
//     #define NOINLINE __attribute__((noinline))
// #else
//     #define NOINLINE
// #endif

// struct vm_state {
//     void *vm_stack;
//     void *heap_ptr;
// };

// NOINLINE uint64_t call_vm_code(void *code_ptr, vm_state *state) {
//     uint64_t result = 0;
// #if defined(__x86_64__)

//     #if defined(_MSC_VER)
//         #error "TODO: add inline ASM for MSVC"
//     #elif defined(__GNUC__)
//         register uint64_t result_reg asm("rax");
//         register void *stack_reg asm("r12") = state->vm_stack;
//         register void *heap_reg asm("r13") = state->heap_ptr;
//         __asm__(
//             "jmp %[code];"
//             ".global vm_return;"
//             "vm_return:;"
//             /* outputs  */: "=r"(result_reg), "+r"(stack_reg), "+r"(heap_reg)
//             /* inputs   */: [code]"r"(code_ptr)
//             /* clobbers */: // TODO: lots, probably
//         );
//         state->vm_stack = stack_reg;
//         state->heap_ptr = heap_reg;
//         result = result_reg;
//     #else
//         #error "Unknown compiler!"
//     #endif

// #elif defined(__aarch64__)
//     #error "ARM64 is not currently supported!"
// #else
//     #error "Unknown architecture!"
// #endif

//     return result;
// }
fn main() {
    cc::Build::new()
        .cpp(true)
        .flag_if_supported("-masm=intel") // intel syntax on GCC, pls.
        .file("exceptionwrap.cpp")
        .compile("exceptionwrap");
}
fn main() {
    let target = std::env::var("TARGET").unwrap();
    let mut build = cc::Build::new();

    if target == "x86_64-pc-windows-gnu" {
        // we're specifying it ourselves since we need a static version.
        build.cpp_link_stdlib(None);
    }

    build.cpp(true)
        // we're not actually using inline assembly on the c++ side of things right now.
        // .flag_if_supported("-masm=intel") // intel syntax on GCC, pls.
        .file("exceptionwrap.cpp")
        .compile("exceptionwrap");

    if target == "x86_64-pc-windows-gnu" {
        // think this is technically meant to be unstable, static:-bundle certainly is, but meh, whatever. it works.
        println!("cargo:rustc-link-lib=static-nobundle=stdc++");
    }
}

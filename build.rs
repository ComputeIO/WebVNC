use std::env;
use std::path::PathBuf;

fn main() {
    // Only run bindgen when the `libvnc` feature is enabled.
    if env::var("CARGO_FEATURE_LIBVNC").is_err() {
        // Nothing to do for non-libvnc builds.
        return;
    }

    // Try to find libvncserver through pkg-config — if it's not available
    // emit a helpful message for the user rather than failing hard here.
    let probe_succeeded = match pkg_config::probe_library("libvncserver") {
        Ok(_) => true,
        Err(e) => {
            println!("cargo:warning=libvnc feature enabled but pkg-config failed to find libvncserver: {}", e);
            false
        }
    };

    // Run bindgen on our wrapper header and write the generated file to OUT_DIR
    let header = "src/bindgen/wrapper.h";
    let out_path = PathBuf::from(env::var("OUT_DIR").unwrap());

    let bindings = bindgen::Builder::default()
        .header(header)
        .parse_callbacks(Box::new(bindgen::CargoCallbacks))
        // whitelist common symbols we will use; keep it conservative
        .allowlist_function("rfbGetScreen")
        .allowlist_function("rfbInitServer")
        .allowlist_function("rfbScreenCleanup")
        .allowlist_type("rfbScreenInfo")
        .generate();

    let bindings_file = out_path.join("bindings.rs");
    if let Ok(bindings) = bindings {
        bindings
            .write_to_file(&bindings_file)
            .expect("Couldn't write bindings!");
    } else {
        // bindgen generation failed (e.g. missing headers). Produce a small
        // fallback bindings.rs that declares the minimal symbols we need so
        // builds with --features libvnc still succeed (useful for dev/test
        // systems where libvncserver headers are not present).
        println!("cargo:warning=bindgen failed; writing fallback stubs into bindings.rs (probe succeeded={})", probe_succeeded);
        let fallback = r#"
        #[repr(C)]
        pub struct rfbScreenInfo {
            pub frameBuffer: *mut ::libc::c_uchar,
            pub width: ::libc::c_int,
            pub height: ::libc::c_int,
            pub bitsPerPixel: ::libc::c_int,
            pub bytesPerPixel: ::libc::c_int,
        }

        extern "C" {
            pub fn rfbGetScreen(argc: *mut ::libc::c_int, argv: *mut *mut *mut ::libc::c_char,
                width: ::libc::c_int, height: ::libc::c_int,
                bitsPerSample: ::libc::c_int, samplesPerPixel: ::libc::c_int,
                bytesPerPixel: ::libc::c_int) -> *mut rfbScreenInfo;

            pub fn rfbInitServer(screen: *mut rfbScreenInfo) -> ::libc::c_int;
            pub fn rfbScreenCleanup(screen: *mut rfbScreenInfo);

            pub fn rfbMarkRectAsModified(screen: *mut rfbScreenInfo, x: ::libc::c_int, y: ::libc::c_int, w: ::libc::c_int, h: ::libc::c_int);
        }
        "#;
        std::fs::write(&bindings_file, fallback).expect("Couldn't write fallback bindings");
    }
}

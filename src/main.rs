use clap::Parser;
use std::process;

/// Minimal x11vnc (PoC) CLI - entrypoint for the Rust rewrite.
#[derive(Parser, Debug)]
#[command(author, version, about = "x11vnc (Rust PoC) - work in progress", long_about = None)]
struct Args {
    /// Show a quick status and exit
    #[arg(long)]
    status: bool,

    /// Run in debug mode
    #[arg(short, long)]
    debug: bool,

    /// Optional RFB listen port (defaults to an ephemeral port)
    #[arg(long)]
    port: Option<u16>,
}

fn main() {
    let args = Args::parse();

    if args.status {
        println!("x11vnc (Rust PoC) status: OK");
        process::exit(0);
    }

    if args.debug {
        println!("Starting x11vnc (Rust PoC) in debug mode");
    }

    // NOTE: This is a skeleton. The next steps are to integrate libvncserver
    // via a thin FFI layer (or a safe wrapper) and then progressively port
    // X11 and other logic into Rust. For now we just start a minimal loop.

    println!("x11vnc (Rust PoC) starting - attempt to initialize a minimal VNC server...");

    // Try to initialize a headless VNC server (PoC). The real libvncserver
    // integration will be behind the `libvnc` Cargo feature. For now this
    // returns a stubbed object so the PoC can be exercised without a native
    // dependency.
    // create a minimal in-memory framebuffer (screen) and perform a few ops
    let mut screen = src_utils::screen::Screen::new(800, 600);
    screen.fill(0x0, 0x80, 0xff); // bluish background for PoC
    let _ = screen.set_pixel(10, 10, 0xff, 0x0, 0x0);

    // create a tiny event loop and run a single tick
    let mut ev = src_utils::event_loop::EventLoop::new(100);
    if let Err(e) = ev.tick() {
        eprintln!("event loop tick failed: {}", e);
    }

    // attempt to connect to X11 and capture a small snapshot to copy into our screen
    match src_utils::x11::DisplayHandle::connect(None) {
        Ok(dpy) => {
            if let Ok(buf) = dpy.capture_root(20, 20) {
                // copy sample into the screen at (0,0)
                for y in 0..20u32 {
                    for x in 0..20u32 {
                        let idx = ((y * 20 + x) * 3) as usize;
                        let r = buf[idx];
                        let g = buf[idx + 1];
                        let b = buf[idx + 2];
                        let _ = screen.set_pixel(x, y, r, g, b);
                    }
                }
            }
        }
        Err(_) => {
            eprintln!("x11: could not connect (stub)");
        }
    }

    // initialize the (stubbed) VNC server and attach the screen
    match src_utils::libvnc_wrapper::VncServer::init_headless(800, 600, args.port) {
        Ok(server) => {
            #[allow(unused_mut)]
            let mut server = server;
            // If libvnc feature is enabled, attach the application's framebuffer
            // to the native server and push an initial update so clients can see it.
            #[cfg(feature = "libvnc")]
            {
                let buf = screen.raw_pixels_mut();
                match server.attach_framebuffer(
                    buf,
                    800,
                    600,
                    24,
                    None,
                    src_utils::libvnc_wrapper::Endianness::Little,
                    src_utils::libvnc_wrapper::ColorOrder::RGB,
                ) {
                    Ok(()) => {
                        // copy the current pixels into the native framebuffer and mark modified
                        let _ = server.update_framebuffer(buf);
                    }
                    Err(e) => eprintln!("attach_framebuffer failed: {}", e),
                }
            }
            println!(
                "Server initialized (stubbed: feature=libvnc {}) — listening on port {}",
                cfg!(feature = "libvnc"),
                server.listen_port()
            );

            // Wait a short while for a client to connect — in stub mode this
            // will immediately return Ok so the program can showcase the
            // intended call flow.
            if let Err(e) = server.wait_for_client(5_000) {
                eprintln!("error waiting for client: {}", e);
                process::exit(3);
            }

            // read back a sample pixel to illustrate integration
            if let Some((r, g, b)) = screen.get_pixel(10, 10) {
                println!("Sample pixel @ (10,10) = #{:02x}{:02x}{:02x}", r, g, b);
            }

            // drive a little sample loop and push updates to libvnc framebuffer
            for i in 0..3 {
                // mutate the screen so updates are visible
                let x = (10 + i) % 800;
                let y = (10 + i) % 600;
                let _ = screen.set_pixel(x, y, 0xff, 0xff, 0xff);
                #[cfg(feature = "libvnc")]
                if let Err(e) = server.update_framebuffer(screen.raw_pixels()) {
                    eprintln!("update_framebuffer failed: {}", e);
                }
                std::thread::sleep(std::time::Duration::from_millis(200));
            }

            println!("Done (PoC): server ran a sample update loop and is shutting down.");
        }
        Err(e) => {
            eprintln!("Failed to initialize server: {}", e);
            process::exit(2);
        }
    }

    // clean exit for PoC - the real server would continue running and accept clients
    process::exit(0);
}

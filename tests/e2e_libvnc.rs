#![cfg(feature = "libvnc")]

use std::time::Duration;

// This test executes an end-to-end path using the real libvncserver
// integration. The test is feature-gated and will only run when you
// invoke `cargo test --features libvnc` on a machine with libvncserver
// installed and visible via pkg-config.

#[test]
fn e2e_init_attach_update_shutdown() {
    // init a small headless server
    let mut server = src_utils::libvnc_wrapper::VncServer::init_headless(64, 48, None)
        .expect("failed to init headless server");

    // create a simple screen buffer and fill with a pattern
    let mut screen = src_utils::screen::Screen::new(64, 48);
    screen.fill(0x12, 0x34, 0x56);

    // attach the framebuffer into the native server
    server
        .attach_framebuffer(
            screen.raw_pixels_mut(),
            64,
            48,
            24,
            None,
            src_utils::libvnc_wrapper::Endianness::Little,
            src_utils::libvnc_wrapper::ColorOrder::RGB,
        )
        .expect("attach_framebuffer failed");

    // push an update and ensure it returns Ok
    server
        .update_framebuffer(screen.raw_pixels())
        .expect("update_framebuffer failed");

    // wait a short while to let the server finish any startup
    server.wait_for_client(100).expect("wait_for_client failed");

    // finally shutdown and cleanup
    server.shutdown();
}

#[test]
fn e2e_real_client_connects() {
    use std::io::{Read, Write};
    use std::net::{SocketAddr, TcpStream};

    // start server
    let mut server = src_utils::libvnc_wrapper::VncServer::init_headless(64, 48, None)
        .expect("failed to init headless server");

    // attach & update
    let mut screen = src_utils::screen::Screen::new(64, 48);
    screen.fill(0xaa, 0xbb, 0xcc);
    server
        .attach_framebuffer(
            screen.raw_pixels_mut(),
            64,
            48,
            24,
            None,
            src_utils::libvnc_wrapper::Endianness::Little,
            src_utils::libvnc_wrapper::ColorOrder::RGB,
        )
        .expect("attach_framebuffer failed");
    server
        .update_framebuffer(screen.raw_pixels())
        .expect("update_framebuffer failed");

    // Give the server a short moment to start listening
    std::thread::sleep(Duration::from_millis(200));

    // Try connecting to the port the server requested. Use the configured/allocated
    // listening port so tests avoid assuming 5900 on the CI host.
    let port = server.listen_port();
    let addr: SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();
    match TcpStream::connect_timeout(&addr, Duration::from_secs(1)) {
        Ok(mut stream) => {
            // read banner (server should send 'RFB xxx\n')
            let mut buf = [0u8; 32];
            let n = stream.read(&mut buf).expect("read failed");
            let banner = String::from_utf8_lossy(&buf[..n]);
            assert!(banner.starts_with("RFB "), "unexpected banner: {}", banner);

            // respond with client version to finish minimal handshake
            let client_ver = b"RFB 003.003\n";
            stream.write_all(client_ver).expect("write failed");

            // basic sanity: attempt a short read to see if server proceeds
            let _ = stream.set_read_timeout(Some(Duration::from_millis(200)));
            let mut small = [0u8; 8];
            let _ = stream.read(&mut small);
        }
        Err(e) => {
            // Connection refused may mean the server isn't listening; fail to
            // indicate the env isn't correctly configured for this test.
            panic!("failed to connect to RFB server on 127.0.0.1:5900: {}", e);
        }
    }

    server.shutdown();
}

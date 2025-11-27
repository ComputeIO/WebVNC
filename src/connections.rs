use std::io::{self, Read, Write};
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

/// Minimal representation of a connected client in the PoC.
#[derive(Debug, Clone)]
pub struct ClientData {
    pub uid: u32,
    pub host: String,
    pub client_port: Option<u16>,
    pub username: Option<String>,
    pub unixname: Option<String>,
    pub login_viewonly: bool,
    pub login_time: u64,
}

impl ClientData {
    pub fn new(uid: u32, host: impl Into<String>, client_port: Option<u16>) -> Self {
        Self {
            uid,
            host: host.into(),
            client_port,
            username: None,
            unixname: None,
            login_viewonly: false,
            login_time: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
        }
    }
}

/// Manager for clients — this intentionally keeps logic simple for the PoC
/// and can be extended to hold behaviour that interacts with libvncserver.
#[derive(Debug, Default)]
pub struct Connections {
    inner: Mutex<Vec<Arc<ClientData>>>,
}

impl Connections {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(Vec::new()),
        }
    }

    pub fn add_client(&self, client: ClientData) {
        let mut guard = self.inner.lock().unwrap();
        guard.push(Arc::new(client));
    }

    pub fn remove_client_by_uid(&self, uid: u32) -> Option<Arc<ClientData>> {
        let mut guard = self.inner.lock().unwrap();
        if let Some(pos) = guard.iter().position(|c| c.uid == uid) {
            Some(guard.remove(pos))
        } else {
            None
        }
    }

    /// Return a comma-separated client list string similar in spirit to the
    /// C implementation, but intentionally simpler for the PoC.
    pub fn list_clients(&self) -> String {
        let guard = self.inner.lock().unwrap();
        let mut parts = Vec::with_capacity(guard.len());
        for c in guard.iter() {
            parts.push(format!(
                "0x{:x}:{}:{}:{}:{}",
                c.uid,
                c.host.replace(':', "#"),
                c.client_port.map(|p| p.to_string()).unwrap_or_else(|| "-".into()),
                c.username.as_deref().unwrap_or("-"),
                c.login_viewonly as u8
            ));
        }
        parts.join(",")
    }

    pub fn count(&self) -> usize {
        let guard = self.inner.lock().unwrap();
        guard.len()
    }
}

/// Simplified access control function. Behavior mirrors the C binary's basic
/// semantics: an empty allow_list accepts any address, otherwise the allow
/// string is treated as comma-separated list of prefixes or exact matches.
pub fn check_access(allow_list: Option<&str>, allow_once: Option<&str>, addr: &str) -> bool {
    let ssl_allowed = false; // placeholder for PoC

    if addr.is_empty() {
        return false;
    }

    if allow_list.is_none() && allow_once.is_none() && !ssl_allowed {
        return true;
    }

    let mut list = String::new();
    if let Some(s) = allow_list {
        list.push_str(s);
    }
    if let Some(s) = allow_once {
        if !list.is_empty() {
            list.push(',');
        }
        list.push_str(s);
    }

    if list.is_empty() {
        return true;
    }

    for part in list.split(|c| c == ',' || c == '\n' || c == ' ' || c == '\t') {
        if part.is_empty() {
            continue;
        }
        if addr == part || addr.starts_with(part) {
            return true;
        }
    }

    false
}

/// Run an external user command while setting up a few RFB_* environment
/// variables. This is a small, safer reimplementation of `run_user_command`
/// from the C source used for accept/gone hooks.
pub fn run_user_command(
    cmd: &str,
    client: Option<&ClientData>,
    mode: &str,
    input: Option<&[u8]>,
) -> io::Result<i32> {
    // Security: do not run anything when cmd is empty
    if cmd.trim().is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "empty cmd"));
    }

    // If mode is `env` we only set env variables and return success as C did.
    if mode == "env" {
        // set some environment variables for consumers or tests
        if let Some(c) = client {
            std::env::set_var("RFB_CLIENT_IP", &c.host);
            if let Some(p) = c.client_port {
                std::env::set_var("RFB_CLIENT_PORT", format!("{}", p));
            } else {
                std::env::set_var("RFB_CLIENT_PORT", "-");
            }
            std::env::set_var("RFB_USERNAME", c.username.as_deref().unwrap_or("-"));
        }
        std::env::set_var("RFB_MODE", mode);
        return Ok(1);
    }

    // build command invocation using shell to preserve compatibility with
    // arbitrary command strings the original did (system()/execlp style).
    let mut child = Command::new("/bin/sh")
        .arg("-c")
        .arg(cmd)
        .stdin(if input.is_some() { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    if let Some(input_buf) = input {
        if let Some(mut stdin) = child.stdin.take() {
            stdin.write_all(input_buf)?;
        }
    }

    let output = child.wait_with_output()?;

    // in the C code, rc >= 256 is divided by 256, so return the normal status
    let rc = output.status.code().unwrap_or(1);

    Ok(rc)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn check_access_empty_allows() {
        assert!(check_access(None, None, "1.2.3.4"));
    }

    #[test]
    fn check_access_matches_prefix() {
        assert!(check_access(Some("192.168"), None, "192.168.1.42"));
        assert!(!check_access(Some("10.0"), None, "192.168.1.1"));
    }

    #[test]
    fn run_user_command_env_mode_sets_vars() {
        let client = ClientData::new(1, "127.0.0.1", Some(5901));
        let rc = run_user_command("true", Some(&client), "env", None).unwrap();
        assert_eq!(rc, 1);
        assert_eq!(std::env::var("RFB_CLIENT_IP").unwrap(), "127.0.0.1");
    }

    #[test]
    fn run_user_command_runs_and_returns_status() {
        let rc = run_user_command("/bin/true", None, "accept", None).unwrap();
        assert_eq!(rc, 0);
        let rc2 = run_user_command("/bin/sh -c 'exit 42'", None, "accept", None).unwrap();
        assert_eq!(rc2, 42);
    }
}

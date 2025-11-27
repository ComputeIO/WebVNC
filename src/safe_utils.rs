//! Safe, pure-Rust replacements for a subset of `util.c` functions
use libc;
use std::env;

pub fn nfix(i: i32, n: i32) -> i32 {
    if i < 0 {
        0
    } else if i >= n {
        n - 1
    } else {
        i
    }
}

pub fn nmin(n: i32, m: i32) -> i32 {
    if n < m {
        n
    } else {
        m
    }
}

pub fn nmax(n: i32, m: i32) -> i32 {
    if n > m {
        n
    } else {
        m
    }
}

pub fn nabs(n: i32) -> i32 {
    if n < 0 {
        -n
    } else {
        n
    }
}

pub fn dabs(x: f64) -> f64 {
    if x < 0.0 {
        -x
    } else {
        x
    }
}

pub fn is_decimal(s: &str) -> bool {
    let mut chars = s.chars();
    if let Some(c) = chars.next() {
        if c == '-' {
            // allow leading minus
        } else if !c.is_ascii_digit() {
            return false;
        }
    } else {
        return false;
    }
    for ch in chars {
        if !ch.is_ascii_digit() {
            return false;
        }
    }
    true
}

pub fn scan_hexdec(s: &str) -> Option<u64> {
    let st = s.trim();
    if st.starts_with("0x") || st.starts_with("0X") {
        u64::from_str_radix(&st[2..], 16).ok()
    } else {
        st.parse::<u64>().ok()
    }
}

pub fn parse_geom(s: &str, total_w: i32, total_h: i32) -> Option<(i32, i32, i32, i32)> {
    // parse forms like WxH{+-}X{+-}Y
    let (w_part, rest) = s.split_once('x')?;
    let parsed_w = w_part.parse::<i32>().ok()?;

    // rest starts with height digits
    let mut idx = 0usize;
    let bytes = rest.as_bytes();
    while idx < bytes.len() && (bytes[idx] as char).is_ascii_digit() {
        idx += 1;
    }
    if idx == 0 {
        return None;
    }
    let h_part = &rest[..idx];
    let parsed_h = h_part.parse::<i32>().ok()?;
    let rem = &rest[idx..];
    if rem.len() < 2 {
        return None;
    }

    // parse {sign}{num}{sign}{num}
    let mut pos = 0usize;
    let sign1 = rem.chars().nth(pos)?;
    if sign1 != '+' && sign1 != '-' {
        return None;
    }
    pos += 1;
    let start1 = pos;
    while pos < rem.len() && (rem.as_bytes()[pos] as char).is_ascii_digit() {
        pos += 1;
    }
    if start1 == pos {
        return None;
    }
    let num1 = &rem[start1..pos];
    if pos >= rem.len() {
        return None;
    }
    let sign2 = rem.chars().nth(pos)?;
    if sign2 != '+' && sign2 != '-' {
        return None;
    }
    pos += 1;
    let start2 = pos;
    while pos < rem.len() && (rem.as_bytes()[pos] as char).is_ascii_digit() {
        pos += 1;
    }
    if start2 == pos {
        return None;
    }
    let num2 = &rem[start2..pos];
    if pos != rem.len() {
        return None;
    }

    let mut nx = num1.parse::<i32>().ok()?;
    let mut ny = num2.parse::<i32>().ok()?;
    if sign1 == '-' {
        nx = nx.abs();
        nx = total_w - nx - parsed_w;
    }
    if sign2 == '-' {
        ny = ny.abs();
        ny = total_h - ny - parsed_h;
    }

    Some((parsed_w, parsed_h, nx, ny))
}

pub fn bitprint(st: u32, nbits: usize) -> String {
    let nb = nbits.min(32);
    let mut out = String::with_capacity(nb);
    for i in (0..nb).rev() {
        if (st & (1 << i)) != 0 {
            out.push('1');
        } else {
            out.push('0');
        }
    }
    out
}

pub fn get_user_name() -> String {
    env::var("USER")
        .or_else(|_| env::var("LOGNAME"))
        .unwrap_or_else(|_| "unknown-user".to_string())
}

pub fn get_home_dir() -> String {
    env::var("HOME").unwrap_or_else(|_| "/".to_string())
}

pub fn get_shell() -> String {
    env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string())
}

pub fn this_host() -> Option<String> {
    // Prefer HOSTNAME env var when available (useful in CI/container)
    if let Ok(h) = env::var("HOSTNAME") {
        if !h.is_empty() {
            return Some(h);
        }
    }

    // Fallback to libc gethostname for a lightweight, portable approach.
    unsafe {
        let mut buf = [0u8; 256];
        let rc = libc::gethostname(buf.as_mut_ptr() as *mut i8, buf.len());
        if rc == 0 {
            if let Ok(s) = std::ffi::CStr::from_ptr(buf.as_ptr() as *const i8).to_str() {
                return Some(s.to_string());
            }
        }
    }
    None
}

pub fn match_str_list(s: &str, list: &[&str]) -> bool {
    for &item in list {
        if item == "*" || s.contains(item) {
            return true;
        }
    }
    false
}

pub fn create_str_list(csv: &str) -> Vec<String> {
    csv.split(',').map(|p| p.trim().to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_decimal() {
        assert!(is_decimal("123"));
        assert!(is_decimal("-12"));
        assert!(!is_decimal("12a"));
    }

    #[test]
    fn test_parse_geom() {
        assert_eq!(parse_geom("10x20+3+4", 100, 200), Some((10, 20, 3, 4)));
    }
}

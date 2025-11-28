//! Partial port of pointer button remap parsing from x11vnc C
//!
//! This module implements a simplified parser for the `-buttonmap` format
//! that the original C code supports. It intentionally focuses on the
//! `:sym+sym:`, `ButtonN` and numeric hex forms, returning a testable
//! mapping structure without X11 dependency.

// no std collections required yet — keep simple parser only

pub const MAX_BUTTONS: usize = 64;

/// An action that a pointer button event can be remapped to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RemapAction {
    /// Target is another button (1-based)
    Button(u8),
    /// Keysym indicated as a numeric value (e.g. 0xFF0A)
    KeysymNum(u32),
    /// Keysym indicated as a textual name (e.g. "Up")
    KeysymName(String),
}

/// Mapping for a single button (index 1..MAX_BUTTONS) containing an ordered
/// list of remap actions that should be sent when that source button is
/// pressed.
pub type ButtonMap = Vec<Vec<RemapAction>>;

use crate::userinput::InputEvent;

/// Convert a single remap action into one or more `InputEvent`s given a
/// source `pressed` boolean. For KeysymNum we emit a down then up event
/// (pressed true -> send down/up), for Button we emit a pointer mask
/// change event. `x`,`y` and `current_mask` are used for pointer events.
fn remap_action_to_input_events(
    action: &RemapAction,
    pressed: bool,
    x: i32,
    y: i32,
    current_mask: u32,
) -> Vec<InputEvent> {
    match action {
        RemapAction::Button(n) => {
            let bit = 1u32 << ((n.saturating_sub(1)) as u32);
            let new_mask = if pressed { current_mask | bit } else { current_mask & !bit };
            vec![InputEvent::Pointer { x, y, button_mask: new_mask as u8 }]
        }
        RemapAction::KeysymNum(k) => {
            // produce down then up events; if `pressed` is false, only produce up
            let mut out = Vec::new();
            if pressed {
                out.push(InputEvent::Key { keysym: *k, pressed: true, modifiers: 0 });
                out.push(InputEvent::Key { keysym: *k, pressed: false, modifiers: 0 });
            } else {
                out.push(InputEvent::Key { keysym: *k, pressed: false, modifiers: 0 });
            }
            out
        }
        RemapAction::KeysymName(_) => {
            // textual keysym names are not converted in the PoC yet; ignore
            Vec::new()
        }
    }
}

/// Parse a `:sym+sym:` style list (without the surrounding colons) or a
/// single token into a vector of actions.
fn parse_sym_list(list: &str) -> Vec<RemapAction> {
    let mut out = Vec::new();
    for token in list.split('+') {
        let t = token.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with("Button") {
            if let Ok(n) = t[6..].parse::<u8>() {
                out.push(RemapAction::Button(n));
                continue;
            }
        }
        if t.starts_with("0x") {
            if let Ok(v) = u32::from_str_radix(&t[2..], 16) {
                out.push(RemapAction::KeysymNum(v));
                continue;
            }
        }
        // otherwise treat as keysym name
        out.push(RemapAction::KeysymName(t.to_string()));
    }
    out
}

/// Parse a mapping for a single source button from either the `:sym+sym:`
/// style string or a single-digit. Returns actions (possibly empty).
fn parse_button_remap_for(_from: usize, s: &str) -> Vec<RemapAction> {
    let mut out = Vec::new();
    let q = s.trim();
    if q.is_empty() {
        return out;
    }
    if q.starts_with(':') {
        // find the closing ':'
        if let Some(endpos) = q[1..].find(':') {
            let inner = &q[1..1 + endpos];
            out.extend(parse_sym_list(inner));
        }
    } else if q.ends_with(':') {
        // allow the form `2:0x12+Button3:` where the tail doesn't start
        // with ':' but ends with one — strip the terminal colon and parse
        // the remaining as a list.
        let inner = q.trim_end_matches(':');
        out.extend(parse_sym_list(inner));
    } else {
        // single digit like "4" or "12"
        if let Ok(n) = q.parse::<u8>() {
            out.push(RemapAction::Button(n));
        }
    }
    out
}

/// Initialize an empty ButtonMap with MAX_BUTTONS+1 slots (we keep index by
/// button ID so slot 0 is unused).
fn empty_button_map() -> ButtonMap {
    let mut v = Vec::with_capacity(MAX_BUTTONS + 1);
    for _ in 0..=MAX_BUTTONS {
        v.push(Vec::new());
    }
    v
}

/// Initialize pointer map from a remap string in a small subset of the C
/// format. We accept only simple remaps in two forms:
/// - `2`: remap button 2 to be button 2 (identity; included for tests)
/// - `4::`: remap button 4 to events described by `:Sym+...:` format (no leading 'from' digit, the caller must
///   already provide the `from` digit)
/// - `2:Up+Down:` explicit `from` and `:...:` forms like `2:Up+Down:` or `3:Button1:`
/// - multiple remaps can be separated by `,`
///
/// This intentionally leaves out more advanced forms such as the `12-21=2`
/// header syntax present in the C sources.
pub fn initialize_pointer_map(remap_str: Option<&str>) -> ButtonMap {
    let mut map = empty_button_map();
    if remap_str.is_none() {
        // default identity mapping for first few buttons
        for i in 1..=5usize {
            map[i].push(RemapAction::Button(i as u8));
        }
        return map;
    }
    let s = remap_str.unwrap().trim();
    if s.is_empty() {
        return map;
    }

    // split into tokens by ',' where each token is either like `2:...` or
    // `:...` or `4` (single numeric)
    for token in s.split(',') {
        let t = token.trim();
        if t.is_empty() {
            continue;
        }
        // check for prefix `N:` where N is source button number
        if let Some(colon_pos) = t.find(':') {
            let head = &t[..colon_pos];
            let tail = &t[colon_pos + 1..];
            // support both forms: '2:Up+Down:' or '2::Up+...' if given
            if let Ok(from) = head.parse::<usize>() {
                let actions = parse_button_remap_for(from, tail);
                if from <= MAX_BUTTONS {
                    map[from] = actions;
                }
                continue;
            } else {
                // if head is empty then it's like `:Up+Down:` without leading button; skip
                if head.is_empty() {
                    // interpret as actions for button 1 by default in this simplified parser
                    let actions = parse_button_remap_for(1, t);
                    map[1] = actions;
                    continue;
                }
            }
        }
        // otherwise if token is a number it is a button id mapping to itself or the rest of the token
        if let Ok(n) = t.parse::<usize>() {
            if n <= MAX_BUTTONS {
                map[n].push(RemapAction::Button(n as u8));
            }
            continue;
        }

        // attempts to parse as `:sym+...:` etc
        if t.starts_with(':') {
            let actions = parse_button_remap_for(1, t); // default source 1
            map[1] = actions;
        }
    }

    map
}

/// Given a button map and a source button event (button id and pressed
/// state), emit a list of InputEvents representing remapped actions. The
/// `x`,`y` and `current_mask` parameters are used to construct pointer
/// events; callers should pass the most recent pointer mask.
pub fn remap_button_event_to_input_events(
    map: &ButtonMap,
    src_button: u8,
    pressed: bool,
    x: i32,
    y: i32,
    current_mask: u32,
) -> Vec<InputEvent> {
    if src_button == 0 || (src_button as usize) >= map.len() {
        return Vec::new();
    }
    let mut out = Vec::new();
    for action in map[src_button as usize].iter() {
        let mut evs = remap_action_to_input_events(action, pressed, x, y, current_mask);
        out.append(&mut evs);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_map_has_identity_first_buttons() {
        let m = initialize_pointer_map(None);
        assert_eq!(m[1], vec![RemapAction::Button(1u8)]);
        assert_eq!(m[2], vec![RemapAction::Button(2u8)]);
    }

    #[test]
    fn parse_single_button_token() {
        let m = initialize_pointer_map(Some("4"));
        assert_eq!(m[4], vec![RemapAction::Button(4u8)]);
    }

    #[test]
    fn parse_colon_sym_list() {
        let m = initialize_pointer_map(Some("1::Up+Down:"));
        // because we use a very forgiving parser the leading empty head is allowed
        // and the mapping gets stored in slot 1
        assert!(m[1].len() >= 2);
        assert_eq!(m[1][0], RemapAction::KeysymName("Up".to_string()));
        assert_eq!(m[1][1], RemapAction::KeysymName("Down".to_string()));
    }

    #[test]
    fn parse_button_with_hex_keysym() {
        let m = initialize_pointer_map(Some("2:0x46+Button3:"));
        assert_eq!(m[2][0], RemapAction::KeysymNum(0x46));
        assert_eq!(m[2][1], RemapAction::Button(3));
    }

    #[test]
    fn parse_named_button_string() {
        let m = initialize_pointer_map(Some(":Foo+Bar:"));
        assert_eq!(m[1][0], RemapAction::KeysymName("Foo".to_string()));
        assert_eq!(m[1][1], RemapAction::KeysymName("Bar".to_string()));
    }

    #[test]
    fn remap_button_event_to_input_events_generates_key_and_pointer() {
        use crate::userinput::InputEvent;
        let m = initialize_pointer_map(Some("2:0x46+Button3:"));
        // emulate src button 2 pressed with initial mask 0
        let evs = remap_button_event_to_input_events(&m, 2u8, true, 10, 20, 0);
        // should contain both the keysym send (down/up) and a pointer event
        assert!(evs.iter().any(|e| matches!(e, InputEvent::Key { keysym: 0x46, .. })));
        assert!(evs.iter().any(|e| matches!(e, InputEvent::Pointer { button_mask, .. } if *button_mask == 1u8 << (3-1))));
    }
}

// Partial port of the `userinput` subsystem from the C implementation.
//
// This file focuses on parsing the wireframe/scroll-copyrect parameter
// strings and exposing a few simple state values used by higher-level
// components in the PoC. It is intentionally conservative and test-driven
// so the more complex, stateful event loop machinery can be done next.

#[derive(Debug, Clone, PartialEq)]
pub struct ScrollCopyRectParams {
    pub top: i32,
    pub bottom: i32,
    pub left: i32,
    pub right: i32,
    pub key_time: f64,
    pub key_persist: f64,
    pub key_bdpush_time: f64,
    pub mouse_time: f64,
    pub mouse_persist: f64,
    pub mouse_bdpush_time: f64,
    pub mouse_pointer_delay: f64,
    pub mouse_maxtime: f64,
}

impl Default for ScrollCopyRectParams {
    fn default() -> Self {
        Self {
            top: 0,
            bottom: 0,
            left: 0,
            right: 0,
            key_time: 0.0,
            key_persist: 0.0,
            key_bdpush_time: 0.0,
            mouse_time: 0.0,
            mouse_persist: 0.0,
            mouse_bdpush_time: 0.0,
            mouse_pointer_delay: 0.0,
            mouse_maxtime: 0.0,
        }
    }
}

/// Parse a comma-separated scroll-copyrect string into typed parameters.
/// Supported forms are identical to the C PoC's subset:
/// "T+B+L+R" for top/bottom/left/right and timings for key/mouse.
pub fn parse_scroll_copyrect_str(s: &str) -> ScrollCopyRectParams {
    let mut out = ScrollCopyRectParams::default();
    if s.trim().is_empty() {
        return out;
    }

    // Split into parts by comma. C code expects part0 to be T+B+L+R,
    // part1: key timings, part2: mouse timings
    let parts: Vec<&str> = s.split(',').collect();

    // part 0: T+B+L+R
    if let Some(p) = parts.get(0) {
        let mut ints = p.split('+').filter_map(|v| v.trim().parse::<i32>().ok());
        if let (Some(t), Some(b), Some(l), Some(r)) = (ints.next(), ints.next(), ints.next(), ints.next()) {
            out.top = t;
            out.bottom = b;
            out.left = l;
            out.right = r;
        }
    }

    // part1: key timings t1+t2+t3
    if let Some(p) = parts.get(1) {
        let mut floats = p.split('+').filter_map(|v| v.trim().parse::<f64>().ok());
        if let (Some(t1), Some(t2), Some(t3)) = (floats.next(), floats.next(), floats.next()) {
            out.key_time = t1;
            out.key_persist = t2;
            out.key_bdpush_time = t3;
        }
    }

    // part2: mouse timings t1+t2+t3+t4+t5
    if let Some(p) = parts.get(2) {
        let mut floats = p.split('+').filter_map(|v| v.trim().parse::<f64>().ok());
        if let (Some(t1), Some(t2), Some(t3), Some(t4), Some(t5)) = (
            floats.next(),
            floats.next(),
            floats.next(),
            floats.next(),
            floats.next(),
        ) {
            out.mouse_time = t1;
            out.mouse_persist = t2;
            out.mouse_bdpush_time = t3;
            out.mouse_pointer_delay = t4;
            out.mouse_maxtime = t5;
        }
    }

    out
}

#[derive(Debug, Clone, PartialEq)]
pub struct WireframeParams {
    pub shade: u32,
    pub linewidth: i32,
    pub frac: f64,
    pub top: i32,
    pub bottom: i32,
    pub left: i32,
    pub right: i32,
}

impl Default for WireframeParams {
    fn default() -> Self {
        Self {
            shade: 0xff,
            linewidth: 2,
            frac: 0.0,
            top: 0,
            bottom: 0,
            left: 0,
            right: 0,
        }
    }
}

/// Parse the wireframe parameter string (a subset of C's parse_wireframe_str
/// semantics). We accept: <shade>,<lw>,<frac>,<top+bot+left+right> as first
/// four fields; other fields are ignored for this PoC.
pub fn parse_wireframe_str(s: &str) -> WireframeParams {
    let mut out = WireframeParams::default();
    if s.trim().is_empty() {
        return out;
    }

    let parts: Vec<&str> = s.split(',').collect();

    // part 0: shade - accept hex 0xRRGGBB or decimal
    if let Some(p) = parts.get(0) {
        let ptrim = p.trim();
        if ptrim.starts_with("0x") {
            if let Ok(n) = u32::from_str_radix(&ptrim[2..], 16) {
                out.shade = n;
            }
        } else if let Ok(n) = ptrim.parse::<u32>() {
            out.shade = n;
        }
    }

    // part1: linewidth
    if let Some(p) = parts.get(1) {
        if let Ok(n) = p.trim().parse::<i32>() {
            let n = n.max(1).min(8);
            out.linewidth = n;
        }
    }

    // part2: frac (percentage or floating)
    if let Some(p) = parts.get(2) {
        let t = p.trim();
        if !t.is_empty() {
            if t.contains('.') {
                if let Ok(f) = t.parse::<f64>() { out.frac = f; }
            } else if let Ok(i) = t.parse::<i32>() { out.frac = (i as f64) / 100.0; }
        }
    }

    // part3: top+bot+left+right
    if let Some(p) = parts.get(3) {
        let mut ints = p.split('+').filter_map(|v| v.trim().parse::<i32>().ok());
        if let (Some(t), Some(b), Some(l), Some(r)) = (ints.next(), ints.next(), ints.next(), ints.next()) {
            out.top = t;
            out.bottom = b;
            out.left = l;
            out.right = r;
        }
    }

    out
}

/// Small input event model used by the PoC server.
#[derive(Debug, Clone, PartialEq)]
pub enum InputEvent {
    Key { keysym: u32, pressed: bool, modifiers: u32 },
    Pointer { x: i32, y: i32, button_mask: u8 },
}

/// Simple FIFO event queue for keyboard + pointer events.
#[derive(Debug, Default)]
pub struct EventQueue {
    keys: Vec<InputEvent>,
    ptrs: Vec<InputEvent>,
}

impl EventQueue {
    pub fn new() -> Self {
        EventQueue {
            keys: Vec::new(),
            ptrs: Vec::new(),
        }
    }

    pub fn push(&mut self, ev: InputEvent) {
        match ev {
            InputEvent::Key { .. } => self.keys.push(ev),
            InputEvent::Pointer { .. } => self.ptrs.push(ev),
        }
    }

    pub fn len(&self) -> usize {
        self.keys.len() + self.ptrs.len()
    }

    pub fn clear(&mut self) {
        self.keys.clear();
        self.ptrs.clear();
    }

    /// Remove up to `max_eat` input events when a client is view-only.
    /// Returns the number of events eaten.
    pub fn eat_viewonly_input(&mut self, max_eat: usize, keep: bool) -> usize {
        let mut eaten = 0usize;
        // Prefer to drop pointer events first
        while eaten < max_eat && !self.ptrs.is_empty() {
            if keep {
                // if we keep, we simply break (we won't discard)
                break;
            }
            self.ptrs.remove(0);
            eaten += 1;
        }
        while eaten < max_eat && !self.keys.is_empty() {
            if keep {
                break;
            }
            self.keys.remove(0);
            eaten += 1;
        }
        eaten
    }

    /// Drain and return up to `limit` events (FIFO order across pointers and keys).
    /// We yield pointer events first followed by keys as they were inserted.
    pub fn drain(&mut self, limit: Option<usize>) -> Vec<InputEvent> {
        let mut out = Vec::new();
        let mut taken = 0usize;
        let max = limit.unwrap_or(usize::MAX);

        while taken < max && !self.ptrs.is_empty() {
            out.push(self.ptrs.remove(0));
            taken += 1;
        }
        while taken < max && !self.keys.is_empty() {
            out.push(self.keys.remove(0));
            taken += 1;
        }

        out
    }
}

/// Process up to `limit` events by draining them and invoking the provided
/// callback for each event. Returns the number of processed events.
pub fn check_user_input<F>(queue: &mut EventQueue, limit: Option<usize>, mut cb: F) -> usize
where
    F: FnMut(InputEvent),
{
    let evs = queue.drain(limit);
    let mut processed = 0usize;
    for ev in evs.into_iter() {
        // Dispatch to callback; for pointer/key events the callback may
        // choose to inject into a DisplayHandle or other system.
        cb(ev);
        processed += 1;
    }
    processed
}

/// Dispatch a single InputEvent to the provided DisplayHandle (if any).
/// This currently delegates to `x11::DisplayHandle` for real injection
/// when the x11 feature is enabled; otherwise it is a no-op.
pub fn dispatch_event_to_display(
    display: Option<&mut crate::inject::Injector>,
    ev: &InputEvent,
) -> Result<(), String> {
    if let Some(d) = display {
        match ev {
            InputEvent::Key { keysym, pressed, .. } => d
                .send_key(*keysym as u16, *pressed)
                .map_err(|e| e.to_string()),
            InputEvent::Pointer { x, y, button_mask } => d
                .send_pointer(*x, *y, *button_mask as u32)
                .map_err(|e| e.to_string()),
        }
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_scroll_copyrect_basic() {
        let s = "10+20+3+4,0.5+0.3+2.0,0.1+0.2+0.3+0.4+0.5";
        let p = parse_scroll_copyrect_str(s);
        assert_eq!(p.top, 10);
        assert_eq!(p.bottom, 20);
        assert_eq!(p.left, 3);
        assert_eq!(p.right, 4);
        assert_eq!(p.key_time, 0.5);
        assert_eq!(p.key_persist, 0.3);
        assert_eq!(p.key_bdpush_time, 2.0);
        assert_eq!(p.mouse_maxtime, 0.5);
    }

    #[test]
    fn event_queue_eat_viewonly() {
        let mut q = EventQueue::new();
        q.push(InputEvent::Pointer { x: 10, y: 20, button_mask: 1 });
        q.push(InputEvent::Key { keysym: 10, pressed: true, modifiers: 0 });
        q.push(InputEvent::Pointer { x: 11, y: 21, button_mask: 0 });

        assert_eq!(q.len(), 3);
        let eaten = q.eat_viewonly_input(2, false);
        assert_eq!(eaten, 2);
        assert_eq!(q.len(), 1);
    }

    #[test]
    fn event_queue_drain_and_clear() {
        let mut q = EventQueue::new();
        q.push(InputEvent::Pointer { x: 1, y: 2, button_mask: 0 });
        q.push(InputEvent::Key { keysym: 2, pressed: false, modifiers: 0 });
        let drained = q.drain(None);
        assert_eq!(drained.len(), 2);
        assert_eq!(q.len(), 0);
        q.push(InputEvent::Key { keysym: 4, pressed: true, modifiers: 1 });
        q.clear();
        assert_eq!(q.len(), 0);
    }

    #[test]
    fn check_user_input_callback_runs() {
        let mut q = EventQueue::new();
        q.push(InputEvent::Pointer { x: 3, y: 4, button_mask: 1 });
        q.push(InputEvent::Key { keysym: 7, pressed: true, modifiers: 0 });

        let mut seen = Vec::new();
        let processed = super::check_user_input(&mut q, Some(10), |ev| {
            seen.push(ev);
        });
        assert_eq!(processed, 2);
        assert_eq!(seen.len(), 2);
        assert_eq!(q.len(), 0);
    }

    #[test]
    fn dispatch_event_to_display_noop() {
        let mut q = EventQueue::new();
        q.push(InputEvent::Key { keysym: 10, pressed: true, modifiers: 0 });
        // dispatch without a display should be Ok
        let evs = q.drain(None);
        for ev in evs.iter() {
            assert!(super::dispatch_event_to_display(None, ev).is_ok());
        }
    }

    #[test]
    fn parse_wireframe_basic_hex() {
        let s = "0xff,4,32,10+20+30+40";
        let p = parse_wireframe_str(s);
        assert_eq!(p.shade, 0xff);
        assert_eq!(p.linewidth, 4);
        // The C code interprets "32" as a percentage => 0.32
        assert!((p.frac - 0.32).abs() < 1e-6);
        assert_eq!(p.top, 10);
        assert_eq!(p.right, 40);
    }

    #[test]
    fn parse_wireframe_decimal_shade_and_frac_percent() {
        let s = "127,2,15,1+2+3+4";
        let p = parse_wireframe_str(s);
        assert_eq!(p.shade, 127);
        assert_eq!(p.linewidth, 2);
        assert!((p.frac - 0.15).abs() < 1e-6);
        assert_eq!(p.left, 3);
    }
}

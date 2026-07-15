//! The live guided tour: a screen-wide overlay (layer-shell mask strips +
//! a floating callout, see `mask`/`callout`) that walks the user through
//! real on-screen components instead of describing them in a window of
//! breadhelp's own. Replaces the old in-window `Onboarding` wizard.
//!
//! Steps are user-paced, so the engine always tears down and rebuilds the 5
//! layer-shell windows on every transition rather than repositioning them in
//! place — simpler and cheap enough at this frequency.

mod callout;
mod mask;
mod target;

use std::cell::RefCell;
use std::time::Duration;

use gtk4::prelude::*;
use gtk4::Window;

use crate::config::State;
use crate::content::keybinds::{self, Keybind};
use crate::content::tour::{self, Step, Success};
use crate::services::{exec, hyprland};

use callout::Callout;

/// How long a step with no detected target waits before offering a "Show
/// me" fallback button. Deliberately NOT a timer that fires the demo
/// automatically — an auto-fired demo can't be told apart from the user's
/// own keypress from their side, silently teaching them nothing while
/// looking like it worked. The button preserves that the user chose it.
const HINT_DELAY_SECS: u32 = 10;
const WATCH_POLL_INTERVAL: Duration = Duration::from_millis(400);

struct TourState {
    steps: Vec<Step>,
    binds: Vec<Keybind>,
    display: gdk4::Display,
    index: usize,
    masks: Vec<Window>,
    callout: Option<Callout>,
    poll_source: Option<glib::SourceId>,
    timeout_source: Option<glib::SourceId>,
    hint_source: Option<glib::SourceId>,
    /// Whether the current step's `success_event` has already fired. Doesn't
    /// auto-advance — just switches the callout into a "done, click Next
    /// when ready" state, so the user actually has time to read the step
    /// instead of it vanishing the instant the target event arrives.
    confirmed: bool,
    /// Whether the "Show me" fallback button has appeared for this step yet
    /// (see `HINT_DELAY_SECS`).
    hint_visible: bool,
}

thread_local! {
    static STATE: RefCell<Option<TourState>> = const { RefCell::new(None) };
}

/// Begin the tour from wherever `state.toml`'s `onboarding.step` left off —
/// the every-login autostart's genuine-first-run trigger.
pub fn start(display: &gdk4::Display) {
    self_heal();
    let steps = tour::load();
    if steps.is_empty() {
        return;
    }
    let start_index = (State::load().onboarding_step() as usize).min(steps.len() - 1);
    STATE.with(|cell| {
        *cell.borrow_mut() = Some(TourState {
            steps,
            binds: keybinds::load(),
            display: display.clone(),
            index: 0,
            masks: Vec::new(),
            callout: None,
            poll_source: None,
            timeout_source: None,
            hint_source: None,
            confirmed: false,
            hint_visible: false,
        });
    });
    enter_step(start_index);
}

/// `breadhelp --onboard` — restart from step 0 regardless of prior progress.
pub fn restart(display: &gdk4::Display) {
    let mut state = State::load();
    state.set_onboarding_completed(false);
    state.set_onboarding_step(0);
    start(display);
}

/// Reverts a keybind left rebound by a crashed previous run before it can
/// surprise the user (e.g. popping breadhelp open on a later, unrelated
/// press) — see `config::State::pending_rebind`. Safe to call any time,
/// including when nothing is pending.
pub fn self_heal() {
    revert_pending_rebind();
}

/// A `--tour-event <id>` arrived, forwarded by `breadhelp-tour.lua` over the
/// same D-Bus argv-forwarding `--suggest` already uses. Hard no-op unless
/// the active tour's current step is actually waiting for this exact id —
/// a stray/late event must never be able to do anything visible. Does NOT
/// auto-advance to the next step — it confirms the current one (so the
/// callout can show a "done" state) and leaves moving on to an explicit
/// Next click, since jumping topics the instant the target event fires
/// gives the user no time to actually read anything about it.
pub fn on_tour_event(id: &str) {
    let hit = STATE.with(|cell| {
        let mut borrow = cell.borrow_mut();
        let Some(state) = borrow.as_mut() else { return None };
        // Only a *visibly* confirmed step blocks a re-fire. `confirmed` gets
        // set below, before `render_step` runs, so that render actually
        // paints the "done" state instead of the stale pre-confirmation one
        // — but that means a step whose render silently no-ops (e.g. the
        // near-fullscreen suppression branch racing a `closelayer` event)
        // would otherwise latch `confirmed = true` with no callout to show
        // for it, permanently blocking every later retry. Checking
        // `callout.is_some()` alongside `confirmed` is what actually
        // prevents that: a confirmed-but-invisible step is not treated as
        // handled, so a later duplicate event gets another chance.
        if state.confirmed && state.callout.is_some() {
            return None;
        }
        let matches = matches!(&state.steps[state.index].success(), Success::Event(e) if e == id);
        if !matches {
            return None;
        }
        state.confirmed = true;
        Some((state.index, state.steps[state.index].clone(), state.binds.clone(), state.steps.len()))
    });
    if let Some((index, step, binds, total)) = hit {
        // force_no_target=true: we already have authoritative confirmation
        // from the real compositor event that fired this — see the comment
        // in `render_step` on why re-querying live state here is unsafe.
        render_step(index, &step, &binds, index + 1 >= total, true);
    }
}

fn go_next() {
    let next = STATE.with(|cell| cell.borrow().as_ref().map(|s| (s.index + 1, s.steps.len())));
    let Some((next_index, total)) = next else { return };
    if next_index >= total {
        finish();
    } else {
        enter_step(next_index);
    }
}

fn go_back() {
    let index = STATE.with(|cell| cell.borrow().as_ref().map(|s| s.index));
    let Some(index) = index else { return };
    if index > 0 {
        enter_step(index - 1);
    }
}

fn finish() {
    revert_pending_rebind();
    teardown_visuals();
    let mut state = State::load();
    state.set_onboarding_completed(true);
    STATE.with(|cell| *cell.borrow_mut() = None);
}

fn teardown_visuals() {
    STATE.with(|cell| {
        if let Some(state) = cell.borrow_mut().as_mut() {
            if let Some(id) = state.poll_source.take() {
                id.remove();
            }
            if let Some(id) = state.timeout_source.take() {
                id.remove();
            }
            if let Some(id) = state.hint_source.take() {
                id.remove();
            }
        }
    });
    clear_visuals();
}

/// Just the mask + callout windows — never touches `poll_source`/
/// `timeout_source`, so this is safe to call from inside their own
/// still-executing callbacks (unlike `teardown_visuals`).
fn clear_visuals() {
    STATE.with(|cell| {
        if let Some(state) = cell.borrow_mut().as_mut() {
            for w in state.masks.drain(..) {
                w.close();
            }
            if let Some(c) = state.callout.take() {
                c.window.close();
            }
        }
    });
}

fn enter_step(index: usize) {
    revert_pending_rebind();
    teardown_visuals();

    let (step, binds, total) = STATE.with(|cell| {
        let mut borrow = cell.borrow_mut();
        let state = borrow.as_mut().expect("enter_step called with no active tour");
        state.index = index;
        state.confirmed = false;
        state.hint_visible = false;
        (state.steps[index].clone(), state.binds.clone(), state.steps.len())
    });

    let mut cfg = State::load();
    cfg.set_onboarding_step(index as i64);

    render_step(index, &step, &binds, index + 1 >= total, false);

    // Watch for the target appearing from the user's OWN action (pressing
    // the real shortcut) — never fire `launch` here. A step that teaches a
    // shortcut only actually teaches it if the user is the one who presses
    // it; see `HINT_DELAY_SECS` for the deliberate, user-initiated fallback.
    if !target::is_resolved(&step.target()) {
        schedule_poll(index);
        if step.launch.is_some() {
            schedule_hint(index);
        }
    }

    if step.rebind_combo.is_some() {
        apply_rebind_for_step(&step, &binds);
    }

    if let Success::Timeout(secs) = step.success() {
        let source = glib::timeout_add_seconds_local(secs as u32, move || {
            let still_current = STATE.with(|cell| cell.borrow().as_ref().map(|s| s.index)) == Some(index);
            // Clear our own handle before advancing rather than letting
            // `teardown_visuals` call `.remove()` on us — this closure is
            // already mid-dispatch and about to return `Break`, and self-
            // removing via `SourceId::remove()` from inside your own
            // callback is a GLib footgun (can log a spurious critical).
            STATE.with(|cell| {
                if let Some(s) = cell.borrow_mut().as_mut() {
                    s.timeout_source = None;
                }
            });
            if still_current {
                go_next();
            }
            glib::ControlFlow::Break
        });
        STATE.with(|cell| {
            if let Some(s) = cell.borrow_mut().as_mut() {
                s.timeout_source = Some(source);
            }
        });
    }
}

/// Watches for the step's target appearing — from the user's own real
/// keypress, or from clicking "Show me" (see `schedule_hint`), the poll
/// doesn't care which. Runs for as long as the step stays current; there's
/// no attempt cap because we're waiting on the user, not a fast-launching
/// process, and Next is always available as a manual escape hatch anyway.
fn schedule_poll(index: usize) {
    let source = glib::timeout_add_local(WATCH_POLL_INTERVAL, move || {
        let still_current = STATE.with(|cell| cell.borrow().as_ref().map(|s| s.index)) == Some(index);
        if !still_current {
            return glib::ControlFlow::Break;
        }
        let (step, binds, total) = STATE.with(|cell| {
            let borrow = cell.borrow();
            let s = borrow.as_ref().unwrap();
            (s.steps[index].clone(), s.binds.clone(), s.steps.len())
        });
        if target::is_resolved(&step.target()) {
            render_step(index, &step, &binds, index + 1 >= total, false);
            clear_poll_source();
        } else {
            schedule_poll(index);
        }
        glib::ControlFlow::Break
    });
    STATE.with(|cell| {
        if let Some(s) = cell.borrow_mut().as_mut() {
            s.poll_source = Some(source);
        }
    });
}

fn clear_poll_source() {
    STATE.with(|cell| {
        if let Some(s) = cell.borrow_mut().as_mut() {
            s.poll_source = None;
        }
    });
}

/// After `HINT_DELAY_SECS` with no target detected, reveals the "Show me"
/// fallback button by re-rendering the step with `hint_visible` set.
fn schedule_hint(index: usize) {
    let source = glib::timeout_add_seconds_local(HINT_DELAY_SECS, move || {
        // Clear our own handle first — see the self-removal note on the
        // Timeout branch in `enter_step`, same footgun applies here.
        STATE.with(|cell| {
            if let Some(s) = cell.borrow_mut().as_mut() {
                s.hint_source = None;
            }
        });
        let should_show = STATE.with(|cell| {
            let borrow = cell.borrow();
            borrow.as_ref().map(|s| s.index == index && !s.confirmed).unwrap_or(false)
        });
        if should_show {
            let (step, binds, total) = STATE.with(|cell| {
                let mut borrow = cell.borrow_mut();
                let s = borrow.as_mut().unwrap();
                s.hint_visible = true;
                (s.steps[index].clone(), s.binds.clone(), s.steps.len())
            });
            render_step(index, &step, &binds, index + 1 >= total, false);
        }
        glib::ControlFlow::Break
    });
    STATE.with(|cell| {
        if let Some(s) = cell.borrow_mut().as_mut() {
            s.hint_source = Some(source);
        }
    });
}

/// The "Show me" button's click handler — the only place `launch` is ever
/// fired, and only because the user explicitly asked for it.
fn trigger_demo() {
    let launch = STATE.with(|cell| {
        let borrow = cell.borrow();
        borrow.as_ref().and_then(|s| s.steps[s.index].launch.clone())
    });
    if let Some(cmd) = launch {
        exec::run(&cmd);
    }
}

/// Marks the tour done without finishing every step — same end state as
/// completing it normally (won't auto-relaunch on next login), just
/// reachable at any point via the callout's low-emphasis "Skip tour".
fn skip_tour() {
    finish();
}

/// Builds the mask + callout for `step` and installs them, tearing down
/// whatever was there before — called once immediately in `enter_step`
/// (usually with no target yet) and again by `schedule_poll` once a
/// `launch`ed app's window/layer actually appears.
fn render_step(index: usize, step: &Step, binds: &[Keybind], is_last: bool, force_no_target: bool) {
    let (display, total, confirmed, hint_visible) = STATE.with(|cell| {
        let borrow = cell.borrow();
        let s = borrow.as_ref().unwrap();
        (s.display.clone(), s.steps.len(), s.confirmed, s.hint_visible)
    });

    // `force_no_target` is set by `on_tour_event`'s confirmation path: a
    // fresh `hyprctl -j layers` query right after a real `closelayer` event
    // has been observed to still report the surface as present — Hyprland's
    // event and its own layer-list snapshot aren't tightly ordered — which
    // re-triggers the near-fullscreen suppression below and drops the
    // confirmed callout right back into invisible limbo. `on_tour_event`
    // already has authoritative confirmation the target is gone from the
    // real compositor event itself, so that path skips re-querying live
    // state entirely rather than trusting a `hyprctl` snapshot that may not
    // have caught up yet.
    let resolved = if force_no_target { None } else { target::resolve(&step.target(), &display) };
    let monitor = resolved.as_ref().map(|r| r.monitor.clone()).or_else(|| target::focused_monitor(&display));
    let Some(monitor) = monitor else { return };
    let geo = monitor.geometry();
    let monitor_size = (geo.width(), geo.height());
    let rect = resolved.map(|r| r.rect);

    // breadbox/breadclip/breadsearch anchor all 4 edges as fullscreen click-
    // catchers — their visible launcher panel is small, but the surface
    // hyprctl reports covers the whole monitor, and that surface owns all
    // pointer input while it's open. A callout drawn now would be buried
    // underneath it with no way to click Back/Next. The instructional
    // callout already showed (with working buttons) before the app opened;
    // once it's confirmed up, get out of the way entirely until it closes.
    // A step whose target resolves this way should key `success_event` off
    // the surface *closing* (e.g. `layer-closed:breadbox`), not opening —
    // by the time that fires the surface is already gone, so `confirmed`'s
    // re-render lands on a normal (non-fullscreen) target and is clickable.
    if let Some(r) = rect {
        if is_near_fullscreen(r, monitor_size) {
            // Just the visuals, not `teardown_visuals()` — this can run from
            // inside `schedule_poll`'s own still-executing closure, and that
            // function also cancels `poll_source` via `SourceId::remove()`,
            // which must never be called on a source's own in-progress
            // dispatch (see the identical self-removal note in `enter_step`).
            clear_visuals();
            return;
        }
    }

    let masks = match rect {
        Some(r) => mask::around(&monitor, r, monitor_size),
        None => mask::full_screen(&monitor),
    };

    // Only worth offering "Show me" while the target genuinely isn't up yet
    // — once `rect` resolves, the user can plainly see it themselves.
    let show_demo = hint_visible && !confirmed && rect.is_none() && step.launch.is_some();

    let callout = callout::build(&monitor, rect, monitor_size, step, binds, index, total, is_last, confirmed, show_demo);
    callout.back_btn.connect_clicked(|_| go_back());
    callout.next_btn.connect_clicked(|_| go_next());
    callout.skip_btn.connect_clicked(|_| skip_tour());
    if let Some(btn) = &callout.demo_btn {
        btn.connect_clicked(|_| trigger_demo());
    }

    clear_visuals();
    STATE.with(|cell| {
        if let Some(s) = cell.borrow_mut().as_mut() {
            s.masks = masks;
            s.callout = Some(callout);
        }
    });
}

/// breadbox/breadclip/breadsearch anchor all 4 edges as fullscreen click-
/// catchers, so their reported surface geometry is the whole monitor even
/// though the visible launcher panel is small.
fn is_near_fullscreen(rect: target::Rect, monitor_size: (i32, i32)) -> bool {
    let (mon_w, mon_h) = monitor_size;
    rect.w * 10 >= mon_w * 9 && rect.h * 10 >= mon_h * 9
}

fn apply_rebind_for_step(step: &Step, binds: &[Keybind]) {
    let (Some(combo), Some(event_id)) = (&step.rebind_combo, &step.success_event) else { return };
    let Some(kb) = keybinds::find(binds, combo) else { return };
    let Some(raw) = &kb.raw else { return };
    let Some(command) = raw.strip_prefix("exec,") else { return };
    let mods_key = keybinds::to_hypr_mods_key(&kb.combo);
    let original_bind_value = format!("{mods_key},{raw}");
    let escaped = command.replace('\'', "'\\''");
    let chained_bind_value = format!("{mods_key},exec,sh -c '{escaped} ; breadhelp --tour-event {event_id}'");
    let mut cfg = State::load();
    cfg.set_pending_rebind(&mods_key, &original_bind_value);
    hyprland::rebind_temp(&chained_bind_value);
}

fn revert_pending_rebind() {
    let mut cfg = State::load();
    if let Some((_key, original)) = cfg.pending_rebind() {
        hyprland::rebind_temp(&original);
    }
    cfg.clear_pending_rebind();
}

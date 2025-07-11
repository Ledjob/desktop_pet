use std::fs;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use once_cell::sync::Lazy;

use crate::utils;

static REMINDERS: Lazy<Vec<String>> = Lazy::new(|| {
    fs::read_to_string("reminders.txt")
        .map(|content| content.lines().filter(|l| !l.trim().is_empty()).map(|l| l.to_string()).collect())
        .unwrap_or_else(|_| vec![])
});

static STATE: Lazy<Mutex<SchedulerState>> = Lazy::new(|| Mutex::new(SchedulerState::new()));

struct SchedulerState {
    last_reminder_time: Instant,
    pending_message: Option<String>,
    reminder_index: usize,
}

impl SchedulerState {
    fn new() -> Self {
        SchedulerState {
            last_reminder_time: Instant::now(),
            pending_message: None,
            reminder_index: 0,
        }
    }
}

pub fn tick() {
    let mut state = STATE.lock().unwrap();
    if state.pending_message.is_none() && !REMINDERS.is_empty() {
        if state.last_reminder_time.elapsed() >= Duration::from_secs( utils::REMINDER_INTERVAL) {
            // Pick next reminder (round robin)
            let msg = REMINDERS[state.reminder_index % REMINDERS.len()].clone();
            state.pending_message = Some(msg);
            state.reminder_index = (state.reminder_index + 1) % REMINDERS.len();
            state.last_reminder_time = Instant::now();
        }
    }
}

pub fn has_message_ready() -> bool {
    let state = STATE.lock().unwrap();
    state.pending_message.is_some()
}

pub fn get_message() -> Option<String> {
    let mut state = STATE.lock().unwrap();
    state.pending_message.take()
} 
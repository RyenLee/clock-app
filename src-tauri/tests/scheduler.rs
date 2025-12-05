use std::time::{Duration, Instant};

use clock_app_lib::{compute_target_from_mode, ScheduleMode};

#[test]
fn countdown_computes_correct_target() {
    let start = Instant::now();
    let target = compute_target_from_mode(ScheduleMode::CountdownMinutes(2));
    let delta = target.duration_since(start);
    assert!(delta >= Duration::from_secs(120));
    assert!(delta < Duration::from_secs(125));
}

#[test]
fn specific_time_in_future_or_tomorrow() {
    use chrono::Timelike;
    let now = chrono::Local::now();
    let hour = now.hour() as u32;
    let minute = (now.minute() + 1) % 60; // ensure future within next hour
    let target = compute_target_from_mode(ScheduleMode::SpecificTime { hour, minute });
    assert!(target > Instant::now());
}


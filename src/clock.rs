use std::time::{Duration, SystemTime};
pub struct Clock {
    pub tick: u64, // ticks since clock creation
    pub time: f32, // seconds since clock creation, updated every tick
    pub tps: f32, // ticks per second, updated every tps_update_interval to take a substantial average (0.5s by default)

    pub tick_time: f32, // time between this tick and the last
    time_at_last_tick: f32,

    tick_at_last_tps_update: u64,
    time_at_last_tps_update: f32,
    tps_update_interval: f32,

    start_time: SystemTime,
}
impl Clock {
    pub fn new() -> Self {
        Self {
            tick: 0u64,
            time: 0f32,
            tps: 0f32,

            tick_time: 0f32,
            time_at_last_tick: 0f32,

            time_at_last_tps_update: 0f32,
            tick_at_last_tps_update: 0u64,
            tps_update_interval: 0.5f32,

            start_time: SystemTime::now(),
        }
    }

    fn duration_to_s(dur: Duration) -> f32 {
        (dur.as_secs() as f32) + (dur.subsec_nanos() as f32) / 1000_000_000f32
    }

    pub fn tick(&mut self) {
        self.time = Clock::duration_to_s(self.start_time.elapsed().unwrap());
        self.tick += 1;

        self.tick_time = self.time - self.time_at_last_tick;
        self.time_at_last_tick = self.time;

        let time_since_last_tps_update = self.time - self.time_at_last_tps_update;
        if time_since_last_tps_update >= self.tps_update_interval {
            self.tps = ((self.tick - self.tick_at_last_tps_update) as f32) / time_since_last_tps_update;
            self.tick_at_last_tps_update = self.tick;
            self.time_at_last_tps_update = self.time;
        }
    }
}
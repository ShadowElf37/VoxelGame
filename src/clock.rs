use std::time::{Duration, SystemTime};
pub struct Clock {
    pub frame: u64,
    pub time: f32,
    pub fps: f32,

    pub frame_time: f32,
    time_at_last_frame: f32,

    frame_at_last_fps_update: u64,
    time_at_last_fps_update: f32,

    start_time: SystemTime,
}
impl Clock {
    pub fn new() -> Self {
        Self {
            frame: 0u64,
            time: 0f32,
            fps: 0f32,

            frame_time: 0f32,
            time_at_last_frame: 0f32,

            time_at_last_fps_update: 0f32,
            frame_at_last_fps_update: 0u64,

            start_time: SystemTime::now(),
        }
    }

    fn duration_to_s(dur: Duration) -> f32 {
        (dur.as_secs() as f32) + (dur.subsec_millis() as f32) / 1000f32
    }

    pub fn tick(&mut self) {
        self.time = Clock::duration_to_s(self.start_time.elapsed().unwrap());
        self.frame += 1;

        self.frame_time = self.time - self.time_at_last_frame;
        self.time_at_last_frame = self.time;

        let time_since_last_fps_update = self.time - self.time_at_last_fps_update;
        if time_since_last_fps_update >= 0.5f32 {
            self.fps = ((self.frame - self.frame_at_last_fps_update) as f32) / time_since_last_fps_update;
            self.frame_at_last_fps_update = self.frame;
            self.time_at_last_fps_update = self.time;
        }
    }
}
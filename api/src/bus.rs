use std::ops::{Index, IndexMut};

pub struct Bus {
    data: Vec<f32>,
    channels: usize,
    frames: usize,
    silence: Vec<bool>,
}

impl Bus {
    pub fn new(channels: usize, frames: usize) -> Self {
        Self {
            data: vec![0.0; channels * frames],
            channels,
            frames,
            silence: vec![false; channels],
        }
    }

    pub fn channels(&self) -> usize {
        self.channels
    }

    pub fn frames(&self) -> usize {
        self.frames
    }

    pub fn resize(&mut self, channels: usize, frames: usize) {
        self.data = vec![0.0; channels * frames];
        self.channels = channels;
        self.frames = frames;
        self.silence = vec![false; channels];
    }

    pub fn get(&self, channel: usize, frame: usize) -> Option<&f32> {
        if channel >= self.channels || frame >= self.frames {
            return None;
        }
        self.data.get(channel * self.frames + frame)
    }

    pub fn get_mut(&mut self, channel: usize, frame: usize) -> Option<&mut f32> {
        if channel >= self.channels || frame >= self.frames {
            return None;
        }
        self.data.get_mut(channel * self.frames + frame)
    }

    pub fn set(&mut self, channel: usize, frame: usize, value: f32) {
        self.data[channel * self.frames + frame] = value;
    }

    pub fn channel(&self, channel: usize) -> &[f32] {
        let start = channel * self.frames;
        &self.data[start..start + self.frames]
    }

    pub fn channel_mut(&mut self, channel: usize) -> &mut [f32] {
        let start = channel * self.frames;
        &mut self.data[start..start + self.frames]
    }

    pub fn as_slice(&self) -> &[f32] {
        &self.data
    }

    pub fn as_mut_slice(&mut self) -> &mut [f32] {
        &mut self.data
    }

    pub fn is_silent(&self, channel: usize) -> bool {
        self.silence[channel]
    }

    pub fn set_silent(&mut self, channel: usize, silent: bool) {
        self.silence[channel] = silent;
    }

    pub fn silence_flags(&self) -> &[bool] {
        &self.silence
    }

    pub fn silence_flags_mut(&mut self) -> &mut [bool] {
        &mut self.silence
    }
}

impl Index<(usize, usize)> for Bus {
    type Output = f32;

    fn index(&self, (channel, frame): (usize, usize)) -> &f32 {
        &self.data[channel * self.frames + frame]
    }
}

impl IndexMut<(usize, usize)> for Bus {
    fn index_mut(&mut self, (channel, frame): (usize, usize)) -> &mut f32 {
        &mut self.data[channel * self.frames + frame]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_is_zero_filled_and_not_silent() {
        let b = Bus::new(2, 3);
        assert_eq!(b.channels(), 2);
        assert_eq!(b.frames(), 3);
        assert_eq!(b.as_slice(), &[0.0; 6]);
        assert_eq!(b.silence_flags(), &[false; 2]);
    }

    #[test]
    fn index_get_and_set() {
        let mut b = Bus::new(2, 3);
        b[(1, 2)] = 4.2;
        assert_eq!(b[(1, 2)], 4.2);
        assert_eq!(b.get(1, 2), Some(&4.2));
        assert_eq!(b.get(2, 0), None);
    }

    #[test]
    fn set_writes_value() {
        let mut b = Bus::new(2, 3);
        b.set(1, 2, 7.5);
        assert_eq!(b.get(1, 2), Some(&7.5));
    }

    #[test]
    fn channel_access() {
        let mut b = Bus::new(2, 3);
        b.channel_mut(0).copy_from_slice(&[1.0, 2.0, 3.0]);
        assert_eq!(b.channel(0), &[1.0, 2.0, 3.0]);
        assert_eq!(b.channel(1), &[0.0, 0.0, 0.0]);
    }

    #[test]
    fn silence_flags_are_per_channel() {
        let mut b = Bus::new(3, 4);
        b.set_silent(1, true);
        assert!(!b.is_silent(0));
        assert!(b.is_silent(1));
        assert!(!b.is_silent(2));
    }

    #[test]
    fn resize_reallocates_zeroed_and_not_silent() {
        let mut b = Bus::new(2, 2);
        b[(0, 0)] = 9.0;
        b.set_silent(0, true);
        b.resize(3, 4);
        assert_eq!(b.channels(), 3);
        assert_eq!(b.frames(), 4);
        assert_eq!(b.as_slice(), &[0.0; 12]);
        assert_eq!(b.silence_flags(), &[false; 3]);
    }

    #[test]
    #[should_panic]
    fn index_out_of_bounds_panics() {
        let b = Bus::new(2, 2);
        let _ = b[(2, 0)];
    }
}

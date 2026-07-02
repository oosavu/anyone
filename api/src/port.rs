use std::ops::{Index, IndexMut};

pub struct Port {
    data: Vec<f32>,
    silence: Vec<bool>,
}

impl Port {
    pub fn new(len: usize) -> Self {
        Self {
            data: vec![0.0; len],
            silence: vec![false; len],
        }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn resize(&mut self, len: usize) {
        self.data = vec![0.0; len];
        self.silence = vec![false; len];
    }

    pub fn get(&self, index: usize) -> Option<&f32> {
        self.data.get(index)
    }

    pub fn get_mut(&mut self, index: usize) -> Option<&mut f32> {
        self.data.get_mut(index)
    }

    pub fn set(&mut self, index: usize, value: f32) {
        self.data[index] = value;
    }

    pub fn as_slice(&self) -> &[f32] {
        &self.data
    }

    pub fn as_mut_slice(&mut self) -> &mut [f32] {
        &mut self.data
    }

    pub fn is_silent(&self, index: usize) -> bool {
        self.silence[index]
    }

    pub fn set_silent(&mut self, index: usize, silent: bool) {
        self.silence[index] = silent;
    }

    pub fn silence_flags(&self) -> &[bool] {
        &self.silence
    }

    pub fn silence_flags_mut(&mut self) -> &mut [bool] {
        &mut self.silence
    }
}

impl Index<usize> for Port {
    type Output = f32;

    fn index(&self, index: usize) -> &f32 {
        &self.data[index]
    }
}

impl IndexMut<usize> for Port {
    fn index_mut(&mut self, index: usize) -> &mut f32 {
        &mut self.data[index]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_is_zero_filled_and_not_silent() {
        let p = Port::new(4);
        assert_eq!(p.len(), 4);
        assert_eq!(p.as_slice(), &[0.0; 4]);
        assert_eq!(p.silence_flags(), &[false; 4]);
    }

    #[test]
    fn index_get_and_set() {
        let mut p = Port::new(4);
        p[2] = 4.2;
        assert_eq!(p[2], 4.2);
        assert_eq!(p.get(2), Some(&4.2));
        assert_eq!(p.get(4), None);
    }

    #[test]
    fn set_writes_value() {
        let mut p = Port::new(4);
        p.set(1, 7.5);
        assert_eq!(p.get(1), Some(&7.5));
    }

    #[test]
    fn silence_flags_are_independent_of_data() {
        let mut p = Port::new(3);
        p.set_silent(1, true);
        assert!(!p.is_silent(0));
        assert!(p.is_silent(1));
        assert!(!p.is_silent(2));
    }

    #[test]
    fn resize_reallocates_zeroed_and_not_silent() {
        let mut p = Port::new(2);
        p[0] = 9.0;
        p.set_silent(0, true);
        p.resize(5);
        assert_eq!(p.len(), 5);
        assert_eq!(p.as_slice(), &[0.0; 5]);
        assert_eq!(p.silence_flags(), &[false; 5]);
    }

    #[test]
    #[should_panic]
    fn index_out_of_bounds_panics() {
        let p = Port::new(2);
        let _ = p[2];
    }
}

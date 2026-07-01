use std::ops::{Index, IndexMut};

pub struct Cable {
    data: Vec<f32>,
    rows: usize,
    cols: usize,
}

impl Cable {
    pub fn new(rows: usize, cols: usize) -> Self {
        Self {
            data: vec![0.0; rows * cols],
            rows,
            cols,
        }
    }

    pub fn rows(&self) -> usize {
        self.rows
    }

    pub fn cols(&self) -> usize {
        self.cols
    }

    pub fn resize(&mut self, rows: usize, cols: usize) {
        self.data = vec![0.0; rows * cols];
        self.rows = rows;
        self.cols = cols;
    }

    pub fn get(&self, row: usize, col: usize) -> Option<&f32> {
        if row >= self.rows || col >= self.cols {
            return None;
        }
        self.data.get(row * self.cols + col)
    }

    pub fn get_mut(&mut self, row: usize, col: usize) -> Option<&mut f32> {
        if row >= self.rows || col >= self.cols {
            return None;
        }
        self.data.get_mut(row * self.cols + col)
    }

    pub fn set(&mut self, row: usize, col: usize, value: f32) {
        self.data[row * self.cols + col] = value;
    }

    pub fn row(&self, row: usize) -> &[f32] {
        let start = row * self.cols;
        &self.data[start..start + self.cols]
    }

    pub fn row_mut(&mut self, row: usize) -> &mut [f32] {
        let start = row * self.cols;
        &mut self.data[start..start + self.cols]
    }

    pub fn as_slice(&self) -> &[f32] {
        &self.data
    }

    pub fn as_mut_slice(&mut self) -> &mut [f32] {
        &mut self.data
    }
}

impl Index<(usize, usize)> for Cable {
    type Output = f32;

    fn index(&self, (row, col): (usize, usize)) -> &f32 {
        &self.data[row * self.cols + col]
    }
}

impl IndexMut<(usize, usize)> for Cable {
    fn index_mut(&mut self, (row, col): (usize, usize)) -> &mut f32 {
        &mut self.data[row * self.cols + col]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_is_zero_filled() {
        let m = Cable::new(2, 3);
        assert_eq!(m.rows(), 2);
        assert_eq!(m.cols(), 3);
        assert_eq!(m.as_slice(), &[0.0; 6]);
    }

    #[test]
    fn index_get_and_set() {
        let mut m = Cable::new(2, 3);
        m[(1, 2)] = 4.2;
        assert_eq!(m[(1, 2)], 4.2);
        assert_eq!(m.get(1, 2), Some(&4.2));
        assert_eq!(m.get(2, 0), None);
    }

    #[test]
    fn set_writes_value() {
        let mut m = Cable::new(2, 3);
        m.set(1, 2, 7.5);
        assert_eq!(m.get(1, 2), Some(&7.5));
    }

    #[test]
    fn row_access() {
        let mut m = Cable::new(2, 3);
        m.row_mut(0).copy_from_slice(&[1.0, 2.0, 3.0]);
        assert_eq!(m.row(0), &[1.0, 2.0, 3.0]);
        assert_eq!(m.row(1), &[0.0, 0.0, 0.0]);
    }

    #[test]
    fn resize_reallocates_zeroed() {
        let mut m = Cable::new(2, 2);
        m[(0, 0)] = 9.0;
        m.resize(3, 4);
        assert_eq!(m.rows(), 3);
        assert_eq!(m.cols(), 4);
        assert_eq!(m.as_slice(), &[0.0; 12]);
    }

    #[test]
    #[should_panic]
    fn index_out_of_bounds_panics() {
        let m = Cable::new(2, 2);
        let _ = m[(2, 0)];
    }
}

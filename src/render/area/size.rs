//! Terminal-cell dimensions and even grid partitioning.

use std::ops::Add;

/// Width and height in terminal cells; the default is `(0, 0)`.
///
/// Addition combines dimensions with ordinary `u16` arithmetic, so callers
/// must ensure that both sums fit in `u16`.
#[derive(Debug, Clone, Hash, Copy, Default, PartialEq, Eq)]
pub struct Size {
    /// Number of terminal columns.
    pub width: u16,
    /// Number of terminal rows.
    pub height: u16,
}

impl Size {
    /// Splits width into `m` columns and height into `n` rows.
    ///
    /// Returns `(column_widths, row_heights)`, assigning remainder cells to the
    /// first partitions. If either count is zero, both vectors are empty.
    /// Counts larger than a dimension produce some zero-sized partitions.
    ///
    /// ```
    /// use rsille::render::area::Size;
    ///
    /// let size = Size::from((10, 11));
    /// assert_eq!(size.split_mxn(3, 3), (vec![4, 3, 3], vec![4, 4, 3]));
    /// ```
    pub fn split_mxn(&self, m: u16, n: u16) -> (Vec<u16>, Vec<u16>) {
        if m == 0 || n == 0 {
            return (Vec::new(), Vec::new());
        }

        // Split width into m parts
        let mut width_parts = Vec::with_capacity(m as usize);
        let base_width = self.width / m;
        let remainder_width = self.width % m;

        for i in 0..m {
            let part_width = if i < remainder_width {
                base_width + 1
            } else {
                base_width
            };
            width_parts.push(part_width);
        }

        // Split height into n parts
        let mut height_parts = Vec::with_capacity(n as usize);
        let base_height = self.height / n;
        let remainder_height = self.height % n;

        for i in 0..n {
            let part_height = if i < remainder_height {
                base_height + 1
            } else {
                base_height
            };
            height_parts.push(part_height);
        }

        (width_parts, height_parts)
    }

    /// Returns whether either dimension is smaller than its counterpart.
    pub fn less_any(&self, other: Size) -> bool {
        self.width < other.width || self.height < other.height
    }
}

impl From<(u16, u16)> for Size {
    fn from((width, height): (u16, u16)) -> Self {
        Self { width, height }
    }
}

impl From<Size> for (u16, u16) {
    fn from(size: Size) -> Self {
        (size.width, size.height)
    }
}

impl Add<Size> for Size {
    type Output = Size;
    fn add(self, rhs: Size) -> Self::Output {
        (self.width + rhs.width, self.height + rhs.height).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_mxn_basic() {
        let size = Size {
            width: 11,
            height: 10,
        };
        let (width_parts, height_parts) = size.split_mxn(2, 2);

        assert_eq!(width_parts, vec![6, 5]);
        assert_eq!(height_parts, vec![5, 5]);
    }

    #[test]
    fn test_split_mxn_3x3() {
        let size = Size {
            width: 10,
            height: 11,
        };
        let (width_parts, height_parts) = size.split_mxn(3, 3);

        assert_eq!(width_parts, vec![4, 3, 3]);
        assert_eq!(height_parts, vec![4, 4, 3]);
    }

    #[test]
    fn test_split_mxn_even_division() {
        let size = Size {
            width: 12,
            height: 8,
        };
        let (width_parts, height_parts) = size.split_mxn(3, 2);

        assert_eq!(width_parts, vec![4, 4, 4]);
        assert_eq!(height_parts, vec![4, 4]);
    }

    #[test]
    fn test_split_mxn_remainder_distribution() {
        let size = Size {
            width: 7,
            height: 5,
        };
        let (width_parts, height_parts) = size.split_mxn(3, 2);

        assert_eq!(width_parts, vec![3, 2, 2]);
        assert_eq!(height_parts, vec![3, 2]);
    }

    #[test]
    fn test_split_mxn_single_row_col() {
        let size = Size {
            width: 10,
            height: 5,
        };
        let (width_parts, height_parts) = size.split_mxn(1, 1);

        assert_eq!(width_parts, vec![10]);
        assert_eq!(height_parts, vec![5]);
    }

    #[test]
    fn test_split_mxn_zero_dimensions() {
        let size = Size {
            width: 10,
            height: 5,
        };
        let (width_parts, height_parts) = size.split_mxn(0, 2);

        assert!(width_parts.is_empty());
        assert!(height_parts.is_empty());

        let (width_parts, height_parts) = size.split_mxn(2, 0);
        assert!(width_parts.is_empty());
        assert!(height_parts.is_empty());
    }

    #[test]
    fn test_split_mxn_sum_equals_original() {
        let size = Size {
            width: 15,
            height: 12,
        };
        let (width_parts, height_parts) = size.split_mxn(4, 3);

        assert_eq!(width_parts.iter().sum::<u16>(), size.width);
        assert_eq!(height_parts.iter().sum::<u16>(), size.height);
    }

    #[test]
    fn test_split_mxn_max_difference_one() {
        let size = Size {
            width: 20,
            height: 15,
        };
        let (width_parts, height_parts) = size.split_mxn(6, 4);

        let max_width = width_parts.iter().max().unwrap();
        let min_width = width_parts.iter().min().unwrap();
        assert!(max_width - min_width <= 1);

        let max_height = height_parts.iter().max().unwrap();
        let min_height = height_parts.iter().min().unwrap();
        assert!(max_height - min_height <= 1);
    }
}

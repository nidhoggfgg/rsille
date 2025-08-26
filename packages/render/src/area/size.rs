use std::cmp::Ordering;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Copy, Default)]
pub struct Size {
    pub width: u16,
    pub height: u16,
}

impl Size {
    /// split the size into m * n grid
    /// for example, if the size is (11, 10) and n = 2, m = 2, then the result is:
    /// ([6, 5], [5, 5])
    /// size = (10, 11), n = 3, m = 3, then the result is: ([4, 3, 3], [4, 4, 3])
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

impl PartialEq<(u16, u16)> for Size {
    fn eq(&self, other: &(u16, u16)) -> bool {
        self.width == other.0 && self.height == other.1
    }
}

impl PartialEq<Size> for (u16, u16) {
    fn eq(&self, other: &Size) -> bool {
        self.0 == other.width && self.1 == other.height
    }
}

impl PartialOrd<(u16, u16)> for Size {
    fn partial_cmp(&self, other: &(u16, u16)) -> Option<Ordering> {
        Some(self.width.cmp(&other.0).then(self.height.cmp(&other.1)))
    }
}

impl PartialOrd<Size> for (u16, u16) {
    fn partial_cmp(&self, other: &Size) -> Option<Ordering> {
        Some(self.0.cmp(&other.width).then(self.1.cmp(&other.height)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_partial_eq() {
        assert_eq!(
            Size {
                width: 10,
                height: 10
            },
            (10, 10)
        );
        assert_eq!(
            (10, 10),
            Size {
                width: 10,
                height: 10
            }
        );
    }

    #[test]
    fn test_partial_ord() {
        assert!(
            Size {
                width: 10,
                height: 10
            } < (11, 10)
        );
        assert!(
            Size {
                width: 10,
                height: 10
            } < (10, 11)
        );
        assert!(
            Size {
                width: 10,
                height: 10
            } > (9, 10)
        );
        assert!(
            Size {
                width: 10,
                height: 10
            } > (10, 9)
        );
    }

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

        assert_eq!(width_parts, vec![]);
        assert_eq!(height_parts, vec![]);

        let (width_parts, height_parts) = size.split_mxn(2, 0);
        assert_eq!(width_parts, vec![]);
        assert_eq!(height_parts, vec![]);
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

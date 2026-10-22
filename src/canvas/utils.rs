#[inline]
pub fn round<T>(v: T) -> i32
where
    T: Into<f64>,
{
    v.into().round() as i32
}

#[inline]
pub fn round_f64(v: f64) -> i32 {
    v.round() as i32
}

/// return (col, row)
pub fn get_pos<T>(x: T, y: T) -> (i32, i32)
where
    T: Into<f64>,
{
    let (x, y) = (round(x), round(y));
    let row = get_row(y);
    let col = get_col(x);
    (col, row)
}

pub fn get_row(y: i32) -> i32 {
    if y < 0 {
        (y + 1) / 4 - 1
    } else {
        y / 4
    }
}

pub fn get_col(x: i32) -> i32 {
    if x < 0 {
        (x - 1) / 2
    } else {
        x / 2
    }
}

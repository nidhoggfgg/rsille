pub fn round<T>(v: T) -> i32
where
    T: Into<f64>,
{
    v.into().round() as i32
}

pub fn get_pos<T>(x: T, y: T) -> (i32, i32)
where
    T: Into<f64>,
{
    let row = get_row(y);
    let col = get_col(x);
    (col, row)
}

pub fn get_row<T>(y: T) -> i32
where
    T: Into<f64>,
{
    let y = round(y);
    if y < 0 {
        (y + 1) / 4 - 1
    } else {
        y / 4
    }
}

pub fn get_col<T>(x: T) -> i32
where
    T: Into<f64>,
{
    let x = round(x);
    if x < 0 {
        (x - 1) / 2
    } else {
        x / 2
    }
}

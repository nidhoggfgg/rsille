pub fn round<T>(v: T) -> i32
where
    T: Into<f64>,
{
    v.into().round() as i32
}

pub fn get_pos<T, N>(x: T, y: N) -> (i32, i32)
where
    T: Into<f64>,
    N: Into<f64>,
{
    let (x, y) = (round(x), round(y));
    let row = if y < 0 { (y + 1) / 4 - 1 } else { y / 4 };
    let col = if x < 0 { (x - 1) / 2 } else { x / 2 };
    (col, row)
}

use rsille::{extra::{math::glm::Vec3, rainbow::{self, ToColor}, Object3D}, Animation};

fn main() {
    let side_len = 100.0;
    let mut anime = Animation::new();
    let object = Object3D::cube(side_len);
    let rainbow = rainbow::rainbow();
    let mut k = 0;
    anime.push(
        object,
        move |obj| {
            let angle = Vec3::new(1.0, 2.0, 3.0);
            let mut v = (k as f64 / 300.0 + (k % 13 - 7) as f64 * 0.05).abs();
            while v > 1.0 {
                v -= 1.0;
            }
            let color = rainbow.at(v);
            obj.sides[k as usize % 12].color = Some(color.to_color());
            let s = obj.sides[k as usize % 12].side;
            obj.sides[k as usize % 12].side = ((s.0 + 1) % 8, (s.1 + 1) % 8);
            obj.rotate(angle);
            k += 1;
            false
        },
        (0.0, 0.0),
    );
    anime.set_maxy(side_len);
    anime.set_minx(-side_len);
    anime.run();
}

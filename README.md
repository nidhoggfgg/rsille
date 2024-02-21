# Rsille 🎨

The rsille is a Rust library for drawing graphics in the terminal.
It leverages Braille code to provide an 8x resolution equivalent to ASCII art, enabling you to create beautiful graphics right in your terminal.
This library offers an API and functionality similar to the turtle module in Python, while also supporting rendering of 3D objects.

<div align="center">
  <img src="https://github.com/nidhoggfgg/rsille/raw/main/imgs/anime.gif" width="320" alt="anime">
  <img src="https://github.com/nidhoggfgg/rsille/raw/main/imgs/objects.gif" width="320" alt="3d object">
  <img src="https://github.com/nidhoggfgg/rsille/raw/main/imgs/lifegame.gif" width="320" alt="life game">
  <img src="https://github.com/nidhoggfgg/rsille/raw/main/imgs/mandelbort.png" width="320" alt="mandelbrot set">
  <img src="https://github.com/nidhoggfgg/rsille/raw/main/imgs/lena.png" width="320" alt="lena">
  <img src="https://github.com/nidhoggfgg/rsille/raw/main/imgs/turtle-multi.png" width="320" alt="turtle">
</div>

## Features

- High-resolution drawing using Braille code.
- Provides an intuitive API similar to the turtle module in Python.
- Supports basic 2D graphics drawing and 3D object rendering.
- Lightweight and easy to integrate into your Rust projects.

## Try it

If you want to see some examples, you can just try it without coding
```
git clone https://github.com/nidhoggfgg/rsille.git
cd rsille
cargo run --example cube
```

You will see a rotating cube, and there is more examples.
You can just use `cargo run --example` to find some examples or see the `examples` dir for more information.

## Installation

To use the rsille in your Rust project, simply add the following dependency to your `Cargo.toml` file:

```toml
[dependencies]
rsille = "2.0.0"
```

## Usage

Here's a simple example demonstrating how to use the rsille to draw a star:

```rust
use rsille::{extra::Turtle, Canvas};
fn main() {
    let mut canvas = Canvas::new();
    let mut t = Turtle::new();
    let mut length = 1.0;
    for _ in 0..150 {
        t.forward(length);
        t.right(10.0);
        length += 0.05;
    }
    canvas.paint(&t, 50.0, 50.0).unwrap();
    println!("{}", canvas.render());
}
```

For more information, you can check [doc](https://docs.rs/rsille/latest/rsille/)

## Examples

You can find more example code and in the `examples` directory, showcasing various features and use cases of the rsille.

## License

This project is licensed under the MIT License. See the LICENSE file for more information.

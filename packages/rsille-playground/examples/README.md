# Launch the playground from a checkout

Run these commands from the repository root in an interactive terminal. To pick a mode from the menu, run `cargo run -p rsille-playground` without arguments. To skip the menu, run either command below.

**Draw:**

```sh
cargo run -p rsille-playground -- draw
```

**Pet** (defaults to DeepSeek if the character is omitted):

```sh
cargo run -p rsille-playground -- pet deepseek
```

To see the available commands without starting a session, run `cargo run -p rsille-playground -- --help`.

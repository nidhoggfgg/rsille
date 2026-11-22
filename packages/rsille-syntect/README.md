# rsille-syntect

Syntax highlighting for rsille's `CodeViewer`, using Syntect's bundled languages and the base16-ocean.dark theme.

```toml
[dependencies]
rsille = "3.3"
rsille-syntect = "0.1"
```

```rust
use rsille::tui::prelude::CodeViewer;
use rsille_syntect::highlight;

let source = "/* a comment\n   spanning two lines */\nfn main() {}";
let viewer = CodeViewer::<()>::from_lines(highlight(source, "rs"));
```

Pass a language name or file extension. Unknown languages use plain text. Highlight once when the source changes. Views that rebuild frequently can store the result in an `Arc<[Line<'static>]>` and pass a clone to `CodeViewer::from_lines`.

# Editor Integration

This crate is designed to be used in real games without forcing editor code into production builds.
Use the editor **only in dev builds** and keep runtime dependencies minimal.

## Recommended plugin setup

**Runtime only (shipping build)**

```rust,ignore
use bevy::prelude::*;
use funkus_dialogue_core::DialoguePlugin;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, DialoguePlugin))
        .run();
}
```

**Runtime + UI (in‑game display)**

```rust,ignore
use bevy::prelude::*;
use funkus_dialogue_core::DialoguePlugin;
use funkus_dialogue_ui::DialogueUIPlugin;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, DialoguePlugin, DialogueUIPlugin))
        .run();
}
```

**Runtime + Editor (dev only)**

```rust,ignore
use bevy::prelude::*;
use funkus_dialogue_core::DialoguePlugin;

#[cfg(feature = "editor")]
use funkus_dialogue_editor::DialogueEditorPlugin;

fn main() {
    let mut app = App::new();
    app.add_plugins((DefaultPlugins, DialoguePlugin));

    #[cfg(feature = "editor")]
    app.add_plugins(DialogueEditorPlugin::with_assets_root("assets"));

    app.run();
}
```

## Cargo feature pattern

Use an optional dependency so the editor only compiles in dev builds:

```toml
[dependencies]
funkus_dialogue_core = { path = "../funkus_dialogue_core" }
funkus_dialogue_ui = { path = "../funkus_dialogue_ui", optional = true }
funkus_dialogue_editor = { path = "../funkus_dialogue_editor", optional = true }

[features]
ui = ["funkus_dialogue_ui"]
editor = ["funkus_dialogue_editor"]
```

Then run with `--features editor` when you want the editor.

## Asset roots (important)

Both the runtime and editor assume the **same asset root**.

- Bevy uses `AssetPlugin::file_path` (defaults to `"assets"` relative to the current working directory).
- The editor uses the same root to list and import files.

If your project uses a custom assets folder, pass it to both:

```rust,ignore
use bevy::asset::AssetPlugin;

App::new()
    .add_plugins(DefaultPlugins.set(AssetPlugin {
        file_path: "assets".to_string(),
        ..default()
    }))
    .add_plugins(DialogueEditorPlugin::with_assets_root("assets"))
    .run();
```

## Portrait workflow

Portraits are stored as **asset paths** in dialogue data.

- Editor import location: `assets/dialogue/portraits/`
- Node data stores: `"dialogue/portraits/your_image.png"`
- The runtime resolves this via `AssetServer`.

## Image format features (editor)

The editor enables image formats through crate features:

- `image_formats_basic`: `png`, `jpeg`, `webp`
- `image_formats_all` (default): all Bevy-supported image formats

If you want to reduce compile time:

```toml
funkus_dialogue_editor = { path = "../funkus_dialogue_editor", default-features = false, features = ["image_formats_basic"] }
```

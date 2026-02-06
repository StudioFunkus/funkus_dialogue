# Editor UX Roadmap (noise_gui Learnings)

This document captures what we can learn from `noise_gui` (a polished `egui-snarl` graph editor)
and how to apply those patterns to `funkus_dialogue` in a way that keeps the core runtime clean,
the asset format stable, and the editor robust for real projects.

## Goals

- Make the dialogue editor feel *production-grade* for large graphs:
  predictable layouts, low UI friction, fast scanning, and good performance.
- Keep `funkus_dialogue_core` runtime/editor separation:
  the runtime should not depend on editor-only state, and games should be able to ignore editor
  metadata safely.
- Preserve forward/backward compatibility of `.dialogue.json` files.

## Non-goals (for this roadmap)

- Replacing Bevy integration with `eframe` (we stay Bevy-native).
- Serializing `egui-snarl::Snarl<T>` directly as the canonical dialogue data model.
- Building a fully-featured "Unity-like" content browser (we want a small, correct editor).

## Summary of What `noise_gui` Gets Right

1. **Layout persistence**: node positions/collapsed state are stable across sessions and file saves.
2. **Canvas-first workflow**: most graph actions are available from the canvas context menu; the
   UI stays uncluttered.
3. **Strong visual language**: a grid background, consistent colors, and pin styling make graphs
   easy to read.
4. **Performance hygiene**: expensive work is done incrementally and/or off-thread, using
   "dirty sets" and versioning to avoid wasted work.

Everything below is a concrete plan to apply these ideas to `funkus_dialogue_editor`.

---

## Workstream A: Persist Editor Layout Metadata

### Problem
Currently, node positions are derived from a grid on load/rebuild. This causes:

- Layout "resets" when reloading a file or making certain changes.
- Large graphs to become hard to manage.
- A "prototype feel" (designers expect layouts to be stable).

### Proposed solution
Add **optional editor metadata** to `DialogueAsset` (not the runtime graph semantics) and make the
editor read/write that metadata on load/save:

- Node position: `NodeId -> (x, y)`
- Node collapsed state: `NodeId -> open/collapsed`
- (Optional later) canvas viewport transform: pan/zoom (only if the API supports it cleanly)

The runtime ignores this metadata.

### Where to implement

Core:
- `crates/funkus_dialogue_core/src/asset/dialogue_asset.rs`
  - Add `editor: Option<DialogueEditorMetadata>`
  - `#[serde(default, skip_serializing_if = "Option::is_none")]`
  - Rustdoc: "Tooling-only metadata. Safe to ignore."
- `crates/funkus_dialogue_core/src/asset/mod.rs`
  - Re-export the metadata type if it's part of the public format contract.

Editor:
- `crates/funkus_dialogue_editor/src/lib.rs`
  - `load_dialogue_from_disk` should return both graph + editor metadata (or construct
    `OpenDialogue` with it).
  - `save_dialogue_to_disk` should write `DialogueAsset { graph, editor: Some(meta), ... }`.
- `crates/funkus_dialogue_editor/src/state.rs`
  - Extend `OpenDialogue` to store `editor_meta` (or store it in `DialogueNodeEditorState`).
- `crates/funkus_dialogue_editor/src/node_editor.rs`
  - When building the snarl from a graph, apply stored positions/collapsed state if present.
  - On save, extract current snarl positions/open flags into metadata.

### Implementation checklist

- [ ] Define `DialogueEditorMetadata` (and per-node layout data) as part of the serialized asset format.
- [ ] Add `editor: Option<DialogueEditorMetadata>` to `DialogueAsset` with `#[serde(default, skip_serializing_if = "Option::is_none")]`.
- [ ] Keep the legacy loader path working (raw `DialogueGraph` JSON should still load; metadata = `None`).
- [ ] Store metadata in the editor session (`OpenDialogue` or `DialogueNodeEditorState`).
- [ ] On load: apply stored node positions + collapsed/open state when constructing snarl nodes (instead of grid).
- [ ] On save: extract snarl node `pos` + `open` and write them back into `DialogueAsset.editor`.
- [ ] Keep metadata in sync as nodes are added/removed (prune removed ids; add defaults for new ids).
- [ ] Decide whether to persist viewport pan/zoom (implement or explicitly defer and document).
- [ ] Add a round-trip serialization test for the optional `editor` field.
- [ ] Update docs to clarify metadata is tooling-only and safe to ignore in runtime.

### What it touches (high confidence)
- Asset format (adds optional field, backward compatible).
- Editor load/save IO code paths.
- Node editor rebuild logic.

### Benefits
- Massive UX improvement: graphs reload exactly as authored.
- Safer editing: reduced accidental layout loss.
- Enables future features: comment boxes, groups, subgraphs, "jump to node", etc.

### Risks / mitigations
- **Schema change**: keep metadata optional + defaulted; runtime ignores it.
- **Two ways to load files** (DialogueAsset vs DialogueGraph):
  - Keep the existing fallback: if a file is a raw `DialogueGraph`, metadata is just `None`.

### Acceptance criteria
- Opening a dialogue, moving nodes, saving, and reloading preserves positions/collapsed state.
- No runtime behavior changes for dialogue playback.

---

## Workstream B: Canvas-First Workflow (Reduce UI Clutter)

### Problem
Repeated controls and "panel UI" clutter reduce canvas space and create competing interaction
patterns (toolbar vs inspector vs canvas). `noise_gui` demonstrates how far you can go with
context menus and a minimal top UI.

### Proposed solution
Lean into the canvas:

- Make the **graph context menu** the primary "Add node" entry point (we already have this).
- Keep the top toolbar for global actions only (Save, Validate, etc.).
- Remove or minimize "Add node" buttons above the canvas.
- Add a lightweight "Command Palette" pattern later (searchable node creation).

### Where to implement

Editor:
- `crates/funkus_dialogue_editor/src/node_editor.rs`
  - Remove/minimize the `ui.horizontal` add-node buttons.
  - Expand `show_graph_menu` into a categorized menu (Text/Choice/Effect/...).
- `crates/funkus_dialogue_editor/src/widgets/toolbar.rs`
  - Ensure global actions are discoverable and not duplicated elsewhere.
- `crates/funkus_dialogue_editor/src/widgets/left_panel.rs`
  - Node list already provides a "navigate" affordance; keep it focused on navigation/search.

### Implementation checklist

- [ ] Remove (or heavily reduce) the top-of-canvas "Add Text/Choice/Effect" buttons.
- [ ] Expand the canvas right-click menu into categorized "Add node" entries (Text / Choice / Effect / ...).
- [ ] Ensure all node creation is possible from the canvas menu (no feature loss).
- [ ] Keep the top toolbar limited to global actions (Save / Save As / Validate / ...).
- [ ] Remove/avoid duplicate actions in the inspector (single place for global actions).
- [ ] Add keyboard shortcuts for common graph actions (Delete, Set Start, Validate) where appropriate.
- [ ] (Optional, later) Add a "command palette" style node creation/search affordance.

### Benefits
- More canvas space for the graph.
- Fewer "two places to do the same thing" problems.
- A clearer mental model: "graph changes happen on the graph".

### Acceptance criteria
- All node creation is possible from the canvas menu.
- Top-of-canvas buttons are removed (or reduced to a single compact affordance).

---

## Workstream C: Strong Visual Language (Readability at Scale)

### Problem
Without a consistent visual system, node graphs get hard to read as they grow. `noise_gui` uses a
grid background, consistent pin shapes/colors, and collapsible nodes to make graphs legible.

### Proposed solution
Adopt a deliberate "graph style":

- Add a subtle background grid (`BackgroundPattern::Grid`).
- Enable collapsible nodes (where it makes sense).
- Make node type distinctions obvious:
  - frame fill/stroke per node type
  - consistent pin colors/shapes
  - clear "Start" affordance
- Improve edge semantics:
  - emphasize choice ordering visually (#1, #2, ... is already present; ensure it's consistent)
  - optionally add a "wire label" style (later)

### Where to implement

Editor:
- `crates/funkus_dialogue_editor/src/node_editor.rs`
  - Extend `SnarlStyle` passed to `SnarlWidget`:
    - background grid
    - collapsible enabled
    - any scale limits / panning tweaks we choose
  - Tune `PinInfo` usage for input/output.
  - Tune `node_frame`, `show_header`, `show_body` for consistent styling.

Docs:
- `docs/integration/editor.md` (optional)
  - Document basic editor interactions (right-click add, collapse, etc.).

### Implementation checklist

- [ ] Enable a background grid on the snarl canvas (`BackgroundPattern::Grid`).
- [ ] Enable collapsible nodes and verify selection/drag/wiring behavior remains correct.
- [ ] Define a single editor "graph theme" (colors, strokes, padding) and apply consistently.
- [ ] Make node types visually distinct (Text / Choice / Effect).
- [ ] Standardize pin visuals (shapes/colors) and ensure they read well at multiple zoom levels.
- [ ] Ensure choice ordering is clear and matches runtime ordering.
- [ ] Add hover help for truncated labels (tooltips on outputs/body summaries).
- [ ] (Optional) Document key graph interactions in `docs/integration/editor.md`.

### Benefits
- Faster comprehension of graphs.
- Reduced wiring mistakes.
- Easier to demo and adopt in real teams.

### Acceptance criteria
- Graph canvas has grid pattern.
- Nodes are visually distinct (Text/Choice/Effect).
- Collapsing nodes works and doesn't break interaction.

---

## Workstream D: Performance Hygiene (Async + Incremental Work)

### Problem
As projects grow, synchronous work in UI frames becomes visible:

- scanning large asset folders
- loading thumbnails / portraits
- regenerating derived editor state

`noise_gui` keeps the UI responsive by doing expensive work off-thread and by tracking what
actually changed.

### Proposed solution
Introduce a small, explicit async pipeline for editor-only tasks:

1. **Asset scanning tasks**
   - Run dialogue/portrait directory scans off-thread.
   - Store results in a "pending scan" state and swap results in when complete.

2. **Thumbnail pipeline (later)**
   - Load/prepare small portrait previews asynchronously (where possible).
   - Cache `TextureId` per path and only refresh when the underlying file changes.

3. **Dirty-set discipline**
   - Prefer "incremental rebuilds" over "rebuild everything".
   - We already do this somewhat; formalize it in the editor state.

### Where to implement

Editor:
- `crates/funkus_dialogue_editor/src/state.rs`
  - `EditorAssetBrowser` / `EditorPortraitBrowser`:
    - add `pending_task: Option<Task<ScanResult>>` (or similar)
    - store scan results + errors
- New editor systems:
  - `crates/funkus_dialogue_editor/src/lib.rs` (or a new module)
    - system that polls tasks and updates resources
    - systems should be frame-budget friendly

### Implementation checklist

- [ ] Add async directory scanning for dialogues (`EditorAssetBrowser`) with a background task + poll system.
- [ ] Add async directory scanning for portraits (`EditorPortraitBrowser`) with a background task + poll system.
- [ ] Track scan status (idle/scanning/failed) and surface it unobtrusively in UI.
- [ ] Avoid per-frame rescans; only rescan on explicit refresh or when marked dirty.
- [ ] Add portrait preview caching rules (TextureId per path; invalidate when file list changes).
- [ ] Ensure texture/handle caches do not grow unbounded (retain only what is referenced).
- [ ] Verify refresh on large asset folders does not hitch noticeably.

### Benefits
- Editor remains smooth with real project asset folders.
- Clear scaling story as teams adopt it.

### Acceptance criteria
- Refreshing assets doesn't hitch noticeably in large folders.
- The editor UI stays responsive while scans are pending.

---

## What We Should *Not* Copy (and Why)

`noise_gui` serializes `Snarl<NoiseNode>` directly as the project format. That's correct for an
app whose core data model *is the snarl graph itself*.

For `funkus_dialogue` we must keep:

- **DialogueGraph** = runtime semantics (nodes, edges, labels, ordering, effects).
- **Editor metadata** = optional tooling state (positions, collapse, etc.).

This separation preserves:
- compatibility (tools can change without breaking games)
- testability (runtime can be tested without egui/snarl)
- extensibility (other editors can exist)

---

## Suggested Implementation Order (Pragmatic)

1. Workstream A (layout persistence): biggest UX win, low risk.
2. Workstream C (grid + collapsible + style tuning): quick visual payoff, no schema risk.
3. Workstream B (canvas-first): remove clutter once the editor feels stable.
4. Workstream D (async scans/thumbnails): do once real projects start stressing performance.

## Validation / Testing Notes

- Add a serialization round-trip test for editor metadata (optional field must be stable).
- Ensure legacy files (no `editor` section) still load.
- Manual QA checklist:
  - move nodes -> save -> reload -> positions preserved
  - collapse nodes -> save -> reload -> state preserved
  - add/remove nodes -> metadata doesn't accumulate garbage

## Global QA checklist (applies to all workstreams)

- [ ] Legacy file without `editor` metadata loads unchanged.
- [ ] Open -> edit layout -> save -> reload: layout preserved.
- [ ] Large graphs remain usable (panning/zooming/selection/wiring stable).
- [ ] Asset refresh doesn't freeze the UI.
- [ ] No runtime behavior changes in `funkus_dialogue_core` playback.

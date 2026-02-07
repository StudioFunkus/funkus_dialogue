# Dialogue Resources (Variables)

Dialogue resources provide a data-driven way to read and mutate game state
from dialogue nodes. They are registered via Bevy reflection, then accessed
by key inside dialogue assets.

## Registering Resources

```rust,ignore
use bevy::prelude::*;
use funkus_dialogue_core::{DialoguePlugin, DialogueResource};

#[derive(Resource, Reflect, DialogueResource)]
#[dialogue(key = "game")]
struct GameState {
    pub gold: i32,
    pub met_npc: bool,
}

fn main() {
    App::new()
        .add_plugins(DialoguePlugin)
        .insert_resource(GameState { gold: 0, met_npc: false })
        .run();
}
```

The registry will expose fields using `<resource_key>.<field_name>` keys
(e.g. `game.gold`, `game.met_npc`).

## Effect Nodes

Effect nodes apply operations to registered fields:

```json
{
  "type": "Effect",
  "effect": {
    "key": "game.gold",
    "op": "add",
    "value": { "type": "int", "value": 50 }
  }
}
```

Supported field kinds:

- `bool`
- `i32`/`i64`/`u32` (treated as int)
- `f32`/`f64` (treated as float)
- `String`
- unit enums (fieldless enums)
- `Vec<T>` / `[T; N]` for supported `T`

List operations:

- `set`: replace the whole list
- `push`: append one item
- `remove`: remove first matching item
- `clear`: remove all items

## Message Nodes

In addition to effects, dialogues can dispatch typed Bevy messages.

```rust,ignore
use bevy::prelude::*;
use funkus_dialogue_core::DialogueMessage;

#[derive(Message, Reflect, FromReflect, DialogueMessage)]
#[dialogue(key = "game.quest_step")]
struct QuestStepMessage {
    quest_id: String,
    step: i32,
}
```

```json
{
  "type": "Message",
  "message": {
    "key": "game.quest_step",
    "params": [
      { "name": "quest_id", "value": { "type": "string", "value": "intro" } },
      { "name": "step", "value": { "type": "int", "value": 2 } }
    ]
  }
}
```

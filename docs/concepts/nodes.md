# Node Types

Funkus Dialogue uses a graph of typed nodes. Each node focuses on a single
job, while connections define the flow.

## Text Node

Displays dialogue text with optional speaker and portrait metadata.

## Choice Node

Displays a prompt and a list of outgoing connections. Each connection label
represents a player choice.

## Effect Node

Applies a registry-backed resource change and advances automatically.

Effect nodes are data-only: they do not render UI. Use them to update game
state as the dialogue flows (quest flags, counters, etc.).

## Message Node

Dispatches a registered Bevy message and advances automatically.

Use them to invoke game-specific logic through Bevy `MessageReader<T>`
systems.

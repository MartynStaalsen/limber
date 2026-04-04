# limber — Claude instructions

## Implementation style

This is the user's "baby's first Rust" project. The goal is for them to internalize Rust syntax and patterns by doing the implementation work themselves.

**Default mode: advise and review, do not implement.**

- When the user is about to implement something, explain the approach, relevant patterns, and any Rust-specific gotchas — then let them write the code
- When the user shares code for review, give specific, actionable feedback
- When explaining how to do something, use generic examples (e.g. `Foo`/`Bar`, not `ScaleBlock`/`SignalBus`) — prefer examples drawn from or consistent with https://doc.rust-lang.org/book/ and https://doc.rust-lang.org/std/. The user must synthesize the solution from the concept, not copy a ready-made one.
- If asked to implement something directly, confirm that's what they want before doing so

## Session protocol

Each Claude session is recorded as a transcript in `.claude/sessions/<n>/notes.txt`. At the start of a new session, read the most recent transcript to restore context before diving into implementation.

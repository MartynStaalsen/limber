# limber — implementation plan

This document captures design decisions reached during early planning. It is a living doc — update it as decisions evolve.

---

## Core goals recap

- Configurable function-block runtime targeting microcontrollers (ESP32-C3, Arduino, STM32)
- Hardware capabilities baked in at compile time; behavior configured at startup/runtime
- Architecture-agnostic core library: testable offline
- Minimal footprint: only compile what the deployment needs

---

## Value types

### Decision: tagged enum, not strings, not raw memory

Strings were considered as a universal wire type (legible, protocol-friendly, architecture-agnostic). Rejected for runtime use: string parsing in the execute loop is expensive and "type safety" via strings is just deferred runtime parsing, not real safety.

A C++ approach of a base `Signal` owning raw bytes with `TypedSignal<T>` accessors was also considered. Rejected because:
- Type mismatch between writer and reader is silent without an explicit type tag
- The shared-memory aliasing trick (downstream input points at upstream output's memory) is directly prevented by Rust's borrow checker
- It's a C++ idiom being forced into Rust

**Preferred approach: `enum Value`**

```rust
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Bool(bool),
    #[cfg(feature = "type-int")]
    Int(i32),
    #[cfg(feature = "type-float")]
    Float(f32),
    // extend here
}
```

Benefits:
- Type tag is intrinsic — mismatches are detectable at wiring time
- Stack-allocated for small types, no heap use in the run loop
- Exhaustive `match` forces explicit handling of every type
- Feature flags control which variants are compiled in, satisfying the lightweight goal

### Closed-world assumption

A given firmware build is a closed world. The deployer selects which `Value` variants (and which `Block` types) are compiled in via `Cargo.toml` features. This makes exhaustive `match` a feature, not a burden — there is no "unknown type" to handle at runtime.

---

## Signals

A signal is a typed value flowing between blocks. Signals are the wires in the block graph.

### Signal handles

Rather than shared memory pointers (C++ style) or raw index access, signals are accessed via typed handle objects: `SignalWriter<T>` and `SignalReader<T>`. The backing index into the `SignalBus` is private to these handles.

```rust
use std::marker::PhantomData;

// NOT Clone, NOT Copy — exactly one writer per signal, enforced by the type system
pub struct SignalWriter<T>(usize, PhantomData<T>);

// Clone — multiple readers are fine
#[derive(Clone)]
pub struct SignalReader<T>(usize, PhantomData<T>);
```

`SignalWriter<T>` being non-`Clone`/non-`Copy` means moving it into a block transfers sole ownership. Passing the same writer to two blocks is a compile error. No two outputs can fight over a signal.

### Type safety via `SignalType` trait

```rust
pub trait SignalType: Into<Value> + TryFrom<Value> {}

impl SignalType for bool {}
impl SignalType for f32 {}
impl SignalType for i32 {}
```

### Static signals

`StaticSignal<T>` is a signal whose writer is dropped at construction — nobody can write to it after that point. Used for config values.

```rust
pub struct StaticSignal<T>(SignalReader<T>);

impl<T: SignalType> StaticSignal<T> {
    pub fn new(bus: &mut SignalBus, value: T) -> Self {
        let (writer, reader) = bus.allocate(value);
        drop(writer); // only write this signal ever receives — writer ceases to exist
        Self(reader)
    }

    pub fn reader(&self) -> SignalReader<T> {
        self.0.clone()
    }
}
```

### Signal bus

The `SignalBus` owns the backing storage and is the only constructor for handles. Each call to `allocate` produces exactly one `(SignalWriter<T>, SignalReader<T>)` pair. It is an implementation detail of `Context` — construction-phase code interacts with it directly, but it is not exposed at runtime.

```rust
pub struct SignalBus {
    values: Vec<Value>,
}

impl SignalBus {
    pub fn allocate<T: SignalType>(&mut self, initial: T) -> (SignalWriter<T>, SignalReader<T>) {
        let idx = self.values.len();
        self.values.push(initial.into());
        (SignalWriter(idx, PhantomData), SignalReader(idx, PhantomData))
    }

    pub fn read<T: SignalType>(&self, r: &SignalReader<T>) -> T {
        T::try_from(self.values[r.0].clone())
            .unwrap_or_else(|_| unreachable!("type guaranteed by SignalReader<T>"))
    }

    pub fn write<T: SignalType>(&mut self, w: &SignalWriter<T>, value: T) {
        self.values[w.0] = value.into();
    }
}
```

The `unreachable!` in `read` is genuinely unreachable: only `SignalWriter<T>` can write to a given index, so the stored `Value` variant is always `T`. The `Value` enum is an implementation detail of the bus — callers interact only with concrete types.

### Type mismatch is a compile error

Wiring a `SignalWriter<f32>` to a block expecting a `SignalReader<i32>` fails at compile time. No runtime type checking is needed in the execute loop.

---

## Blocks

A `Block` is the atomic unit of behavior. It owns three bundles — inputs, config, and outputs — and implements `execute`.

```rust
pub trait Block {
    fn execute(&mut self, bus: &mut SignalBus);
}
```

### Bundles

Each block type defines three bundle structs. All three are uniform collections of signals:

| Bundle | Signal type | Writable after construction? |
|---|---|---|
| Inputs | `SignalReader<T>` | by upstream block or hardware |
| Config | `StaticSignal<T>` | no — writer dropped at construction |
| Outputs | `SignalReader<T>` (public) + `SignalWriter<T>` (private) | by this block only |

### Blocks beget signals

Output signals are allocated inside the block constructor, not externally. The block owns the `SignalWriter<T>` privately and exposes `SignalReader<T>` clones publicly for downstream blocks to wire up.

```rust
pub struct ScaleInputs {
    pub input: SignalReader<f32>,
}

pub struct ScaleConfig {
    pub factor: StaticSignal<f32>,
}

pub struct ScaleOutputs {
    pub scaled: SignalReader<f32>,
}

pub struct ScaleBlock {
    inputs: ScaleInputs,
    config: ScaleConfig,
    writer: SignalWriter<f32>,      // private
    pub outputs: ScaleOutputs,
}

impl ScaleBlock {
    pub fn new(bus: &mut SignalBus, inputs: ScaleInputs, config: ScaleConfig) -> Self {
        let (writer, scaled) = bus.allocate(0.0f32);
        Self {
            inputs,
            config,
            writer,
            outputs: ScaleOutputs { scaled },
        }
    }
}

impl Block for ScaleBlock {
    fn execute(&mut self, bus: &mut SignalBus) {
        let v = bus.read(&self.inputs.input);
        let factor = bus.read(&self.config.factor.reader());
        bus.write(&self.writer, v * factor);
    }
}
```

### Wiring

Construction reads naturally as a graph. Downstream blocks take clones of upstream output readers:

```rust
let foo = ScaleBlock::new(&mut bus,
    ScaleInputs { input: hw_pin.clone() },
    ScaleConfig { factor: StaticSignal::new(&mut bus, 2.0) },
);
let bar = ScaleBlock::new(&mut bus,
    ScaleInputs { input: foo.outputs.scaled.clone() },
    ScaleConfig { factor: StaticSignal::new(&mut bus, 0.5) },
);
```

### Feature flags

Block types are included at compile time via feature flags:

```toml
# Cargo.toml
[features]
block-scale = ["type-float"]
block-pid   = ["type-float"]
block-latch = ["type-bool"]
```

---

## Execution order

### Construction-order guarantee (code-defined graphs)

When a block graph is defined in code, outputs are passed as signal indices that only exist once their source block is constructed. Construction order is therefore a valid topological order, and execute order matches construction order.

### Config-defined graphs

When the block graph is loaded from a config (e.g. received over serial, stored in EEPROM), the config layer owns:
1. Cycle detection
2. Topological sort before construction

This is a known requirement, deferred to the config layer implementation. The runtime makes no guarantees about execution order for graphs not constructed in dependency order.

---

## Input snapshot / execution atomicity

Inputs are frozen for the duration of a single execution cycle. Whether this is implemented as a snapshot copy or a mutex on the input bus is an implementation decision per use case. The run loop always operates on a consistent view of inputs.

---

## Heap allocation policy

**All allocation happens at startup. The run loop never allocates.**

The block graph is fully constructed and validated before execution begins. A `Context` object is constructed from a complete, validated graph. Possessing a `Context` is proof that construction succeeded.

```rust
pub struct Context {
    bus: SignalBus,
    blocks: Vec<Box<dyn Block>>,  // allocated once at construction
}

impl Context {
    pub fn run_cycle(&mut self) {
        for block in self.blocks.iter_mut() {
            block.execute(&mut self.bus);
        }
    }
}
```

If construction fails (type mismatch, cycle in graph, missing signal), the system does not produce a `Context`. Failures are startup-time only. The run loop is allocation-free.

---

## Configuration and persistence

Whether configuration is persisted (EEPROM), received at runtime from an external system, or hardcoded is a **use-case decision**, not a library decision. The library provides:

- A validated, constructable block graph description
- Construction/validation logic
- A runtime `Context`

The application layer decides how to obtain and store the graph description. EEPROM, serial receive, compile-time const — all valid.

EEPROM wear is not a design concern: reconfiguration is sparse and user-initiated.

---

## External interface / control loop clock

Whether the execution cycle is clocked by:
- Incoming external control messages (blocking)
- An independent timer
- Some other mechanism

...is a use-case decision. The library imposes one constraint: **the input bus must not be mutated during an execution cycle** (see Input snapshot above). The application layer is responsible for enforcing this boundary.

---

## Lightweight / feature selection summary

| Concern | Mechanism |
|---|---|
| Value types in binary | `Cargo.toml` features (`type-float`, `type-int`, etc.) |
| Block types in binary | `Cargo.toml` features (`block-pid`, `block-scale`, etc.) |
| Config persistence | Application layer — not the library's concern |
| Control loop clock | Application layer — not the library's concern |

---

## Implementation roadmap

### Phase 1 — execution loop skeleton ✓ (session 0, mostly)
Block out `Context`, `SignalBus`, and the `Block` trait. Implement `run_cycle`. No real blocks yet — just enough structure to execute a graph.

Done: `Context`, `SignalBus` stub, `Block` trait, `run_cycle`, `Value` enum, `SignalType` trait + `map_signal_types!` macro, `SignalReader<T>`, `SignalWriter<T>`.

Remaining: `Block::execute` needs `&mut SignalBus` parameter (currently no args); `SignalBus` needs `values: Vec<Value>` field and `allocate`/`read`/`write`.

### Phase 2 — signals, bundles, and blocks
Implement `SignalWriter<T>`, `SignalReader<T>`, `StaticSignal<T>`, `SignalType` trait, and `SignalBus::allocate`. Define the three-bundle pattern. Implement one or two concrete blocks (e.g. `ScaleBlock`, `AdderBlock`) as a worked example. Scaffold tests for signal wiring and block execution — structure for testability, not full coverage yet.

### Phase 3 — JSON configuration
Implement a config layer that reads a JSON description of a block graph, instantiates blocks with wired signals, and produces a `Context`. Includes config-layer validation (cycle detection, topo sort, type checking where not already compile-time guaranteed).

### Phase 4 — external interface and hardware abstraction (design)
With a working config-driven runtime in hand, design the external control/state interface and the hardware I/O abstraction. Decisions deferred until the core is proven.

### Phase 5 — file/stdin stub implementations
Implement hardware pins as files, control/state interface as stdin/stdout. Use these to run full end-to-end tests across the core without real hardware.

### Phase 6 — architecture-agnostic hardware layer
Plan and implement the HAL abstraction for real deployment targets (ESP32-C3, Arduino, etc.), informed by what the stub implementations revealed.

---

## Open questions

- Exact `Block` trait signature: does `execute` take `&SignalBus` + `&mut SignalBus` separately, or a single `&mut SignalBus`? Separate read/write buses would enforce that blocks don't read their own outputs mid-cycle.
- Hardware abstraction layer (HAL) interface for pin I/O — should hardware pins be allocated from the same `SignalBus` so they are uniform with logical signals?

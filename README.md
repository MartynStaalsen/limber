# limber
Library of Interfaced Modular Blocks for Embedded Remotes

## the story
I have something like 4 failed attempts at this the kernel of inspiration behind this project.
It might have started [here](https://github.com/MartynStaalsen?tab=repositories) when I picked up an old milspec aviation control panel and started wiring it up to be a HID (human interface device) for flight-sim style video games. Initially, I had an audio-jack-chained PCB scheme in mind which I POC'd out in python sim in which the devices in the chain could procedurally discover their place in the network and their offset in the dataframe. Then, I got in way over my head on an ethercat-inspired [serial bus protocol](https://github.com/MartynStaalsen/Sericat) which was (obviously in retrospect) WAY more complex than my actual usecase demanded. So I burned out writing unit tests and dropped it. Soon after, I went on a months long code grind to spin out a library that promised to mary the compile-time determinism of PLC-style function blocks with the an optionally-dynamic config/registry system as part of a novel Ethercat master (for real this time) that I hoped would be an ammenable compromise between an ardently monolith-ofilic controls team and a distribut-ofilic R&D team. I actually finished that one, just in time to leave the company.
In fact, I've actually implement another version of the same core idea for a very different (distinctly not embedded) application as well. IDK what that puts the count up to...

So here I am, with this dead-eyed button box staring me in the face every evening. There's only two ways see this project getting done:
1. In a manic creative rage some sunday afternoon, I lock in and hardcode the interface just to cross the thing off my backlog
2. In a manic creative rage some weekday evening, I hatch a scheme to simultaneously finish this project and solve a valid real-work use case

`limber` is my attempt at option 2

# The use case
Here's the pitch:

You have some application for which a microcontroller would be very convenient. The actual control perforamnce need is minimal (latency is a non-issue), and the hardware, once defined, is basically guaranteed to remain unchanged. BUT, you might want to tweak the control logic a bit after flashing the code and deploying the system. In fact, you might want to give instances of this system to technically competent but not software-savy users. That means any tweaking needs to happen in configuration space, not compilation space (no re-flashing).

# Implementation scheme
I'm drawing on what I've done before here, but I've no doubt it'll change a bit as I go. Let me start with the highest-level convictions, and work down from there:

## Language
I'm gonna do this in Rust. Reason: trust me bro
I've tried this in C++/Arduino before, and it takes a hellish amount of macros and templates. I think Rust will be easier. But even if it isn't that much better, I've got institutional biases to accommodate.

## Value types
The runtime wire type is a tagged enum, not a string:
```rust
pub enum Value {
    Bool(bool),
    Int(i32),
    Float(f32),
    // ...
}
```
Strings are still relevant as a _serialization_ format for config and control interfaces, but are not used as the in-memory signal type. Which variants are compiled in is controlled by feature flags — only pay for what you use.

A given firmware build is a closed world: the deployer selects the type set at flash time. This makes exhaustive `match` a feature, not a burden.


## configurable behavior on static hardware
The aim is to let the user configure inputs and outputs essentially at runtime*, based on "hardcoded" hardware capabilities. I'm aiming specifically at `ESP32-C3` and Arduino for initial applications, which have differences. I should be able to compile in valid hardware expectations to whatever I deploy (i.e. what pins are available and their modes). Then, the system can run a pipeline of logical operations on top of configured inputs and wire the results to configured outputs. Any given pipeline can be validated for compatibility with the hardware (and validated for cycles, missing signals, etc) as well. But that validation can happen at "_configuration_" or "_construction_" time, not at "compile" time.
For a microcontroller, that means evaluated at startup. OR maybe, evaluated at transmission.
> How the active configuration is obtained and persisted is a use-case decision, not a library decision. Options include: stored in EEPROM, received over serial at boot, or hardcoded. The library provides construction and validation — the application layer decides the source.

## Light-weight
Ideally, only compile what the system needs. That might even mean only compiling translators for the a subset of datatypes and/or "function blocks". Regardless, code memory space on microcontrollers is painfully finite.

## tested to death
Perhaps lofty, but I aim for 100% test coverage. And the way I'll do that is for the core library to be architecture agnostic. Iterating on a board is a pain, so do it offline first.

# Implementation, but more specific this time
Ok so how to do this?

Seemingly thus:

## Hardware header
First off, you gotta configure the hardware. I use "configure" here loosely, becase this stuff is static / baked in. You set up:
- a bus of inputs - for each pin:
    - what's its mode (pull up/down, analog/digital)
    - what's its datatype (boolean, int, float etc)
- a bus of outputs (define per pin, see above)
- a control input interface
- a state output interface
- a cycle rate

### Design decisions:
- control loop clock: whether to cycle on incoming external messages or an independent timer is a use-case decision. The library imposes one constraint: the input bus must not be mutated during an execution cycle.
- heterogeneous control/state interfaces: no. Theoretically possible but no practical use case at this time.


## Signals and Blocks
Once the static stuff is defined, you instantiate the behavioral pipeline.

### Signals
A signal is a typed value flowing between blocks. There are three signal kinds, used in three corresponding bundles per block:

| Bundle | Signal type | Writable after construction? |
|---|---|---|
| Inputs | `SignalReader<T>` | by upstream block or hardware |
| Config | `StaticSignal<T>` | no — writer dropped at construction |
| Outputs | `SignalReader<T>` (public) + `SignalWriter<T>` (private) | by this block only |

Signals live in a central `SignalBus`. Access is via typed handle objects that keep the backing index private. `SignalWriter<T>` is not `Clone` or `Copy` — exactly one writer per signal, enforced by the type system. Wiring type mismatches are compile errors.

### `Block`
The Block is the atomic unit of behavior. Each block type defines input, config, and output bundles, and **begets its own output signals** at construction time — the block allocates them internally and exposes readers publicly for downstream blocks to wire up.

```rust
let foo = AdderBlock::new(&mut bus,
    AdderInputs { a: hw_pin1.clone(), b: hw_pin2.clone() },
    AdderConfig { clamp_max: StaticSignal::new(&mut bus, 10.0) },
);
let bar = ScaleBlock::new(&mut bus,
    ScaleInputs { input: foo.outputs.sum.clone() },
    ScaleConfig { factor: StaticSignal::new(&mut bus, 2.0) },
);
```

This is where the math happens — PID, scaling, logic, whatever. Types are guaranteed by the handles; no type-matching is needed in the execute body.

### Registration & Instantiation
This is maybe the trickiest part, and it's implementation is gonna be most susceptible to how building & deployment works. One constraint you might try to leverage on an implementation of such a scheme is to be able to trivially add new Block definitions to the system. You could specialize all this to minimize the boilerplate you need to write a new block, or to let you easily select only a subset (or superset) of blocks to compile.
Whatever. The important part for this application is that there's a way to read in a config file, identify which blocks its describing and how it wants them wired up, and to then construct those blocks with their inputs fed from the hardware and/or control input bus and their outputs feeding either to other blocks or to the output/control bus. And because all the types are so safe and all, this system should be able to ensure the validity of the configuration at or before block construction time.

#### does this mean my precious blocks are allocated on the heap?
Yes — but strictly at startup. The run loop never allocates. A valid `Context` object is proof that construction and validation succeeded; if anything is going to go wrong, it goes wrong at startup. The run loop is allocation-free by design.

## Core loop
Ok so now you've got your blocks instantiated and wired up in between the inputs and outputs. Now for the easy part:

For each block in the graph, you've just got to run it's execute(), then pass the new resulting outputs on to the next thing that cares about them. Repeat this for every block, and you've got a fresh set of outputs to write to your hardware & state publication.
You can do this on a cyclic real time clock, but for my application I'll be blocking/clocking on some external interface.

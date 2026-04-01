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

## oops, all strings
Ye olde string shall be the core data element. Not saying strings will be the only supported datatype, but at the end of the day it's all bytes in memory so let's just start there.
I have other reasons for this, which include:
- architecture agnostic (see testing aspirations)
- legible
- maximum compatibility with array of relevent communication protocols (serial, network, idk what else)
  - I have *not* picked a communication protocol yet. It would be _neat_ if there were multiple supported options, but we'll see


## configurable behavior on static hardware
The aim is to let the user configure inputs and outputs essentially at runtime*, based on "hardcoded" hardware capabilities. I'm aiming specifically at `ESP32-C3` and Arduino for initial applications, which have differences. I should be able to compile in valid hardware expectations to whatever I deploy (i.e. what pins are available and their modes). Then, the system can run a pipeline of logical operations on top of configured inputs and wire the results to configured outputs. Any given pipeline can be validated for compatibility with the hardware (and validated for cycles, missing signals, etc) as well. But that validation can happen at "_configuration_" or "_construction_" time, not at "compile" time.
For a microcontroller, that means evaluated at startup. OR maybe, evaluated at transmission.
> I'm getting into implementation here, but I'm imagining that the currently-active behavior description is stored in EEPROM, and is perhaps writable to the system at runtime in some fashion. If a proposed behavioral description is invalid, you can just fall back to what persisted from last time...

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
- control vs hardware interfaces: do we limit cycle rate by control interface or nah?
    - if simpler, we can limit on the external interface rate. But we (maybe?) want to avoid the system blocking on an external clock....
    - let's call this "synchronous" (blocking) vs "asynchronous" C/S. Synchronous seems simpler for now -- async can come later, perhaps
- do we allow heterogeneous control/state interfaces?
    - probably not. Theoretically possible but I see no practical use case at this time.


## Signals, (Bundles?), and Blocks
Once the static stuff is defined, you instantiate the behavioral pipeline. I'm gonna describe this in very object oriented terms, cognizant that Rust will probably morph this a lot. Anyway:

### `Block<InputBundle, OutputBundle>`
The Block is the atomic unit of behavior. It has inputs, it does stuff with them, and that stuff results in output values.
Baked into the block's definition (in C++ I do this with templating) is a contract about what it's inputs and outputs will be.
I've called these "bundles" in the past -- basically a tuple of named types. The specific implementation of this impacts how the configuration works, but the core idea is to be able to "wire up" the outputs of one block as inputs to another block. And because of the templating, you get compile-time type safety even if the bundle's constituent elements are defined at runtime:

### `super::execute()`
> Yeah my syntax is wrong, deal with it
Basically, you make all blocks inherit from an abstract base class that forces them to define an execute/update function.
This is where the actual math happens, like running a pid or scaling a value or whatever. And yes, those applications imply the existence of block configuration. We've got some options there...

### `SomeBlock::InputBundle = SomeBundle<float: some_float, bool some_bool>`
This is the next fancy bit: the bundle is also templated. It's actually called something else in C++ which I've forgotten, but the core idea here is to be able to define the block's input and output interfaces as a group of named and type attributes. This guarentees two things:
- *internal type safety*: when used in the block itself, the data stays the same type
- *external type safety*: when you wire one block's output as another's input, the types stay the same
    - _bonus: through pointer shenanigans, downstream block's input can actually use the same data location as the upstream output_

### Registration & Instantiation
This is maybe the trickiest part, and it's implementation is gonna be most susceptible to how building & deployment works. One constraint you might try to leverage on an implementation of such a scheme is to be able to trivially add new Block definitions to the system. You could specialize all this to minimize the boilerplate you need to write a new block, or to let you easily select only a subset (or superset) of blocks to compile.
Whatever. The important part for this application is that there's a way to read in a config file, identify which blocks its describing and how it wants them wired up, and to then construct those blocks with their inputs fed from the hardware and/or control input bus and their outputs feeding either to other blocks or to the output/control bus. And because all the types are so safe and all, this system should be able to ensure the validity of the configuration at or before block construction time.

#### does this mean my precious blocks are allocated on the heap?
yes. If you choose to use a dynamic configuration mechanism. Which I am choosing to do. In principle, you could compile and deploy a compile-time instantiated block graph instead. Go do that yourself.
Personally, I don't care about this because I'm too young to know why I should, and because if anything's going to go wrong it'll go wrong at startup, which is fine by me.

## Core loop
Ok so now you've got your blocks instantiated and wired up in between the inputs and outputs. Now for the easy part:

For each block in the graph, you've just got to run it's execute(), then pass the new resulting outputs on to the next thing that cares about them. Repeat this for every block, and you've got a fresh set of outputs to write to your hardware & state publication.
You can do this on a cyclic real time clock, but for my application I'll be blocking/clocking on some external interface.

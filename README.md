# limber
we'll see where (if) this goes...

## the story
I have something like 4 failed attempts at this the kernel of inspiration behind this project.
It might have started [here](https://github.com/MartynStaalsen?tab=repositories) when I picked up an old milspec aviation control panel and started wiring it up to be a HID (human interface device) for flight-sim style video games. Initially, I had an audio-jack-chained PCB scheme in mind which I POC'd out in python sim in which the devices in the chain could procedurally discover their place in the network and their offset in the dataframe. Then, I got in way over my head on an ethercat-inspired [serial bus protocol](https://github.com/MartynStaalsen/Sericat) which was (obviously in retrospect) WAY more complex than my actual usecase demanded. So I burned out writing unit tests and dropped it. Soon after, I went on a months long code grind to spin out a library that promised to mary the compile-time determinism of PLC-style function blocks with the an optionally-dynamic config/registry system as part of a novel Ethercat master (for real this time) that I hoped would be an ammenable compromise between an ardently monolith-ofilic controls team and a distribut-ofilic R&D team. I actually finished that one, just in time to leave the company.

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

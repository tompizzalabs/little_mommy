# Little Mommy III

**Modular Node-Based synthesizer**
<img width="1040" height="660" alt="image" src="https://github.com/user-attachments/assets/ba8cbceb-7342-493d-94c6-b7bf46bf80ce" />



# WARNING
If you make **Big Daddy V** in response to this synth,
I'll break your ribs and ass bone ( not a threat )

## Feature status

There are no reading and writing presets, at the moment presets we have are just hardcoded
First we want to focus on stabilizing the generation engine, there are still a lot of upgrades and expansions to be made
So making preset stuff for now is not feasible


## Vision

Node based Synthesizer, intuitive to use

( To people who never used nodes they might seem intimidating, but they're actually not! )

Several Oscillator types -  Noise, Analog, Wavetable, FM, Chiptune

Would be amazing to also have sampler and granular stuff 

## Building

Requires [cargo-nice-plug](https://codeberg.org/RustAudio/nice-plug).

```bash
cargo nice-plug bundle little_mommy --release
```

## Credits

Some parts of the synthesis engine were yoinked from [Odin2](https://github.com/TheWaveWarden/odin2) ( GPL Licensed ! )
Which is an absolute banger synth, although i hate it's UI

Sadly this synth seems to be abandoned, as of Novemer 2026 last commit on Odin2 was in October 2025, so Odin2 devs, we take it from where you left off, so your work evolves and is not being forgotten

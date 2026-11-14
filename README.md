# Little Mommy III

**Modular Node-Based Analog Synthesizer**
<img width="1040" height="660" alt="image" src="https://github.com/user-attachments/assets/ba8cbceb-7342-493d-94c6-b7bf46bf80ce" />


# WARNING
If you make **Big Daddy V** in response to this synth,
I'll break your ribs and ass bone ( not a threat )

## Feature status

### Oscillators

Analog - very simple, does their job nicely but they can have a BIG upgrade

FM - Does the job nicely, but needs serious upgrades too

PM - Potential for some crazy, but you gotta tinker..

### Presets
There are no reading and writing presets, at the moment presets we have are just hardcoded, preset part 

First we want to focus on stabilizing the generation enginethere are still a lot of upgrades and expansions to be made

So making preset stuff for now is not feasible, they will break too often


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

This synth was not made from absolute scratch
Most parts of the synthesis engine were yoinked from [Odin2](https://github.com/TheWaveWarden/odin2) ( GPL Licensed ! )
And the it's algorithms were consistently referenced,  
Odin is a banger synth, although i absolutely hate it's UI

Sadly this synth seems to be abandoned, as of November 2026 last commit on Odin2 was in October 2025, so Odin2 devs, we take it from where you left off, your work evolves and is not being forgotten

// lofi-kit.sc — Voices for lo-fi compositions
// Uses instruments and fx chains for richer, layered sound design.

// --- Effect chains ---

fx tape_room = chorus(0.012, 0.004, 0.2) >> reverb(0.6, 0.5, 0.2)
fx mel_space = vibrato(4.5, 12.0) >> delay(0.4, 0.3, 0.25)

// "Next room" effect — lowpass, sample rate reduction, bit crush, compressed
// Makes the melody sound like it's bleeding through a wall on a worn tape
fx next_room = lowpass(150, 0.9, 0.2) >> degrade(0.06) >> chorus(0.03, 0.006, 0.2) reverb(0.4, 0.3, 0.15)

// --- Instruments ---

// Chord instrument — filtered noise-like texture, more atmosphere than tone
// Heavy lowpass turns the saw into a warm rumble that doesn't fight the melody
instrument chord_pad = (0.15 * saw(freq) + 0.12 * triangle(freq * 1.002)) >> lowpass(500, 0.4, 0.7) >> tape_room

// Sub bass — sine fundamental + pink noise undertone for analog warmth
instrument bass = (sine(freq) + 0.3 * triangle(freq * 2) + 0.02 * (pink() >> lowpass(freq * 2, 0.5))) >> lowpass(350, 0.8) >> distort(1.5) >> compress(-18, 3, 0.01, 0.15)

// Melody — soft sine with noise replacing the highs (tape degradation)
// The signal gets lowpassed, pink noise fills in where the signal used to be
// Then the whole thing gets crushed and compressed
instrument mel = 0.08 * sine(freq) + 0.03 * sine(freq * 2) >> mel_space >> next_room

// Pad layer — airy texture for depth in climax sections
instrument pad = (0.12 * triangle(freq) + 0.1 * triangle(freq * 1.003) + 0.03 * (pink() >> lowpass(freq * 2, 0.3))) >> chorus(0.018, 0.006, 0.12) >> reverb(0.8, 0.5, 0.3)

// --- Fixed voices (no pitch variation) ---

// Vinyl texture layers — pink noise for warmer crackle character
voice hiss = 0.025 * pink() >> highpass(2000, 0.5) >> lowpass(8000, 0.5)
voice pop = 0.04 * noise() >> highpass(1000, 0.8) >> lowpass(4000, 0.6) >> decay(80)
voice click = 0.05 * noise() >> highpass(4000, 1.0) >> lowpass(10000, 0.5) >> decay(120)
voice scratch = 0.03 * pink() >> highpass(2000, 0.7) >> lowpass(6000, 0.8) >> decay(30)

// Drums — layered for punch and body
// Kick: sub sine for weight + octave-up sine for punch (fast decay) + noise thud for transient
voice kick = (0.7 * sine(A1) + (0.3 * sine(A2) >> decay(35)) + 0.12 * (noise() >> lowpass(200, 0.5) >> decay(50))) >> decay(10) >> compress(-15, 4, 0.003, 0.08)
// Snare: mid-range thwack (triangle for body) + tight noise crack
voice snare = (0.1 * (triangle(G3) >> decay(30)) + 0.2 * (noise() >> highpass(2500, 1.2) >> lowpass(5000, 0.7) >> decay(20))) >> distort(1.4) >> compress(-16, 4, 0.002, 0.05)
// Ghost snare: quieter, higher, thinner
voice ghost_snare = (0.04 * (sine(E3) >> decay(25)) + 0.06 * (noise() >> highpass(2000, 1.0) >> lowpass(5000, 0.6))) >> decay(18)
// Hat: sharp transient with metallic shimmer — fast decay on the click, longer on the ring
voice hat = (0.08 * (noise() >> highpass(8000, 1.5) >> decay(40)) + 0.05 * (noise() >> highpass(5000, 0.8) >> decay(20))) >> compress(-12, 3, 0.001, 0.04)

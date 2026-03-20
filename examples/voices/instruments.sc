// instruments.sc — Default instrument library
// A collection of instruments with distinct timbres and attack characters.
// Import this file to use any of these in your compositions.

// ============================================================
// KEYS
// ============================================================

// Warm Rhodes-style electric piano — bell-like overtones with vibrato
instrument rhodes = (sine(freq) + 0.3 * sine(freq * 2) + 0.12 * sine(freq * 3) + 0.06 * sine(freq * 7.1)) >> vibrato(4.5, 10.0) >> decay(6)

// Lo-fi keys — detuned pair with chorus for wobbly tape character
instrument lofi_keys = (0.4 * triangle(freq) + 0.3 * triangle(freq * 1.003) + 0.1 * sine(freq * 0.998)) >> lowpass(freq * 3, 0.7) >> chorus(0.015, 0.005, 0.2) >> decay(8)

// Bright tine piano — Wurlitzer-style, metallic attack with fast decay
instrument tine = (0.5 * sine(freq) + 0.4 * sine(freq * 2) + 0.3 * sine(freq * 3) + 0.15 * sine(freq * 4.07)) >> lowpass(freq * 4, 0.5) >> decay(10)

// Muted piano — dark, compressed, lo-fi character
instrument muted_piano = (0.45 * saw(freq) + 0.3 * triangle(freq * 2.01)) >> lowpass(freq * 2.5, 0.4) >> compress(-18, 6, 0.001, 0.05) >> decay(5)

// Glass keys — bell-like, high harmonics, quick decay
instrument glass = (0.3 * sine(freq) + 0.25 * sine(freq * 2.76) + 0.2 * sine(freq * 4.07) + 0.15 * sine(freq * 6.21)) >> decay(15)

// ============================================================
// PLUCKED STRINGS
// ============================================================

// Nylon guitar — warm, soft attack, filtered harmonics
instrument nylon = (0.4 * saw(freq) + 0.3 * triangle(freq) + 0.15 * pulse(freq, 0.3)) >> lowpass(freq * 3, 0.6) >> decay(8)

// Steel string — brighter, more harmonics, sharper attack
// Steel string — brighter, more harmonics, sharper attack
instrument steel = (0.5 * saw(freq) + 0.3 * saw(freq * 2.003)) >> lowpass(freq * 4, 0.7) >> decay(10)

// Pizzicato — short pluck, orchestral character
instrument pizz = (0.5 * triangle(freq) + 0.3 * saw(freq)) >> lowpass(freq * 3, 0.5) >> decay(18)

// Harp — bright attack, long ring, slight detuning for shimmer
instrument harp = (0.4 * triangle(freq) + 0.3 * sine(freq * 2) + 0.15 * sine(freq * 3) + 0.1 * triangle(freq * 1.002)) >> lowpass(freq * 4, 0.6) >> decay(4)

// ============================================================
// PADS & SUSTAINED
// ============================================================

// Warm analog pad — detuned saws with gentle filtering
instrument warm_pad = (0.3 * saw(freq) + 0.3 * saw(freq * 1.005) + 0.2 * saw(freq * 0.995)) >> lowpass(freq * 3, 0.6) >> chorus(0.012, 0.004, 0.15)

// String ensemble — layered detuned triangle waves for orchestral warmth
instrument strings = (0.25 * triangle(freq) + 0.25 * triangle(freq * 1.003) + 0.2 * triangle(freq * 0.997) + 0.15 * triangle(freq * 2.002)) >> lowpass(freq * 4, 0.5) >> chorus(0.010, 0.003, 0.12)

// Dark drone — low-passed saw with subtle movement
instrument drone = (0.4 * saw(freq) + 0.3 * saw(freq * 0.501) + 0.2 * sine(freq * 2)) >> lowpass(freq * 1.5, 0.8) >> chorus(0.020, 0.008, 0.08)

// Airy pad — triangle with breath noise layered underneath
instrument airy = (0.35 * triangle(freq) + 0.25 * sine(freq * 2) + 0.05 * (pink() >> lowpass(freq * 2, 0.3))) >> chorus(0.018, 0.006, 0.1)

// ============================================================
// BASS
// ============================================================

// Sub bass — pure low-end weight
instrument sub = sine(freq) >> lowpass(freq * 2, 0.7) >> decay(3)

// Analog bass — warm saw with resonant filter
instrument analog_bass = (0.5 * saw(freq) + 0.3 * square(freq * 0.5)) >> lowpass(freq * 3, 1.2) >> decay(5)

// Plucked bass — bright attack that quickly darkens
instrument pluck_bass = (0.5 * saw(freq) + 0.3 * pulse(freq, 0.25)) >> lowpass(freq * 4, 0.8) >> decay(8)

// FM bass — metallic, punchy
instrument fm_bass = (0.5 * sine(freq) + 0.35 * sine(freq * 2.01) + 0.2 * sine(freq * 3.99)) >> lowpass(freq * 4, 0.6) >> decay(8)

// ============================================================
// MALLETS & BELLS
// ============================================================

// Marimba — sine with slightly inharmonic overtones, fast attack
instrument marimba = (0.5 * sine(freq) + 0.25 * sine(freq * 4.01) + 0.1 * sine(freq * 9.98)) >> decay(12)

// Vibraphone — sine harmonics with vibrato and long sustain
instrument vibes = (0.4 * sine(freq) + 0.25 * sine(freq * 2) + 0.15 * sine(freq * 3) + 0.1 * sine(freq * 4)) >> vibrato(5.0, 8.0) >> decay(4)

// Music box — high, thin, crystalline
instrument music_box = (0.3 * sine(freq) + 0.2 * sine(freq * 2) + 0.15 * sine(freq * 5.04) + 0.1 * sine(freq * 8.02)) >> decay(6)

// Kalimba — thumb piano, distinctive odd harmonics
instrument kalimba = (0.4 * sine(freq) + 0.2 * sine(freq * 3) + 0.15 * sine(freq * 5) + 0.08 * sine(freq * 7)) >> lowpass(freq * 4, 0.5) >> decay(10)

// Glockenspiel — bright, metallic, high harmonics
instrument glock = (0.3 * sine(freq) + 0.25 * sine(freq * 2.76) + 0.2 * sine(freq * 5.4) + 0.15 * sine(freq * 8.93)) >> decay(8)

// ============================================================
// TEXTURES & EFFECTS
// ============================================================

// Vinyl crackle — constant low-level noise, filtered for vintage character
voice vinyl = 0.015 * pink() >> lowpass(4000, 0.5)

// Tape hiss — higher frequency noise floor
voice tape_hiss = 0.01 * noise() >> lowpass(8000, 0.3)

// Room tone — very low frequency rumble
voice room = 0.02 * brown() >> lowpass(200, 0.5)

// ============================================================
// EFFECT CHAINS
// ============================================================

// Lo-fi processing — warmth + compression + subtle saturation
fx lofi = chorus(0.012, 0.004, 0.18) >> distort(1.3) >> compress(-18, 3, 0.01, 0.1)

// Small room — tight, intimate reverb
fx small_room = reverb(0.3, 0.3, 0.15)

// Hall — large, open reverb
fx hall = reverb(0.8, 0.5, 0.3)

// Tape delay — warm echo with filtering
fx tape_echo = delay(0.375, 0.45, 0.35) >> lowpass(3000, 0.5)

// Radio — bandpass filtered for AM radio character
fx radio = lowpass(3500, 0.8) >> distort(1.5) >> compress(-15, 4, 0.005, 0.05)

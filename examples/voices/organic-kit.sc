// organic-kit.sc — Downtempo electronic with organic textures
// Warm, layered, patient. Plucked melodics, deep bass, textured percussion.

// --- Effect chains ---

fx wide_space = chorus(0.010, 0.003, 0.15) >> reverb(0.7, 0.4, 0.35)
fx tight_verb = reverb(0.3, 0.5, 0.2)
fx dub_echo = delay(0.375, 0.35, 0.3) >> lowpass(2500, 0.5)

// --- Melodic instruments ---

// Plucked string — nylon-like, warm attack, long reverb tail
instrument pluck = (0.3 * saw(freq) + 0.25 * triangle(freq) + 0.1 * pulse(freq, 0.35)) >> lowpass(freq * 3, 0.5) >> decay(10) >> wide_space

// Kalimba — sparse melodic pings, odd harmonics
instrument thumb = (0.3 * sine(freq) + 0.15 * sine(freq * 3) + 0.1 * sine(freq * 5) + 0.05 * sine(freq * 7)) >> lowpass(freq * 5, 0.4) >> decay(8) >> dub_echo

// Glass tone — high register shimmer
instrument shimmer = (0.2 * sine(freq) + 0.15 * sine(freq * 2.76) + 0.1 * sine(freq * 4.07)) >> decay(12) >> wide_space

// --- Bass ---

// Deep melodic bass — warm saw with sub, moves with the harmony
instrument deep = (0.4 * saw(freq) + 0.5 * sine(freq) + 0.15 * triangle(freq * 2)) >> lowpass(freq * 2.5, 0.8) >> distort(1.2) >> compress(-16, 3, 0.008, 0.12) >> decay(4)

// --- Pads ---

// Warm atmospheric wash — slow chorus, wide reverb
instrument atmos = (0.2 * saw(freq) + 0.18 * saw(freq * 1.004) + 0.15 * saw(freq * 0.996) + 0.04 * (pink() >> lowpass(freq * 2, 0.3))) >> lowpass(freq * 2, 0.5) >> chorus(0.015, 0.005, 0.1) >> reverb(0.85, 0.5, 0.4)

// String-like pad — triangle based, less buzzy than saw
instrument silk = (0.2 * triangle(freq) + 0.18 * triangle(freq * 1.003) + 0.15 * triangle(freq * 0.997)) >> lowpass(freq * 3, 0.4) >> chorus(0.012, 0.004, 0.12) >> reverb(0.7, 0.4, 0.3)

// --- Percussion ---

// Kick — deep, round, organic thud. Gate tightens the tail.
voice dk = (0.8 * sine(G1) + (0.25 * sine(G2) >> decay(40)) + 0.08 * (noise() >> lowpass(150, 0.4) >> decay(55))) >> decay(10) >> compress(-14, 4, 0.002, 0.08) >> gate(0.004)

// Snare — dry, crackly, not too prominent
voice sn = (0.08 * (triangle(Bb3) >> decay(28)) + 0.15 * (noise() >> highpass(2000, 1.0) >> lowpass(5500, 0.7) >> decay(22))) >> distort(1.3) >> compress(-18, 3, 0.003, 0.05)

// Rim — short, woody click. Bandpass focuses the tone, gate snaps it off.
voice rim = (0.12 * (sine(E4) >> decay(50)) + 0.08 * (noise() >> bandpass(5000, 2.0) >> decay(60))) >> compress(-15, 3, 0.001, 0.03) >> gate(0.006)

// Shaker — continuous texture, filtered noise
voice shk = 0.04 * noise() >> highpass(6000, 0.8) >> lowpass(12000, 0.5) >> decay(18)

// Hat — crisp, short
voice ht = (0.06 * (noise() >> highpass(7000, 1.2) >> decay(45)) + 0.03 * (noise() >> highpass(4000, 0.7) >> decay(22))) >> compress(-14, 3, 0.001, 0.03)

// Open hat — longer ring
voice oht = (0.05 * (noise() >> highpass(5000, 1.0) >> decay(12)) + 0.03 * (noise() >> highpass(3500, 0.6) >> decay(8))) >> compress(-14, 3, 0.001, 0.04)

// Ambient texture — brown noise rumble, very quiet
voice earth = 0.012 * brown() >> lowpass(300, 0.5) >> reverb(0.9, 0.6, 0.5)

// Field texture — pink noise filtered for organic air
voice air = 0.008 * pink() >> highpass(1500, 0.4) >> lowpass(6000, 0.5) >> reverb(0.8, 0.5, 0.4)

// Resonant mid hum — bandpass-filtered noise for warm presence
voice hum = 0.015 * brown() >> bandpass(300, 3.0) >> chorus(0.020, 0.008, 0.08) >> reverb(0.7, 0.5, 0.3)

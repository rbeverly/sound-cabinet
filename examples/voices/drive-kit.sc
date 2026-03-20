// drive-kit.sc — Progressive electronic, driving four-on-the-floor
// Thick saws, massive pads, relentless kick, long builds.

// --- Effect chains ---

fx big_room = reverb(0.75, 0.4, 0.25)
fx huge_verb = reverb(0.9, 0.3, 0.4)
fx ping_delay = delay(0.333, 0.3, 0.25) >> lowpass(3000, 0.5)

// --- Lead instruments ---

// Supersaw lead — 5 detuned saws, wide and aggressive, with reverb tail for sustain
instrument supersaw = (0.25 * saw(freq) + 0.22 * saw(freq * 1.007) + 0.22 * saw(freq * 0.993) + 0.18 * saw(freq * 1.012) + 0.18 * saw(freq * 0.988)) >> lowpass(freq * 6, 0.7) >> compress(-12, 3, 0.003, 0.04) >> reverb(0.4, 0.4, 0.3)

// Pluck lead — short attack, for riffs
instrument syn_pluck = (0.3 * saw(freq) + 0.2 * pulse(freq, 0.3) + 0.15 * saw(freq * 1.005)) >> lowpass(freq * 5, 0.5) >> decay(12) >> ping_delay

// --- Bass ---

// Saw bass — thick, loud, with sub and mid-range harmonics for speaker presence
// Extra sine at 2x and 3x so it's audible on small speakers too
// Reverb tail helps notes blend into each other
instrument saw_bass = (0.6 * saw(freq) + 0.5 * sine(freq) + 0.3 * sine(freq * 2) + 0.15 * sine(freq * 3) + 0.2 * saw(freq * 1.003)) >> lowpass(freq * 5, 1.0) >> distort(1.8) >> compress(-12, 4, 0.003, 0.06) >> reverb(0.2, 0.5, 0.2)

// --- Pads ---

// Massive pad — evolving saw wash, light saturation for presence
instrument massive = (0.15 * saw(freq) + 0.14 * saw(freq * 1.005) + 0.13 * saw(freq * 0.995) + 0.12 * saw(freq * 1.01) + 0.1 * saw(freq * 0.99)) >> lowpass(freq * 2, 0.5) >> distort(1.2) >> chorus(0.012, 0.004, 0.15) >> huge_verb

// String pad — softer, triangle-based, for breakdowns
instrument soft_pad = (0.18 * triangle(freq) + 0.15 * triangle(freq * 1.003) + 0.12 * triangle(freq * 0.997)) >> lowpass(freq * 3, 0.4) >> chorus(0.010, 0.003, 0.1) >> big_room

// --- Percussion ---

// Kick — punchy, tight, electronic. Gate cleans up the tail.
voice kick4 = (0.9 * sine(E1) + (0.4 * sine(E2) >> decay(40)) + 0.15 * (noise() >> lowpass(250, 0.5) >> decay(50))) >> decay(12) >> compress(-12, 5, 0.001, 0.06) >> gate(0.005)

// Clap — layered noise burst, gated for snap, reverb for spread
voice clap = (0.2 * (noise() >> highpass(1500, 1.0) >> lowpass(4000, 0.8) >> decay(18)) + 0.08 * (noise() >> highpass(2000, 1.2) >> lowpass(5000, 0.7) >> decay(20))) >> distort(1.3) >> compress(-16, 3, 0.002, 0.04) >> gate(0.008) >> reverb(0.3, 0.5, 0.2)

// Closed hat — crisp
voice chh = 0.07 * noise() >> highpass(7000, 1.3) >> decay(35) >> compress(-14, 3, 0.001, 0.03)

// Open hat — longer
voice ohh = 0.06 * noise() >> highpass(5000, 1.0) >> decay(10) >> compress(-14, 3, 0.001, 0.04)

// Ride — shimmery
voice ride = (0.04 * (noise() >> highpass(4000, 0.8) >> decay(8)) + 0.02 * (noise() >> highpass(6000, 1.0) >> decay(6))) >> compress(-14, 3, 0.001, 0.05)

// Noise riser — builds tension, filtered and compressed for smooth sweep
voice riser = 0.04 * noise() >> lowpass(200 -> 6000, 0.4) >> compress(-20, 4, 0.01, 0.1) >> reverb(0.5, 0.4, 0.3)

// Impact — big hit for drops
voice impact = (0.5 * sine(C1) + 0.3 * (noise() >> lowpass(400, 0.6))) >> decay(4) >> compress(-10, 4, 0.001, 0.1) >> big_room

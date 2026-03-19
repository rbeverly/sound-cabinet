// therapy-kit.sc — Voices for therapeutic lo-fi compositions
// Combines lo-fi aesthetics with frequencies studied for physiological effects

// --- Effect chains ---

fx dreamy = chorus(0.010, 0.003, 0.15) >> reverb(0.7, 0.5, 0.2)

// --- Therapeutic layers ---

// 40 Hz gamma entrainment — click-train approach (matches Tsai lab protocol)
voice gamma_clicks = (0.18 * sine(10000) >> highpass(6000, 1.0)) * triangle(40)

// Softer version — noise carrier with triangle envelope for gentler pulsing
voice gamma_soft = (0.22 * noise() >> highpass(3000, 0.8) >> lowpass(9000, 0.6)) * triangle(40)

// Alpha-range (10 Hz) amplitude shimmer — two close frequencies beat at ~10 Hz
voice alpha_shimmer = 0.02 * (sine(200) + sine(210)) >> lowpass(400, 0.4) >> chorus(0.020, 0.008, 0.1)

// Deep sub-bass drone at 38 Hz — bone-resonant range (30-50 Hz)
voice bone_drone = (0.05 * sine(43.5) + 0.02 * sine(76)) >> distort(1.2)

// --- Instruments ---

// Chord instrument — warm muted saws through dreamy chorus/reverb
// Therapy chords are tuned slightly flat for warped-tape feel (use raw Hz in the score)
instrument chord_pad = 0.11 * saw(freq) >> lowpass(700, 0.5) >> dreamy

// Sub bass — sine + triangle octave, warm saturation, compressed
instrument bass = (0.35 * sine(freq) + 0.1 * triangle(freq * 2)) >> lowpass(250, 0.7) >> distort(1.3) >> compress(-18, 3, 0.01, 0.15)

// Melody — detuned triangle + sine for warmth, natural decay, vibrato + delay
// Rhodes/kalimba character
instrument mel = (0.08 * triangle(freq) + 0.05 * sine(freq * 0.997)) >> lowpass(1800, 0.4) >> decay(3) >> vibrato(4.0, 10.0) >> delay(0.5, 0.25, 0.2)

// Lower octave melody — slightly darker, subtler vibrato
instrument mel_low = (0.07 * triangle(freq) + 0.04 * sine(freq * 0.997)) >> lowpass(1400, 0.4) >> decay(3) >> vibrato(3.5, 8.0)

// --- Drums (soft, round) ---
voice kick  = (0.3 * sine(50) + 0.2 * sine(65)) >> decay(10) >> compress(-15, 3, 0.003, 0.08)
voice snare = 0.2 * noise() >> highpass(1200, 1.0) >> lowpass(5000, 0.7) >> decay(14)
voice ghost = 0.08 * noise() >> highpass(1800, 0.8) >> lowpass(4500, 0.5) >> decay(18)
voice hat   = 0.10 * noise() >> highpass(6000, 1.0) >> decay(28)

// --- Vinyl ---
voice hiss    = 0.025 * noise() >> highpass(3000, 0.5) >> lowpass(8000, 0.5)
voice pop     = 0.04 * noise() >> highpass(1000, 0.8) >> lowpass(4000, 0.6) >> decay(80)
voice click   = 0.05 * noise() >> highpass(4000, 1.0) >> lowpass(10000, 0.5) >> decay(120)

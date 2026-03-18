// therapy-kit.sc — Voices for therapeutic lo-fi compositions
// Combines lo-fi aesthetics with frequencies studied for physiological effects

// --- Therapeutic layers ---

// 40 Hz gamma entrainment — click-train approach (matches Tsai lab protocol)
// 10 kHz sine modulated by triangle(40) = 40 smooth bursts/sec
// Triangle avoids the discontinuity artifacts that saw/square create
// Works on any speaker/headphone — carrier is 10 kHz, only repetition rate is 40 Hz
voice gamma_clicks = (0.18 * sine(10000) >> highpass(6000, 1.0)) * triangle(40)

// Softer version — noise carrier with triangle envelope for gentler pulsing
voice gamma_soft = (0.22 * noise() >> highpass(3000, 0.8) >> lowpass(9000, 0.6)) * triangle(40)

// Alpha-range (10 Hz) amplitude shimmer — two close frequencies beat at ~10 Hz
// The brain tends to entrain to alpha during relaxation / eyes-closed rest
voice alpha_shimmer = 0.01 * (sine(200) + sine(210)) >> lowpass(400, 0.4)

// Deep sub-bass drone at 38 Hz — bone-resonant range (30-50 Hz)
// Whole-body vibration studies show osteoblast stimulation in this range
// Needs real speakers/subs to feel the physical effect; headphones give the tone
voice bone_drone = 0.05 * sine(43.5) + 0.02 * sine(76)

// --- Lo-fi chords (warm, muted) ---
// Dm9 → Bbmaj7 → Gm7 → A7
// Tuned slightly flat for that warped-tape feel

// Chords scaled down — 5 saws need headroom for the mix
voice chord_dm  = (0.12 * saw(146) + 0.12 * saw(174) + 0.10 * saw(220) + 0.08 * saw(262) + 0.06 * saw(330)) >> lowpass(700, 0.5)
voice chord_bb  = (0.12 * saw(116) + 0.12 * saw(146) + 0.10 * saw(175) + 0.08 * saw(220)) >> lowpass(700, 0.5)
voice chord_gm  = (0.12 * saw(196) + 0.12 * saw(233) + 0.10 * saw(294) + 0.08 * saw(349)) >> lowpass(700, 0.5)
voice chord_a7  = (0.12 * saw(220) + 0.12 * saw(277) + 0.10 * saw(330) + 0.08 * saw(196)) >> lowpass(800, 0.6)

// --- Sub bass (musically correct roots, several land in the 35-50 Hz therapeutic zone) ---
// D1=36.7, Bb0=29.1, G1=49.0, A1=55.0
voice bass_d  = (0.35 * sine(36.7) + 0.1 * triangle(73.4))  >> lowpass(250, 0.7)
voice bass_bb = (0.35 * sine(29.1) + 0.1 * triangle(58.3))  >> lowpass(250, 0.7)
voice bass_g  = (0.35 * sine(49.0) + 0.1 * triangle(98.0))  >> lowpass(250, 0.7)
voice bass_a  = (0.35 * sine(55.0) + 0.1 * triangle(110.0)) >> lowpass(250, 0.7)

// --- Drums (soft, round) ---
voice kick  = (0.3 * sine(50) + 0.2 * sine(65)) >> decay(10)
voice snare = 0.2 * noise() >> highpass(1200, 1.0) >> lowpass(5000, 0.7) >> decay(14)
voice ghost = 0.08 * noise() >> highpass(1800, 0.8) >> lowpass(4500, 0.5) >> decay(18)
voice hat   = 0.10 * noise() >> highpass(6000, 1.0) >> decay(28)

// --- Vinyl ---
voice hiss    = 0.025 * noise() >> highpass(3000, 0.5) >> lowpass(8000, 0.5)
voice pop     = 0.04 * noise() >> highpass(1000, 0.8) >> lowpass(4000, 0.6) >> decay(80)
voice click   = 0.05 * noise() >> highpass(4000, 1.0) >> lowpass(10000, 0.5) >> decay(120)

// --- Melody (pentatonic minor — D, F, G, A, C) ---
// Detuned triangle + sine for warmth, decay for natural fade (like a kalimba/Rhodes)
voice mel_d5  = (0.08 * triangle(587) + 0.05 * sine(585))  >> lowpass(1800, 0.4) >> decay(3)
voice mel_f5  = (0.08 * triangle(698) + 0.05 * sine(696))  >> lowpass(1800, 0.4) >> decay(3)
voice mel_g5  = (0.08 * triangle(784) + 0.05 * sine(782))  >> lowpass(1800, 0.4) >> decay(3)
voice mel_a4  = (0.08 * triangle(440) + 0.05 * sine(438))  >> lowpass(1800, 0.4) >> decay(3)
voice mel_c5  = (0.08 * triangle(523) + 0.05 * sine(521))  >> lowpass(1800, 0.4) >> decay(3)

// Added notes for full D natural minor (E, Bb)
voice mel_e5  = (0.08 * triangle(659) + 0.05 * sine(657))  >> lowpass(1800, 0.4) >> decay(3)
voice mel_bb4 = (0.08 * triangle(466) + 0.05 * sine(464))  >> lowpass(1800, 0.4) >> decay(3)

// Lower octave for arp bass notes
voice mel_d4  = (0.07 * triangle(294) + 0.04 * sine(293))  >> lowpass(1400, 0.4) >> decay(3)
voice mel_e4  = (0.07 * triangle(329) + 0.04 * sine(328))  >> lowpass(1400, 0.4) >> decay(3)
voice mel_f4  = (0.07 * triangle(349) + 0.04 * sine(348))  >> lowpass(1400, 0.4) >> decay(3)
voice mel_g4  = (0.07 * triangle(392) + 0.04 * sine(391))  >> lowpass(1400, 0.4) >> decay(3)
voice mel_a3  = (0.07 * triangle(220) + 0.04 * sine(219))  >> lowpass(1400, 0.4) >> decay(3)
voice mel_bb3 = (0.07 * triangle(233) + 0.04 * sine(232))  >> lowpass(1400, 0.4) >> decay(3)
voice mel_c4  = (0.07 * triangle(262) + 0.04 * sine(261))  >> lowpass(1400, 0.4) >> decay(3)

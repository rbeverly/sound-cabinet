// lofi-kit.sc — Voices for lo-fi compositions

// Chords (Cm7 → Abmaj7 → Fm7 → G7)
voice chord1 = (0.3 * saw(C3) + 0.3 * saw(Eb3) + 0.3 * saw(G3) + 0.25 * saw(Bb3)) >> lowpass(800, 0.6)
voice chord2 = (0.3 * saw(Ab3) + 0.3 * saw(C4) + 0.3 * saw(Eb4) + 0.25 * saw(G4)) >> lowpass(800, 0.6)
voice chord3 = (0.3 * saw(F3) + 0.3 * saw(Ab3) + 0.3 * saw(C4) + 0.25 * saw(Eb4)) >> lowpass(800, 0.6)
voice chord4 = (0.3 * saw(G3) + 0.3 * saw(B3) + 0.3 * saw(D4) + 0.25 * saw(F4)) >> lowpass(900, 0.7)

// Sub bass
voice bass_c  = (sine(C2) + 0.3 * triangle(C3)) >> lowpass(300, 0.8)
voice bass_ab = (sine(Ab1) + 0.3 * triangle(Ab2)) >> lowpass(300, 0.8)
voice bass_f  = (sine(F1) + 0.3 * triangle(F2))  >> lowpass(300, 0.8)
voice bass_g  = (sine(G1) + 0.3 * triangle(G2))  >> lowpass(300, 0.8)

// Vinyl texture layers
voice hiss = 0.03 * noise() >> highpass(3000, 0.5) >> lowpass(8000, 0.5)
voice pop = 0.05 * noise() >> highpass(1000, 0.8) >> lowpass(4000, 0.6) >> decay(80)
voice click = 0.06 * noise() >> highpass(4000, 1.0) >> lowpass(10000, 0.5) >> decay(120)
voice scratch = 0.04 * noise() >> highpass(2000, 0.7) >> lowpass(6000, 0.8) >> decay(30)

// Drums
voice kick  = (0.7 * sine(A1) + 0.5 * sine(B0)) >> decay(12)
voice snare = 0.3 * noise() >> highpass(1500, 1.2) >> lowpass(6000, 0.8) >> decay(15)
voice ghost_snare = 0.1 * noise() >> highpass(2000, 1.0) >> lowpass(5000, 0.6) >> decay(20)
voice hat   = 0.12 * noise() >> highpass(6000, 1.0) >> decay(25)

// Melody notes (C minor pentatonic)
voice mel_c5  = 0.15 * triangle(C5)  >> lowpass(2000, 0.5)
voice mel_eb5 = 0.15 * triangle(Eb5) >> lowpass(2000, 0.5)
voice mel_g4  = 0.15 * triangle(G4)  >> lowpass(2000, 0.5)
voice mel_bb4 = 0.15 * triangle(Bb4) >> lowpass(2000, 0.5)
voice mel_f5  = 0.15 * triangle(F5)  >> lowpass(2000, 0.5)

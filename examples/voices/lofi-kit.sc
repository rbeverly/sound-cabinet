// lofi-kit.sc — Voices for lo-fi compositions

// Chords (Cm7 → Abmaj7 → Fm7 → G7)
voice chord1 = (0.3 * saw(131) + 0.3 * saw(156) + 0.3 * saw(196) + 0.25 * saw(233)) >> lowpass(800, 0.6)
voice chord2 = (0.3 * saw(208) + 0.3 * saw(262) + 0.3 * saw(311) + 0.25 * saw(392)) >> lowpass(800, 0.6)
voice chord3 = (0.3 * saw(175) + 0.3 * saw(208) + 0.3 * saw(262) + 0.25 * saw(311)) >> lowpass(800, 0.6)
voice chord4 = (0.3 * saw(196) + 0.3 * saw(247) + 0.3 * saw(294) + 0.25 * saw(349)) >> lowpass(900, 0.7)

// Sub bass
voice bass_c  = (sine(65) + 0.3 * triangle(131)) >> lowpass(300, 0.8)
voice bass_ab = (sine(52) + 0.3 * triangle(104)) >> lowpass(300, 0.8)
voice bass_f  = (sine(44) + 0.3 * triangle(87))  >> lowpass(300, 0.8)
voice bass_g  = (sine(49) + 0.3 * triangle(98))  >> lowpass(300, 0.8)

// Vinyl texture layers
voice hiss = 0.03 * noise() >> highpass(3000, 0.5) >> lowpass(8000, 0.5)
voice pop = 0.05 * noise() >> highpass(1000, 0.8) >> lowpass(4000, 0.6) >> decay(80)
voice click = 0.06 * noise() >> highpass(4000, 1.0) >> lowpass(10000, 0.5) >> decay(120)
voice scratch = 0.04 * noise() >> highpass(2000, 0.7) >> lowpass(6000, 0.8) >> decay(30)

// Drums
voice kick  = (0.7 * sine(55) + 0.5 * sine(30)) >> decay(12)
voice snare = 0.3 * noise() >> highpass(1500, 1.2) >> lowpass(6000, 0.8) >> decay(15)
voice ghost_snare = 0.1 * noise() >> highpass(2000, 1.0) >> lowpass(5000, 0.6) >> decay(20)
voice hat   = 0.12 * noise() >> highpass(6000, 1.0) >> decay(25)

// Melody notes (C minor pentatonic)
voice mel_c5  = 0.15 * triangle(523)  >> lowpass(2000, 0.5)
voice mel_eb5 = 0.15 * triangle(622)  >> lowpass(2000, 0.5)
voice mel_g4  = 0.15 * triangle(392)  >> lowpass(2000, 0.5)
voice mel_bb4 = 0.15 * triangle(466)  >> lowpass(2000, 0.5)
voice mel_f5  = 0.15 * triangle(698)  >> lowpass(2000, 0.5)

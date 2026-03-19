// wave-test.sc — Custom waveform demo
// Demonstrates user-defined wavetable oscillators

bpm 120

// A soft, rounded wave — sine-like but with a flat top
wave plateau = [0.0, 0.4, 0.8, 1.0, 1.0, 1.0, 0.8, 0.4, 0.0, -0.4, -0.8, -1.0, -1.0, -1.0, -0.8, -0.4]

// An aggressive, spiky wave — sharp attack, quick decay
wave spike = [0.0, 1.0, 0.3, 0.1, 0.0, -0.1, -0.3, -1.0]

// Asymmetric wave — different positive and negative shapes (adds even harmonics)
wave asym = [0.0, 0.6, 1.0, 0.8, 0.3, 0.0, -0.2, -0.5, -0.5, -0.2]

// Staircase / ziggurat — stepped approximation of a sine
wave ziggurat = [0.0, 0.0, 0.5, 0.5, 1.0, 1.0, 0.5, 0.5, 0.0, 0.0, -0.5, -0.5, -1.0, -1.0, -0.5, -0.5]

fx room = reverb(0.6, 0.4, 0.25)

// Play each wave so you can hear the character

// Plateau — warm, organ-like
at 0 play 0.3 * plateau(C3) >> lowpass(2000, 0.7) >> room for 4 beats

// Spike — bright, harsh, almost like a plucked string
at 5 play 0.3 * spike(C3) >> lowpass(3000, 0.7) >> room for 4 beats

// Asymmetric — has that tube/tape warmth from even harmonics
at 10 play 0.3 * asym(C3) >> lowpass(2000, 0.7) >> room for 4 beats

// Ziggurat — digital, 8-bit character
at 15 play 0.3 * ziggurat(C3) >> lowpass(2000, 0.7) >> room for 4 beats

// Now use them in a chord — the plateau wave as a pad
at 21 play (0.15 * plateau(C3) + 0.15 * plateau(E3) + 0.15 * plateau(G3)) >> lowpass(1500, 0.6) >> room for 8 beats

// And the spike as a melody over the pad
at 21 play 0.2 * spike(G4) >> lowpass(4000, 0.8) >> room >> delay(0.3, 0.3, 0.2) for 2 beats
at 23 play 0.2 * spike(E4) >> lowpass(4000, 0.8) >> room >> delay(0.3, 0.3, 0.2) for 2 beats
at 25 play 0.2 * spike(C4) >> lowpass(4000, 0.8) >> room >> delay(0.3, 0.3, 0.2) for 4 beats

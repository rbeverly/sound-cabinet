// lofi-afternoon.sc
// A ~2 minute lofi hip-hop style composition
// Dusty chords, simple bass, gentle noise texture

// -- Voices --

// Warm detuned chord pad (Cm7 voicing: C3-Eb3-G3-Bb3)
voice chord1 = (0.3 * saw(131) + 0.3 * saw(156) + 0.3 * saw(196) + 0.25 * saw(233)) >> lowpass(800, 0.6)

// Second chord (Ab maj7: Ab3-C4-Eb4-G4)
voice chord2 = (0.3 * saw(208) + 0.3 * saw(262) + 0.3 * saw(311) + 0.25 * saw(392)) >> lowpass(800, 0.6)

// Third chord (Fm7: F3-Ab3-C4-Eb4)
voice chord3 = (0.3 * saw(175) + 0.3 * saw(208) + 0.3 * saw(262) + 0.25 * saw(311)) >> lowpass(800, 0.6)

// Fourth chord (G7: G3-B3-D4-F4)
voice chord4 = (0.3 * saw(196) + 0.3 * saw(247) + 0.3 * saw(294) + 0.25 * saw(349)) >> lowpass(900, 0.7)

// Sub bass — round sine
voice bass_c  = (sine(65) + 0.3 * triangle(131)) >> lowpass(300, 0.8)
voice bass_ab = (sine(52) + 0.3 * triangle(104)) >> lowpass(300, 0.8)
voice bass_f  = (sine(44) + 0.3 * triangle(87))  >> lowpass(300, 0.8)
voice bass_g  = (sine(49) + 0.3 * triangle(98))  >> lowpass(300, 0.8)

// Vinyl crackle texture
voice crackle = 0.04 * noise() >> highpass(3000, 0.5) >> lowpass(8000, 0.5)

// Kick — low sine thump
voice kick = 0.8 * sine(55) + 0.3 * sine(30)

// Snare — filtered noise
voice snare = 0.3 * noise() >> highpass(1500, 1.2) >> lowpass(6000, 0.8)

// Hi-hat — high filtered noise
voice hat = 0.12 * noise() >> highpass(6000, 1.0)

// Simple melodic motif — pentatonic noodling
voice mel_c5  = 0.15 * triangle(523)  >> lowpass(2000, 0.5)
voice mel_eb5 = 0.15 * triangle(622)  >> lowpass(2000, 0.5)
voice mel_g4  = 0.15 * triangle(392)  >> lowpass(2000, 0.5)
voice mel_bb4 = 0.15 * triangle(466)  >> lowpass(2000, 0.5)
voice mel_f5  = 0.15 * triangle(698)  >> lowpass(2000, 0.5)

bpm 75

// ============================================================
// The chord progression is 4 bars, each bar = 4 beats
// | Cm7 (4 beats) | Abmaj7 (4 beats) | Fm7 (4 beats) | G7 (4 beats) |
// Total loop = 16 beats. At 75 bpm that's ~12.8 seconds.
// We repeat the loop ~10 times for about 2 minutes.
// ============================================================

// -- Vinyl crackle runs the whole time --
at 0 play crackle for 160 beats

// ========== Loop 1 (bars 1-4): Intro — chords + bass only ==========

at 0  play chord1 for 4 beats
at 4  play chord2 for 4 beats
at 8  play chord3 for 4 beats
at 12 play chord4 for 4 beats

at 0  play bass_c  for 4 beats
at 4  play bass_ab for 4 beats
at 8  play bass_f  for 4 beats
at 12 play bass_g  for 4 beats

// ========== Loop 2 (bars 5-8): Add drums ==========

at 16 play chord1 for 4 beats
at 20 play chord2 for 4 beats
at 24 play chord3 for 4 beats
at 28 play chord4 for 4 beats

at 16 play bass_c  for 4 beats
at 20 play bass_ab for 4 beats
at 24 play bass_f  for 4 beats
at 28 play bass_g  for 4 beats

// Drums — boom bap pattern, every bar
// kick on 1 and 3, snare on 2 and 4, hats on every beat
at 16 play kick for 0.5 beats
at 17 play snare for 0.25 beats
at 18 play kick for 0.5 beats
at 19 play snare for 0.25 beats

at 16 play hat for 0.2 beats
at 17 play hat for 0.2 beats
at 18 play hat for 0.2 beats
at 19 play hat for 0.2 beats

at 20 play kick for 0.5 beats
at 21 play snare for 0.25 beats
at 22 play kick for 0.5 beats
at 23 play snare for 0.25 beats

at 20 play hat for 0.2 beats
at 21 play hat for 0.2 beats
at 22 play hat for 0.2 beats
at 23 play hat for 0.2 beats

at 24 play kick for 0.5 beats
at 25 play snare for 0.25 beats
at 26 play kick for 0.5 beats
at 27 play snare for 0.25 beats

at 24 play hat for 0.2 beats
at 25 play hat for 0.2 beats
at 26 play hat for 0.2 beats
at 27 play hat for 0.2 beats

at 28 play kick for 0.5 beats
at 29 play snare for 0.25 beats
at 30 play kick for 0.5 beats
at 31 play snare for 0.25 beats

at 28 play hat for 0.2 beats
at 29 play hat for 0.2 beats
at 30 play hat for 0.2 beats
at 31 play hat for 0.2 beats

// ========== Loop 3 (bars 9-12): Add melody ==========

at 32 play chord1 for 4 beats
at 36 play chord2 for 4 beats
at 40 play chord3 for 4 beats
at 44 play chord4 for 4 beats

at 32 play bass_c  for 4 beats
at 36 play bass_ab for 4 beats
at 40 play bass_f  for 4 beats
at 44 play bass_g  for 4 beats

// Drums
at 32 play kick for 0.5 beats
at 33 play snare for 0.25 beats
at 34 play kick for 0.5 beats
at 35 play snare for 0.25 beats
at 32 play hat for 0.2 beats
at 33 play hat for 0.2 beats
at 34 play hat for 0.2 beats
at 35 play hat for 0.2 beats

at 36 play kick for 0.5 beats
at 37 play snare for 0.25 beats
at 38 play kick for 0.5 beats
at 39 play snare for 0.25 beats
at 36 play hat for 0.2 beats
at 37 play hat for 0.2 beats
at 38 play hat for 0.2 beats
at 39 play hat for 0.2 beats

at 40 play kick for 0.5 beats
at 41 play snare for 0.25 beats
at 42 play kick for 0.5 beats
at 43 play snare for 0.25 beats
at 40 play hat for 0.2 beats
at 41 play hat for 0.2 beats
at 42 play hat for 0.2 beats
at 43 play hat for 0.2 beats

at 44 play kick for 0.5 beats
at 45 play snare for 0.25 beats
at 46 play kick for 0.5 beats
at 47 play snare for 0.25 beats
at 44 play hat for 0.2 beats
at 45 play hat for 0.2 beats
at 46 play hat for 0.2 beats
at 47 play hat for 0.2 beats

// Melody — simple pentatonic phrase over the chords
at 32   play mel_c5  for 1 beat
at 33.5 play mel_eb5 for 0.5 beats
at 34   play mel_g4  for 2 beats

at 36   play mel_bb4 for 1.5 beats
at 38   play mel_g4  for 1 beat
at 39.5 play mel_eb5 for 0.5 beats

at 40   play mel_c5  for 1 beat
at 41   play mel_eb5 for 1 beat
at 42   play mel_f5  for 1 beat
at 43   play mel_eb5 for 1 beat

at 44   play mel_g4  for 2 beats
at 46   play mel_bb4 for 1 beat
at 47   play mel_c5  for 1 beat

// ========== Loops 4-5 (bars 13-20): Full groove repeats ==========

// Loop 4
at 48 play chord1 for 4 beats
at 52 play chord2 for 4 beats
at 56 play chord3 for 4 beats
at 60 play chord4 for 4 beats

at 48 play bass_c  for 4 beats
at 52 play bass_ab for 4 beats
at 56 play bass_f  for 4 beats
at 60 play bass_g  for 4 beats

at 48 play kick for 0.5 beats
at 49 play snare for 0.25 beats
at 50 play kick for 0.5 beats
at 51 play snare for 0.25 beats
at 48 play hat for 0.2 beats
at 49 play hat for 0.2 beats
at 50 play hat for 0.2 beats
at 51 play hat for 0.2 beats

at 52 play kick for 0.5 beats
at 53 play snare for 0.25 beats
at 54 play kick for 0.5 beats
at 55 play snare for 0.25 beats
at 52 play hat for 0.2 beats
at 53 play hat for 0.2 beats
at 54 play hat for 0.2 beats
at 55 play hat for 0.2 beats

at 56 play kick for 0.5 beats
at 57 play snare for 0.25 beats
at 58 play kick for 0.5 beats
at 59 play snare for 0.25 beats
at 56 play hat for 0.2 beats
at 57 play hat for 0.2 beats
at 58 play hat for 0.2 beats
at 59 play hat for 0.2 beats

at 60 play kick for 0.5 beats
at 61 play snare for 0.25 beats
at 62 play kick for 0.5 beats
at 63 play snare for 0.25 beats
at 60 play hat for 0.2 beats
at 61 play hat for 0.2 beats
at 62 play hat for 0.2 beats
at 63 play hat for 0.2 beats

// Melody variation
at 48   play mel_eb5 for 1 beat
at 49   play mel_c5  for 1.5 beats
at 51   play mel_g4  for 1 beat

at 52   play mel_bb4 for 1 beat
at 53   play mel_c5  for 1 beat
at 54.5 play mel_eb5 for 1.5 beats

at 56   play mel_f5  for 1 beat
at 57   play mel_eb5 for 0.5 beats
at 57.5 play mel_c5  for 1.5 beats
at 59   play mel_g4  for 1 beat

at 60   play mel_c5  for 1.5 beats
at 62   play mel_eb5 for 1 beat
at 63   play mel_g4  for 1 beat

// Loop 5
at 64 play chord1 for 4 beats
at 68 play chord2 for 4 beats
at 72 play chord3 for 4 beats
at 76 play chord4 for 4 beats

at 64 play bass_c  for 4 beats
at 68 play bass_ab for 4 beats
at 72 play bass_f  for 4 beats
at 76 play bass_g  for 4 beats

at 64 play kick for 0.5 beats
at 65 play snare for 0.25 beats
at 66 play kick for 0.5 beats
at 67 play snare for 0.25 beats
at 64 play hat for 0.2 beats
at 65 play hat for 0.2 beats
at 66 play hat for 0.2 beats
at 67 play hat for 0.2 beats

at 68 play kick for 0.5 beats
at 69 play snare for 0.25 beats
at 70 play kick for 0.5 beats
at 71 play snare for 0.25 beats
at 68 play hat for 0.2 beats
at 69 play hat for 0.2 beats
at 70 play hat for 0.2 beats
at 71 play hat for 0.2 beats

at 72 play kick for 0.5 beats
at 73 play snare for 0.25 beats
at 74 play kick for 0.5 beats
at 75 play snare for 0.25 beats
at 72 play hat for 0.2 beats
at 73 play hat for 0.2 beats
at 74 play hat for 0.2 beats
at 75 play hat for 0.2 beats

at 76 play kick for 0.5 beats
at 77 play snare for 0.25 beats
at 78 play kick for 0.5 beats
at 79 play snare for 0.25 beats
at 76 play hat for 0.2 beats
at 77 play hat for 0.2 beats
at 78 play hat for 0.2 beats
at 79 play hat for 0.2 beats

// Melody — call and response feel
at 64   play mel_g4  for 1 beat
at 65   play mel_bb4 for 1 beat
at 66   play mel_c5  for 2 beats

at 68   play mel_eb5 for 2 beats
at 70   play mel_c5  for 1 beat
at 71   play mel_bb4 for 1 beat

at 72   play mel_c5  for 0.5 beats
at 72.5 play mel_eb5 for 0.5 beats
at 73   play mel_f5  for 1 beat
at 74   play mel_eb5 for 2 beats

at 76   play mel_c5  for 2 beats
at 78   play mel_g4  for 2 beats

// ========== Loops 6-7 (bars 21-28): Continue groove ==========

// Loop 6
at 80 play chord1 for 4 beats
at 84 play chord2 for 4 beats
at 88 play chord3 for 4 beats
at 92 play chord4 for 4 beats

at 80 play bass_c  for 4 beats
at 84 play bass_ab for 4 beats
at 88 play bass_f  for 4 beats
at 92 play bass_g  for 4 beats

at 80 play kick for 0.5 beats
at 81 play snare for 0.25 beats
at 82 play kick for 0.5 beats
at 83 play snare for 0.25 beats
at 80 play hat for 0.2 beats
at 81 play hat for 0.2 beats
at 82 play hat for 0.2 beats
at 83 play hat for 0.2 beats

at 84 play kick for 0.5 beats
at 85 play snare for 0.25 beats
at 86 play kick for 0.5 beats
at 87 play snare for 0.25 beats
at 84 play hat for 0.2 beats
at 85 play hat for 0.2 beats
at 86 play hat for 0.2 beats
at 87 play hat for 0.2 beats

at 88 play kick for 0.5 beats
at 89 play snare for 0.25 beats
at 90 play kick for 0.5 beats
at 91 play snare for 0.25 beats
at 88 play hat for 0.2 beats
at 89 play hat for 0.2 beats
at 90 play hat for 0.2 beats
at 91 play hat for 0.2 beats

at 92 play kick for 0.5 beats
at 93 play snare for 0.25 beats
at 94 play kick for 0.5 beats
at 95 play snare for 0.25 beats
at 92 play hat for 0.2 beats
at 93 play hat for 0.2 beats
at 94 play hat for 0.2 beats
at 95 play hat for 0.2 beats

// Melody — repeats motif from loop 3
at 80   play mel_c5  for 1 beat
at 81.5 play mel_eb5 for 0.5 beats
at 82   play mel_g4  for 2 beats

at 84   play mel_bb4 for 1.5 beats
at 86   play mel_g4  for 1 beat
at 87.5 play mel_eb5 for 0.5 beats

at 88   play mel_c5  for 1 beat
at 89   play mel_eb5 for 1 beat
at 90   play mel_f5  for 1 beat
at 91   play mel_eb5 for 1 beat

at 92   play mel_g4  for 2 beats
at 94   play mel_bb4 for 1 beat
at 95   play mel_c5  for 1 beat

// Loop 7
at 96  play chord1 for 4 beats
at 100 play chord2 for 4 beats
at 104 play chord3 for 4 beats
at 108 play chord4 for 4 beats

at 96  play bass_c  for 4 beats
at 100 play bass_ab for 4 beats
at 104 play bass_f  for 4 beats
at 108 play bass_g  for 4 beats

at 96  play kick for 0.5 beats
at 97  play snare for 0.25 beats
at 98  play kick for 0.5 beats
at 99  play snare for 0.25 beats
at 96  play hat for 0.2 beats
at 97  play hat for 0.2 beats
at 98  play hat for 0.2 beats
at 99  play hat for 0.2 beats

at 100 play kick for 0.5 beats
at 101 play snare for 0.25 beats
at 102 play kick for 0.5 beats
at 103 play snare for 0.25 beats
at 100 play hat for 0.2 beats
at 101 play hat for 0.2 beats
at 102 play hat for 0.2 beats
at 103 play hat for 0.2 beats

at 104 play kick for 0.5 beats
at 105 play snare for 0.25 beats
at 106 play kick for 0.5 beats
at 107 play snare for 0.25 beats
at 104 play hat for 0.2 beats
at 105 play hat for 0.2 beats
at 106 play hat for 0.2 beats
at 107 play hat for 0.2 beats

at 108 play kick for 0.5 beats
at 109 play snare for 0.25 beats
at 110 play kick for 0.5 beats
at 111 play snare for 0.25 beats
at 108 play hat for 0.2 beats
at 109 play hat for 0.2 beats
at 110 play hat for 0.2 beats
at 111 play hat for 0.2 beats

// Melody variation
at 96   play mel_eb5 for 1 beat
at 97   play mel_c5  for 1.5 beats
at 99   play mel_g4  for 1 beat

at 100  play mel_bb4 for 1 beat
at 101  play mel_c5  for 1 beat
at 102.5 play mel_eb5 for 1.5 beats

at 104  play mel_f5  for 1 beat
at 105  play mel_eb5 for 0.5 beats
at 105.5 play mel_c5  for 1.5 beats
at 107  play mel_g4  for 1 beat

at 108  play mel_c5  for 1.5 beats
at 110  play mel_eb5 for 1 beat
at 111  play mel_g4  for 1 beat

// ========== Loops 8-9 (bars 29-36): Continue ==========

// Loop 8
at 112 play chord1 for 4 beats
at 116 play chord2 for 4 beats
at 120 play chord3 for 4 beats
at 124 play chord4 for 4 beats

at 112 play bass_c  for 4 beats
at 116 play bass_ab for 4 beats
at 120 play bass_f  for 4 beats
at 124 play bass_g  for 4 beats

at 112 play kick for 0.5 beats
at 113 play snare for 0.25 beats
at 114 play kick for 0.5 beats
at 115 play snare for 0.25 beats
at 112 play hat for 0.2 beats
at 113 play hat for 0.2 beats
at 114 play hat for 0.2 beats
at 115 play hat for 0.2 beats

at 116 play kick for 0.5 beats
at 117 play snare for 0.25 beats
at 118 play kick for 0.5 beats
at 119 play snare for 0.25 beats
at 116 play hat for 0.2 beats
at 117 play hat for 0.2 beats
at 118 play hat for 0.2 beats
at 119 play hat for 0.2 beats

at 120 play kick for 0.5 beats
at 121 play snare for 0.25 beats
at 122 play kick for 0.5 beats
at 123 play snare for 0.25 beats
at 120 play hat for 0.2 beats
at 121 play hat for 0.2 beats
at 122 play hat for 0.2 beats
at 123 play hat for 0.2 beats

at 124 play kick for 0.5 beats
at 125 play snare for 0.25 beats
at 126 play kick for 0.5 beats
at 127 play snare for 0.25 beats
at 124 play hat for 0.2 beats
at 125 play hat for 0.2 beats
at 126 play hat for 0.2 beats
at 127 play hat for 0.2 beats

at 112  play mel_g4  for 1 beat
at 113  play mel_bb4 for 1 beat
at 114  play mel_c5  for 2 beats
at 116  play mel_eb5 for 2 beats
at 118  play mel_c5  for 1 beat
at 119  play mel_bb4 for 1 beat
at 120  play mel_c5  for 0.5 beats
at 120.5 play mel_eb5 for 0.5 beats
at 121  play mel_f5  for 1 beat
at 122  play mel_eb5 for 2 beats
at 124  play mel_c5  for 2 beats
at 126  play mel_g4  for 2 beats

// Loop 9
at 128 play chord1 for 4 beats
at 132 play chord2 for 4 beats
at 136 play chord3 for 4 beats
at 140 play chord4 for 4 beats

at 128 play bass_c  for 4 beats
at 132 play bass_ab for 4 beats
at 136 play bass_f  for 4 beats
at 140 play bass_g  for 4 beats

at 128 play kick for 0.5 beats
at 129 play snare for 0.25 beats
at 130 play kick for 0.5 beats
at 131 play snare for 0.25 beats
at 128 play hat for 0.2 beats
at 129 play hat for 0.2 beats
at 130 play hat for 0.2 beats
at 131 play hat for 0.2 beats

at 132 play kick for 0.5 beats
at 133 play snare for 0.25 beats
at 134 play kick for 0.5 beats
at 135 play snare for 0.25 beats
at 132 play hat for 0.2 beats
at 133 play hat for 0.2 beats
at 134 play hat for 0.2 beats
at 135 play hat for 0.2 beats

at 136 play kick for 0.5 beats
at 137 play snare for 0.25 beats
at 138 play kick for 0.5 beats
at 139 play snare for 0.25 beats
at 136 play hat for 0.2 beats
at 137 play hat for 0.2 beats
at 138 play hat for 0.2 beats
at 139 play hat for 0.2 beats

at 140 play kick for 0.5 beats
at 141 play snare for 0.25 beats
at 142 play kick for 0.5 beats
at 143 play snare for 0.25 beats
at 140 play hat for 0.2 beats
at 141 play hat for 0.2 beats
at 142 play hat for 0.2 beats
at 143 play hat for 0.2 beats

at 128  play mel_c5  for 1 beat
at 129.5 play mel_eb5 for 0.5 beats
at 130  play mel_g4  for 2 beats
at 132  play mel_bb4 for 1.5 beats
at 134  play mel_g4  for 1 beat
at 135.5 play mel_eb5 for 0.5 beats
at 136  play mel_c5  for 1 beat
at 137  play mel_eb5 for 1 beat
at 138  play mel_f5  for 1 beat
at 139  play mel_eb5 for 1 beat
at 140  play mel_g4  for 2 beats
at 142  play mel_bb4 for 1 beat
at 143  play mel_c5  for 1 beat

// ========== Loop 10 (bars 37-40): Outro — drop drums, fade melody ==========

at 144 play chord1 for 4 beats
at 148 play chord2 for 4 beats
at 152 play chord3 for 4 beats
at 156 play chord4 for 4 beats

at 144 play bass_c  for 4 beats
at 148 play bass_ab for 4 beats
at 152 play bass_f  for 4 beats
at 156 play bass_g  for 4 beats

// Sparse final melody notes
at 144  play mel_c5  for 2 beats
at 148  play mel_eb5 for 2 beats
at 152  play mel_g4  for 4 beats

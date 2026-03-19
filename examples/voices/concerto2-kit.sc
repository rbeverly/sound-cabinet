// Auto-generated voice kit from MIDI
// Piano voice recipe (hammer + strings + overall decay)
//
// Voices use the proven recipe:
//   HAMMER: saw >> bright lowpass >> fast decay
//   STRINGS: saw + octave-up saw >> dark lowpass >> chorus
//   OVERALL: decay >> reverb (high damping)

// ── Octave 1 ──
voice p_c1 = ((0.55 * saw(C1) >> lowpass(1200, 0.7) >> decay(6)) + (1.8 * saw(C1) + 0.35 * saw(C2)) >> lowpass(180, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(1.2) >> reverb(0.6, 0.3, 0.2)
voice p_db1 = ((0.55 * saw(Db1) >> lowpass(1200, 0.7) >> decay(6)) + (1.8 * saw(Db1) + 0.35 * saw(Db2)) >> lowpass(180, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(1.2) >> reverb(0.6, 0.3, 0.2)
voice p_d1 = ((0.55 * saw(D1) >> lowpass(1200, 0.7) >> decay(6)) + (1.8 * saw(D1) + 0.35 * saw(D2)) >> lowpass(180, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(1.2) >> reverb(0.6, 0.3, 0.2)
voice p_eb1 = ((0.55 * saw(Eb1) >> lowpass(1200, 0.7) >> decay(6)) + (1.8 * saw(Eb1) + 0.35 * saw(Eb2)) >> lowpass(180, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(1.2) >> reverb(0.6, 0.3, 0.2)
voice p_e1 = ((0.55 * saw(E1) >> lowpass(1200, 0.7) >> decay(6)) + (1.8 * saw(E1) + 0.35 * saw(E2)) >> lowpass(180, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(1.2) >> reverb(0.6, 0.3, 0.2)
voice p_f1 = ((0.55 * saw(F1) >> lowpass(1200, 0.7) >> decay(6)) + (1.8 * saw(F1) + 0.35 * saw(F2)) >> lowpass(180, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(1.2) >> reverb(0.6, 0.3, 0.2)
voice p_gb1 = ((0.55 * saw(Gb1) >> lowpass(1200, 0.7) >> decay(6)) + (1.8 * saw(Gb1) + 0.35 * saw(Gb2)) >> lowpass(180, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(1.2) >> reverb(0.6, 0.3, 0.2)
voice p_g1 = ((0.55 * saw(G1) >> lowpass(1200, 0.7) >> decay(6)) + (1.8 * saw(G1) + 0.35 * saw(G2)) >> lowpass(180, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(1.2) >> reverb(0.6, 0.3, 0.2)
voice p_ab1 = ((0.55 * saw(Ab1) >> lowpass(1200, 0.7) >> decay(6)) + (1.8 * saw(Ab1) + 0.35 * saw(Ab2)) >> lowpass(180, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(1.2) >> reverb(0.6, 0.3, 0.2)

// ── Octave 2 ──
voice p_c2 = ((0.5 * saw(C2) >> lowpass(1400, 0.7) >> decay(7)) + (1.8 * saw(C2) + 0.35 * saw(C3)) >> lowpass(200, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(1.5) >> reverb(0.6, 0.3, 0.2)
voice p_db2 = ((0.5 * saw(Db2) >> lowpass(1400, 0.7) >> decay(7)) + (1.8 * saw(Db2) + 0.35 * saw(Db3)) >> lowpass(200, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(1.5) >> reverb(0.6, 0.3, 0.2)
voice p_d2 = ((0.5 * saw(D2) >> lowpass(1400, 0.7) >> decay(7)) + (1.8 * saw(D2) + 0.35 * saw(D3)) >> lowpass(200, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(1.5) >> reverb(0.6, 0.3, 0.2)
voice p_eb2 = ((0.5 * saw(Eb2) >> lowpass(1400, 0.7) >> decay(7)) + (1.8 * saw(Eb2) + 0.35 * saw(Eb3)) >> lowpass(200, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(1.5) >> reverb(0.6, 0.3, 0.2)
voice p_e2 = ((0.5 * saw(E2) >> lowpass(1400, 0.7) >> decay(7)) + (1.8 * saw(E2) + 0.35 * saw(E3)) >> lowpass(200, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(1.5) >> reverb(0.6, 0.3, 0.2)
voice p_f2 = ((0.5 * saw(F2) >> lowpass(1400, 0.7) >> decay(7)) + (1.8 * saw(F2) + 0.35 * saw(F3)) >> lowpass(200, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(1.5) >> reverb(0.6, 0.3, 0.2)
voice p_gb2 = ((0.5 * saw(Gb2) >> lowpass(1400, 0.7) >> decay(7)) + (1.8 * saw(Gb2) + 0.35 * saw(Gb3)) >> lowpass(200, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(1.5) >> reverb(0.6, 0.3, 0.2)
voice p_g2 = ((0.5 * saw(G2) >> lowpass(1400, 0.7) >> decay(7)) + (1.8 * saw(G2) + 0.35 * saw(G3)) >> lowpass(200, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(1.5) >> reverb(0.6, 0.3, 0.2)
voice p_ab2 = ((0.5 * saw(Ab2) >> lowpass(1400, 0.7) >> decay(7)) + (1.8 * saw(Ab2) + 0.35 * saw(Ab3)) >> lowpass(200, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(1.5) >> reverb(0.6, 0.3, 0.2)
voice p_a2 = ((0.5 * saw(A2) >> lowpass(1400, 0.7) >> decay(7)) + (1.8 * saw(A2) + 0.35 * saw(A3)) >> lowpass(200, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(1.5) >> reverb(0.6, 0.3, 0.2)
voice p_bb2 = ((0.5 * saw(Bb2) >> lowpass(1400, 0.7) >> decay(7)) + (1.8 * saw(Bb2) + 0.35 * saw(Bb3)) >> lowpass(200, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(1.5) >> reverb(0.6, 0.3, 0.2)
voice p_b2 = ((0.5 * saw(B2) >> lowpass(1400, 0.7) >> decay(7)) + (1.8 * saw(B2) + 0.35 * saw(B3)) >> lowpass(200, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(1.5) >> reverb(0.6, 0.3, 0.2)

// ── Octave 3 ──
voice p_c3 = ((0.45 * saw(C3) >> lowpass(1800, 0.7) >> decay(8)) + (1.8 * saw(C3) + 0.35 * saw(C4)) >> lowpass(261, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(2.0) >> reverb(0.6, 0.3, 0.2)
voice p_db3 = ((0.45 * saw(Db3) >> lowpass(1800, 0.7) >> decay(8)) + (1.8 * saw(Db3) + 0.35 * saw(Db4)) >> lowpass(277, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(2.0) >> reverb(0.6, 0.3, 0.2)
voice p_d3 = ((0.45 * saw(D3) >> lowpass(1800, 0.7) >> decay(8)) + (1.8 * saw(D3) + 0.35 * saw(D4)) >> lowpass(293, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(2.0) >> reverb(0.6, 0.3, 0.2)
voice p_eb3 = ((0.45 * saw(Eb3) >> lowpass(1800, 0.7) >> decay(8)) + (1.8 * saw(Eb3) + 0.35 * saw(Eb4)) >> lowpass(311, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(2.0) >> reverb(0.6, 0.3, 0.2)
voice p_e3 = ((0.45 * saw(E3) >> lowpass(1800, 0.7) >> decay(8)) + (1.8 * saw(E3) + 0.35 * saw(E4)) >> lowpass(329, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(2.0) >> reverb(0.6, 0.3, 0.2)
voice p_f3 = ((0.45 * saw(F3) >> lowpass(1800, 0.7) >> decay(8)) + (1.8 * saw(F3) + 0.35 * saw(F4)) >> lowpass(349, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(2.0) >> reverb(0.6, 0.3, 0.2)
voice p_gb3 = ((0.45 * saw(Gb3) >> lowpass(1800, 0.7) >> decay(8)) + (1.8 * saw(Gb3) + 0.35 * saw(Gb4)) >> lowpass(369, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(2.0) >> reverb(0.6, 0.3, 0.2)
voice p_g3 = ((0.45 * saw(G3) >> lowpass(1800, 0.7) >> decay(8)) + (1.8 * saw(G3) + 0.35 * saw(G4)) >> lowpass(392, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(2.0) >> reverb(0.6, 0.3, 0.2)
voice p_ab3 = ((0.45 * saw(Ab3) >> lowpass(1800, 0.7) >> decay(8)) + (1.8 * saw(Ab3) + 0.35 * saw(Ab4)) >> lowpass(415, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(2.0) >> reverb(0.6, 0.3, 0.2)
voice p_a3 = ((0.45 * saw(A3) >> lowpass(1800, 0.7) >> decay(8)) + (1.8 * saw(A3) + 0.35 * saw(A4)) >> lowpass(440, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(2.0) >> reverb(0.6, 0.3, 0.2)
voice p_bb3 = ((0.45 * saw(Bb3) >> lowpass(1800, 0.7) >> decay(8)) + (1.8 * saw(Bb3) + 0.35 * saw(Bb4)) >> lowpass(466, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(2.0) >> reverb(0.6, 0.3, 0.2)
voice p_b3 = ((0.45 * saw(B3) >> lowpass(1800, 0.7) >> decay(8)) + (1.8 * saw(B3) + 0.35 * saw(B4)) >> lowpass(493, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(2.0) >> reverb(0.6, 0.3, 0.2)

// ── Octave 4 ──
voice p_c4 = ((0.42 * saw(C4) >> lowpass(2200, 0.7) >> decay(9)) + (1.5 * saw(C4) + 0.35 * saw(C5)) >> lowpass(600, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(2.5) >> reverb(0.6, 0.6, 0.35)
voice p_db4 = ((0.42 * saw(Db4) >> lowpass(2200, 0.7) >> decay(9)) + (1.5 * saw(Db4) + 0.35 * saw(Db5)) >> lowpass(600, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(2.5) >> reverb(0.6, 0.6, 0.35)
voice p_d4 = ((0.42 * saw(D4) >> lowpass(2200, 0.7) >> decay(9)) + (1.5 * saw(D4) + 0.35 * saw(D5)) >> lowpass(600, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(2.5) >> reverb(0.6, 0.6, 0.35)
voice p_eb4 = ((0.42 * saw(Eb4) >> lowpass(2200, 0.7) >> decay(9)) + (1.5 * saw(Eb4) + 0.35 * saw(Eb5)) >> lowpass(600, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(2.5) >> reverb(0.6, 0.6, 0.35)
voice p_e4 = ((0.42 * saw(E4) >> lowpass(2200, 0.7) >> decay(9)) + (1.5 * saw(E4) + 0.35 * saw(E5)) >> lowpass(600, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(2.5) >> reverb(0.6, 0.6, 0.35)
voice p_f4 = ((0.42 * saw(F4) >> lowpass(2200, 0.7) >> decay(9)) + (1.5 * saw(F4) + 0.35 * saw(F5)) >> lowpass(600, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(2.5) >> reverb(0.6, 0.6, 0.35)
voice p_gb4 = ((0.42 * saw(Gb4) >> lowpass(2200, 0.7) >> decay(9)) + (1.5 * saw(Gb4) + 0.35 * saw(Gb5)) >> lowpass(600, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(2.5) >> reverb(0.6, 0.6, 0.35)
voice p_g4 = ((0.42 * saw(G4) >> lowpass(2200, 0.7) >> decay(9)) + (1.5 * saw(G4) + 0.35 * saw(G5)) >> lowpass(600, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(2.5) >> reverb(0.6, 0.6, 0.35)
voice p_ab4 = ((0.42 * saw(Ab4) >> lowpass(2200, 0.7) >> decay(9)) + (1.5 * saw(Ab4) + 0.35 * saw(Ab5)) >> lowpass(600, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(2.5) >> reverb(0.6, 0.6, 0.35)
voice p_a4 = ((0.42 * saw(A4) >> lowpass(2200, 0.7) >> decay(9)) + (1.5 * saw(A4) + 0.35 * saw(A5)) >> lowpass(600, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(2.5) >> reverb(0.6, 0.6, 0.35)
voice p_bb4 = ((0.42 * saw(Bb4) >> lowpass(2200, 0.7) >> decay(9)) + (1.5 * saw(Bb4) + 0.35 * saw(Bb5)) >> lowpass(600, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(2.5) >> reverb(0.6, 0.6, 0.35)
voice p_b4 = ((0.42 * saw(B4) >> lowpass(2200, 0.7) >> decay(9)) + (1.5 * saw(B4) + 0.35 * saw(B5)) >> lowpass(600, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(2.5) >> reverb(0.6, 0.6, 0.35)

// ── Octave 5 ──
voice p_c5 = ((0.38 * saw(C5) >> lowpass(2800, 0.7) >> decay(10)) + (2.0 * saw(C5) + 0.35 * saw(C6)) >> lowpass(600, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(3.0) >> reverb(0.6, 0.6, 0.4)
voice p_db5 = ((0.38 * saw(Db5) >> lowpass(2800, 0.7) >> decay(10)) + (2.0 * saw(Db5) + 0.35 * saw(Db6)) >> lowpass(600, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(3.0) >> reverb(0.6, 0.6, 0.4)
voice p_d5 = ((0.38 * saw(D5) >> lowpass(2800, 0.7) >> decay(10)) + (2.0 * saw(D5) + 0.35 * saw(D6)) >> lowpass(600, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(3.0) >> reverb(0.6, 0.6, 0.4)
voice p_eb5 = ((0.38 * saw(Eb5) >> lowpass(2800, 0.7) >> decay(10)) + (2.0 * saw(Eb5) + 0.35 * saw(Eb6)) >> lowpass(600, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(3.0) >> reverb(0.6, 0.6, 0.4)
voice p_f5 = ((0.38 * saw(F5) >> lowpass(2800, 0.7) >> decay(10)) + (2.0 * saw(F5) + 0.35 * saw(F6)) >> lowpass(600, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(3.0) >> reverb(0.6, 0.6, 0.4)
voice p_gb5 = ((0.38 * saw(Gb5) >> lowpass(2800, 0.7) >> decay(10)) + (2.0 * saw(Gb5) + 0.35 * saw(Gb6)) >> lowpass(600, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(3.0) >> reverb(0.6, 0.6, 0.4)
voice p_g5 = ((0.38 * saw(G5) >> lowpass(2800, 0.7) >> decay(10)) + (2.0 * saw(G5) + 0.35 * saw(G6)) >> lowpass(600, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(3.0) >> reverb(0.6, 0.6, 0.4)
voice p_ab5 = ((0.38 * saw(Ab5) >> lowpass(2800, 0.7) >> decay(10)) + (2.0 * saw(Ab5) + 0.35 * saw(Ab6)) >> lowpass(600, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(3.0) >> reverb(0.6, 0.6, 0.4)
voice p_a5 = ((0.38 * saw(A5) >> lowpass(2800, 0.7) >> decay(10)) + (2.0 * saw(A5) + 0.35 * saw(A6)) >> lowpass(600, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(3.0) >> reverb(0.6, 0.6, 0.4)
voice p_bb5 = ((0.38 * saw(Bb5) >> lowpass(2800, 0.7) >> decay(10)) + (2.0 * saw(Bb5) + 0.35 * saw(Bb6)) >> lowpass(600, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(3.0) >> reverb(0.6, 0.6, 0.4)
voice p_b5 = ((0.38 * saw(B5) >> lowpass(2800, 0.7) >> decay(10)) + (2.0 * saw(B5) + 0.35 * saw(B6)) >> lowpass(600, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(3.0) >> reverb(0.6, 0.6, 0.4)

// ── Octave 6 ──
voice p_c6 = ((0.33 * saw(C6) >> lowpass(2800, 0.7) >> decay(10)) + (2.0 * saw(C6) + 0.35 * saw(C7)) >> lowpass(650, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(3.0) >> reverb(0.6, 0.6, 0.4)
voice p_d6 = ((0.33 * saw(D6) >> lowpass(2800, 0.7) >> decay(10)) + (2.0 * saw(D6) + 0.35 * saw(D7)) >> lowpass(650, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(3.0) >> reverb(0.6, 0.6, 0.4)
voice p_eb6 = ((0.33 * saw(Eb6) >> lowpass(2800, 0.7) >> decay(10)) + (2.0 * saw(Eb6) + 0.35 * saw(Eb7)) >> lowpass(650, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(3.0) >> reverb(0.6, 0.6, 0.4)
voice p_f6 = ((0.33 * saw(F6) >> lowpass(2800, 0.7) >> decay(10)) + (2.0 * saw(F6) + 0.35 * saw(F7)) >> lowpass(650, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(3.0) >> reverb(0.6, 0.6, 0.4)
voice p_g6 = ((0.33 * saw(G6) >> lowpass(2800, 0.7) >> decay(10)) + (2.0 * saw(G6) + 0.35 * saw(G7)) >> lowpass(650, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(3.0) >> reverb(0.6, 0.6, 0.4)
voice p_a6 = ((0.33 * saw(A6) >> lowpass(2800, 0.7) >> decay(10)) + (2.0 * saw(A6) + 0.35 * saw(A7)) >> lowpass(650, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(3.0) >> reverb(0.6, 0.6, 0.4)

// ── Octave 7 ──
voice p_c7 = ((0.28 * saw(C7) >> lowpass(2800, 0.7) >> decay(10)) + (2.0 * saw(C7) + 0.3 * saw(C8)) >> lowpass(700, 0.6) >> chorus(0.016, 0.006, 0.1)) >> decay(3.0) >> reverb(0.6, 0.6, 0.4)

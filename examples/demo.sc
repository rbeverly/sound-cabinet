// Sound Cabinet demo score
voice pad = (saw(40) + 0.5 * sine(80)) >> lowpass(2000, 0.7)
voice bass = triangle(55) >> lowpass(400, 1.0)
voice kick  = (0.7 * sine(55) + 0.5 * sine(30)) >> decay(12)
voice ghost_snare = 0.1 * noise() >> highpass(2000, 1.0) >> lowpass(5000, 0.6) >> decay(20)

bpm 80

at 0 play pad for 4 beats
at 0 play bass for 4 beats
at 2 play sine(440) for 1 beat
at 3 play sine(880) for 1 beat
at 5 play kick for 0.5 beats
at 5.25 play ghost_snare for 0.5 beats
at 5.33 play ghost_snare for 0.5 beats
at 6 play kick for 0.5 beats
at 6.25 play ghost_snare for 0.5 beats
at 6.44 play ghost_snare for 0.5 beats
at 7 play kick for 0.5 beats
at 7.25 play ghost_snare for 0.5 beats
at 7.33 play ghost_snare for 0.5 beats
at 8 play kick for 0.5 beats

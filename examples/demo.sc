// Sound Cabinet demo score
voice pad = (saw(40) + 0.5 * sine(80)) >> lowpass(2000, 0.7)
voice bass = triangle(55) >> lowpass(400, 1.0)

bpm 120

at 0 play pad for 4 beats
at 0 play bass for 4 beats
at 2 play sine(440) for 1 beat
at 3 play sine(880) for 1 beat

// Sound Cabinet demo score
voice pad = (saw(40) + 0.5 * sine(80)) >> lowpass(2000, 0.7)
voice bass = triangle(55) >> lowpass(400, 1.0)
voice kick  = (0.7 * sine(55) + 0.5 * sine(30)) >> decay(12)
voice ghost_snare = 0.1 * noise() >> highpass(2000, 1.0) >> lowpass(5000, 0.6) >> decay(20)
voice gamma_clicks = (0.3 * sine(10000) >> highpass(6000, 1.0)) * saw(40)
voice gamma_soft = (0.4 * noise() >> highpass(3000, 0.8) >> lowpass(9000, 0.6)) * triangle(40)
voice alpha_shimmer = 0.05 * (sine(200) + sine(210)) >> lowpass(400, 0.4)

bpm 80

at 0 play pad for 4 beats
at 0 play bass for 4 beats
at 2 play alpha_shimmer for 8 beats
at 2 play 0.5 * sine(440) for 1 beat
at 3 play 0.5 * sine(880) for 1 beat
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
at 9 play gamma_clicks for 1 beat 
at 10 play gamma_soft for 1 beat 
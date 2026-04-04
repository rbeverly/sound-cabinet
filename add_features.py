import re
import sys

with open("src/engine/effects.rs", "r") as f:
    content = f.read()

# 1. Add gain_reduction_db() to MasterStage
old_masterstage = """pub trait MasterStage: Send {
    fn process_stereo(&mut self, left: &mut [f32], right: &mut [f32]);
    fn name(&self) -> &'static str;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
}"""
new_masterstage = """pub trait MasterStage: Send {
    fn process_stereo(&mut self, left: &mut [f32], right: &mut [f32]);
    fn name(&self) -> &'static str;
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any;
    /// Return instantaneous gain reduction in dB
    fn gain_reduction_db(&self) -> f32 { 0.0 }
}"""
content = content.replace(old_masterstage, new_masterstage)

# 2. Add GR to MasterCompressor
# First we need to store it
old_mcomp_struct = """pub struct MasterCompressor {
    threshold: f32,    // dB
    ratio: f32,
    attack_coeff: f32,
    release_coeff: f32,
    envelope_sq: f32,  // squared RMS envelope (avoids sqrt per sample)
    makeup_gain: f32,  // compensate for gain reduction
    upward: bool,      // upward compression: boost quiet content instead of reducing loud
    knee_width: f32,   // soft knee width in dB (Giannoulis/Massberg/Reiss JAES 2012)
}"""
new_mcomp_struct = """pub struct MasterCompressor {
    threshold: f32,    // dB
    ratio: f32,
    attack_coeff: f32,
    release_coeff: f32,
    envelope_sq: f32,  // squared RMS envelope (avoids sqrt per sample)
    makeup_gain: f32,  // compensate for gain reduction
    upward: bool,      // upward compression: boost quiet content instead of reducing loud
    knee_width: f32,   // soft knee width in dB (Giannoulis/Massberg/Reiss JAES 2012)
    last_gr_db: f32,   // instantaneous gain reduction for metering
}"""
content = content.replace(old_mcomp_struct, new_mcomp_struct)

old_mcomp_new = """        let makeup_gain = 10.0_f32.powf(makeup_db.min(12.0) / 20.0);
        MasterCompressor {
            threshold: threshold_db,
            ratio: ratio.max(1.0),
            attack_coeff,
            release_coeff,
            envelope_sq: 0.0,
            makeup_gain,
            upward: false,
            knee_width: 6.0,
        }
    }"""
new_mcomp_new = """        let makeup_gain = 10.0_f32.powf(makeup_db.min(12.0) / 20.0);
        MasterCompressor {
            threshold: threshold_db,
            ratio: ratio.max(1.0),
            attack_coeff,
            release_coeff,
            envelope_sq: 0.0,
            makeup_gain,
            upward: false,
            knee_width: 6.0,
            last_gr_db: 0.0,
        }
    }"""
content = content.replace(old_mcomp_new, new_mcomp_new)

old_mcomp_proc_end = """            let gain = 10.0_f32.powf(gain_db / 20.0) * self.makeup_gain;
            *l *= gain;
            *r *= gain;
        }
    }"""
new_mcomp_proc_end = """            let gain = 10.0_f32.powf(gain_db / 20.0) * self.makeup_gain;
            self.last_gr_db = gain_db.min(0.0); // only track reduction, not makeup
            *l *= gain;
            *r *= gain;
        }
    }"""
content = content.replace(old_mcomp_proc_end, new_mcomp_proc_end)

# 3. Add GR to StageCompress
old_stage_comp_impl = """impl MasterStage for StageCompress {
    fn process_stereo(&mut self, left: &mut [f32], right: &mut [f32]) {
        self.compressor.process_stereo(left, right);
    }
    fn name(&self) -> &'static str { "compress" }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
}"""
new_stage_comp_impl = """impl MasterStage for StageCompress {
    fn process_stereo(&mut self, left: &mut [f32], right: &mut [f32]) {
        self.compressor.process_stereo(left, right);
    }
    fn name(&self) -> &'static str { "compress" }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any { self }
    fn gain_reduction_db(&self) -> f32 { self.compressor.last_gr_db }
}"""
content = content.replace(old_stage_comp_impl, new_stage_comp_impl)

# 4. MasterBus Bypass + GR + True Peak Lookahead
old_mb_struct = """pub struct MasterBus {
    // Highpass coefficients (shared across channels)
    hp_a1: f32,
    hp_a2: f32,
    hp_b0: f32,
    hp_b1: f32,
    hp_b2: f32,
    // Highpass per-channel state [left, right]
    hp_state: [BiquadState; 2],
    // Lowpass coefficients (shared across channels)
    lp_a1: f32,
    lp_a2: f32,
    lp_b0: f32,
    lp_b1: f32,
    lp_b2: f32,
    // Lowpass per-channel state [left, right]
    lp_state: [BiquadState; 2],
    // Stereo-linked Brickwall Limiter
    limiter: BrickwallLimiter,
    // Output gain (linear). Applied before everything else in the chain.
    gain: f32,
    // User-definable chain (between filters and limiter)
    chain: Vec<Box<dyn MasterStage>>,
}"""
new_mb_struct = """pub struct MasterBus {
    // Highpass coefficients (shared across channels)
    hp_a1: f32,
    hp_a2: f32,
    hp_b0: f32,
    hp_b1: f32,
    hp_b2: f32,
    // Highpass per-channel state [left, right]
    hp_state: [BiquadState; 2],
    // Lowpass coefficients (shared across channels)
    lp_a1: f32,
    lp_a2: f32,
    lp_b0: f32,
    lp_b1: f32,
    lp_b2: f32,
    // Lowpass per-channel state [left, right]
    lp_state: [BiquadState; 2],
    // Stereo-linked Brickwall Limiter
    limiter: BrickwallLimiter,
    // Output gain (linear). Applied before everything else in the chain.
    gain: f32,
    // User-definable chain (between filters and limiter)
    chain: Vec<Box<dyn MasterStage>>,
    pub bypass: bool,
    // For bypass loudness matching
    rms_dry: f32,
    rms_wet: f32,
}"""
content = content.replace(old_mb_struct, new_mb_struct)

old_mb_new2 = """            lp_state: [BiquadState::default(), BiquadState::default()],
            limiter,
            gain: 1.0,
            chain,
        }
    }"""
new_mb_new2 = """            lp_state: [BiquadState::default(), BiquadState::default()],
            limiter,
            gain: 1.0,
            chain,
            bypass: false,
            rms_dry: 0.0,
            rms_wet: 0.0,
        }
    }"""
content = content.replace(old_mb_new2, new_mb_new2)

old_mb_proc = """    pub fn process_stereo(&mut self, left: &mut [f32], right: &mut [f32]) {
        // Apply master gain first
        if (self.gain - 1.0).abs() > 1e-6 {
            for sample in left.iter_mut() {
                *sample *= self.gain;
            }
            for sample in right.iter_mut() {
                *sample *= self.gain;
            }
        }

        // HP + LP filters per channel
        self.process_channel(left, 0);
        self.process_channel(right, 1);

        // User chain
        for stage in &mut self.chain {
            stage.process_stereo(left, right);
        }

        // Limiter stereo
        self.limiter.process_stereo(left, right);
    }"""
new_mb_proc = """    pub fn process_stereo(&mut self, left: &mut [f32], right: &mut [f32]) {
        // Save dry buffers for bypass
        let dry_l = left.to_vec();
        let dry_r = right.to_vec();

        // Apply master gain first
        if (self.gain - 1.0).abs() > 1e-6 {
            for sample in left.iter_mut() { *sample *= self.gain; }
            for sample in right.iter_mut() { *sample *= self.gain; }
        }

        // HP + LP filters per channel
        self.process_channel(left, 0);
        self.process_channel(right, 1);

        // User chain
        for stage in &mut self.chain {
            stage.process_stereo(left, right);
        }

        // Limiter stereo
        self.limiter.process_stereo(left, right);

        // Calculate RMS for auto-gain volume matching during bypass
        let mut dry_sq = 0.0;
        let mut wet_sq = 0.0;
        for i in 0..left.len() {
            dry_sq += dry_l[i]*dry_l[i] + dry_r[i]*dry_r[i];
            wet_sq += left[i]*left[i] + right[i]*right[i];
        }
        let alpha = 0.005; // slow follower
        self.rms_dry = self.rms_dry * (1.0 - alpha) + dry_sq * alpha;
        self.rms_wet = self.rms_wet * (1.0 - alpha) + wet_sq * alpha;

        if self.bypass {
            let makeup = if self.rms_dry > 1e-6 { (self.rms_wet / self.rms_dry).sqrt() } else { 1.0 };
            // cap makeup to avoid exploding noise
            let makeup = makeup.min(10.0);
            for i in 0..left.len() {
                left[i] = dry_l[i] * makeup;
                right[i] = dry_r[i] * makeup;
            }
        }
    }
    
    pub fn current_gain_reduction(&self) -> f32 {
        if self.bypass { return 0.0; }
        let mut gr = 0.0;
        for stage in &self.chain {
            gr += stage.gain_reduction_db();
        }
        // Limiter gain reduction is tracked as linear needed gain. Convert to dB.
        if self.limiter.gain_reduction > 0.0 {
            let lim_gain_db = 20.0 * (1.0 - self.limiter.gain_reduction).log10();
            gr += lim_gain_db;
        }
        gr
    }"""
content = content.replace(old_mb_proc, new_mb_proc)

# True Peak processing via Hermite inside BrickwallLimiter
old_bl_struct = """pub struct BrickwallLimiter {
    ceiling: f32,
    release_coeff: f32,
    gain_reduction: f32, // current gain reduction (0.0 = no reduction, higher = more)
    lookahead_buf_l: Vec<f32>,
    lookahead_buf_r: Vec<f32>,
    lookahead_pos: usize,
    lookahead_len: usize,
    peak_hold_timer: usize,
}"""
new_bl_struct = """pub struct BrickwallLimiter {
    pub ceiling: f32,
    pub release_coeff: f32,
    pub gain_reduction: f32, // current gain reduction (0.0 = no reduction, higher = more)
    lookahead_buf_l: Vec<f32>,
    lookahead_buf_r: Vec<f32>,
    lookahead_pos: usize,
    lookahead_len: usize,
    peak_hold_timer: usize,
    history_l: [f32; 3], // for 4-point hermite true peak interpolation
    history_r: [f32; 3],
}"""
content = content.replace(old_bl_struct, new_bl_struct)

old_bl_new = """            lookahead_buf_r: vec![0.0; lookahead_len],
            lookahead_pos: 0,
            lookahead_len,
            peak_hold_timer: 0,
        }
    }"""
new_bl_new = """            lookahead_buf_r: vec![0.0; lookahead_len],
            lookahead_pos: 0,
            lookahead_len,
            peak_hold_timer: 0,
            history_l: [0.0; 3],
            history_r: [0.0; 3],
        }
    }
    
    #[inline(always)]
    fn hermite(y0: f32, y1: f32, y2: f32, y3: f32, mu: f32) -> f32 {
        let mu2 = mu * mu;
        let mu3 = mu2 * mu;
        let m0 = (y1 - y0) * 0.5 + (y2 - y1) * 0.5;
        let m1 = (y2 - y1) * 0.5 + (y3 - y2) * 0.5;
        let a0 = 2.0 * mu3 - 3.0 * mu2 + 1.0;
        let a1 = mu3 - 2.0 * mu2 + mu;
        let a2 = mu3 - mu2;
        let a3 = -2.0 * mu3 + 3.0 * mu2;
        a0 * y1 + a1 * m0 + a2 * m1 + a3 * y2
    }"""
content = content.replace(old_bl_new, new_bl_new)

old_bl_proc = """            // Compute required gain reduction from current (pre-delay) sample
            // Stereo linking: use the maximum absolute value of both channels
            let max_abs = l.abs().max(r.abs());
            let needed = if max_abs > self.ceiling {"""
new_bl_proc = """            // Compute required gain reduction using True Peak (4x Hermite interpolation)
            let mut max_abs = l.abs().max(r.abs());
            for mu in [0.25, 0.5, 0.75].iter() {
                let interp_l = Self::hermite(self.history_l[0], self.history_l[1], self.history_l[2], *l, *mu);
                let interp_r = Self::hermite(self.history_r[0], self.history_r[1], self.history_r[2], *r, *mu);
                max_abs = max_abs.max(interp_l.abs()).max(interp_r.abs());
            }
            self.history_l = [self.history_l[1], self.history_l[2], *l];
            self.history_r = [self.history_r[1], self.history_r[2], *r];

            let needed = if max_abs > self.ceiling {"""
content = content.replace(old_bl_proc, new_bl_proc)

with open("src/engine/effects.rs", "w") as f:
    f.write(content)
print("effects.rs replaced successfully")

with open("src/render/realtime.rs", "r") as f:
    realtime_content = f.read()

# Add keyboard listener to play_realtime_inner
old_rt_loop = """    // Track number of VU lines printed for terminal cleanup
    let mut vu_lines: usize = 0;

    // Wait until the engine finishes
    loop {
        std::thread::sleep(std::time::Duration::from_millis(50));
        let eng = engine.lock().unwrap();"""
new_rt_loop = """    // Track number of VU lines printed for terminal cleanup
    let mut vu_lines: usize = 0;

    use crossterm::event::{self, Event, KeyCode, KeyModifiers};

    // Wait until the engine finishes
    loop {
        std::thread::sleep(std::time::Duration::from_millis(50));
        
        // Handle Master Bypass shortcut: m or \\
        if crossterm::event::poll(std::time::Duration::from_millis(0)).unwrap_or(false) {
            if let Ok(Event::Key(key)) = crossterm::event::read() {
                if key.code == KeyCode::Char('m') || key.code == KeyCode::Char('\\\\') {
                    let mut eng = engine.lock().unwrap();
                    eng.master_bus.bypass = !eng.master_bus.bypass;
                    if eng.master_bus.bypass {
                        eprintln!("  \\x1b[93m[ MASTER BYPASSED ]\\x1b[0m");
                        vu_lines += 1;
                    } else {
                        eprintln!("  \\x1b[92m[ MASTER ACTIVE ]\\x1b[0m");
                        vu_lines += 1;
                    }
                }
                if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                    break;
                }
            }
        }

        let eng = engine.lock().unwrap();"""
realtime_content = realtime_content.replace(old_rt_loop, new_rt_loop)

# Add GR printing to the VuMeter block
old_vu_print = """                    let name_color = if level_db < -50.0 { "\\x1b[90m" } else { "\\x1b[97m" };

                    eprint!(
                        "  {}{:<14}\\x1b[0m {} {:>6.1} dB{}\\r\\n",
                        name_color, name, bar, level_db, status
                    );
                }
                use std::io::Write;"""
new_vu_print = """                    let name_color = if level_db < -50.0 { "\\x1b[90m" } else { "\\x1b[97m" };

                    eprint!(
                        "  {}{:<14}\\x1b[0m {} {:>6.1} dB{}\\r\\n",
                        name_color, name, bar, level_db, status
                    );
                }
                
                // Print Master Gain Reduction
                let gr = eng.master_bus.current_gain_reduction();
                if gr < -0.1 {
                    eprint!("  \\x1b[96m{:<14}\\x1b[0m       [ GR {:>5.1} dB ]\\r\\n", "MASTER", gr);
                    vu_lines += 1;
                } else {
                    eprint!("  \\x1b[90m{:<14}\\x1b[0m       [ GR   0.0 dB ]\\r\\n", "MASTER");
                    vu_lines += 1;
                }
                
                use std::io::Write;"""
realtime_content = realtime_content.replace(old_vu_print, new_vu_print)

with open("src/render/realtime.rs", "w") as f:
    f.write(realtime_content)
print("realtime.rs replaced successfully")

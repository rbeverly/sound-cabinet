import wave
import struct
import math
import sys
import os

def analyze_wav(filename):
    try:
        with wave.open(filename, 'rb') as wf:
            nchannels = wf.getnchannels()
            sampwidth = wf.getsampwidth()
            framerate = wf.getframerate()
            # Analyze up to 60 seconds to save time/memory, or whole file if shorter
            nframes = min(wf.getnframes(), framerate * 60) 
            data = wf.readframes(nframes)
            
            if sampwidth == 2:
                fmt = f"<{nframes * nchannels}h"
                max_val = 32768.0
            else:
                return "Unsupported bit depth (needs 16-bit PCM)"
                
            samples = struct.unpack(fmt, data)
            
            # Find peak
            peak = max((abs(s) for s in samples), default=0)
            peak_db = 20 * math.log10(peak / max_val) if peak > 0 else -float('inf')
            
            # Calculate RMS
            sum_sq = sum(float(s)**2 for s in samples)
            rms = math.sqrt(sum_sq / len(samples)) if len(samples) > 0 else 0
            rms_db = 20 * math.log10(rms / max_val) if rms > 0 else -float('inf')
            
            # Crest factor
            crest_factor = peak_db - rms_db
            
            return {
                'peak_db': peak_db,
                'rms_db': rms_db,
                'crest_factor': crest_factor
            }
    except Exception as e:
        return str(e)

def main():
    if len(sys.argv) < 2:
        print(f"Usage: python3 {os.path.basename(__file__)} <file1.wav> [file2.wav ...]")
        sys.exit(1)

    print("\n=== Waveform Dynamics Analysis ===")
    print(f"{'Filename':<30} | {'Peak (dBFS)':<12} | {'RMS (dBFS)':<12} | {'Crest Factor':<12}")
    print("-" * 75)

    for filepath in sys.argv[1:]:
        if not os.path.exists(filepath):
            print(f"{filepath[:27] + '...':<30} | File not found")
            continue
            
        result = analyze_wav(filepath)
        name = os.path.basename(filepath)
        if len(name) > 30:
            name = name[:27] + "..."
            
        if isinstance(result, dict):
            print(f"{name:<30} | {result['peak_db']:>8.2f} dBFS | {result['rms_db']:>8.2f} dBFS | {result['crest_factor']:>9.2f} dB")
        else:
            print(f"{name:<30} | Error: {result}")
            
    print()

if __name__ == "__main__":
    main()

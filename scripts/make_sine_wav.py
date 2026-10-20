#!/usr/bin/env python3
"""Write the four synthetic WAV fixtures: a one-second middle-C sine (MIDI note 60,
261.63 Hz) at 48000 and 22050 Hz, mono and stereo, 16-bit PCM.

Stereo files carry the same sine on both channels with the right at half the left's
amplitude, so a test can tell a channel swap or a mono fold from a faithful decode. A
5 ms linear fade at each end keeps the one-shot from clicking when it is heard.

Standard library only, and byte-reproducible: run it twice and `cmp` the outputs.

Usage: scripts/make_sine_wav.py [output_dir]   (default: crates/music_midi/fixtures)
"""
import math
import os
import struct
import sys
import wave

MIDDLE_C_HERTZ = 440.0 * 2 ** ((60 - 69) / 12)
DURATION_SECONDS = 1.0
FADE_SECONDS = 0.005
LEFT_AMPLITUDE = 0.5
RIGHT_AMPLITUDE = 0.25
SAMPLE_RATES = (48000, 22050)


def sine_frames(sample_rate, channel_amplitudes):
    frame_count = int(sample_rate * DURATION_SECONDS)
    fade_frames = int(sample_rate * FADE_SECONDS)
    frames = bytearray()
    for frame_index in range(frame_count):
        envelope = min(1.0, frame_index / fade_frames, (frame_count - 1 - frame_index) / fade_frames)
        value = math.sin(2 * math.pi * MIDDLE_C_HERTZ * frame_index / sample_rate) * envelope
        for amplitude in channel_amplitudes:
            frames += struct.pack("<h", round(value * amplitude * 32767))
    return bytes(frames)


def write_wav(path, sample_rate, channel_amplitudes):
    with wave.open(path, "wb") as wav_file:
        wav_file.setnchannels(len(channel_amplitudes))
        wav_file.setsampwidth(2)
        wav_file.setframerate(sample_rate)
        wav_file.writeframes(sine_frames(sample_rate, channel_amplitudes))


def main():
    output_dir = sys.argv[1] if len(sys.argv) > 1 else os.path.join(
        os.path.dirname(os.path.abspath(__file__)), "..", "crates", "music_midi", "fixtures")
    os.makedirs(output_dir, exist_ok=True)
    layouts = {"mono": (LEFT_AMPLITUDE,), "stereo": (LEFT_AMPLITUDE, RIGHT_AMPLITUDE)}
    for sample_rate in SAMPLE_RATES:
        for layout_name, channel_amplitudes in layouts.items():
            path = os.path.join(output_dir, f"sine_{sample_rate}_{layout_name}.wav")
            write_wav(path, sample_rate, channel_amplitudes)
            print(path)


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Derive `dansenapolitaine-cast.mid`, the Napolitan Dance with its instruments cast,
from the type 0 fixture `dansenapolitaine-type0.mid`.

LilyPond sets no program, so every melodic channel of the type 0 file plays the General
MIDI default, the piano, and a listener cannot tell its parts apart. One transformation,
and nothing else: a program change at tick 0 on each melodic channel, inserted right
after the track name — the flute's channel 2 (index 1) gets program 73, Flute, and the
two piano channels 4 and 5 (indexes 3 and 4) program 0, Acoustic Grand Piano. Channel 10
(index 9), where the type 0 derivation moved the tambourine, is General MIDI percussion
and needs no program.

The script refuses a source whose channels are not exactly those four, so a changed
source cannot be cast wrongly in silence. Every status byte is written in full, as the
source writes them. Standard library only, and byte-reproducible: run it twice and `cmp`.

Usage: scripts/make_cast_fixture.py [source_path] [output_path]
       (default: crates/music_midi/fixtures/dansenapolitaine-type0.mid ->
                 crates/music_midi/fixtures/dansenapolitaine-cast.mid)
"""
import os
import struct
import sys

PERCUSSION_CHANNEL = 9
# Channel index -> General MIDI program, counted from zero.
CAST = {1: 73, 3: 0, 4: 0}
PROGRAM_CHANGE = 0xC0
META_TRACK_NAME = 0x03


def read_variable_length(data, position):
    value = 0
    while True:
        byte = data[position]
        position += 1
        value = (value << 7) | (byte & 0x7F)
        if not byte & 0x80:
            return value, position


def event_end(data, position):
    """The end of the event whose delta starts at `position`, and its status byte."""
    _, position = read_variable_length(data, position)
    status = data[position]
    if status == 0xFF:
        length, payload_start = read_variable_length(data, position + 2)
        return payload_start + length, status, data[position + 1]
    if status in (0xF0, 0xF7):
        length, payload_start = read_variable_length(data, position + 1)
        return payload_start + length, status, None
    if not status & 0x80:
        raise ValueError("running status: expected every status byte in full")
    size = 1 if status & 0xF0 in (0xC0, 0xD0) else 2
    return position + 1 + size, status, None


def channels_used(track):
    channels = set()
    position = 0
    while position < len(track):
        position, status, _ = event_end(track, position)
        if status < 0xF0:
            channels.add(status & 0x0F)
    return channels


def cast_bytes(source):
    if source[:4] != b"MThd":
        raise ValueError("not a Standard MIDI File")
    header_length = struct.unpack(">I", source[4:8])[0]
    smf_format, track_count, _ = struct.unpack(">HHH", source[8:14])
    if smf_format != 0 or track_count != 1:
        raise ValueError(f"expected SMF type 0 with one track, found type {smf_format}")
    track_start = 8 + header_length
    if source[track_start:track_start + 4] != b"MTrk":
        raise ValueError("expected the track chunk after the header")
    track_length = struct.unpack(">I", source[track_start + 4:track_start + 8])[0]
    track = source[track_start + 8:track_start + 8 + track_length]

    channels = channels_used(track)
    expected = set(CAST) | {PERCUSSION_CHANNEL}
    if channels != expected:
        raise ValueError(f"expected channels {sorted(expected)}, found {sorted(channels)}")
    first_end, status, meta_type = event_end(track, 0)
    if status != 0xFF or meta_type != META_TRACK_NAME or track[0] != 0:
        raise ValueError("expected the track name at tick 0 as the first event")

    programs = b"".join(
        bytes([0, PROGRAM_CHANGE | channel, program]) for channel, program in sorted(CAST.items())
    )
    cast_track = track[:first_end] + programs + track[first_end:]
    return (
        source[:track_start]
        + b"MTrk"
        + struct.pack(">I", len(cast_track))
        + cast_track
        + source[track_start + 8 + track_length:]
    )


def main():
    fixtures = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "crates", "music_midi", "fixtures")
    source_path = sys.argv[1] if len(sys.argv) > 1 else os.path.join(fixtures, "dansenapolitaine-type0.mid")
    output_path = sys.argv[2] if len(sys.argv) > 2 else os.path.join(fixtures, "dansenapolitaine-cast.mid")
    with open(source_path, "rb") as source_file:
        output = cast_bytes(source_file.read())
    with open(output_path, "wb") as output_file:
        output_file.write(output)
    print(output_path)


if __name__ == "__main__":
    main()

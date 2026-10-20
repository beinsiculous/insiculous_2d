#!/usr/bin/env python3
"""Derive `dansenapolitaine-type0.mid`, the SMF type 0 fixture that mixes melody and
percussion channels, from Mutopia's type 1 `dansenapolitaine.mid`.

Mutopia's LilyPond output is always type 1, so the conductor's type 0 path needs a
derived file. Two transformations, and nothing else:

1. Every track's events are merged into the one track a type 0 file has, in absolute
   tick order (ties keep track order, then file order). Each track's own end-of-track,
   track-name and repeated key-signature events are dropped; one end-of-track closes the
   merged track at the latest tick, and the track name says what the file is.
2. The `tambourin` track is moved to General MIDI percussion channel 10 (index 9) as
   key 54, Tambourine. LilyPond writes an unpitched rhythmic staff as middle C on an
   ordinary melodic channel, which a General MIDI synthesizer plays as a piano note; on
   channel 10 it is the percussion part the score means.

Every status byte is written in full (no running status), so the output is easy to read
in a hex dump. Standard library only, and byte-reproducible: run it twice and `cmp`.

Usage: scripts/make_type0_fixture.py [source_path] [output_path]
       (default: crates/music_midi/fixtures/dansenapolitaine.mid ->
                 crates/music_midi/fixtures/dansenapolitaine-type0.mid)
"""
import os
import struct
import sys

PERCUSSION_TRACK_NAME = b"tambourin"
PERCUSSION_CHANNEL = 9
TAMBOURINE_KEY = 54
MERGED_TRACK_NAME = b"Napolitan Dance (type 0, derived)"
META_TRACK_NAME = 0x03
META_END_OF_TRACK = 0x2F
META_KEY_SIGNATURE = 0x59


def read_variable_length(data, position):
    value = 0
    while True:
        byte = data[position]
        position += 1
        value = (value << 7) | (byte & 0x7F)
        if not byte & 0x80:
            return value, position


def write_variable_length(value):
    encoded = [value & 0x7F]
    value >>= 7
    while value:
        encoded.append((value & 0x7F) | 0x80)
        value >>= 7
    return bytes(reversed(encoded))


def parse_track(data):
    """Yields (absolute_tick, status, payload) with running status resolved."""
    position = 0
    tick = 0
    running_status = None
    while position < len(data):
        delta, position = read_variable_length(data, position)
        tick += delta
        status = data[position]
        if status == 0xFF:
            length, payload_start = read_variable_length(data, position + 2)
            payload = data[position + 1:position + 2] + data[position + 2:payload_start + length]
            yield tick, status, payload
            position = payload_start + length
        elif status in (0xF0, 0xF7):
            length, payload_start = read_variable_length(data, position + 1)
            yield tick, status, data[position + 1:payload_start + length]
            position = payload_start + length
        else:
            if status & 0x80:
                running_status = status
                position += 1
            else:
                status = running_status
            size = 1 if status & 0xF0 in (0xC0, 0xD0) else 2
            yield tick, status, data[position:position + size]
            position += size


def read_tracks(data):
    if data[:4] != b"MThd":
        raise ValueError("not a Standard MIDI File")
    header_length = struct.unpack(">I", data[4:8])[0]
    smf_format, _, division = struct.unpack(">HHH", data[8:14])
    if smf_format != 1:
        raise ValueError(f"expected SMF type 1, found type {smf_format}")
    position = 8 + header_length
    tracks = []
    while position < len(data):
        chunk_length = struct.unpack(">I", data[position + 4:position + 8])[0]
        if data[position:position + 4] == b"MTrk":
            tracks.append(list(parse_track(data[position + 8:position + 8 + chunk_length])))
        position += 8 + chunk_length
    return division, tracks


def track_name(events):
    for _, status, payload in events:
        if status == 0xFF and payload[0] == META_TRACK_NAME:
            return payload[2:]
    return b""


def merged_events(tracks):
    merged = []
    key_signature_seen = False
    for track_index, events in enumerate(tracks):
        is_percussion = track_name(events) == PERCUSSION_TRACK_NAME
        for event_index, (tick, status, payload) in enumerate(events):
            if status == 0xFF:
                meta_type = payload[0]
                if meta_type in (META_END_OF_TRACK, META_TRACK_NAME):
                    continue
                if meta_type == META_KEY_SIGNATURE:
                    if key_signature_seen:
                        continue
                    key_signature_seen = True
            elif is_percussion and status & 0xF0 in (0x80, 0x90):
                status = (status & 0xF0) | PERCUSSION_CHANNEL
                payload = bytes([TAMBOURINE_KEY]) + payload[1:]
            merged.append((tick, track_index, event_index, status, payload))
    merged.sort(key=lambda event: event[:3])
    return merged


def encode_event(status, payload):
    if status == 0xFF:
        meta_type, body = payload[0], payload[1:]
        # The parser kept the length bytes inside the payload; re-encode them cleanly.
        length, body_start = read_variable_length(body, 0)
        return bytes([status, meta_type]) + write_variable_length(length) + body[body_start:]
    if status in (0xF0, 0xF7):
        length, body_start = read_variable_length(payload, 0)
        return bytes([status]) + write_variable_length(length) + payload[body_start:]
    return bytes([status]) + payload


def type0_bytes(source):
    division, tracks = read_tracks(source)
    track_body = bytearray()
    track_body += b"\x00\xff" + bytes([META_TRACK_NAME]) + write_variable_length(len(MERGED_TRACK_NAME))
    track_body += MERGED_TRACK_NAME
    previous_tick = 0
    events = merged_events(tracks)
    for tick, _, _, status, payload in events:
        track_body += write_variable_length(tick - previous_tick) + encode_event(status, payload)
        previous_tick = tick
    last_tick = max(tick for events_of_track in tracks for tick, _, _ in events_of_track)
    track_body += write_variable_length(last_tick - previous_tick) + b"\xff\x2f\x00"
    header = b"MThd" + struct.pack(">IHHH", 6, 0, 1, division)
    return header + b"MTrk" + struct.pack(">I", len(track_body)) + bytes(track_body)


def main():
    fixtures = os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "crates", "music_midi", "fixtures")
    source_path = sys.argv[1] if len(sys.argv) > 1 else os.path.join(fixtures, "dansenapolitaine.mid")
    output_path = sys.argv[2] if len(sys.argv) > 2 else os.path.join(fixtures, "dansenapolitaine-type0.mid")
    with open(source_path, "rb") as source_file:
        output = type0_bytes(source_file.read())
    with open(output_path, "wb") as output_file:
        output_file.write(output)
    print(output_path)


if __name__ == "__main__":
    main()

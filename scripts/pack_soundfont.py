#!/usr/bin/env python3
"""Pack finite mono PCM16 WAV zones into a deterministic, nonlooping SoundFont."""
import json
import math
import os
from pathlib import Path
import struct
import tempfile
import wave

LIMIT = 4 * 1024 * 1024


def chunk(tag, payload):
    return tag + struct.pack('<I', len(payload)) + payload + bytes(len(payload) % 2)


def text_chunk(tag, value):
    payload = value.encode("ascii") + b"\0"
    return chunk(tag, payload + bytes(len(payload) % 2))


def listing(tag, chunks):
    return chunk(b'LIST', tag + b''.join(chunks))


def name(value):
    encoded = value.encode('ascii')
    if not 0 < len(encoded) <= 19:
        raise ValueError('names must contain 1–19 ASCII bytes')
    return encoded.ljust(20, b'\0')


def generator(operator, value):
    return struct.pack('<HH', operator, value & 65535)


def pack(recipe_path):
    recipe_path = Path(recipe_path)
    recipe = json.loads(recipe_path.read_text())
    if recipe['version'] != 1:
        raise ValueError('unsupported recipe version')
    samples = bytearray()
    headers = bytearray()
    instruments = bytearray()
    bags = bytearray()
    generators = bytearray()
    presets = bytearray()
    preset_bags = bytearray()
    preset_generators = bytearray()
    zone_count = 0
    seen = set()
    for preset_index, preset in enumerate(recipe['presets']):
        identity = (preset['bank'], preset['program'])
        if identity in seen or identity[0] not in (0, 128) or not 0 <= identity[1] <= 127:
            raise ValueError('duplicate or invalid preset')
        seen.add(identity)
        presets += name(preset['name']) + struct.pack('<HHHIII', identity[1], identity[0], preset_index, 0, 0, 0)
        preset_bags += struct.pack('<HH', preset_index, 0)
        preset_generators += generator(41, preset_index)
        instruments += name(preset['name']) + struct.pack('<H', zone_count)
        covered = set()
        for zone in preset['zones']:
            lower, upper = zone['keys']
            root = zone['root']
            if not 0 <= lower <= root <= upper <= 127 or covered.intersection(range(lower, upper + 1)):
                raise ValueError('invalid or overlapping key zone')
            covered.update(range(lower, upper + 1))
            release = zone['release_seconds']
            if not 0.001 <= release <= 1:
                raise ValueError('release must be 1 ms–1 second')
            source = recipe_path.parent / zone['wav']
            if source.stat().st_size > LIMIT:
                raise ValueError('WAV exceeds bank budget')
            with wave.open(str(source), 'rb') as wav:
                if wav.getnchannels() != 1 or wav.getsampwidth() != 2 or wav.getcomptype() != 'NONE':
                    raise ValueError('expected mono PCM16 WAV')
                rate, frames = wav.getframerate(), wav.getnframes()
                if not 8000 <= rate <= 96000 or not 48 <= frames <= rate * 10:
                    raise ValueError('invalid sample rate or duration')
                audio = wav.readframes(frames)
                if len(audio) != frames * 2:
                    raise ValueError('truncated WAV')
            duration = zone['longest_note_seconds']
            if not math.isfinite(duration) or duration <= 0:
                raise ValueError('note duration must be finite and positive')
            required = (duration + release) * 2 ** ((upper - root) / 12)
            if frames / rate < required:
                raise ValueError('sample too short for note plus release at highest key')
            start = len(samples) // 2
            samples += audio + bytes(92)
            if len(samples) > LIMIT:
                raise ValueError('bank exceeds 4 MiB')
            headers += name(f'sample{zone_count}') + struct.pack('<IIIIIBbHH', start, start + frames,
                start + 8, start + frames - 8, rate, root, 0, 0, 1)
            bags += struct.pack('<HH', len(generators) // 4, 0)
            generators += generator(43, lower | upper << 8)
            generators += generator(38, round(1200 * math.log2(release)))
            generators += generator(54, 0) + generator(53, zone_count)
            zone_count += 1
    if not zone_count:
        raise ValueError('empty bank')
    presets += name('EOP') + struct.pack('<HHHIII', 0, 0, len(seen), 0, 0, 0)
    preset_bags += struct.pack('<HH', len(seen), 0)
    preset_generators += generator(0, 0)
    instruments += name('EOI') + struct.pack('<H', zone_count)
    bags += struct.pack('<HH', len(generators) // 4, 0)
    generators += generator(0, 0)
    headers += name('EOS') + bytes(26)
    info = listing(b'INFO', [chunk(b'ifil', struct.pack('<HH', 2, 1)), chunk(b'isng', b'EMU8000\0'),
        text_chunk(b'INAM', recipe['name']),
        text_chunk(b'ICOP', recipe['copyright'])])
    tables = [(b'phdr', presets), (b'pbag', preset_bags), (b'pmod', bytes(10)),
        (b'pgen', preset_generators), (b'inst', instruments), (b'ibag', bags),
        (b'imod', bytes(10)), (b'igen', generators), (b'shdr', headers)]
    body = b'sfbk' + info + listing(b'sdta', [chunk(b'smpl', samples)]) + listing(b'pdta', [chunk(tag, data) for tag, data in tables])
    result = chunk(b'RIFF', body)
    if len(result) > LIMIT:
        raise ValueError('bank exceeds 4 MiB including metadata')
    return result


def write(recipe, output):
    data = pack(recipe)
    output = Path(output)
    with tempfile.NamedTemporaryFile(dir=output.parent, delete=False) as temporary:
        temporary_path = Path(temporary.name)
        try:
            temporary.write(data)
            temporary.flush()
            os.fsync(temporary.fileno())
            os.replace(temporary_path, output)
        finally:
            temporary_path.unlink(missing_ok=True)


if __name__ == '__main__':
    import argparse
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('recipe')
    parser.add_argument('output')
    arguments = parser.parse_args()
    write(arguments.recipe, arguments.output)

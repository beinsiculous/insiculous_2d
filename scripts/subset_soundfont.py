#!/usr/bin/env python3
"""Cut a SoundFont 2 down to the presets a game needs.

Reads an SF2, keeps the presets named by `bank:program`, the instruments those presets
play and the samples those instruments play, re-indexes all of it, and writes a valid SF2
holding nothing else. Generators, modulators and zones are copied unchanged apart from the
two generators that are indices (`instrument` and `sampleID`), which follow the re-index.
Subsetting is selection, not authoring: no sample point, envelope or tuning is edited.

Two things change shape on the way, both because the original would point at data that is
no longer in the file:
- a stereo sample whose partner is not kept becomes a mono sample (rustysynth and
  FluidSynth place a voice with the zone's `pan` generator, not with the sample link, so
  the sound is unchanged);
- the optional `sm24` chunk (24-bit low bytes) is dropped; the 16-bit samples remain.

Usage:
  scripts/subset_soundfont.py --sizes SOURCE.sf2 [bank:program ...]
      print each preset's name and the bytes of sample data it alone would cost
  scripts/subset_soundfont.py SOURCE.sf2 OUTPUT.sf2 bank:program [bank:program ...]
      write the subset and print its preset list and size

Standard library only; the output is a function of the source bytes and the argument
order, so the same command reproduces the same file.
"""
import struct
import sys

RECORD_FORMATS = {
    b"phdr": "<20sHHHIII",
    b"pbag": "<HH",
    b"pmod": "<HHhHH",
    # Generator amounts are read and written unsigned, so a signed amount (a negative
    # tuning, a release in timecents) keeps its exact bit pattern through the copy.
    b"pgen": "<HH",
    b"inst": "<20sH",
    b"ibag": "<HH",
    b"imod": "<HHhHH",
    b"igen": "<HH",
    b"shdr": "<20sIIIIIBbHH",
}
GENERATOR_INSTRUMENT = 41
GENERATOR_SAMPLE_ID = 53
SAMPLE_TYPE_MONO = 1
SAMPLE_TYPE_ROM_FLAG = 0x8000
# The SoundFont 2.01 specification asks for at least 46 zero points after every sample.
TRAILING_ZERO_POINTS = 46


def read_chunks(data, start, end):
    """Yields (identifier, payload_start, payload_end) for each chunk in data[start:end]."""
    position = start
    while position + 8 <= end:
        identifier = data[position:position + 4]
        length = struct.unpack("<I", data[position + 4:position + 8])[0]
        yield identifier, position + 8, position + 8 + length
        position += 8 + length + (length & 1)


def read_soundfont(data):
    if data[:4] != b"RIFF" or data[8:12] != b"sfbk":
        raise ValueError("not a SoundFont 2 file")
    lists = {}
    for identifier, payload_start, payload_end in read_chunks(data, 12, len(data)):
        if identifier == b"LIST":
            list_type = data[payload_start:payload_start + 4]
            lists[list_type] = {
                sub_identifier: data[sub_start:sub_end]
                for sub_identifier, sub_start, sub_end in read_chunks(data, payload_start + 4, payload_end)
            }
    hydra = {}
    for identifier, record_format in RECORD_FORMATS.items():
        payload = lists[b"pdta"][identifier]
        hydra[identifier] = [list(record) for record in struct.iter_unpack(record_format, payload)]
    return lists[b"INFO"], lists[b"sdta"][b"smpl"], hydra


def zone_ranges(headers, bag_field, bags, generator_field):
    """For each header (terminal excluded), the list of (generator_start, generator_end,
    modulator_start, modulator_end) of its zones."""
    zones_by_header = []
    for header_index in range(len(headers) - 1):
        zones = []
        for bag_index in range(headers[header_index][bag_field], headers[header_index + 1][bag_field]):
            zones.append((bags[bag_index][generator_field], bags[bag_index + 1][generator_field],
                          bags[bag_index][1], bags[bag_index + 1][1]))
        zones_by_header.append(zones)
    return zones_by_header


def name_of(record):
    return record[0].split(b"\0", 1)[0].decode("latin-1")


class SoundFont:
    def __init__(self, data):
        self.info, self.sample_data, self.hydra = read_soundfont(data)
        hydra = self.hydra
        self.preset_zones = zone_ranges(hydra[b"phdr"], 3, hydra[b"pbag"], 0)
        self.instrument_zones = zone_ranges(hydra[b"inst"], 1, hydra[b"ibag"], 0)

    def preset_index(self, bank, program):
        for index, header in enumerate(self.hydra[b"phdr"][:-1]):
            if header[2] == bank and header[1] == program:
                return index
        raise ValueError(f"no preset at bank {bank} program {program}")

    def referenced(self, zones_list, generators, index, operator):
        found = []
        for generator_start, generator_end, _, _ in zones_list[index]:
            for generator in generators[generator_start:generator_end]:
                if generator[0] == operator and generator[1] not in found:
                    found.append(generator[1])
        return found

    def closure(self, preset_indices):
        """The instruments and samples the given presets play, in original index order."""
        instruments, samples = set(), set()
        for preset in preset_indices:
            instruments.update(self.referenced(self.preset_zones, self.hydra[b"pgen"], preset, GENERATOR_INSTRUMENT))
        for instrument in instruments:
            samples.update(self.referenced(self.instrument_zones, self.hydra[b"igen"], instrument, GENERATOR_SAMPLE_ID))
        for sample in list(samples):
            _, _, _, _, _, _, _, _, link, sample_type = self.hydra[b"shdr"][sample]
            if sample_type != SAMPLE_TYPE_MONO and sample_type & SAMPLE_TYPE_ROM_FLAG == 0:
                samples.add(link)
        return sorted(instruments), sorted(samples)

    def sample_bytes(self, samples):
        return sum((self.hydra[b"shdr"][sample][2] - self.hydra[b"shdr"][sample][1]) * 2 for sample in samples)


def chunk(identifier, payload):
    return identifier + struct.pack("<I", len(payload)) + payload + (b"\0" if len(payload) & 1 else b"")


def list_chunk(list_type, chunks):
    return chunk(b"LIST", list_type + b"".join(chunks))


def info_text(text):
    encoded = text.encode("latin-1") + b"\0"
    return encoded + (b"\0" if len(encoded) & 1 else b"")


def pack_records(identifier, records):
    record_format = RECORD_FORMATS[identifier]
    return b"".join(struct.pack(record_format, *record) for record in records)


def copy_zones(zones, generators, modulators, operator, index_map, bags_out, generators_out, modulators_out):
    for generator_start, generator_end, modulator_start, modulator_end in zones:
        bags_out.append([len(generators_out), len(modulators_out)])
        for generator in generators[generator_start:generator_end]:
            if generator[0] == operator:
                generators_out.append([operator, index_map[generator[1]]])
            else:
                generators_out.append(list(generator))
        modulators_out.extend(list(modulator) for modulator in modulators[modulator_start:modulator_end])


def subset(soundfont, preset_indices, source_name):
    hydra = soundfont.hydra
    instruments, samples = soundfont.closure(preset_indices)
    instrument_map = {old: new for new, old in enumerate(instruments)}
    sample_map = {old: new for new, old in enumerate(samples)}

    preset_headers, preset_bags, preset_generators, preset_modulators = [], [], [], []
    for preset in preset_indices:
        header = list(hydra[b"phdr"][preset])
        header[3] = len(preset_bags)
        preset_headers.append(header)
        copy_zones(soundfont.preset_zones[preset], hydra[b"pgen"], hydra[b"pmod"], GENERATOR_INSTRUMENT,
                   instrument_map, preset_bags, preset_generators, preset_modulators)

    instrument_headers, instrument_bags, instrument_generators, instrument_modulators = [], [], [], []
    for instrument in instruments:
        header = list(hydra[b"inst"][instrument])
        header[1] = len(instrument_bags)
        instrument_headers.append(header)
        copy_zones(soundfont.instrument_zones[instrument], hydra[b"igen"], hydra[b"imod"], GENERATOR_SAMPLE_ID,
                   sample_map, instrument_bags, instrument_generators, instrument_modulators)

    sample_points = bytearray()
    sample_headers = []
    for sample in samples:
        name, start, end, loop_start, loop_end, rate, pitch, correction, link, sample_type = hydra[b"shdr"][sample]
        new_start = len(sample_points) // 2
        sample_points += soundfont.sample_data[start * 2:end * 2] + b"\0\0" * TRAILING_ZERO_POINTS
        shift = new_start - start
        if sample_type != SAMPLE_TYPE_MONO and link in sample_map:
            link = sample_map[link]
        else:
            link, sample_type = 0, SAMPLE_TYPE_MONO
        sample_headers.append([name, new_start, end + shift, loop_start + shift, loop_end + shift,
                               rate, pitch, correction, link, sample_type])

    preset_headers.append([b"EOP", 0, 0, len(preset_bags), 0, 0, 0])
    preset_bags.append([len(preset_generators), len(preset_modulators)])
    preset_generators.append([0, 0])
    preset_modulators.append([0, 0, 0, 0, 0])
    instrument_headers.append([b"EOI", len(instrument_bags)])
    instrument_bags.append([len(instrument_generators), len(instrument_modulators)])
    instrument_generators.append([0, 0])
    instrument_modulators.append([0, 0, 0, 0, 0])
    sample_headers.append([b"EOS", 0, 0, 0, 0, 0, 0, 0, 0, 0])

    preset_list = ", ".join(f"{name_of(hydra[b'phdr'][preset])} ({hydra[b'phdr'][preset][2]}:{hydra[b'phdr'][preset][1]})"
                            for preset in preset_indices)
    original_name = soundfont.info.get(b"INAM", b"").split(b"\0", 1)[0].decode("latin-1")
    original_copyright = soundfont.info.get(b"ICOP", b"").split(b"\0", 1)[0].decode("latin-1")
    info_chunks = [
        chunk(b"ifil", soundfont.info[b"ifil"]),
        chunk(b"isng", info_text("EMU8000")),
        chunk(b"INAM", info_text(f"{original_name} subset")),
        chunk(b"ICMT", info_text(f"Subset of {source_name} made by scripts/subset_soundfont.py: {preset_list}")),
    ]
    if original_copyright:
        info_chunks.append(chunk(b"ICOP", info_text(original_copyright)))
    body = b"sfbk" + list_chunk(b"INFO", info_chunks) + list_chunk(b"sdta", [chunk(b"smpl", bytes(sample_points))])
    body += list_chunk(b"pdta", [
        chunk(b"phdr", pack_records(b"phdr", preset_headers)),
        chunk(b"pbag", pack_records(b"pbag", preset_bags)),
        chunk(b"pmod", pack_records(b"pmod", preset_modulators)),
        chunk(b"pgen", pack_records(b"pgen", preset_generators)),
        chunk(b"inst", pack_records(b"inst", instrument_headers)),
        chunk(b"ibag", pack_records(b"ibag", instrument_bags)),
        chunk(b"imod", pack_records(b"imod", instrument_modulators)),
        chunk(b"igen", pack_records(b"igen", instrument_generators)),
        chunk(b"shdr", pack_records(b"shdr", sample_headers)),
    ])
    return b"RIFF" + struct.pack("<I", len(body)) + body, preset_list


def parse_preset_arguments(soundfont, arguments):
    indices = []
    for argument in arguments:
        bank, program = (int(part) for part in argument.split(":"))
        indices.append(soundfont.preset_index(bank, program))
    return indices


def print_sizes(soundfont, preset_indices):
    if not preset_indices:
        preset_indices = range(len(soundfont.hydra[b"phdr"]) - 1)
    for preset in sorted(preset_indices, key=lambda index: (soundfont.hydra[b"phdr"][index][2], soundfont.hydra[b"phdr"][index][1])):
        header = soundfont.hydra[b"phdr"][preset]
        _, samples = soundfont.closure([preset])
        print(f"{header[2]:3}:{header[1]:<3} {name_of(header):20} {len(samples):4} samples "
              f"{soundfont.sample_bytes(samples) / 1024:9.1f} KiB")


def main():
    arguments = sys.argv[1:]
    if arguments and arguments[0] == "--sizes":
        source_path = arguments[1]
        with open(source_path, "rb") as source_file:
            soundfont = SoundFont(source_file.read())
        print_sizes(soundfont, parse_preset_arguments(soundfont, arguments[2:]))
        return
    if len(arguments) < 3:
        sys.exit(__doc__)
    source_path, output_path, preset_arguments = arguments[0], arguments[1], arguments[2:]
    with open(source_path, "rb") as source_file:
        soundfont = SoundFont(source_file.read())
    output, preset_list = subset(soundfont, parse_preset_arguments(soundfont, preset_arguments),
                                 source_path.rsplit("/", 1)[-1])
    with open(output_path, "wb") as output_file:
        output_file.write(output)
    print(f"{output_path}: {len(output)} bytes ({len(output) / 1024 / 1024:.2f} MiB): {preset_list}")


if __name__ == "__main__":
    main()

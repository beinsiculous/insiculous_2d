#!/usr/bin/env python3
"""Exercise decoded PCM, atomic refusal and deterministic SoundFont packaging."""
import copy
import json
from pathlib import Path
import tempfile
import unittest
import wave
import pack_soundfont


class PackingTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.home = Path(self.directory.name)
        with wave.open(str(self.home/'note.wav'), 'wb') as output:
            output.setparams((1, 2, 32000, 32000, 'NONE', 'not compressed'))
            output.writeframes(b'\x00\x10' * 32000)
        self.recipe = dict(version=1, name='Test', copyright='CC0', presets=[dict(name='Note', bank=0, program=0,
            zones=[dict(wav='note.wav', keys=[60,60], root=60, release_seconds=0.04, longest_note_seconds=0.2)])])
        self.path = self.home/'recipe.json'
        self.output = self.home/'bank.sf2'

    def write_recipe(self):
        self.path.write_text(json.dumps(self.recipe))

    def test_decoded_pcm_is_preserved_and_rebuild_is_identical(self):
        self.write_recipe()
        first = pack_soundfont.pack(self.path)
        self.assertEqual(first, pack_soundfont.pack(self.path))
        self.assertIn(b'\x00\x10' * 32000, first)
        self.assertTrue(first.startswith(b'RIFF'))

    def test_invalid_note_duration_preserves_previous_output(self):
        self.output.write_bytes(b'previous bank')
        for duration in [float('nan'), float('inf'), -1, 0]:
            with self.subTest(duration=duration):
                self.recipe['presets'][0]['zones'][0]['longest_note_seconds'] = duration
                self.write_recipe()
                with self.assertRaisesRegex(ValueError, 'finite and positive'):
                    pack_soundfont.write(self.path, self.output)
                self.assertEqual(self.output.read_bytes(), b'previous bank')

    def test_malformed_wav_does_not_overwrite_existing_bank(self):
        self.write_recipe()
        self.output.write_bytes(b'previous bank')
        (self.home/'note.wav').write_bytes(b'not a wave')
        with self.assertRaises((wave.Error, EOFError)):
            pack_soundfont.write(self.path, self.output)
        self.assertEqual(self.output.read_bytes(), b'previous bank')

    def test_over_budget_and_short_sample_refuse_before_overwrite(self):
        self.output.write_bytes(b'previous bank')
        original = copy.deepcopy(self.recipe['presets'][0])
        self.recipe['presets'] = [dict(original, program=program) for program in range(70)]
        self.write_recipe()
        with self.assertRaisesRegex(ValueError, '4 MiB'):
            pack_soundfont.write(self.path, self.output)
        self.assertEqual(self.output.read_bytes(), b'previous bank')
        self.recipe['presets'] = [original]
        original['zones'][0]['longest_note_seconds'] = 2
        self.write_recipe()
        with self.assertRaisesRegex(ValueError, 'too short'):
            pack_soundfont.write(self.path, self.output)
        self.assertEqual(self.output.read_bytes(), b'previous bank')


if __name__ == '__main__':
    unittest.main()

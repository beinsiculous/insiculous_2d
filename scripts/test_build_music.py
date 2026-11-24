#!/usr/bin/env python3
"""Source preservation checks for the shared authoring command."""
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

sys.dont_write_bytecode = True
from build_music import SNAKE_CASE_PATTERN

TOOL = Path(__file__).with_name('build_music.py')


class SourcePreservationTests(unittest.TestCase):
    def test_source_directory_and_aliased_recipe_are_refused_without_writes(self):
        with tempfile.TemporaryDirectory() as temporary:
            source = Path(temporary) / 'source'
            source.mkdir()
            project = source / 'song.evil'
            project.write_text('authored source')
            bank = source / 'bank.json'
            bank.write_text('{"presets": []}')
            staging = Path(temporary) / 'staging'
            staging.mkdir()
            (staging / 'bank.json').symlink_to(bank)
            hardlinked = Path(temporary) / 'hardlinked'
            hardlinked.mkdir()
            (hardlinked / 'bank.json').hardlink_to(bank)
            for output in (source, staging, hardlinked):
                result = subprocess.run([sys.executable, str(TOOL), str(project), 'song', str(output)], capture_output=True, text=True)
                self.assertNotEqual(result.returncode, 0)
                self.assertIn('must not overwrite', result.stderr)
                self.assertEqual(bank.read_text(), '{"presets": []}')
                self.assertEqual(project.read_text(), 'authored source')
                self.assertFalse((output / 'song.mid').exists())

    def test_invalid_note_recipe_fails_before_staging_is_created(self):
        with tempfile.TemporaryDirectory() as temporary:
            source = Path(temporary)
            (source / 'song.evil').write_text('authored source')
            (source / 'bank.json').write_text('{"presets": []}')
            for key, ticks in ((60.0, 480), (128, 480), (60, 0), (True, 480)):
                (source / 'render.json').write_text(json.dumps({'renders': [
                    {'wav': 'lead.wav', 'instrument': 0, 'key': key, 'ticks': ticks}]}))
                output = source / 'staging'
                result = subprocess.run([sys.executable, str(TOOL), str(source / 'song.evil'), 'song', str(output)], capture_output=True, text=True)
                self.assertNotEqual(result.returncode, 0)
                self.assertFalse(output.exists())
                self.assertNotIn('Traceback', result.stderr)


    def test_wrong_pitch_and_same_basename_different_source_are_rejected(self):
        with tempfile.TemporaryDirectory() as temporary:
            source = Path(temporary)
            (source / 'song.evil').write_text('authored source')
            (source / 'render.json').write_text(json.dumps({'renders': [
                {'wav': 'rendered/lead.wav', 'instrument': 0, 'key': 74, 'ticks': 480}]}))
            for wav, root in (('rendered/lead.wav', 72), ('layers/lead.wav', 74)):
                (source / 'bank.json').write_text(json.dumps({'presets': [{'zones': [
                    {'wav': wav, 'root': root, 'keys': [root, root]}]}]}))
                output = source / 'staging'
                result = subprocess.run([sys.executable, str(TOOL), str(source / 'song.evil'), 'song', str(output)], capture_output=True, text=True)
                self.assertNotEqual(result.returncode, 0)
                self.assertFalse(output.exists())
                self.assertTrue('rendered pitch' in result.stderr or 'exact render path' in result.stderr)

    def test_asset_stem_rule_matches_layout_gate(self):
        checker = TOOL.with_name('check_project_layout.sh').read_text()
        self.assertIn('is_snake_case() { [[ $1 =~ ^' + SNAKE_CASE_PATTERN + '$ ]]; }', checker)

    def test_missing_project_and_failed_cargo_report_clean_errors(self):
        with tempfile.TemporaryDirectory() as temporary:
            source = Path(temporary)
            output = source / 'staging'
            command = [sys.executable, str(TOOL), str(source / 'song.evil'), 'song', str(output)]
            missing = subprocess.run(command, capture_output=True, text=True)
            self.assertNotEqual(missing.returncode, 0)
            self.assertIn('project file does not exist', missing.stderr)
            self.assertFalse(output.exists())
            (source / 'song.evil').write_text('authored source')
            (source / 'bank.json').write_text('{"presets": []}')
            (source / 'render.json').write_text('{"renders": []}')
            cargo = source / 'cargo'
            cargo.write_text('#!/bin/sh\necho "fixture cargo failure" >&2\nexit 23\n')
            cargo.chmod(0o755)
            failed = subprocess.run(command, capture_output=True, text=True,
                                    env={**os.environ, 'PATH': str(source) + os.pathsep + os.environ['PATH']})
            self.assertNotEqual(failed.returncode, 0)
            self.assertIn('fixture cargo failure', failed.stderr)
            self.assertNotIn('Traceback', failed.stderr)
            self.assertFalse((output / 'song.mid').exists())


if __name__ == '__main__':
    unittest.main()

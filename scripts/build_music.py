#!/usr/bin/env python3
"""Export a MIDI Evil project and package its original finite samples into staging."""
import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile


# Mirrored by check_project_layout.sh:is_snake_case; test_build_music.py holds parity.
SNAKE_CASE_PATTERN = r'[a-z][a-z0-9]*(_[a-z0-9]+)*'

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('project', type=Path)
    parser.add_argument('output_stem')
    parser.add_argument('staging_directory', nargs='?', type=Path)
    arguments = parser.parse_args()
    if not re.fullmatch(SNAKE_CASE_PATTERN, arguments.output_stem):
        parser.error('output_stem must be a snake_case filename stem')
    project = arguments.project.resolve()
    if not project.is_file():
        parser.error('project file does not exist')
    output = (arguments.staging_directory or project.parent / 'rendered').resolve()
    staged_recipe = output / 'bank.json'
    source_recipe = project.parent / 'bank.json'
    aliases_source = staged_recipe.exists() and source_recipe.exists() and staged_recipe.samefile(source_recipe)
    if output == project.parent or staged_recipe.resolve() == source_recipe.resolve() or aliases_source:
        parser.error('staging must not overwrite the source recipe')
    recipe = json.loads((project.parent / 'bank.json').read_text())
    renders = json.loads((project.parent / 'render.json').read_text())['renders']
    for render in renders:
        for field in ('instrument', 'key', 'ticks'):
            if type(render[field]) is not int or render[field] < 0:
                parser.error(f'render {field} must be a nonnegative integer')
        if render['key'] > 127 or render['ticks'] == 0:
            parser.error('render key must be <=127 and ticks must be positive')
        if not isinstance(render['wav'], str) or Path(render['wav']).suffix != '.wav':
            parser.error('render wav must name a .wav file')
    names = [Path(render['wav']).name for render in renders]
    if len(set(names)) != len(names):
        parser.error('render WAV basenames must be unique')
    renders_by_path = {render['wav']: render for render in renders}
    for preset in recipe['presets']:
        for zone in preset['zones']:
            render = renders_by_path.get(zone['wav'])
            if render is None:
                parser.error(f"bank zone has no exact render path: {zone['wav']}")
            if zone['root'] != render['key']:
                parser.error(f"bank root disagrees with rendered pitch: {zone['wav']}")
            zone['wav'] = Path(zone['wav']).name
    engine = Path(__file__).resolve().parents[1]
    workstation = engine.parent / 'midi-evil'
    output.mkdir(parents=True, exist_ok=True)
    build = subprocess.run(['cargo', 'build', '--manifest-path', str(workstation / 'Cargo.toml'),
                            '-p', 'evil-exports', '--example', 'project_audio', '--message-format=json'],
                           stdout=subprocess.PIPE, text=True)
    executable = None
    for line in build.stdout.splitlines():
        message = json.loads(line)
        if message.get('reason') == 'compiler-message':
            print(message['message'].get('rendered') or message['message']['message'], file=sys.stderr)
        if message.get('reason') == 'compiler-artifact' and message['target']['name'] == 'project_audio':
            executable = message.get('executable') or executable
    build.check_returncode()
    if executable is None:
        raise ValueError('Cargo did not report a project_audio executable')
    tool = Path(executable)
    subprocess.run([str(tool), 'midi', str(project),
                    str(output / f'{arguments.output_stem}.mid')], check=True)
    for render in renders:
        subprocess.run([str(tool), 'note', str(project),
                        str(output / Path(render['wav']).name), str(render['instrument']),
                        str(render['key']), str(render['ticks'])], check=True)
    # Replace the directory entry, never truncate a link to an authored recipe.
    with tempfile.NamedTemporaryFile(mode='w', dir=output, delete=False) as temporary:
        temporary.write(json.dumps(recipe, indent=2) + '\n')
        temporary_recipe = Path(temporary.name)
    try:
        os.replace(temporary_recipe, output / 'bank.json')
    finally:
        temporary_recipe.unlink(missing_ok=True)
    subprocess.run(['python3', str(engine / 'scripts/pack_soundfont.py'),
                    str(output / 'bank.json'),
                    str(output / f'{arguments.output_stem}.sf2')], check=True)
    print(f'Staged MIDI and bank in {output}; validate before installing through the editor save API.')


if __name__ == '__main__':
    try:
        main()
    except (OSError, ValueError, KeyError, TypeError, subprocess.CalledProcessError) as error:
        raise SystemExit(f'music build: {error}') from None

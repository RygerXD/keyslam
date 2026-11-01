"""Check the public asset boundary and generated speech inventory (no dependencies)."""

import hashlib
import json
from pathlib import Path
import string
import subprocess

ROOT = Path(__file__).resolve().parent.parent
ASSETS = ROOT / 'assets'
PACKS = {'animals', 'foods', 'instruments', 'directions'}
AUDIO = {'.opus', '.wav', '.m4a', '.mp3', '.ogg', '.flac'}


def check():
    errors = []
    packs = {path.name for path in (ASSETS / 'packs').iterdir() if path.is_dir()}
    if packs != PACKS:
        errors.append(f'Public packs must be {sorted(PACKS)}, found {sorted(packs)}')
    clips = json.loads((ASSETS / 'public-audio.json').read_text(encoding='utf-8-sig'))['clips']
    inventory = {clip['path']: clip for clip in clips}
    if len(inventory) != len(clips):
        errors.append('Duplicate public audio inventory entries')
    actual = {path.relative_to(ROOT).as_posix(): path for path in ASSETS.rglob('*')
              if path.is_file() and path.suffix.lower() in AUDIO}
    for name in sorted(actual.keys() | inventory.keys()):
        if name not in inventory:
            errors.append(f'Unapproved audio: {name}')
        elif name not in actual:
            errors.append(f'Missing generated audio: {name}')
        elif hashlib.sha256(actual[name].read_bytes()).hexdigest() != inventory[name]['sha256']:
            errors.append(f'Audio differs from generated inventory: {name}')
    folders = [ASSETS / 'sounds' / 'letters' / letter for letter in string.ascii_lowercase]
    folders += [ASSETS / 'sounds' / 'numbers' / str(digit) for digit in range(10)]
    shapes = 'star cross heart oval rectangle triangle square pentagon hexagon septagon octagon trapezoid circle'.split()
    folders += [ASSETS / 'sounds' / 'shapes' / shape for shape in shapes]
    colors = 'red orange yellow green blue indigo violet pink brown white gray black amber lime-green orange-red teal'.split()
    folders += [ASSETS / 'sounds' / 'colors' / reading / color
                for reading in ('standalone', 'modifier') for color in colors]
    for pack in sorted(packs):
        directory = ASSETS / 'packs' / pack
        folders += [item for item in directory.iterdir() if item.is_dir()]
        manifest = json.loads((directory / 'pack.json').read_text(encoding='utf-8'))
        for entry in manifest['keys']:
            if not (directory / entry['item']).is_dir():
                errors.append(f'Manifest refers to missing item: {pack}/{entry["item"]}')
    for folder in folders:
        if len(list(folder.glob('*.opus'))) != 1:
            errors.append(f'Expected one synthetic clip: {folder.relative_to(ROOT)}')
    # Catch personal recordings staged elsewhere, outside the embedded assets.
    tracked = subprocess.check_output(
        ['git', 'ls-files', '--cached', '--others', '--exclude-standard', '-z'], cwd=ROOT
    ).decode('utf-8').split('\0')
    for name in tracked:
        path = ROOT / name
        if path.is_file() and path.suffix.lower() in AUDIO and name not in inventory:
            errors.append(f'Recording outside public inventory: {name}')
    if errors:
        raise SystemExit('\n'.join(errors))
    print(f'Public assets verified: {len(packs)} packs, {len(inventory)} synthetic clips; no extra recordings.')


if __name__ == '__main__':
    check()

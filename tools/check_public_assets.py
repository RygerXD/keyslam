"""Check the public asset boundary and generated speech inventory (no dependencies)."""

import argparse
import hashlib
import json
from pathlib import Path
import string
import subprocess

ROOT = Path(__file__).resolve().parent.parent
ASSETS = ROOT / 'assets'
PACKS = {'animals', 'foods', 'instruments', 'directions'}
AUDIO = {'.opus', '.wav', '.m4a', '.mp3', '.ogg', '.flac'}


def check(assets=ASSETS):
    errors = []
    packs = {path.name for path in (assets / 'packs').iterdir() if path.is_dir()}
    if packs != PACKS:
        errors.append(f'Public packs must be {sorted(PACKS)}, found {sorted(packs)}')
    clips = json.loads((ASSETS / 'public-audio.json').read_text(encoding='utf-8-sig'))['clips']
    inventory = {clip['path']: clip for clip in clips}
    if len(inventory) != len(clips):
        errors.append('Duplicate public audio inventory entries')
    actual = {'assets/' + path.relative_to(assets).as_posix(): path for path in assets.rglob('*')
              if path.is_file() and path.suffix.lower() in AUDIO}
    for name in sorted(actual.keys() | inventory.keys()):
        if name not in inventory:
            errors.append(f'Unapproved audio: {name}')
        elif name not in actual:
            errors.append(f'Missing generated audio: {name}')
        elif hashlib.sha256(actual[name].read_bytes()).hexdigest() != inventory[name]['sha256']:
            errors.append(f'Audio differs from generated inventory: {name}')
    folders = [assets / 'sounds' / 'letters' / letter for letter in string.ascii_lowercase]
    folders += [assets / 'sounds' / 'numbers' / str(digit) for digit in range(10)]
    shapes = 'star cross heart oval rectangle triangle square pentagon hexagon septagon octagon trapezoid circle'.split()
    folders += [assets / 'sounds' / 'shapes' / shape for shape in shapes]
    colors = 'red orange yellow green blue indigo violet pink brown white gray black amber lime-green orange-red teal'.split()
    folders += [assets / 'sounds' / 'colors' / reading / color
                for reading in ('standalone', 'modifier') for color in colors]
    for pack in sorted(packs):
        directory = assets / 'packs' / pack
        folders += [item for item in directory.iterdir() if item.is_dir()]
        manifest = json.loads((directory / 'pack.json').read_text(encoding='utf-8'))
        for entry in manifest['keys']:
            if not (directory / entry['item']).is_dir():
                errors.append(f'Manifest refers to missing item: {pack}/{entry["item"]}')
    for folder in folders:
        if len(list(folder.glob('*.opus'))) != 1:
            errors.append(f'Expected one synthetic clip: {folder.relative_to(assets)}')
    # Catch personal recordings staged elsewhere, outside the embedded assets.
    tracked = subprocess.check_output(
        ['git', 'ls-files', '--cached', '--others', '--exclude-standard', '-z'], cwd=ROOT
    ).decode('utf-8').split('\0') if assets == ASSETS else []
    for name in tracked:
        path = ROOT / name
        if path.is_file() and path.suffix.lower() in AUDIO and name not in inventory:
            errors.append(f'Recording outside public inventory: {name}')
    if errors:
        raise SystemExit('\n'.join(errors))
    print(f'Public assets verified: {len(packs)} packs, {len(inventory)} synthetic clips; no extra recordings.\n{assets}')


if __name__ == '__main__':
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--assets', type=Path, default=ASSETS,
                        help='Audit an installed asset folder against the repository inventory')
    check(parser.parse_args().assets.resolve())

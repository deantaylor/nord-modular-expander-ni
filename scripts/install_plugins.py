"""Install the two locally branded, signed plugin variants without replacing other plugins."""
import argparse
from pathlib import Path
from build_g1 import LEGACY_PLUGIN_NAME
import shutil
import subprocess

ROOT=Path(__file__).resolve().parents[1]


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--source',type=Path,default=ROOT/'build/g1-emu/plugins',help='Folder containing VST3/ and AU/ bundles')
    parser.add_argument('--replace',action='store_true',help='Replace previously installed legacy G1-Emu builds')
    args=parser.parse_args()
    library=Path.home()/'Library/Audio/Plug-Ins'
    planned=[]
    for fmt in ['VST3','AU']:
        directory=args.source/fmt
        if not directory.is_dir():raise SystemExit('Build plugins first: python3 scripts/build_g1.py --plugins')
        for source in sorted(directory.iterdir()):
            if not source.name.startswith(LEGACY_PLUGIN_NAME):continue
            target=library/('VST3' if fmt=='VST3' else 'Components')/source.name
            subprocess.run(['codesign','--verify','--deep','--strict',str(source)],check=True)
            if target.exists() and not args.replace:raise SystemExit(f'Already installed: {target}. Use --replace only to update your legacy G1-Emu build.')
            planned.append((source,target))
    if len(planned)!=4:raise SystemExit('Expected the instrument and FX bundles in both VST3 and AU formats. Build with --plugins first.')
    for source,target in planned:
        target.parent.mkdir(parents=True,exist_ok=True)
        staged=target.with_name('.'+target.name+'.installing')
        if staged.exists():raise SystemExit(f'Unfinished install needs inspection: {staged}')
        shutil.copytree(source,staged)
        subprocess.run(['codesign','--verify','--deep','--strict',str(staged)],check=True)
        if target.exists():shutil.rmtree(target)
        staged.rename(target)
        print(target)


if __name__=='__main__':main()

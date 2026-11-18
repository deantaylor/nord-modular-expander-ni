"""Build the pinned Apple Silicon standalone with the optional MK3 bridge.
Fetches open-source code only. ROMs are never fetched or copied by this script.
"""
import argparse
from pathlib import Path
import shutil
import subprocess

ROOT = Path(__file__).resolve().parents[1]
# Keep the legacy bundle names in sync with standalone.patch and existing DAW sessions.
LEGACY_PLUGIN_NAME = 'Modular Machine G1'
PINS = {
    'source': ('https://github.com/animatek/G1-Emu.git', 'efd178d8a6e13a38c74b5cce02853fc7dccbf5c1'),
    'gearmulator': ('https://github.com/joelanders/gearmulator-md-mm.git', 'ab69ad003f1268d1ef373fbb5a47340269b6dd47'),
    'dsp': ('https://github.com/dsp56300/dsp56300.git', '8b731966f9792789adc39fe180a8cf68508f816a'),
    'juce': ('https://github.com/juce-framework/JUCE.git', '29396c22c93392d6738e021b83196283d6e4d850'),
}

def run(*args, **kwargs):
    subprocess.run([str(a) for a in args], check=True, **kwargs)

def checkout(path, name):
    url, revision = PINS[name]
    if not path.exists():
        path.mkdir(parents=True)
        run('git', 'init', path)
        run('git', '-C', path, 'remote', 'add', 'origin', url)
        run('git', '-C', path, 'fetch', '--depth', '1', 'origin', revision)
        run('git', '-C', path, 'checkout', '--detach', 'FETCH_HEAD')
    actual = subprocess.check_output(['git', '-C', str(path), 'rev-parse', 'HEAD'], text=True).strip()
    if actual != revision:
        raise RuntimeError(f'{path}: expected {revision}, found {actual}; use a fresh build directory')

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in PINS:
        parser.add_argument('--' + name + '-dir', type=Path, default=ROOT / 'build/g1-work' / name)
    parser.add_argument('--build-dir', type=Path, default=ROOT / 'build/g1-work/cmake')
    parser.add_argument('--plugins',action='store_true',help='Also build Maschine-enabled VST3 and AU instrument/FX bundles')
    parser.add_argument('--jobs', type=int, default=4)
    args = parser.parse_args()
    paths = {name: getattr(args, name + '_dir').resolve() for name in PINS}
    for name, path in paths.items():
        checkout(path, name)
    for name, submodule in [('gearmulator', 'source/mc68k'), ('dsp', 'source/asmjit')]:
        run('git', '-C', paths[name], 'submodule', 'update', '--init', '--depth', '1', '--', submodule)
    source = paths['source']
    patch = ROOT / 'integrations/g1-emu/standalone.patch'
    # Idempotent, and never reset somebody's checkout to make a patch apply.
    if subprocess.run(['git', '-C', str(source), 'apply', '--check', str(patch)], capture_output=True).returncode == 0:
        run('git', '-C', source, 'apply', patch)
    else:
        run('git', '-C', source, 'apply', '--reverse', '--check', patch)
    for filename, destination in [('Mk3ControlBridge.h', 'app/gui'), ('Mk3HostCheck.cpp', 'app/gui'), ('LcdText.h', 'app/gui'), ('LcdTextCheck.cpp', 'app/gui'), ('SettingsReplyCheck.cpp', 'app'), ('Mk3PluginHost.h','app/plugin'), ('Mk3PluginCheck.cpp','app/plugin')]:
        shutil.copy2(ROOT / 'integrations/g1-emu' / filename, source / destination / filename)
    build = args.build_dir.resolve()
    run('cmake', '-S', source, '-B', build, '-DCMAKE_BUILD_TYPE=Release', '-DCMAKE_OSX_ARCHITECTURES=arm64',
        f'-DGEARMULATOR_DIR={paths["gearmulator"]}', f'-DG1_DSP56300_DIR={paths["dsp"]}',
        f'-DG1_JUCE_DIR={paths["juce"]}', '-DG1_BACKEND=juce', '-DG1_LTO=OFF', '-DG1_MK3_PLUGINS=' + ('ON' if args.plugins else 'OFF'))
    targets=['g1gui','g1mk3check','g1settingsreplycheck','g1lcdtextcheck']
    if args.plugins:targets+=['g1plugin_VST3','g1pluginfx_VST3','g1plugin_AU','g1pluginfx_AU','g1vst3check','g1mk3plugincheck','g1hostgesturestest']
    run('cmake','--build',build,'--target',*targets,'--parallel',args.jobs)
    run(build / 'app/g1settingsreplycheck')
    run(build / 'app/gui/g1lcdtextcheck')
    output = ROOT / 'build/g1-emu'
    output.mkdir(parents=True, exist_ok=True)
    shutil.copytree(build / 'app/gui/g1gui_artefacts/Release/G1-Emu.app', output / 'G1-Emu.app', dirs_exist_ok=True)
    shutil.copy2(build / 'app/gui/g1mk3check_artefacts/Release/g1mk3check', output / 'g1mk3check')
    # Keep license/source provenance beside local build artifacts.
    shutil.copy2(source / 'LICENSE', output / 'G1-Emu-LICENSE')
    if args.plugins:
        for target,name in [('g1plugin', LEGACY_PLUGIN_NAME), ('g1pluginfx', LEGACY_PLUGIN_NAME+' FX')]:
            for fmt,extension in [('VST3','vst3'),('AU','component')]:
                source_bundle=build/'app/plugin'/(target+'_artefacts/Release')/fmt/(name+'.'+extension)
                destination=output/'plugins'/fmt/source_bundle.name
                destination.parent.mkdir(parents=True,exist_ok=True)
                if destination.exists():shutil.rmtree(destination)
                shutil.copytree(source_bundle,destination)
                run('/usr/bin/codesign','--force','--deep','--sign','-',destination)
                run('/usr/bin/codesign','--verify','--deep','--strict',destination)
        shutil.copy2(build/'tools/vst3check/g1vst3check_artefacts/Release/g1vst3check',output/'g1vst3check')
        shutil.copy2(build/'app/plugin/g1mk3plugincheck_artefacts/Release/g1mk3plugincheck',output/'g1mk3plugincheck')
        run(build/'app/plugin/g1hostgesturestest')

    (output / 'SOURCES.txt').write_text('\n'.join(f'{name} {url} {revision}' for name, (url, revision) in PINS.items()) +
        '\nLocal modifications: integrations/g1-emu in nord-modular-ni-interface. Full source checkouts: ' + str(source.parent) + '\n')
    print(f'Built {output / "G1-Emu.app"}; next: python3 scripts/run_g1.py --demo')

if __name__ == '__main__':
    main()

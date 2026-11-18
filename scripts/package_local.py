"""Make a local ad-hoc-signed macOS app plus a matching source archive, excluding user data/ROMs."""
import argparse
import hashlib
import io
import json
import os
from pathlib import Path
import plistlib
import re
import shutil
import subprocess
import tarfile

ROOT=Path(__file__).resolve().parents[1]
UPSTREAM={'g1-emu':Path('/tmp/modular-machine-g1-emu'),'gearmulator':Path('/tmp/modular-machine-g1-gearmulator'),
          'dsp56300':Path('/tmp/modular-machine-g1-dsp56300'),'juce':Path('/tmp/modular-machine-g1-juce')}


def digest(path):
    result=hashlib.sha256()
    with path.open('rb') as f:
        for block in iter(lambda:f.read(1024*1024),b''):result.update(block)
    return result.hexdigest()


def source_archive(path):
    # Include native source snapshots and exact locally resolved Rust crate sources.
    metadata=json.loads(subprocess.check_output(['cargo','metadata','--locked','--offline','--format-version','1'],cwd=ROOT))
    active={node['id'] for node in metadata['resolve']['nodes']}
    packages=[p for p in metadata['packages'] if p['id'] in active and p.get('source','') and p['source'].startswith('registry+')]
    skip_dirs={'.git','target','node_modules','g1-state','build','.DS_Store','__pycache__','Roms','roms','ROMs','firmware'}
    forbidden={'.rom','.syx','.wav','.dmg','.zip','.exe','.dll','.o','.a','.so','.dylib'}
    checksums={}
    for block in (ROOT/'Cargo.lock').read_text().split('[[package]]')[1:]:
        fields=dict(re.findall(r'^([a-z]+) = "([^"]*)"',block,re.M))
        if 'checksum' in fields:checksums[(fields['name'],fields['version'])]=fields['checksum']
    entries=0
    with tarfile.open(path,'w:gz',compresslevel=4) as archive:
        def add_tree(directory,prefix,project=False,registry=False):
            nonlocal entries
            file_hashes={}
            for current,dirs,files in os.walk(directory,followlinks=False):
                dirs[:]=sorted(d for d in dirs if d not in skip_dirs and not Path(current,d).is_symlink())
                if project and Path(current)==directory/'docs' and 'baseline' in dirs:
                    dirs.remove('baseline')
                for name in sorted(files):
                    file=Path(current,name);relative=file.relative_to(directory)
                    if registry and name in {'.cargo-checksum.json','.cargo-ok'}:continue
                    if file.is_symlink() or name=='.DS_Store' or name.endswith(('.pyc','.log')):continue
                    if project:
                        allowed={'src','screens','scripts','docs','integrations','vendor','patches','.cargo'}
                        top_files={'Cargo.toml','Cargo.lock','README.md','TODO.md','AGENTS.md','.gitignore','Maschine.command','KompleteKontrol.command'}
                        if len(relative.parts)==1 and name not in top_files:continue
                        if len(relative.parts)>1 and relative.parts[0] not in allowed:continue
                        if relative.parts[0]=='patches' and relative.as_posix() not in {'patches/MK3-Bridge.pch','patches/MK3-MIDI.pch','patches/MK3-AudioIn.pch'}:continue
                        if name in {'local-defaults.json','controller-preferences.json','browser-state.json'}:continue
                        if file.suffix.lower() in forbidden:continue
                        if file.suffix.lower()=='.bin':continue
                    elif not registry:
                        if file.suffix.lower() in forbidden|{'.bin','.pch'}:continue
                    archive.add(file,arcname=str(Path('modular-machine-source',prefix,relative)),recursive=False);entries+=1
                    if registry:file_hashes[relative.as_posix()]=digest(file)
            return file_hashes
        add_tree(ROOT,'project',project=True)
        for name,source in UPSTREAM.items():
            if not source.is_dir():raise RuntimeError('Missing source snapshot: '+str(source))
            add_tree(source,'native/'+name)
        for package in packages:
            source=Path(package['manifest_path']).parent
            hashes=add_tree(source,'crates/'+source.name,registry=True)
            checksum=json.dumps({'files':hashes,'package':checksums[(package['name'],package['version'])]},sort_keys=True).encode()
            info=tarfile.TarInfo('modular-machine-source/crates/'+source.name+'/.cargo-checksum.json');info.size=len(checksum);archive.addfile(info,io.BytesIO(checksum));entries+=1
        config=b'[source.crates-io]\nreplace-with="bundled"\n[source.bundled]\ndirectory="../crates"\n'
        info=tarfile.TarInfo('modular-machine-source/project/.cargo/config.toml');info.size=len(config);archive.addfile(info,io.BytesIO(config))
        build=b'''# Source snapshot build (macOS arm64)\n\nFrom project/: cargo build --locked --offline. Native snapshots already include\nthe local integration patch and bridge headers. From the source bundle root:\n\ncmake -S native/g1-emu -B native-build -DCMAKE_BUILD_TYPE=Release -DCMAKE_OSX_ARCHITECTURES=arm64 -DGEARMULATOR_DIR="$PWD/native/gearmulator" -DG1_DSP56300_DIR="$PWD/native/dsp56300" -DG1_JUCE_DIR="$PWD/native/juce" -DG1_BACKEND=juce -DG1_LTO=OFF -DG1_MK3_PLUGINS=ON\ncmake --build native-build --target g1gui g1mk3check g1plugin_VST3 g1pluginfx_VST3 g1plugin_AU g1pluginfx_AU --parallel 4\n\nA compatible user-supplied ROM is needed to run, never to compile. The bundle\ncontains no ROM, user banks, recordings or user patch library. Existing source\nlicenses remain with their respective components. See project/integrations/g1-emu\nand project/vendor for attribution and modification notices.\n'''
        info=tarfile.TarInfo('modular-machine-source/BUILD.md');info.size=len(build);archive.addfile(info,io.BytesIO(build))
    return {'files':entries,'rustCrates':len(packages),'sha256':digest(path)}


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--skip-source',action='store_true',help='Developer-only fast rebuild; default includes full source')
    args=parser.parse_args()
    version=re.search(r'^version\s*=\s*"([^"]+)"',(ROOT/'Cargo.toml').read_text(),re.M)[1]
    output=ROOT/'build/package';output.mkdir(parents=True,exist_ok=True)
    keyboard_plugin_launch=output/'Komplete Keyboard Plugin.command'
    keyboard_plugin_launch.write_text('#!/bin/zsh\npackage_dir="${0:A:h}"\nexec /usr/bin/python3 "$package_dir/Modular Machine '+version+'.app/Contents/Resources/project/scripts/run_packaged.py" --plugin --controller komplete "$@"\n');keyboard_plugin_launch.chmod(0o755)
    keyboard_launch=output/'Komplete Keyboard.command'
    keyboard_launch.write_text('#!/bin/zsh\npackage_dir="${0:A:h}"\nexec /usr/bin/python3 "$package_dir/Modular Machine '+version+'.app/Contents/Resources/project/scripts/run_packaged.py" --controller komplete "$@"\n');keyboard_launch.chmod(0o755)
    app=output/f'Modular Machine {version}.app'
    if app.exists():shutil.rmtree(app)
    contents=app/'Contents';project=contents/'Resources/project';(contents/'MacOS').mkdir(parents=True);project.mkdir(parents=True)
    for dirname in ['scripts','integrations','docs']:
        shutil.copytree(ROOT/dirname,project/dirname,ignore=shutil.ignore_patterns('__pycache__','*.pyc','baseline'))
    for filename in ['README.md','Cargo.toml','Cargo.lock','AGENTS.md']:
        shutil.copy2(ROOT/filename,project/filename)
    (project/'patches').mkdir()
    for name in ['MK3-Bridge.pch','MK3-MIDI.pch','MK3-AudioIn.pch']:shutil.copy2(ROOT/'patches'/name,project/'patches'/name)
    (project/'target/debug').mkdir(parents=True);shutil.copy2(ROOT/'target/release/nord-modular-ni-interface',project/'target/debug/nord-modular-ni-interface')
    (project/'build/g1-emu').mkdir(parents=True)
    for name in ['G1-Emu.app','g1mk3check','G1-Emu-LICENSE','SOURCES.txt']:
        source=ROOT/'build/g1-emu'/name
        if source.is_dir():shutil.copytree(source,project/'build/g1-emu'/name)
        else:shutil.copy2(source,project/'build/g1-emu'/name)
    preferences=ROOT/'g1-state/controller-preferences.json'
    if preferences.exists():
        value=json.loads(preferences.read_text());defaults={k:v for k,v in value.items() if k in {'port','rom','flash','patch_dir'}}
        (project/'local-defaults.json').write_text(json.dumps(defaults,indent=2))
    launch=contents/'Resources/Launch.command'
    launch.write_text('''#!/bin/zsh\napp_resources="${0:A:h}"\nexport MODULAR_MACHINE_STATE_DIR="${MODULAR_MACHINE_STATE_DIR:-${HOME}/Library/Application Support/Modular Machine}"\nexec /usr/bin/python3 "$app_resources/project/scripts/run_packaged.py" "$@"\n''');launch.chmod(0o755)
    executable=contents/'MacOS/Modular Machine'
    executable.write_text('''#!/bin/zsh\napp_contents="${0:A:h:h}"\nexec /usr/bin/open -a Terminal "$app_contents/Resources/Launch.command"\n''');executable.chmod(0o755)
    with (contents/'Info.plist').open('wb') as file:plistlib.dump({'CFBundleName':'Modular Machine','CFBundleDisplayName':'Modular Machine','CFBundleIdentifier':'local.modular-machine.mk3','CFBundleExecutable':'Modular Machine','CFBundlePackageType':'APPL','CFBundleShortVersionString':version,'CFBundleVersion':__import__('datetime').datetime.now().strftime('%Y%m%d'),'NSHighResolutionCapable':True},file)
    licenses=contents/'Resources/Licenses';licenses.mkdir()
    for source,name in [(ROOT/'vendor/encdr/LICENSE','Encdr-GPL.txt'),(ROOT/'vendor/encdr-view/LICENSE','EncdrView-GPL.txt'),(ROOT/'screens/fonts/OFL.txt','Inter-OFL.txt'),(ROOT/'build/g1-emu/G1-Emu-LICENSE','G1-Emu-GPL.txt')]:shutil.copy2(source,licenses/name)
    shutil.copy2(ROOT/'screens/THIRD_PARTY_NOTICES.md',licenses/'Module-Visuals-Attribution.md')
    shutil.copy2(ROOT/'screens/GPL-3.0.txt',licenses/'GPL-3.0.txt')
    for component in [project/'target/debug/nord-modular-ni-interface',project/'build/g1-emu/g1mk3check',project/'build/g1-emu/G1-Emu.app']:
        subprocess.run(['/usr/bin/codesign','--force','--deep','--sign','-',str(component)],check=True,capture_output=True)
    metadata=json.loads(subprocess.check_output(['cargo','metadata','--locked','--offline','--format-version','1'],cwd=ROOT))
    components=[]
    for package in metadata['packages']:
        source=Path(package['manifest_path']).parent
        components.append({key:package.get(key) for key in ['name','version','license','repository']})
        for notice in source.iterdir():
            if notice.is_file() and notice.name.lower().startswith(('license','licence','copying','notice')):
                target=licenses/'Rust'/(package['name']+'-'+package['version']);target.mkdir(parents=True,exist_ok=True);shutil.copy2(notice,target/notice.name)
    (licenses/'Rust-components.json').write_text(json.dumps(components,indent=2))
    for name,source in UPSTREAM.items():
        for notice in source.iterdir():
            if notice.is_file() and notice.name.lower().startswith(('license','licence','copying','notice')):shutil.copy2(notice,licenses/(name+'-'+notice.name))
    manifest={'version':version,'platform':'macos-arm64','controllerSha256':digest(project/'target/debug/nord-modular-ni-interface'),'nativeSha256':digest(project/'build/g1-emu/G1-Emu.app/Contents/MacOS/G1-Emu'),'nativeSources':(ROOT/'build/g1-emu/SOURCES.txt').read_text(),'proprietaryNordRomIncluded':False,'userBanksIncluded':False,'userPatchLibraryIncluded':False,'controllerProfile':'release'}
    plugins=ROOT/'build/g1-emu/plugins'
    if plugins.is_dir():
        destination=output/'Plugins'
        if destination.exists():shutil.rmtree(destination)
        shutil.copytree(plugins,destination)
        items=[]
        for fmt in ['VST3','AU']:
            for bundle in sorted((destination/fmt).iterdir()):
                if not bundle.is_dir() or bundle.suffix not in {'.vst3','.component'}:continue
                subprocess.run(['/usr/bin/codesign','--verify','--deep','--strict',str(bundle)],check=True,capture_output=True)
                with (bundle/'Contents/Info.plist').open('rb') as f:info=plistlib.load(f)
                items.append({'format':fmt,'path':str(bundle.relative_to(output)),'sha256':digest(bundle/'Contents/MacOS'/info['CFBundleExecutable']),'version':info.get('CFBundleShortVersionString')})
        manifest['plugins']=items
        plugin_launch=output/'Modular Machine Plugin.command'
        plugin_launch.write_text('#!/bin/zsh\npackage_dir="${0:A:h}"\nexec /usr/bin/python3 "$package_dir/Modular Machine '+version+'.app/Contents/Resources/project/scripts/run_packaged.py" --plugin "$@"\n');plugin_launch.chmod(0o755)
        installer=output/'Install Plugins.command'
        installer.write_text('#!/bin/zsh\npackage_dir="${0:A:h}"\nexec /usr/bin/python3 "$package_dir/Modular Machine '+version+'.app/Contents/Resources/project/scripts/install_plugins.py" --source "$package_dir/Plugins" "$@"\n');installer.chmod(0o755)
        (destination/'README.txt').write_text('Modular Machine G1 / G1 FX: Apple Silicon VST3 and AU, locally ad-hoc signed.\nInstall Plugins.command copies these newly named builds into your per-user plugin folders.\nLoad one in a native Apple Silicon DAW, select your existing rack OS3.03 ROM in plugin Settings, enable audio processing, then launch Modular Machine Plugin.command.\nQuit the standalone controller before switching. With multiple plugin instances use --list-plugins and --plugin --port PORT; the port is shown in plugin Settings.\nGPL notices and the matching source archive are beside this folder/in the app Resources/Licenses. No ROM or user banks included.\n')
    if not args.skip_source:
        source=output/f'modular-machine-{version}-source.tar.gz';manifest['sourceArchive']={'name':source.name,**source_archive(source)}
    (output/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
    (contents/'Resources/manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
    (contents/'Resources/README.txt').write_text('Local ad-hoc-signed macOS arm64 build (no Developer ID/notarization). Launches the matching G1-Emu/controller pair.\nROM, patch library and banks remain external. Runtime preferences and logs use\n~/Library/Application Support/Modular Machine. Local defaults preserve existing\nROM/library/bank paths; there is no bundled firmware or user patch library.\nThe matching source archive and provenance manifest are beside this app.\n')
    subprocess.run(['/usr/bin/codesign','--force','--sign','-',str(app)],check=True,capture_output=True)
    subprocess.run(['/usr/bin/codesign','--verify','--deep','--strict',str(app)],check=True,capture_output=True)
    print(app)
    print(output/'manifest.json')

if __name__=='__main__':main()

"""Install the external surface watcher as a per-user service, with staged updates and rollback."""
from pathlib import Path
import argparse,json,os,plistlib,shutil,subprocess,tempfile,time
from keyboard_service import CONTROLLERS
from service_paths import STATE_DIR_ENV, service_label, state_directory
ROOT=Path(__file__).resolve().parents[1]
STATE=state_directory()

def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--controller',choices=CONTROLLERS,help='Select a surface and remember it for updates')
    args=parser.parse_args()
    adapter=ROOT/'build/g1-official/adapter-build/g1keyboardadapter_artefacts/Release/g1keyboardadapter'
    controller=ROOT/'target/release/nord-modular-ni-interface'
    if not adapter.is_file() or not controller.is_file():raise SystemExit('Build the keyboard controller and official-protocol adapter first')
    for binary in [adapter,controller]:subprocess.run(['codesign','--verify',str(binary)],check=True)
    STATE.mkdir(parents=True,exist_ok=True)
    # Only names are cached in private user data; never copy the ROM into a runtime.
    from rom_parameter_names import extract
    label_file=STATE/'parameter-names.json'
    try:
        rom=ROOT/'roms/NORD-MODULAR-RACK-VER-3.03.BIN'
        official=Path.home()/'.local/share/Animatek/G1-Emu/settings.conf'
        if official.exists():
            for line in official.read_text().splitlines():
                if '=' in line and line.split('=',1)[0].strip()=='rom':rom=Path(line.split('=',1)[1].strip()).expanduser();break
        labels=extract(rom,ROOT/'build/g1-official/nme-source/data/modules.xml')
        label_file.with_suffix('.tmp').write_text(json.dumps(labels));label_file.with_suffix('.tmp').replace(label_file)
        print('Cached exact OS parameter names:',len(labels['names']))
    except (OSError,ValueError) as error:
        label_file.unlink(missing_ok=True);print('Exact OS names unavailable; preserving editor metadata:',error)

    runtime=Path(tempfile.mkdtemp(prefix='keyboard-runtime-',dir=STATE))
    try:
        (runtime/'bin').mkdir();(runtime/'scripts').mkdir();(runtime/'patches').mkdir()
        for source,name in [(adapter,'g1keyboardadapter'),(controller,'nord-modular-ni-interface')]:shutil.copy2(source,runtime/'bin'/name)
        for script in ('keyboard_service.py', 'service_paths.py'):
            shutil.copy2(ROOT/'scripts'/script, runtime/'scripts'/script)
        for name in ['MK3-Bridge.pch','MK3-MIDI.pch']:shutil.copy2(ROOT/'patches'/name,runtime/'patches'/name)
        notices=runtime/'LICENSES';notices.mkdir()
        for path, name in [
            ('vendor/encdr/LICENSE', 'Encdr-GPL.txt'),
            ('vendor/encdr/PATCHES.md', 'Encdr-Modifications.md'),
            ('vendor/encdr-view/LICENSE', 'EncdrView-GPL.txt'),
            ('vendor/encdr-view/PATCHES.md', 'EncdrView-Modifications.md'),
            ('screens/fonts/OFL.txt', 'Inter-OFL.txt'),
            ('screens/THIRD_PARTY_NOTICES.md', 'Module-Visuals-Attribution.md'),
            ('screens/GPL-3.0.txt', 'GPL-3.0.txt'),
        ]:
            shutil.copy2(ROOT/path, notices/name)
        cache=ROOT/'build/g1-official/adapter-build/CMakeCache.txt'
        for line in cache.read_text().splitlines():
            if line.startswith(('NME_SOURCE:','JUCE_SOURCE:')):
                source=Path(line.partition('=')[2]);label='Animatek-NME' if line.startswith('NME_') else 'JUCE'
                for name in ['LICENSE','LICENSE.md','LICENSE.txt']:
                    license_file=source/name
                    if license_file.is_file():shutil.copy2(license_file,notices/(label+'-LICENSE.txt'));break
        (runtime/'SOURCE.txt').write_text('External adapter sources: '+str(ROOT/'integrations/g1-official')+'\nAnimatek NME GPL-3.0, unchanged protocol/model code: https://github.com/animatek/Animatek-NME/tree/0ded1bb77ca2431b7ff52a8b55076430d85ac48d\nJUCE: https://github.com/juce-framework/JUCE/tree/29396c22c93392d6738e021b83196283d6e4d850\nController sources and dependency notices: '+str(ROOT)+'\nNo synth engine, firmware, modified plugin, or keybed MIDI interception.\n')
    except BaseException:
        shutil.rmtree(runtime);raise
    # launchd cannot inherit the interactive app's Documents-folder access.
    # Import only .pch files into private user data, never into a redistributable runtime.
    preference_file=STATE/'keyboard-preferences.json'
    preferences=json.loads(preference_file.read_text()) if preference_file.exists() else {}
    existing=Path(preferences['patch_dir']).expanduser().resolve() if preferences.get('patch_dir') else None
    source=existing if existing is not None and existing.is_dir() and existing.is_relative_to(STATE) else preferences.get('source_patch_dir') or preferences.get('patch_dir')
    if not source:
        saved=ROOT/'g1-state/plugin-controller-preferences.json'
        if saved.exists():source=json.loads(saved.read_text()).get('patch_dir')
    source=Path(source).expanduser().resolve() if source else ROOT/'patches'
    if source.is_dir() and not source.is_relative_to(STATE):
        libraries=STATE/'patch-libraries';libraries.mkdir(exist_ok=True)
        imported=Path(tempfile.mkdtemp(prefix='library-',dir=libraries));count=0
        for current,dirs,files in os.walk(source,followlinks=False):
            dirs[:]=[d for d in dirs if not (Path(current)/d).is_symlink()]
            for name in files:
                item=Path(current)/name
                if item.suffix.lower()!='.pch' or item.is_symlink():continue
                target=imported/item.relative_to(source);target.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(item,target);count+=1
        preferences.update(source_patch_dir=str(source),patch_dir=str(imported))
        print(f'Imported {count} local patch files into private Application Support data; originals unchanged.')
    elif source.is_dir():preferences['patch_dir']=str(source)
    preference_file.write_text(json.dumps(preferences,indent=2)+'\n')
    agents=Path.home()/'Library/LaunchAgents';agents.mkdir(exist_ok=True);label=service_label(agents);plist=agents/(label+'.plist')
    previous=plist.read_bytes() if plist.exists() else None
    previous_args=plistlib.loads(previous).get('ProgramArguments',[]) if previous else []
    profile=args.controller or next((profile for flag,profile in zip(previous_args,previous_args[1:]) if flag=='--controller' and profile in CONTROLLERS),'komplete')
    config={'Label':label,'ProgramArguments':['/usr/bin/python3','-B',str(runtime/'scripts/keyboard_service.py'),'--controller',profile],'RunAtLoad':True,'KeepAlive':True,'ThrottleInterval':10,'WorkingDirectory':str(runtime),'EnvironmentVariables':{STATE_DIR_ENV:str(STATE)},'StandardErrorPath':str(STATE/'keyboard-launch-error.log')}
    staged=plist.with_suffix('.plist.staged');staged.write_bytes(plistlib.dumps(config))
    subprocess.run(['plutil','-lint',str(staged)],check=True,stdout=subprocess.DEVNULL)
    domain=f'gui/{os.getuid()}'
    subprocess.run(['launchctl','bootout',domain+'/'+label],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
    deadline=time.monotonic()+10
    while subprocess.run(['launchctl','print',domain+'/'+label],capture_output=True).returncode==0 and time.monotonic()<deadline:time.sleep(.1)
    def bootstrap():
        result=None
        for attempt in range(30):
            result=subprocess.run(['launchctl','bootstrap',domain,str(plist)],capture_output=True,text=True)
            if result.returncode==0:return
            # bootout can return before launchd finishes removing the old job.
            time.sleep(.2)
        raise RuntimeError('launchd could not start the keyboard service: '+result.stderr.strip())
    staged.replace(plist)
    try:bootstrap()
    except BaseException:
        if previous is not None:
            plist.write_bytes(previous);bootstrap()
        else:plist.unlink(missing_ok=True)
        raise
    print(f'Installed background bridge for {CONTROLLERS[profile]}. It waits for G1-Emu plugins.')
    print(f'Service: {label}\nPreferences and logs: {STATE}')

if __name__=='__main__':main()

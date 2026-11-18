"""Bootstrap the local app with external user data and a matched native/controller pair."""
import json
import os
from pathlib import Path
from service_paths import STATE_DIR_ENV, state_directory
import sys

ROOT=Path(__file__).resolve().parents[1]
os.environ.setdefault(STATE_DIR_ENV, str(state_directory()))
from run_machine import PREFERENCES, STATE, write_preferences, main

if __name__=='__main__':
    defaults=ROOT/'local-defaults.json'
    preferences=STATE/'plugin-controller-preferences.json' if '--plugin' in sys.argv else PREFERENCES
    if not preferences.exists() and defaults.exists():
        value=json.loads(defaults.read_text())
        if '--plugin' in sys.argv:value.pop('port',None)
        write_preferences(preferences,value)
    if '--no-build' not in sys.argv:sys.argv.append('--no-build')
    main()

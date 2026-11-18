"""Run the existing G1-Emu standalone with its local MK3 bridge enabled."""
import argparse
import os
from pathlib import Path
import socket
import subprocess

ROOT = Path(__file__).resolve().parents[1]

def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--rom', type=Path, default=ROOT / 'roms/NORD-MODULAR-RACK-VER-3.03.BIN')
    parser.add_argument('--flash', type=Path, default=ROOT / 'g1-state/controller-flash.bin')
    parser.add_argument('--port', type=int, default=47410)
    parser.add_argument('--demo', action='store_true', help='Seed a NEW dedicated bank file with the project-owned two-knob patch')
    args = parser.parse_args()
    rom, flash = args.rom.resolve(), args.flash.resolve()
    if not rom.is_file():
        parser.error(f'Local compatible rack OS 3.03 ROM required: {rom}')
    if not 1024 <= args.port <= 65535:
        parser.error('Use a port between 1024 and 65535')
    app = ROOT / 'build/g1-emu/G1-Emu.app/Contents/MacOS/G1-Emu'
    if not app.is_file():
        parser.error('Run python3 scripts/build_g1.py first')
    # Fail before starting another synth if a controller bridge already owns this port.
    with socket.socket() as probe:
        try:
            probe.bind(('127.0.0.1', args.port))
        except OSError:
            parser.error(f'Port {args.port} is in use; close that G1-Emu instance or choose --port')
    flash.parent.mkdir(parents=True, exist_ok=True)
    env = {**os.environ, 'G1_MODEL': 'regular', 'G1_MK3_PORT': str(args.port)}
    if args.demo and not flash.exists():
        # The real host uploads/stores via the OS. Never overwrite an existing bank.
        # Keep the seeding helper off the normal bridge port: a launcher must wait
        # for the actual standalone, not attach to the short-lived helper.
        while True:
            with socket.socket() as probe:
                probe.bind(('127.0.0.1', 0))
                seed_port = probe.getsockname()[1]
            if seed_port != args.port:
                break
        print('Creating dedicated demo bank 1 / patch 1 with the real emulator...', flush=True)
        subprocess.run([str(ROOT / 'build/g1-emu/g1mk3check'), str(rom), str(flash),
                        str(ROOT / 'patches/MK3-Bridge.pch'), str(seed_port), '8'], env=env, check=True)
    print(f'G1-Emu: {flash}\nController in another terminal: cargo run --locked -- --g1-address 127.0.0.1:{args.port}', flush=True)
    os.execve(str(app), [str(app), str(rom), str(flash)], env)

if __name__ == '__main__':
    main()

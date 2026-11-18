"""Targeted launcher failure/recovery checks; no USB, audio or GUI."""
import json
from pathlib import Path
import socket
import tempfile
import threading
import unittest
from types import SimpleNamespace
from unittest.mock import Mock, patch

import run_machine
from run_machine import Bridge, LaunchError, connect_bridge, launcher_lock, read_preferences, write_preferences


class LauncherTests(unittest.TestCase):
    def test_cold_start_waits_for_emulator_before_launching_controller(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            library = root / 'Library with spaces'
            library.mkdir()
            rom = root / 'local.rom'
            rom.write_bytes(b'local test path only')
            for filename in ('target/debug/nord-modular-ni-interface', 'build/g1-emu/G1-Emu.app/Contents/MacOS/G1-Emu'):
                path = root / filename
                path.parent.mkdir(parents=True, exist_ok=True)
                path.touch()
            args = SimpleNamespace(port=47412, patch_dir=library, rom=rom, flash=root/'banks.bin',
                                   no_build=True, demo=True, mock=True, visible=False, controller='komplete')
            emulator, controller = Mock(), Mock()
            controller.wait.return_value = 0
            controller.stdout = []
            controller.poll.return_value = 0
            with patch.object(run_machine, 'ROOT', root), patch.object(run_machine, 'STATE', root/'state'), \
                 patch.object(run_machine, 'connect_bridge', return_value=None), \
                 patch.object(run_machine, 'wait_ready') as ready, \
                 patch.object(run_machine.subprocess, 'Popen', side_effect=[emulator, controller]) as spawn:
                self.assertEqual(run_machine.launch(args, root/'preferences.json'), 0)
                ready.assert_called_once_with(47412, emulator)
                calls = spawn.call_args_list
                self.assertEqual(len(calls), 2)
                self.assertIn('--demo', calls[0].args[0])
                self.assertIn(str(root/'scripts/run_g1.py'), calls[0].args[0])
                self.assertTrue(calls[0].kwargs['start_new_session'])
                self.assertIn('--mock', calls[1].args[0])
                self.assertEqual(calls[1].args[0][calls[1].args[0].index('--controller')+1], 'komplete')
                self.assertIn(str(library.resolve()), calls[1].args[0])
                emulator.send_signal.assert_not_called()

    def test_missing_rom_does_not_start_an_engine_or_write_preferences(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            binary = root/'target/debug/nord-modular-ni-interface'
            binary.parent.mkdir(parents=True)
            binary.touch()
            args = SimpleNamespace(port=47412, patch_dir=root, rom=root/'missing.rom', flash=root/'banks.bin',
                                   no_build=True, demo=False, mock=True, visible=False)
            preferences = root/'preferences.json'
            with patch.object(run_machine, 'ROOT', root), patch.object(run_machine, 'STATE', root/'state'), \
                 patch.object(run_machine, 'connect_bridge', return_value=None), \
                 patch.object(run_machine.subprocess, 'Popen') as spawn:
                with self.assertRaisesRegex(LaunchError, 'local ROM required'):
                    run_machine.launch(args, preferences)
                spawn.assert_not_called()
            self.assertFalse(preferences.exists())

    def test_preferences_are_separate_atomic_and_invalid_file_is_preserved(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / 'controller-preferences.json'
            self.assertEqual(read_preferences(path), {})
            value = {'port': 47411, 'patch_dir': '/a library/patches'}
            write_preferences(path, value)
            self.assertEqual(read_preferences(path), {'version': 1, **value})
            broken = '{"version": 99}'
            path.write_text(broken)
            with self.assertRaises(LaunchError):
                read_preferences(path)
            self.assertEqual(path.read_text(), broken)
            self.assertEqual(len(list(Path(folder).iterdir())), 1)

    def test_duplicate_launcher_is_rejected_and_lock_releases(self):
        with tempfile.TemporaryDirectory() as folder:
            path = Path(folder) / 'lock'
            with launcher_lock(path):
                with self.assertRaises(LaunchError):
                    with launcher_lock(path):
                        self.fail('duplicate lock acquired')
            with launcher_lock(path):
                pass

    def test_framed_read_retains_coalesced_reply_and_detects_closed_peer(self):
        left, right = socket.socketpair()
        try:
            right.sendall(b'{"first":1}\n{"second":2}\n')
            bridge = Bridge(left)
            self.assertEqual(bridge.read(), {'first': 1})
            self.assertEqual(bridge.read(), {'second': 2})
            right.close()
            with self.assertRaisesRegex(LaunchError, 'Another controller'):
                bridge.read()
        finally:
            left.close()
            right.close()

    def test_wrong_service_is_not_treated_as_available_port(self):
        with socket.socket() as listener:
            listener.bind(('127.0.0.1', 0))
            listener.listen()
            port = listener.getsockname()[1]
            def respond():
                with listener.accept()[0] as peer:
                    peer.sendall(b'{"service":"other","protocol":1}\n')
            thread = threading.Thread(target=respond)
            thread.start()
            try:
                with self.assertRaisesRegex(LaunchError, 'incompatible service'):
                    connect_bridge(port)
            finally:
                thread.join(timeout=3)
            self.assertFalse(thread.is_alive())

    def test_plugin_ambiguity_never_selects_or_starts_an_instance(self):
        args=SimpleNamespace(plugin=True,port=None)
        with tempfile.TemporaryDirectory() as folder, patch.object(run_machine,'find_plugins',return_value=[{'port':47420,'name':'G1','instance':1},{'port':47421,'name':'G1 FX','instance':2}]), patch.object(run_machine.subprocess,'Popen') as spawn:
            with self.assertRaisesRegex(LaunchError,'Several plugins'):
                run_machine.launch(args,Path(folder)/'prefs.json')
            spawn.assert_not_called()

    def test_plugin_absent_does_not_launch_the_standalone(self):
        args=SimpleNamespace(plugin=True,port=None)
        with tempfile.TemporaryDirectory() as folder, patch.object(run_machine,'find_plugins',return_value=[]), patch.object(run_machine.subprocess,'Popen') as spawn:
            with self.assertRaisesRegex(LaunchError,'No available'):
                run_machine.launch(args,Path(folder)/'prefs.json')
            spawn.assert_not_called()

    def test_plugin_locked_runner_status_is_accepted(self):
        left,right=socket.socketpair()
        value={'running':True,'upload':{'serial':0,'busy':False,'ok':True},'bankLoad':{},'configuration':{},'encoderInput':'editor-parameters','engineThreadRequests':False,'hostType':'plugin','engineAccess':'runner-lock','padMidi':True,'transportThread':'dedicated'}
        try:
            right.sendall((json.dumps({'id':1,'result':value})+'\n').encode())
            self.assertEqual(Bridge(left).status(),value)
        finally:left.close();right.close()

    def test_status_rejects_outdated_bridge_and_wrong_response_id(self):
        for response, expected in [
            ({'id': 1, 'result': {'running': True}}, 'outdated'),
            ({'id': 1, 'result': {'running': True, 'upload': None}}, 'outdated'),
            ({'id': 1, 'result': {'running': True, 'upload': {'serial': -1, 'busy': False, 'ok': False}}}, 'invalid upload status'),
            ({'id': 7, 'result': {'running': True, 'upload': {}}}, 'invalid response ID'),
            ({'id': 1, 'result': {'running': True, 'upload': {'serial': 0, 'busy': False, 'ok': True}, 'configuration': {}}}, 'updated encoder path'),
            ({'id': 1, 'result': {'running': True, 'upload': {'serial': 0, 'busy': False, 'ok': True}, 'configuration': {}, 'encoderInput': 'editor-parameters', 'engineThreadRequests': True, 'bankLoad': {}}}, 'MIDI pads bridge'),
            ({'id': 1, 'result': {'running': True, 'upload': {'serial': 0, 'busy': False, 'ok': True}, 'configuration': {}, 'encoderInput': 'editor-parameters', 'engineThreadRequests': True, 'bankLoad': {}, 'padMidi': True}}, 'dedicated control transport'),
        ]:
            left, right = socket.socketpair()
            try:
                right.sendall((json.dumps(response) + '\n').encode())
                with self.assertRaisesRegex(LaunchError, expected):
                    Bridge(left).status()
            finally:
                left.close()
                right.close()


if __name__ == '__main__':
    unittest.main()

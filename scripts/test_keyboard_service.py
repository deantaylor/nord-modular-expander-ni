"""Focused discovery, ownership and child cleanup checks for the external watcher."""
import os,socket,tempfile,threading,unittest,signal
from pathlib import Path
from unittest.mock import Mock, patch
import keyboard_service as service
import service_paths
class ServiceTests(unittest.TestCase):
    def greeting(self,value):
        server=socket.socket();server.bind(('127.0.0.1',0));server.listen();port=server.getsockname()[1]
        def reply():
            try:
                client,_=server.accept()
                with client:
                    if value:client.sendall(value)
            finally:server.close()
        thread=threading.Thread(target=reply);thread.start()
        try:return service.discover(port)
        finally:thread.join(timeout=2)
    def test_only_plugin_greeting_is_selected(self):
        self.assertIsNotNone(self.greeting(b'G1-Emu 1 G1-Emu plugin 1\tpcname=x\n'))
        self.assertIsNone(self.greeting(b'G1-Emu 1 G1-Emu\n'))
        self.assertIsNone(self.greeting(b'Other service\n'))
        self.assertIsNone(self.greeting(b''))
    def test_one_owner_and_reacquisition(self):
        with tempfile.TemporaryDirectory() as directory:
            path=Path(directory)/'service.lock';one=service.acquire(path)
            self.assertIsNotNone(one);self.assertIsNone(service.acquire(path));one.close()
            two=service.acquire(path);self.assertIsNotNone(two);two.close()
    def test_cleanup_only_signals_owned_live_child(self):
        process=Mock();process.poll.return_value=None
        service.stop_child(process);process.send_signal.assert_called_once_with(signal.SIGINT);process.wait.assert_called_once_with(timeout=6)
        exited=Mock();exited.poll.return_value=0;service.stop_child(exited);exited.send_signal.assert_not_called()

    def test_new_and_existing_installation_paths(self):
        with tempfile.TemporaryDirectory() as directory, patch.dict(os.environ, {'HOME': directory}, clear=True):
            library = Path(directory) / 'Library'
            agents = library / 'LaunchAgents'
            support = library / 'Application Support'
            self.assertEqual(service_paths.state_directory(), support / service_paths.STATE_DIR_NAME)
            self.assertEqual(service_paths.service_label(agents), service_paths.SERVICE_LABEL)

            previous_state = support / service_paths.LEGACY_STATE_DIR_NAME
            previous_state.mkdir(parents=True)
            agents.mkdir()
            (agents / (service_paths.LEGACY_SERVICE_LABEL + '.plist')).touch()
            self.assertEqual(service_paths.state_directory(), previous_state)
            self.assertEqual(service_paths.service_label(agents), service_paths.LEGACY_SERVICE_LABEL)

            isolated = Path(directory) / 'isolated'
            with patch.dict(os.environ, {service_paths.STATE_DIR_ENV: str(isolated)}):
                self.assertEqual(service_paths.state_directory(), isolated)
if __name__=='__main__':unittest.main()

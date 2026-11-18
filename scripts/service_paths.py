"""Names shared by the installer, watcher and local tools."""
from pathlib import Path
import os

STATE_DIR_ENV = 'NORD_MODULAR_NI_STATE_DIR'
STATE_DIR_NAME = 'Nord Modular NI Interface'
SERVICE_LABEL = 'local.nord-modular-ni-interface.keyboard'

# Keep existing installations, preferences and ownership locks usable.
LEGACY_STATE_DIR_NAME = 'Modular Machine'
LEGACY_SERVICE_LABEL = 'local.modular-machine.keyboard'
LEGACY_STATE_DIR_ENV = 'MODULAR_MACHINE_STATE_DIR'

BRIDGE_SERVICE = 'nord-modular-ni-interface-g1'
LEGACY_BRIDGE_SERVICE = 'modular-machine-g1'


def state_directory(default=None):
    configured = os.environ.get(STATE_DIR_ENV) or os.environ.get(LEGACY_STATE_DIR_ENV)
    if configured:
        return Path(configured)
    if default is not None:
        return default
    support = Path.home() / 'Library' / 'Application Support'
    current = support / STATE_DIR_NAME
    legacy = support / LEGACY_STATE_DIR_NAME
    return legacy if legacy.is_dir() and not current.exists() else current


def service_label(agents):
    current = agents / (SERVICE_LABEL + '.plist')
    legacy = agents / (LEGACY_SERVICE_LABEL + '.plist')
    return LEGACY_SERVICE_LABEL if legacy.is_file() and not current.exists() else SERVICE_LABEL

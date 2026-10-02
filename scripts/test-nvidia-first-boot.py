#!/usr/bin/env python3
"""Verify actual first-boot offer JSON in a temporary home with inert probes."""
import json
import os
from pathlib import Path
import shlex
import subprocess
import tempfile

root = Path(__file__).resolve().parents[1]
first_boot = root / 'iso/airootfs/usr/local/bin/bos-first-boot'
setup = root / 'iso/airootfs/usr/local/bin/bos-nvidia-setup'
with tempfile.TemporaryDirectory(prefix='bos-firstboot-test-') as directory:
    sandbox = Path(directory)
    bins = sandbox / 'bin'
    bins.mkdir()
    for command in ['notify-send', 'hyprctl', 'sleep']:
        script = bins / command
        script.write_text('#!/bin/sh\nexit 0\n')
        script.chmod(0o755)
    (bins / 'systemd-detect-virt').write_text('#!/bin/sh\nprintf none\n')
    (bins / 'systemd-detect-virt').chmod(0o755)
    # Invoke the real PCI detection, with only kernel discovery overridden.
    (bins / 'bos-nvidia-setup').write_text(
        '#!/bin/sh\nexec ' + shlex.quote(str(setup)) + ' "$@" --kernel linux --kernel linux-lts\n'
    )
    (bins / 'bos-nvidia-setup').chmod(0o755)
    for number, (ids, expected) in enumerate([
        (['1e04', '2684'], 'open-dkms'),
        (['1e04', '1b80'], 'legacy-580'),
    ]):
        pci = '\n'.join(
            f'{index:02}:00.0 VGA compatible controller: NVIDIA "GPU\\name"\t [10de:{device}]'
            for index, device in enumerate(ids)
        )
        (bins / 'lspci').write_text('#!/bin/sh\nprintf "%s\\n" ' + shlex.quote(pci) + '\n')
        (bins / 'lspci').chmod(0o755)
        home = sandbox / f'home-{number}'
        home.mkdir()
        environment = os.environ.copy()
        environment.update({
            'HOME': str(home),
            'XDG_STATE_HOME': str(home / 'state'),
            'PATH': str(bins) + ':' + environment['PATH'],
            'WAYLAND_DISPLAY': '',
            'DISPLAY': '',
        })
        subprocess.run([str(first_boot)], env=environment, check=True)
        offer = json.loads((home / 'state/bos/nvidia-offer.json').read_text())
        assert offer['pci'] == pci
        assert offer['driver_plan']['path'] == expected
        assert offer['auto_install'] is False
        assert (home / 'state/bos/first-boot-done').exists()
print('Multiline supported and mixed-card first-boot offers pass')

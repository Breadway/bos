#!/usr/bin/env python3
"""Exercise the actual setup CLI without installing or writing user files."""
import json
from pathlib import Path
import subprocess

setup = Path(__file__).resolve().parents[1] / 'iso/airootfs/usr/local/bin/bos-nvidia-setup'


def plan(ids, kernels):
    args = [str(setup), '--dry-run', '--json']
    for pci in ids:
        args += ['--device-id', pci]
    for kernel in kernels:
        args += ['--kernel', kernel]
    return json.loads(subprocess.check_output(args, text=True))


assert plan(['1e04'], ['linux'])['packages'] == ['nvidia-open', 'nvidia-utils']
assert plan(['2684'], ['linux', 'linux-lts'])['packages'] == [
    'nvidia-open-dkms', 'nvidia-utils', 'linux-headers', 'linux-lts-headers'
]
assert plan(['1e04'], ['linux-zen'])['packages'] == [
    'nvidia-open-dkms', 'nvidia-utils', 'linux-zen-headers'
]
for pci in ['1b80', '13c2']:
    result = plan([pci], ['linux'])
    assert result['path'] == 'legacy-580'
    assert result['packages'] == []
    assert 'nvidia-580xx-dkms' in result['message']
for ids in [['0de1'], ['ffff'], ['1e04', 'ffff']]:
    assert plan(ids, ['linux'])['path'] == 'unsupported'
    assert plan(ids, ['linux'])['packages'] == []
assert plan(['1e04', '1b80'], ['linux'])['packages'] == []
assert plan(['1E04'], ['linux'])['path'] == 'open'
for args in [
    ['--device-id', '1e04'],
    ['--kernel', 'linux'],
    ['--json'],
    ['--dry-run', '--device-id', 'bad'],
    ['--dry-run', '--kernel', 'bad"value'],
    ['--home'],
]:
    assert subprocess.run([str(setup), *args], capture_output=True).returncode == 2
print('NVIDIA CLI selection and rejection checks passed')

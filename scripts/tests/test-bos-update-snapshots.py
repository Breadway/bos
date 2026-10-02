#!/usr/bin/env python3
"""Exercise update ordering and pairing with isolated mock executables."""
import os
import pathlib
import subprocess
import tempfile

SCRIPT = pathlib.Path(__file__).resolve().parents[2] / 'iso/airootfs/usr/local/bin/bos-update'
with tempfile.TemporaryDirectory(prefix='bos28-test-') as root:
    directory = pathlib.Path(root)
    log = directory / 'calls'
    scripts = {
        'sudo': '#!/bin/bash\n[ "$1" = -n ] && shift\nexec "$@"\n',
        'pacman': '#!/bin/bash\nprintf "pacman\\n" >> "$BOS28_LOG"\nexit 0\n',
        'bakery': '''#!/bin/bash
if [ "$1" = capabilities ]; then
    [ "$BOS28_NATIVE" = yes ] && echo snapper-transactions-configured-v1 && exit 0
    exit 2
fi
printf "bakery\\n" >> "$BOS28_LOG"
exit "$BOS28_FAIL"
''',
        'snapper': '''#!/bin/bash
printf "%s\\n" "$*" >> "$BOS28_LOG"
[ "$3" = list ] && { [ "$2" != "$BOS28_MISSING" ]; exit $?; }
if [ "$5" = pre ]; then
    [ "$2" = root ] && echo 41 || echo 72
fi
exit 0
''',
    }
    for name, body in scripts.items():
        executable = directory / name
        executable.write_text(body)
        executable.chmod(0o755)
    for native, fail, missing in [('no', '0', ''), ('no', '7', ''), ('yes', '0', ''), ('no', '0', 'home')]:
        log.write_text('')
        env = dict(os.environ, PATH=str(directory) + ':/usr/bin:/bin', BOS28_LOG=str(log),
                   BOS28_NATIVE=native, BOS28_FAIL=fail, BOS28_MISSING=missing)
        result = subprocess.run(['bash', str(SCRIPT)], env=env, capture_output=True, text=True, check=True)
        calls = log.read_text()
        if native == 'yes':
            assert 'create' not in calls, calls
        else:
            count = 1 if missing else 2
            assert calls.count('-t pre') == count and calls.count('-t post') == count, calls
            assert '--pre-number 41' in calls, calls
            assert ('--pre-number 72' in calls) == (not missing), calls
            assert calls.index('pacman') < calls.index('-t pre') < calls.index('bakery\n') < calls.index('-t post'), calls
            assert ('post, failed' in calls) == (fail != '0'), calls
print('bos-update snapshot pairing, failure, missing home, and duplicate prevention passed')

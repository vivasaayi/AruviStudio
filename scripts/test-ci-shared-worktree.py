#!/usr/bin/env python3
"""Check that the optional CI mount uses the values the helper validated."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest


class SharedCiMountTest(unittest.TestCase):
    def test_shell_cannot_override_validated_mapping(self):
        helper = Path(__file__).resolve().parent / 'ci-shared-worktree-up'
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp).resolve()
            state, host, bin_dir = (root / part for part in ('state', 'host', 'bin'))
            for directory in (state, host, bin_dir):
                directory.mkdir()
            expected = {
                'ARUVI_CI_HOST_REPOSITORY_ROOT': str(host),
                'ARUVI_CI_CONTAINER_REPOSITORY_ROOT': '/repositories/ci',
                'ARUVI_CI_RELAY_URL': 'http://host.docker.internal:1234',
                'ARUVI_CI_RELAY_TOKEN': 'private-test-token',
            }
            (state / 'environment.env').write_text(''.join(f'{k}={v}\n' for k, v in expected.items()))
            plan = json.dumps({'state': str(state), 'project': 'fixture'})
            (bin_dir / 'bash').write_text('#!/bin/sh\nprintf "%s\\n" "$TEST_PLAN"\n')
            (bin_dir / 'docker').write_text(
                '#!/usr/bin/env python3\n'
                'import json, os, sys\n'
                'with open(os.environ["TEST_LOG"], "a") as f:\n'
                '    f.write(json.dumps({k: os.environ[k] for k in json.loads(os.environ["TEST_KEYS"])}) + "\\n")\n'
                'if "port" in sys.argv: print("127.0.0.1:12345")\n'
            )
            for executable in bin_dir.iterdir():
                executable.chmod(0o755)
            env = os.environ.copy()
            env.update({key: '/hostile/shell/value' for key in expected})
            env.update(PATH=str(bin_dir) + os.pathsep + env.get('PATH', ''), TEST_PLAN=plan,
                       TEST_LOG=str(root / 'calls'), TEST_KEYS=json.dumps(list(expected)))
            result = subprocess.run(['python3', str(helper)], env=env, text=True, capture_output=True, check=True)
            self.assertIn('127.0.0.1:12345', result.stdout)
            self.assertEqual([json.loads(line) for line in (root / 'calls').read_text().splitlines()],
                             [expected, expected])


if __name__ == '__main__':
    unittest.main()

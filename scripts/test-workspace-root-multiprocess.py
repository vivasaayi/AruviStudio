#!/usr/bin/env python3
"""Exercise workspace root guards with two disposable Linux Studio servers."""
import hashlib
import json
import os
from pathlib import Path
import secrets
import selectors
import subprocess
import tempfile
import time
import urllib.error
import urllib.request
import uuid

root = Path(__file__).resolve().parent.parent
plan = json.loads(subprocess.check_output(['bash', 'scripts/env', 'worktree', 'plan'], cwd=root, text=True))
values = dict(line.split('=', 1) for line in (Path(plan['state']) / 'environment.env').read_text().splitlines() if line and not line.startswith('#') and '=' in line)
image = os.environ.get('ARUVI_TEST_APP_IMAGE') or values['APP_IMAGE']
suffix = uuid.uuid4().hex[:16]
volume = f'{plan["project"]}_workspace_root_test_{suffix}'
names = [f'{plan["project"]}-root-{suffix}-{number}' for number in (1, 2)]
writer_name = f'{plan["project"]}-root-writer-{suffix}'
token = secrets.token_urlsafe(32)
owned_volume = False
owned_containers = []
writer = None
opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))

def docker(*args):
    return subprocess.check_output(['docker', *args], cwd=root, text=True).strip()

def request(base, path, payload):
    data = json.dumps(payload).encode()
    req = urllib.request.Request(base + path, data=data, headers={'Authorization': 'Bearer ' + token, 'Content-Type': 'application/json', 'Accept': 'application/json, text/event-stream'})
    try:
        with opener.open(req, timeout=30) as response:
            return response.status, json.load(response)
    except urllib.error.HTTPError as error:
        return error.code, error.read().decode('utf-8', 'replace')

def rpc(base, action, arguments):
    status, payload = request(base, '/api/mcp', {'jsonrpc': '2.0', 'id': 1, 'method': 'tools/call', 'params': {'name': 'aruvi_repositories', 'arguments': {'action': action, 'arguments': arguments}}})
    if status != 200 or 'error' in payload:
        raise RuntimeError(f'MCP transport failed: {status}')
    return payload['result']

def result(base, action, arguments):
    value = rpc(base, action, arguments)
    if value.get('isError'):
        raise RuntimeError(f'MCP {action} failed: {value}')
    return value['structuredContent']['result']

def wait_health(base):
    deadline = time.monotonic() + 90
    while time.monotonic() < deadline:
        try:
            with opener.open(base + '/health', timeout=2) as response:
                if response.status == 200: return
        except (OSError, urllib.error.URLError): pass
        time.sleep(.2)
    raise RuntimeError('Disposable server did not become healthy')

try:
    if subprocess.run(['docker', 'volume', 'inspect', volume], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode == 0:
        raise RuntimeError('Disposable volume name already exists')
    for name in [*names, writer_name]:
        if subprocess.run(['docker', 'inspect', name], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode == 0:
            raise RuntimeError('Disposable container name already exists')
    docker('volume', 'create', volume); owned_volume = True
    docker('run', '--rm', '--user', '0', '-v', f'{volume}:/data', '--entrypoint', 'chown', image, '10001:10001', '/data')
    with tempfile.TemporaryDirectory(prefix='aruvi-root-guard-env-') as temporary:
        env_file = Path(temporary) / 'environment.env'
        env_file.write_text(f'ARUVI_MOBILE_API_TOKEN={token}\nARUVI_MCP_API_TOKEN={token}\n'); env_file.chmod(0o600)
        endpoints = []
        for name in names:
            docker('create', '--name', name, '--restart', 'no', '--env-file', str(env_file), '-e', 'ARUVI_APP_DATA_DIR=/data', '-e', 'ARUVI_DB_PATH=/data/studio.db', '-v', f'{volume}:/data', '-p', '127.0.0.1::8787', image)
            owned_containers.append(name); docker('start', name)
            address = docker('port', name, '8787/tcp').splitlines()[0]
            if not address.startswith('127.0.0.1:'): raise RuntimeError('Expected loopback fixture')
            endpoint = 'http://' + address; wait_health(endpoint); endpoints.append(endpoint)
        docker('exec', names[0], 'sh', '-c', 'mkdir -p /data/first /data/second && printf "same content\\n" > /data/first/file.txt && printf "same content\\n" > /data/second/file.txt')
        repo = result(endpoints[0], 'register_repository', {'name': 'Root fixture', 'local_path': '/data/first', 'default_branch': 'main'})
        assert repo['local_path'] == '/data/first'
        read = result(endpoints[0], 'read_repository_file', {'repository_id': repo['id'], 'relative_path': 'file.txt'})
        assert read['workspace_root'] == '/data/first' and read['content'] == 'same content\n'
        updated = result(endpoints[1], 'update_repository', {'id': repo['id'], 'name': repo['name'], 'local_path': '/data/second', 'remote_url': repo['remote_url'], 'default_branch': repo['default_branch'], 'expected': repo})
        assert updated['local_path'] == '/data/second'
        save = {'repository_id': repo['id'], 'relative_path': 'file.txt', 'expected_root': '/data/first', 'expected_content': 'same content\n', 'content': 'wrong destination\n'}
        status, _ = request(endpoints[0], '/api/mobile/repository-file/save', save); assert status == 409
        guarded = rpc(endpoints[0], 'write_repository_file', {'repository_id': repo['id'], 'relative_path': 'file.txt', 'expected_root': '/data/first', 'content': 'wrong MCP destination\n'})
        assert guarded.get('isError')
        sha = hashlib.sha256(b'same content\n').hexdigest()
        patch = '@@ -1,1 +1,1 @@\n-same content\n+patched content'
        status, _ = request(endpoints[0], '/api/mobile/repository-inspect', {'action': 'apply_reviewed_repository_patch', 'repository_id': repo['id'], 'expected_root': '/data/first', 'relative_path': 'file.txt', 'patch': patch, 'base_sha256': sha, 'expected_exists': True}); assert status == 409
        assert result(endpoints[1], 'read_repository_file', {'repository_id': repo['id'], 'relative_path': 'file.txt'})['content'] == 'same content\n'
        # A separate SQLite writer holds the lock while changing the root. The
        # server's write must wait and then reject the old root.
        program = 'import sqlite3,sys,time\nc=sqlite3.connect("/data/studio.db"); c.execute("BEGIN IMMEDIATE"); c.execute("UPDATE repositories SET local_path=? WHERE id=?",("/data/first",sys.argv[1])); print("locked",flush=True); time.sleep(1); c.commit()'
        owned_containers.append(writer_name)
        writer = subprocess.Popen(['docker','run','--rm','--name',writer_name,'--network','none','-v',f'{volume}:/data','python:3.12-slim','python','-u','-c',program,repo['id']], stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
        with selectors.DefaultSelector() as ready:
            ready.register(writer.stdout, selectors.EVENT_READ)
            if not ready.select(timeout=15) or writer.stdout.readline().strip() != 'locked':
                raise RuntimeError('SQLite writer did not acquire lock')
        waiting = rpc(endpoints[0], 'write_repository_file', {'repository_id': repo['id'], 'relative_path': 'file.txt', 'expected_root': '/data/second', 'content': 'raced write\n'})
        assert waiting.get('isError')
        if writer.wait(timeout=15) != 0: raise RuntimeError('SQLite writer failed')
        assert result(endpoints[1], 'read_repository_file', {'repository_id': repo['id'], 'relative_path': 'file.txt'})['content'] == 'same content\n'
        result(endpoints[1], 'write_repository_file', {'repository_id': repo['id'], 'relative_path': 'file.txt', 'expected_root': '/data/first', 'content': 'current root\n'})
        assert result(endpoints[0], 'read_repository_file', {'repository_id': repo['id'], 'relative_path': 'file.txt'})['content'] == 'current root\n'
        print('Two Linux servers rejected stale browser and MCP writes and coordinated a concurrent path update.')
finally:
    if writer and writer.poll() is None:
        writer.terminate()
        try: writer.wait(timeout=10)
        except subprocess.TimeoutExpired: writer.kill(); writer.wait()
    for name in reversed(owned_containers): subprocess.run(['docker','rm','-f',name], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    if owned_volume: subprocess.run(['docker','volume','rm',volume], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

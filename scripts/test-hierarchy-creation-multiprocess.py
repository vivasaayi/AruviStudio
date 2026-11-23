#!/usr/bin/env python3
"""Check hierarchy creation across two disposable Linux Studio servers."""
import concurrent.futures
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
volume = f'{plan["project"]}_hierarchy_create_{suffix}'
names = [f'{plan["project"]}-hierarchy-{suffix}-{number}' for number in (1, 2)]
writer_name = f'{plan["project"]}-hierarchy-writer-{suffix}'
token = secrets.token_urlsafe(32)
owned_volume = False
owned_containers = []
writer = None
opener = urllib.request.build_opener(urllib.request.ProxyHandler({}))

def docker(*args):
    return subprocess.check_output(['docker', *args], cwd=root, text=True).strip()

def request(base, path, payload):
    req = urllib.request.Request(base + path, data=json.dumps(payload).encode(), headers={
        'Authorization': 'Bearer ' + token, 'Content-Type': 'application/json', 'Accept': 'application/json'})
    try:
        with opener.open(req, timeout=30) as response:
            return response.status, json.load(response)
    except urllib.error.HTTPError as error:
        return error.code, error.read().decode('utf-8', 'replace')

def result(base, action, arguments):
    status, message = request(base, '/api/mcp', {'jsonrpc':'2.0','id':1,'method':'tools/call','params':{
        'name':'aruvi_catalog','arguments':{'action':action,'arguments':arguments}}})
    if status != 200 or 'error' in message or message['result'].get('isError'):
        raise RuntimeError(f'Catalog {action} failed: {status} {message}')
    return message['result']['structuredContent']['result']

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
    if subprocess.run(['docker','volume','inspect',volume], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode == 0:
        raise RuntimeError('Disposable volume name already exists')
    for name in [*names, writer_name]:
        if subprocess.run(['docker','inspect',name], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL).returncode == 0:
            raise RuntimeError('Disposable container name already exists')
    docker('volume','create',volume); owned_volume = True
    docker('run','--rm','--user','0','-v',f'{volume}:/data','--entrypoint','chown',image,'10001:10001','/data')
    with tempfile.TemporaryDirectory(prefix='aruvi-hierarchy-create-') as temporary:
        env_file = Path(temporary) / 'environment.env'
        env_file.write_text(f'ARUVI_MOBILE_API_TOKEN={token}\nARUVI_MCP_API_TOKEN={token}\n'); env_file.chmod(0o600)
        endpoints = []
        for name in names:
            docker('create','--name',name,'--restart','no','--env-file',str(env_file),'-e','ARUVI_APP_DATA_DIR=/data','-e','ARUVI_DB_PATH=/data/studio.db','-v',f'{volume}:/data','-p','127.0.0.1::8787',image)
            owned_containers.append(name); docker('start',name)
            address = docker('port',name,'8787/tcp').splitlines()[0]
            if not address.startswith('127.0.0.1:'): raise RuntimeError('Expected loopback fixture')
            base = 'http://' + address; wait_health(base); endpoints.append(base)
        a = result(endpoints[0],'create_product',{'name':'Hierarchy race A'})
        b = result(endpoints[1],'create_product',{'name':'Hierarchy race B'})
        area = result(endpoints[0],'create_product_area',{'product_id':a['id'],'name':'Movable area'})
        program = 'import sqlite3,sys,time\nc=sqlite3.connect("/data/studio.db"); c.execute("BEGIN IMMEDIATE"); c.execute("UPDATE product_areas SET product_id=? WHERE id=?",(sys.argv[1],sys.argv[2])); print("locked",flush=True); time.sleep(1); c.commit()'
        owned_containers.append(writer_name)
        writer = subprocess.Popen(['docker','run','--rm','--name',writer_name,'--network','none','--user','10001:10001','-v',f'{volume}:/data','python:3.12-slim','python','-u','-c',program,b['id'],area['id']],stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True)
        with selectors.DefaultSelector() as ready:
            ready.register(writer.stdout,selectors.EVENT_READ)
            if not ready.select(timeout=15) or writer.stdout.readline().strip() != 'locked': raise RuntimeError('SQLite writer did not acquire lock')
        status, _ = request(endpoints[0],f'/api/mobile/products/{a["id"]}/hierarchy',{'name':'Wrong-scope child','product_area_id':area['id']})
        assert status == 400, f'Expected changed-scope rejection, got {status}'
        if writer.wait(timeout=15) != 0: raise RuntimeError('SQLite writer failed')
        assert result(endpoints[1],'list_capabilities',{'product_area_id':area['id']}) == []
        def pair(browser, mcp):
            with concurrent.futures.ThreadPoolExecutor(max_workers=2) as executor:
                left = executor.submit(browser); right = executor.submit(mcp)
                status, _ = left.result(); assert status == 201, f'Browser creation failed: {status}'
                right.result()
        for index in range(3):
            pair(lambda: request(endpoints[0],f'/api/mobile/products/{b["id"]}/hierarchy',{'name':f'Browser area {index}'}),
                 lambda: result(endpoints[1],'create_product_area',{'product_id':b['id'],'name':f'MCP area {index}'}))
            pair(lambda: request(endpoints[0],f'/api/mobile/products/{b["id"]}/hierarchy',{'name':f'Browser cap {index}','product_area_id':area['id']}),
                 lambda: result(endpoints[1],'create_capability',{'product_area_id':area['id'],'name':f'MCP cap {index}'}))
        areas = result(endpoints[0],'list_product_areas',{'product_id':b['id']})
        added = [item for item in areas if item['name'].startswith(('Browser area ','MCP area '))]
        assert len(added) == 6 and len({item['sort_order'] for item in added}) == 6
        caps = result(endpoints[1],'list_capabilities',{'product_area_id':area['id']})
        assert len(caps) == 6 and len({item['sort_order'] for item in caps}) == 6
        print('Two Linux servers rejected a concurrent scope change and allocated distinct area and capability positions across browser and MCP creation.')
finally:
    if writer and writer.poll() is None:
        writer.terminate()
        try: writer.wait(timeout=10)
        except subprocess.TimeoutExpired: writer.kill(); writer.wait()
    for name in reversed(owned_containers): subprocess.run(['docker','rm','-f',name],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
    if owned_volume: subprocess.run(['docker','volume','rm',volume],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)

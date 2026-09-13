"""Integration tests against an isolated server. Never uses the real config/media.
Build first: cargo build --locked --manifest-path src-tauri/Cargo.toml
Run: python3 -m unittest discover -s tests -p '*_integration.py' -v
"""
import struct
import zlib
from contextlib import contextmanager

@contextmanager
def database(path):
    connection = sqlite3.connect(path)
    try:
        with connection: yield connection
    finally: connection.close()

def png():
    def chunk(kind, body): return struct.pack('>I',len(body))+kind+body+struct.pack('>I',zlib.crc32(kind+body))
    return b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',1,1,8,2,0,0,0))+chunk(b'IDAT',zlib.compress(b'\x00\xff\x00\x00'))+chunk(b'IEND',b'')
import http.client
import io
import json
import os
from pathlib import Path
import re
import socket
import sqlite3
import subprocess
import tempfile
import time
import unittest
import urllib.parse
import zipfile

ROOT = Path(__file__).resolve().parents[1]
BINARY = ROOT / 'src-tauri/target/debug/stickplay-server'
PASSWORD = 'test password only 12345'

class SecurityIntegration(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix='stickplay-test-')
        self.root = Path(self.temp.name).resolve()
        self.config = self.root / 'config'; self.config.mkdir()
        self.media = self.root / 'media'; self.media.mkdir()
        self.lib_a = self.media / 'a'; self.lib_a.mkdir()
        self.lib_b = self.media / 'b'; self.lib_b.mkdir()
        with socket.socket() as sock:
            sock.bind(('127.0.0.1', 0)); self.port = sock.getsockname()[1]
        self.origin = f'http://127.0.0.1:{self.port}'
        self.env = {**os.environ, 'STICKPLAY_CONFIG_DIR': str(self.config), 'STICKPLAY_MEDIA_DIR': str(self.media),
                    'STICKPLAY_PUBLIC_ORIGIN': self.origin, 'STICKPLAY_ALLOW_INSECURE_HTTP': 'true',
                    'STICKPLAY_FRONTEND_DIR': str(ROOT / 'dist'), 'PORT': str(self.port)}
        self.cookie = ''; self.csrf = ''; self.library = 'a'
        self.start()

    def start(self):
        self.log = open(self.root / 'server.log', 'a+')
        self.process = subprocess.Popen([str(BINARY)], env=self.env, stdout=self.log, stderr=self.log)
        for _ in range(150):
            if self.process.poll() is not None:
                self.fail('Isolated server exited during startup')
            try:
                if self.request('/auth/status', authenticated=False)[0] == 200: return
            except OSError: pass
            time.sleep(.05)
        self.fail('Server did not start')

    def stop(self):
        self.process.terminate(); self.process.wait(timeout=10); self.log.close()

    def tearDown(self):
        self.stop(); self.temp.cleanup()

    def request(self, path, data=None, authenticated=True, headers=None, method=None):
        h = {}
        if data is not None: h.update({'Content-Type':'application/json', 'Origin':self.origin})
        if authenticated: h.update({'Cookie':self.cookie, 'X-CSRF-Token':self.csrf, 'X-Library-Id':self.library})
        h.update(headers or {})
        c = http.client.HTTPConnection('127.0.0.1', self.port, timeout=15)
        c.request(method or ('POST' if data is not None else 'GET'), '/api'+path,
                  json.dumps(data) if data is not None else None, h)
        r = c.getresponse(); status=r.status; out=dict(r.getheaders()); body=r.read(); c.close()
        try: body=json.loads(body)
        except (ValueError, UnicodeDecodeError): pass
        return status, body, out

    def setup_admin(self, remember=True):
        text=(self.root/'server.log').read_text()
        code=re.search(r'設定碼（30 分鐘內有效）：([0-9a-f]+)', text).group(1)
        status, _, h=self.request('/auth/setup', {'username':'admin','password':PASSWORD,'code':code,'remember':remember}, False)
        self.assertEqual(status,200)
        self.cookie=h['set-cookie'].split(';')[0]
        status, session,_=self.request('/auth/me'); self.assertEqual(status,200); self.csrf=session['csrf']
        self.session_id=session['id']; return code,h

    def seed(self):
        self.setup_admin()
        libs=[{'id':'default','name':'Default','db_name':'stickplay','paths':[]},
              {'id':'a','name':'Library A','db_name':'a','paths':[str(self.lib_a)]},
              {'id':'b','name':'Library B','db_name':'b','paths':[str(self.lib_b)]}]
        self.assertEqual(self.request('/save_libraries',libs)[0],200)
        for name, root in [('a',self.lib_a),('b',self.lib_b)]:
            self.assertEqual(self.request('/switch_database',{'dbName':name})[0],200)
            folder=root/'nested'/'film'; folder.mkdir(parents=True)
            (folder/'movie.mp4').write_bytes(b'0123456789abcdef')
            (folder/'movie.nfo').write_text('<movie><num>SAME-1</num><title>Original</title><originaltitle>Original kept</originaltitle><studio>Studio kept</studio><actor><name>One</name></actor><actor><name>Two</name></actor><releasedate>2020-01-01</releasedate><genre>無碼</genre><fileinfo><title>Nested title</title></fileinfo></movie>')
            (folder/'image.png').write_bytes(png())
            with database(self.config/f'{name}.db') as db:
                db.execute('INSERT INTO videos(id,title,video_path,folder_path,nfo_path) VALUES(?,?,?,?,?)',
                           ('SAME-1', name, str(folder/'movie.mp4'),str(folder),str(folder/'movie.nfo')))
        self.folder=self.lib_a/'nested'/'film'

    def test_anonymous_routes_fail_closed(self):
        posts=['scan_library','sync_watch_paths','rescan_single_video','query_videos','update_video_info','update_rating','toggle_favorite','get_all_genres','get_all_levels','get_stats','switch_database','delete_database','get_fanart_path','list_dirs','get_libraries','save_libraries','get_folder_images','crop_and_save_poster','move_video_folder','playback-tickets','auth/logout','auth/password','auth/revoke']
        for endpoint in posts:
            with self.subTest(endpoint=endpoint): self.assertEqual(self.request('/'+endpoint,{},False)[0],401)
        for endpoint in ['events','video?path=/etc/passwd','image?path=/etc/passwd','auth/me','auth/devices','player-tools/windows','unknown']:
            with self.subTest(endpoint=endpoint): self.assertEqual(self.request('/'+endpoint,authenticated=False)[0],401)
        self.assertTrue(self.request('/auth/status',authenticated=False)[1]['setupRequired'])

    def test_authenticated_player_tool_downloads(self):
        self.setup_admin()
        expected = {
            'windows': {
                'install.cmd', 'install.ps1', 'launch-player.ps1',
                'uninstall.cmd', 'uninstall.ps1',
            },
            'macos': {
                'Install StickPlay VLC.command',
                'Uninstall StickPlay VLC.command',
            },
        }
        for platform, filenames in expected.items():
            with self.subTest(platform=platform):
                status, body, headers = self.request(f'/player-tools/{platform}')
                self.assertEqual(status, 200)
                self.assertEqual(headers['content-type'], 'application/zip')
                self.assertIn(f'stickplay-player-tools-{platform}.zip', headers['content-disposition'])
                self.assertEqual(body[:4], b'PK\x03\x04')
                with zipfile.ZipFile(io.BytesIO(body)) as archive:
                    self.assertEqual(set(archive.namelist()), filenames)
                    self.assertIsNone(archive.testzip())
                    if platform == 'windows':
                        installer = archive.read('install.ps1').decode()
                        self.assertIn('$env:LOCALAPPDATA', installer)
                        self.assertNotIn('C:\\Users\\', installer)
                    else:
                        info = archive.getinfo('Install StickPlay VLC.command')
                        self.assertEqual((info.external_attr >> 16) & 0o111, 0o111)
        self.assertEqual(self.request('/player-tools/linux')[0], 404)

    def test_setup_cookie_restart_and_logout(self):
        code,h=self.setup_admin()
        self.assertIn('HttpOnly',h['set-cookie']); self.assertIn('SameSite=Lax',h['set-cookie']); self.assertIn('Max-Age=7776000',h['set-cookie'])
        self.assertEqual(self.request('/auth/setup',{'username':'other','password':PASSWORD,'code':code},False)[0],409)
        with database(self.config/'auth.db') as db:
            encoded=db.execute('SELECT password FROM admin').fetchone()[0]
            stored=db.execute('SELECT token_hash FROM sessions').fetchone()[0]
            self.assertTrue(encoded.startswith('$argon2id$')); self.assertNotIn(stored,self.cookie)
        self.stop(); self.start(); self.assertEqual(self.request('/auth/me')[0],200)
        old_cookie=self.cookie
        self.assertEqual(self.request('/auth/logout',{})[0],200)
        self.assertEqual(self.request('/auth/me',headers={'Cookie':old_cookie})[0],401)

    def test_csrf_origin_and_forgery(self):
        self.setup_admin()
        for h in [{'Origin':'https://evil.example'},{'X-CSRF-Token':'wrong'},{'Origin':'null'}]:
            self.assertEqual(self.request('/get_libraries',{},headers=h)[0],403)
        self.assertEqual(self.request('/get_libraries',{},headers={'Content-Type':'text/plain'})[0],415)
        self.assertEqual(self.request('/auth/me',headers={'Cookie':'stickplay_session=forged'})[0],401)
        self.assertNotIn('access-control-allow-origin', self.request('/auth/me')[2])
        self.assertEqual(self.request('/auth/login',{'username':'admin','password':PASSWORD},False,{'Origin':'https://evil.example'})[0],403)

    def test_expiration_and_nonremember_cookie(self):
        _,h=self.setup_admin(False); self.assertNotIn('Max-Age=',h['set-cookie'])
        with database(self.config/'auth.db') as db: db.execute('UPDATE sessions SET expires=0')
        self.assertEqual(self.request('/auth/me')[0],401)

    def test_idle_expiration(self):
        self.setup_admin()
        with database(self.config/'auth.db') as db: db.execute('UPDATE sessions SET last_used=?',(int(time.time())-90*86400-1,))
        self.assertEqual(self.request('/auth/me')[0],401)

    def test_login_rate_limit_and_recovery(self):
        self.setup_admin()
        for _ in range(10): status=self.request('/auth/login',{'username':'wrong','password':'wrong'},False)[0]
        self.assertEqual(status,429)

    def test_recovery_revokes_previous_sessions(self):
        self.setup_admin(); previous=self.cookie
        code=subprocess.check_output([str(BINARY),'--auth-recovery'],env=self.env,text=True).strip().split('：')[1]
        status,_,h=self.request('/auth/recover',{'username':'admin','password':'replacement password 123','code':code,'remember':True},False)
        self.assertEqual(status,200)
        self.assertEqual(self.request('/auth/me',headers={'Cookie':previous})[0],401)
        self.assertEqual(self.request('/auth/recover',{'username':'admin','password':PASSWORD,'code':code},False)[0],401)
        self.assertIn('Max-Age',h['set-cookie'])

    def test_paths_and_database_traversal(self):
        self.seed()
        (self.lib_a/'escape').symlink_to(self.config,target_is_directory=True)
        for path in [str(self.config/'auth.db'), str(self.media/'..'/'config'/'auth.db'),str(self.lib_a/'escape'/'auth.db'),str(self.lib_b/'nested/film/movie.mp4')]:
            q=urllib.parse.urlencode({'path':path,'libraryId':'a'})
            self.assertNotEqual(self.request('/video?'+q)[0],200)
            self.assertNotEqual(self.request('/image?'+q)[0],200)
        self.assertEqual(self.request('/list_dirs',{'path':str(self.lib_a/'escape')})[0],403)
        for db_name in ['../config/auth','/tmp/outside','auth','unregistered']:
            self.assertNotEqual(self.request('/delete_database',{'dbName':db_name})[0],200)
            self.assertNotEqual(self.request('/switch_database',{'dbName':db_name})[0],200)
        self.assertTrue((self.config/'auth.db').exists())
        for route, data in [('scan_library',{'paths':[str(self.config)]}),('get_folder_images',{'folderPath':str(self.config)}),('rescan_single_video',{'folderPath':str(self.config)})]:
            self.assertEqual(self.request('/'+route,data)[0],403)

    def test_library_isolation_conflict_and_nfo_roundtrip(self):
        self.seed()
        self.assertEqual(self.request('/switch_database',{'dbName':'b'})[0],200)
        self.assertEqual(self.request('/query_videos',{'filter':{}})[1][0]['title'],'a')
        self.assertEqual(self.request('/query_videos',{'filter':{}},headers={'X-Library-Id':'b'})[1][0]['title'],'b')
        self.assertEqual(self.request('/toggle_favorite',{'videoId':'SAME-1'})[0],200)
        self.assertFalse(self.request('/query_videos',{'filter':{}},headers={'X-Library-Id':'b'})[1][0]['is_favorite'])
        data={'originalId':'SAME-1','videoId':'NEW-1','title':'A & B <Title>','level':'C','rating':8.5,'criticrating':85,'actors':['One & More','Two <Actor>'],'genres':['劇情','精選','無碼'],'year':'2026','releaseDate':'2026-01-01','dateAdded':'2026-09-01','isFavorite':True,'isUncensored':False,'videoPath':'/etc/passwd','folderPath':'/etc','posterPath':None,'nfoPath':'/etc/evil.nfo'}
        self.assertEqual(self.request('/update_video_info',data)[0],200)
        self.assertEqual(self.request('/rescan_single_video',{'folderPath':str(self.folder)})[0],200)
        v=self.request('/query_videos',{'filter':{}})[1][0]
        self.assertEqual(v['id'],'NEW-1'); self.assertEqual(v['level'],'C'); self.assertEqual(v['title'],data['title']); self.assertCountEqual(v['actors'],data['actors']); self.assertEqual(v['year'],'2026'); self.assertCountEqual(v['genres'],['劇情','精選']); self.assertNotIn('無碼',v['genres']); self.assertTrue(v['is_favorite'])
        xml=(self.folder/'movie.nfo').read_text()
        for text in ['<year>2026</year>','<genre>劇情</genre>','<genre>精選</genre>','<originaltitle>Original kept</originaltitle>','<studio>Studio kept</studio>']:
            self.assertIn(text,xml)
        with database(self.config/'a.db') as db:
            db.execute("INSERT INTO videos(id,title,video_path,folder_path) VALUES('COLLISION','Other','other','other')")
        before=(self.folder/'movie.nfo').read_bytes()
        data.update(originalId='NEW-1',videoId='COLLISION')
        self.assertEqual(self.request('/update_video_info',data)[0],409)
        self.assertEqual((self.folder/'movie.nfo').read_bytes(),before)
        self.assertEqual(len(self.request('/query_videos',{'filter':{}})[1]),2)

    def test_rating_preserves_nfo_and_nested_scan(self):
        self.seed()
        self.assertEqual(self.request('/update_rating',{'videoId':'SAME-1','rating':9,'criticrating':90,'nfoPath':'/etc/forged'})[0],200)
        xml=(self.folder/'movie.nfo').read_text()
        for text in ['Original','<name>One</name>','<name>Two</name>','2020-01-01','<genre>無碼</genre>','<fileinfo><title>Nested title</title></fileinfo>']:
            self.assertIn(text,xml)
        self.assertEqual(self.request('/scan_library',{'paths':[str(self.lib_a)]})[0],200)
        self.assertEqual(len(self.request('/query_videos',{'filter':{}})[1]),1)

    def test_removing_all_library_paths_prunes_only_that_index(self):
        self.seed()
        libraries = self.request('/get_libraries', {})[1]
        for library in libraries:
            if library['id'] == 'a':
                library['paths'] = []
        self.assertEqual(self.request('/save_libraries', libraries)[0], 200)
        self.assertEqual(self.request('/scan_library', {'paths': []})[0], 200)
        self.assertEqual(self.request('/query_videos', {'filter': {}})[1], [])
        videos_b = self.request(
            '/query_videos', {'filter': {}}, headers={'X-Library-Id': 'b'}
        )[1]
        self.assertEqual(len(videos_b), 1)

    def test_crop_preserves_nfo_and_rejects_outside_output(self):
        self.seed()
        payload={'videoId':'SAME-1','imagePath':str(self.folder/'image.png'),'outputFolder':str(self.folder),'x':0,'y':0,'width':1,'height':1}
        status,body,_=self.request('/crop_and_save_poster',payload)
        self.assertEqual(status,200,body)
        xml=(self.folder/'movie.nfo').read_text()
        for text in ['<title>Original</title>','<name>One</name>','<name>Two</name>','<genre>無碼</genre>','<fileinfo><title>Nested title</title></fileinfo>']:
            self.assertIn(text,xml)
        payload['outputFolder']=str(self.config)
        self.assertEqual(self.request('/crop_and_save_poster',payload)[0],403)
        self.assertFalse((self.config/'poster.jpg').exists())

    def test_playback_ticket_range_scope_and_revocation(self):
        self.seed()
        status,url,_=self.request('/playback-tickets',{'videoId':'SAME-1'}); self.assertEqual(status,200)
        path=url.removeprefix('/api')
        for byte_range,expected in [('bytes=0-3',b'0123'),('bytes=8-11',b'89ab')]:
            status,body,h=self.request(path,authenticated=False,headers={'Range':byte_range})
            self.assertEqual(status,206); self.assertEqual(body,expected); self.assertIn('content-range',h)
        self.assertEqual(self.request('/query_videos?'+url.split('?')[1],{},False)[0],401)
        self.assertEqual(self.request(path+'&path=/etc/passwd',authenticated=False)[1],b'0123456789abcdef')
        self.assertEqual(self.request('/auth/logout',{})[0],200)
        self.assertEqual(self.request(path,authenticated=False)[0],401)

    def test_devices_password_and_backup_delete(self):
        self.seed(); previous=self.cookie
        status,_,h=self.request('/auth/login',{'username':'admin','password':PASSWORD,'remember':True},False)
        self.assertEqual(status,200)
        second=h['set-cookie'].split(';')[0]
        self.assertEqual(len(self.request('/auth/devices')[1]),2)
        self.assertEqual(self.request('/auth/revoke',{})[0],200)
        self.assertEqual(self.request('/auth/me',headers={'Cookie':second})[0],401)
        self.assertEqual(self.request('/delete_database',{'dbName':'b'})[0],200)
        self.assertEqual(len(list((self.config/'backups').glob('*.db'))),1)
        self.assertTrue((self.lib_b/'nested/film/movie.mp4').exists())
        status,_,h=self.request('/auth/password',{'currentPassword':PASSWORD,'newPassword':'changed secure password 5678'})
        self.assertEqual(status,200)
        self.assertEqual(self.request('/auth/me',headers={'Cookie':previous})[0],401)
        self.assertEqual(self.request('/auth/me',headers={'Cookie':h['set-cookie'].split(';')[0]})[0],200)

if __name__=='__main__': unittest.main()

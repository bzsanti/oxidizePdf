// Dependency-free Chrome DevTools runner. Node 22+, built web package required.
const http = require('node:http');
const fs = require('node:fs');
const path = require('node:path');
const os = require('node:os');
const assert = require('node:assert/strict');
const {spawn} = require('node:child_process');
const server = http.createServer((req, res) => {
  const file = path.resolve(__dirname, '.' + new URL(req.url, 'http://localhost').pathname);
  if (!file.startsWith(__dirname + path.sep)) { res.writeHead(403).end(); return; }
  fs.readFile(file, (err, bytes) => {
    if (err) { res.writeHead(404).end(); return; }
    res.setHeader('Content-Type', file.endsWith('.wasm') ? 'application/wasm' : file.endsWith('.js') ? 'text/javascript' : 'text/html');
    res.end(bytes);
  });
});
server.listen(0, '127.0.0.1', async () => {
  const profile = fs.mkdtempSync(path.join(os.tmpdir(), 'oxidize-711-chrome-'));
  const chrome = spawn(process.env.CHROME_BIN || 'google-chrome', [
    '--headless', '--no-sandbox', '--disable-dev-shm-usage', '--disable-gpu',
    '--disable-background-networking', '--no-first-run', '--remote-debugging-pipe',
    `--user-data-dir=${profile}`,
  ], {stdio: ['ignore', 'ignore', 'pipe', 'pipe', 'pipe'], detached: true});
  let stderr = '', buffer = '', sequence = 0;
  const pending = new Map();
  chrome.stderr.on('data', bytes => { stderr += bytes; });
  chrome.on('error', err => { console.error(err); process.exitCode = 1; cleanup(); });
  chrome.stdio[4].on('data', bytes => {
    buffer += bytes;
    let end;
    while ((end = buffer.indexOf('\0')) !== -1) {
      const message = JSON.parse(buffer.slice(0, end)); buffer = buffer.slice(end + 1);
      const call = pending.get(message.id);
      if (call) { pending.delete(message.id); message.error ? call.reject(message.error) : call.resolve(message.result); }
    }
  });
  function send(method, params = {}, sessionId) {
    const id = ++sequence;
    return new Promise((resolve, reject) => {
      pending.set(id, {resolve, reject});
      chrome.stdio[3].write(JSON.stringify({id, method, params, sessionId}) + '\0');
    });
  }
  let cleaned = false;
  function cleanup() {
    if (cleaned) return; cleaned = true;
    clearTimeout(timeout);
    if (chrome.pid) { try { process.kill(-chrome.pid, 'SIGKILL'); } catch (e) { if (e.code !== 'ESRCH') throw e; } }
    server.close();
    chrome.on('close', () => fs.rmSync(profile, {recursive: true, force: true, maxRetries: 5, retryDelay: 100}));
  }
  const timeout = setTimeout(() => { console.error('Browser contract timed out', stderr); process.exitCode = 1; cleanup(); }, 60000);
  try {
    const {targetId} = await send('Target.createTarget', {url: 'about:blank'});
    const {sessionId} = await send('Target.attachToTarget', {targetId, flatten: true});
    await send('Page.enable', {}, sessionId);
    await send('Page.navigate', {url: `http://127.0.0.1:${server.address().port}/browser.html`}, sessionId);
    // Wait for the module's explicit result, not virtual-time or DOM-load guesses.
    let result;
    for (let i = 0; i < 300; i++) {
      result = await send('Runtime.evaluate', {expression: "document.querySelector('#result')?.textContent", returnByValue: true}, sessionId);
      if (result.result.value && result.result.value !== 'RUNNING') break;
      await new Promise(resolve => setTimeout(resolve, 100));
    }
    assert.match(result.result.value || '', /^PASS:/);
    console.log(result.result.value);
  } catch (error) { console.error(error, stderr); process.exitCode = 1; }
  finally { cleanup(); }
});

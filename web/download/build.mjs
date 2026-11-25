import { createHash } from 'node:crypto';
import { copyFile, mkdir, mkdtemp, readFile, rename, rm, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const repo = 'nidhoggfgg/rsille';
const root = fileURLToPath(new URL('../../', import.meta.url));
const source = join(root, 'web/download');
const output = join(root, 'target/playground-site');
const manifest = 'SHA256SUMS-rsille-playground';
const platforms = [
  { id: 'windows', title: 'Windows', hint: '适用于 64 位 Intel 和 AMD 电脑。', command: '.\\rsille.exe --help',
    files: [['rsille.exe', 'Windows x64']] },
  { id: 'macos', title: 'macOS', hint: '在“关于本机”中查看芯片类型。',
    files: [['rsille-macos-apple-silicon', 'Apple 芯片版'], ['rsille-macos-intel', 'Intel 芯片版']] },
  { id: 'linux', title: 'Linux / WSL', hint: '用 uname -m 查看处理器架构。',
    files: [['rsille-linux-x64', 'x86_64 版'], ['rsille-linux-arm64', 'ARM64 版（aarch64）']] },
];

async function get(url) {
  const response = await fetch(url, { signal: AbortSignal.timeout(90000) });
  if (!response.ok) throw new Error(`HTTP ${response.status} fetching ${url}`);
  return response;
}

// Companion releases share a repository with several independently versioned crates.
async function latestBinaryRelease() {
  let latest;
  for (let page = 1; ; page++) {
    const response = await get(`https://api.github.com/repos/${repo}/releases?per_page=100&page=${page}`);
    const releases = await response.json();
    for (const release of releases) {
      const match = /^rsille-playground-v(\d+\.\d+\.\d+)$/.exec(release.tag_name);
      if (!match || release.draft || release.prerelease) continue;
      const version = match[1];
      // Up through 0.1.2 the project only shipped the crate.
      if (version.localeCompare('0.1.2', 'en', { numeric: true }) <= 0) continue;
      if (!latest || version.localeCompare(latest, 'en', { numeric: true }) > 0) latest = version;
    }
    if (releases.length < 100) return latest;
  }
}

async function mirror(version, stage) {
  const base = `https://github.com/${repo}/releases/download/rsille-playground-v${version}`;
  const dir = join(stage, 'files', version);
  await mkdir(dir, { recursive: true });
  const sums = await (await get(`${base}/${manifest}`)).text();
  const checksums = new Map();
  for (const line of sums.trimEnd().split('\n')) {
    const entry = /^([a-f0-9]{64})  ([\w.-]+)$/.exec(line);
    if (!entry || checksums.has(entry[2])) throw new Error('Invalid release checksum manifest');
    checksums.set(entry[2], entry[1]);
  }
  const names = platforms.flatMap(p => p.files.map(([name]) => name));
  for (const name of [...names, 'LICENCE']) {
    if (!checksums.has(name)) throw new Error(`Missing checksum for ${name}`);
    const data = Buffer.from(await (await get(`${base}/${name}`)).arrayBuffer());
    if (createHash('sha256').update(data).digest('hex') !== checksums.get(name)) {
      throw new Error(`Checksum mismatch for ${name}`);
    }
    await writeFile(join(dir, name), data);
  }
  await writeFile(join(dir, manifest), sums);
}

function downloadCards(version) {
  const cards = platforms.map(({ id, title, hint, command, files }) => {
    const links = files.map(([name, label]) => {
      const download = id === 'windows' ? 'rsille.exe' : 'rsille';
      return `<a class="button" href="/files/${version}/${name}" download="${download}">下载 ${label}<span aria-hidden="true">↓</span></a>`;
    }).join('\n');
    return `<article id="${id}"><h3>${title}</h3><p class="hint">${hint}</p><div class="links">${links}</div>
      <div class="run">在下载目录运行<pre><code>${command ?? 'chmod +x rsille\n./rsille --help'}</code></pre></div></article>`;
  });
  return `<h2>选择你的系统</h2><div class="cards">${cards.join('\n')}</div>
    <footer><a href="/files/${version}/${manifest}" download>SHA-256 校验清单</a>
    <a href="/files/${version}/LICENCE" download>MIT 许可证</a></footer>`;
}

const version = await latestBinaryRelease();
await mkdir(join(root, 'target'), { recursive: true });
const stage = await mkdtemp(join(root, 'target/playground-site-'));
try {
  if (version) await mirror(version, stage);
  const template = await readFile(join(source, 'index.html'), 'utf8');
  const content = version ? downloadCards(version) :
    '<section class="waiting"><h2>首个独立下载版本准备中</h2><p>发布后，这里将提供 Windows、macOS 和 Linux 可执行文件。</p><a href="https://github.com/nidhoggfgg/rsille/releases?q=rsille-playground-v">查看已有版本 ↗</a></section>';
  await mkdir(join(stage, 'download'));
  await writeFile(join(stage, 'download/index.html'), template
    .replace('{{version}}', version ? `<span class="version">v${version}</span>` : '')
    .replace('{{downloads}}', content));
  await copyFile(join(root, 'packages/rsille-playground/ai-babies/preview-static.png'), join(stage, 'characters.png'));
  await rm(output, { recursive: true, force: true });
  await rename(stage, output);
  console.log(`Built ${output} (${version ?? 'awaiting first binary release'})`);
} finally {
  await rm(stage, { recursive: true, force: true });
}

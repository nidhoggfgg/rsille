import { createHash } from 'node:crypto';
import { copyFile, mkdir, mkdtemp, readFile, rename, rm, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const repo = 'nidhoggfgg/rsille';
const root = fileURLToPath(new URL('../', import.meta.url));
const source = join(root, 'web');
const output = join(root, 'target/site');
const manifest = 'SHA256SUMS-rsille-playground';
const offline = process.argv.includes('--offline');

const platforms = [
  {
    id: 'windows',
    title: 'Windows',
    hintZh: '适用于 64 位 Intel / AMD 电脑（Windows 10 1809+）。',
    hintEn: 'For 64-bit Intel / AMD PCs (Windows 10 1809+).',
    tipZh: '推荐在 <code>Windows Terminal 1.22+</code>（原生支持 Sixel）运行。',
    tipEn: 'Best in <code>Windows Terminal 1.22+</code> (built-in Sixel support).',
    files: [
      {
        name: 'rsille.exe',
        labelZh: 'Windows x64',
        labelEn: 'Windows x64',
        run: '.\\rsille.exe pet -- powershell.exe -NoLogo',
      },
    ],
  },
  {
    id: 'macos',
    title: 'macOS',
    hintZh: '在“关于本机”中查看芯片类型（Apple Silicon / Intel）。',
    hintEn: 'Check chip type in About This Mac (Apple Silicon / Intel).',
    tipZh: '若系统拦截未签名程序，可先运行 <code>xattr -d com.apple.quarantine ./rsille-macos-*</code>。',
    tipEn: 'If Gatekeeper blocks the binary, run <code>xattr -d com.apple.quarantine ./rsille-macos-*</code>.',
    files: [
      {
        name: 'rsille-macos-apple-silicon',
        labelZh: 'Apple 芯片版',
        labelEn: 'Apple Silicon',
        run: 'chmod +x rsille-macos-apple-silicon\n./rsille-macos-apple-silicon pet',
      },
      {
        name: 'rsille-macos-intel',
        labelZh: 'Intel 芯片版',
        labelEn: 'Intel (x86_64)',
        run: 'chmod +x rsille-macos-intel\n./rsille-macos-intel pet',
      },
    ],
  },
  {
    id: 'linux',
    title: 'Linux / WSL',
    hintZh: '用 uname -m 查看架构（x86_64 或 aarch64）。',
    hintEn: 'Run uname -m to check architecture (x86_64 or aarch64).',
    tipZh: '支持 Kitty / WezTerm / Foot / Ghostty 等图形终端；纯字符模式支持任意真彩色终端。',
    tipEn: 'Works in Kitty / WezTerm / Foot / Ghostty; Unicode modes work in any True Color TTY.',
    files: [
      {
        name: 'rsille-linux-x64',
        labelZh: 'Linux x86_64',
        labelEn: 'Linux x86_64',
        run: 'chmod +x rsille-linux-x64\n./rsille-linux-x64 pet',
      },
      {
        name: 'rsille-linux-arm64',
        labelZh: 'Linux ARM64',
        labelEn: 'Linux ARM64 (aarch64)',
        run: 'chmod +x rsille-linux-arm64\n./rsille-linux-arm64 pet',
      },
    ],
  },
];

const mediaAssets = [
  ['examples/lumen/preview-desk.png', 'lumen-desk.png'],
  ['examples/pokemon-battle/preview.png', 'pokemon-battle.png'],
  ['examples/minecraft/preview.png', 'minecraft.png'],
  ['examples/orbit-files/preview.png', 'orbit-files.png'],
  ['examples/clockwork/preview.gif', 'clockwork.gif'],
  ['examples/clockwork/preview.png', 'clockwork-poster.png'],
  ['packages/rsille-playground/function-plotter/preview.gif', 'function-plotter.gif'],
  ['packages/rsille-playground/jellyfish/preview.gif', 'jellyfish.gif'],
  ['packages/rsille-playground/ai-babies/preview-deepseek.gif', 'baby-deepseek.gif'],
  ['packages/rsille-playground/ai-babies/preview-claude.gif', 'baby-claude.gif'],
  ['packages/rsille-playground/ai-babies/preview-pet.gif', 'baby-pet.gif'],
  ['packages/rsille-playground/deepseek-pet/assets/animations/whale/level.png', 'whale-level.png'],
  ['imgs/readme-counter-cn.gif', 'counter-cn.gif'],
  ['imgs/readme-counter-en.gif', 'counter-en.gif'],
];

function compareSemver(a, b) {
  const pa = a.split('.').map(Number);
  const pb = b.split('.').map(Number);
  for (let i = 0; i < 3; i++) {
    if (pa[i] !== pb[i]) return pa[i] - pb[i];
  }
  return 0;
}

async function readCargoVersion(relPath) {
  const toml = await readFile(join(root, relPath), 'utf8');
  const match = /^\[package\][\s\S]*?^version\s*=\s*"([^"]+)"/m.exec(toml);
  if (!match) throw new Error(`Could not read package version from ${relPath}`);
  return match[1];
}

function githubHeaders(isApi) {
  const headers = { 'User-Agent': 'rsille-site-builder' };
  if (isApi) headers.Accept = 'application/vnd.github+json';
  const token = process.env.GITHUB_TOKEN || process.env.GH_TOKEN;
  if (token && isApi) headers.Authorization = `Bearer ${token}`;
  return headers;
}

async function get(url, isApi = false) {
  const response = await fetch(url, {
    headers: githubHeaders(isApi),
    signal: AbortSignal.timeout(90000),
  });
  if (!response.ok) throw new Error(`HTTP ${response.status} fetching ${url}`);
  return response;
}

// Companion releases share a repository with several independently versioned crates.
async function latestBinaryRelease() {
  let latest;
  for (let page = 1; ; page++) {
    const response = await get(`https://api.github.com/repos/${repo}/releases?per_page=100&page=${page}`, true);
    const releases = await response.json();
    for (const release of releases) {
      const match = /^rsille-playground-v(\d+\.\d+\.\d+)$/.exec(release.tag_name);
      if (!match || release.draft || release.prerelease) continue;
      const version = match[1];
      // Up through 0.1.2 the project only shipped the crate.
      if (compareSemver(version, '0.1.2') <= 0) continue;
      if (!latest || compareSemver(version, latest) > 0) latest = version;
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
  const names = platforms.flatMap((p) => p.files.map((f) => f.name));
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

function escapeAttr(str) {
  return str
    .replaceAll('&', '&amp;')
    .replaceAll('"', '&quot;')
    .replaceAll('<', '&lt;')
    .replaceAll('>', '&gt;');
}

function downloadSection(version) {
  if (!version) {
    return `<div class="waiting-box">
      <h3><span data-i18n-zh>独立下载版本准备中</span><span data-i18n-en>Standalone binary release in progress</span></h3>
      <p><span data-i18n-zh>发布完成后，这里将直接提供经 SHA-256 校验的 Windows、macOS 和 Linux 可执行文件。</span><span data-i18n-en>Once published, SHA-256 verified executables for Windows, macOS, and Linux will be mirrored here.</span></p>
      <a class="btn btn-secondary" href="https://github.com/${repo}/releases?q=rsille-playground-v" target="_blank" rel="noopener">
        <span data-i18n-zh>查看 GitHub Releases ↗</span><span data-i18n-en>View GitHub Releases ↗</span>
      </a>
    </div>`;
  }

  const cards = platforms.map((p) => {
    const defaultRun = p.files[0].run;
    const links = p.files.map((f, idx) => {
      const cls = idx === 0 ? 'dl-btn is-primary-arch' : 'dl-btn';
      return `<a class="${cls}" href="/files/${version}/${f.name}" download="${f.name}" data-run-cmd="${escapeAttr(f.run)}">
        <span class="dl-btn-info">
          <span><span data-i18n-zh>下载 ${f.labelZh}</span><span data-i18n-en>Download ${f.labelEn}</span></span>
          <span class="file-tag">${f.name}</span>
        </span>
        <span class="dl-btn-arrow" aria-hidden="true">↓</span>
      </a>`;
    }).join('\n');

    return `<article class="dl-card" data-os="${p.id}">
      <div class="dl-card-top">
        <h3>${p.title}</h3>
        <span class="detected-pill"><span data-i18n-zh>当前系统</span><span data-i18n-en>Detected</span></span>
      </div>
      <p class="dl-hint"><span data-i18n-zh>${p.hintZh}</span><span data-i18n-en>${p.hintEn}</span></p>
      <div class="dl-links">${links}</div>
      <div class="dl-run">
        <div class="dl-run-head">
          <span><span data-i18n-zh>在下载目录运行</span><span data-i18n-en>Run in your download folder</span></span>
          <button type="button" class="copy-btn" data-copy="${escapeAttr(defaultRun)}"><span data-i18n-zh>复制</span><span data-i18n-en>Copy</span></button>
        </div>
        <pre><code>${defaultRun}</code></pre>
        <p class="dl-tip"><span data-i18n-zh>${p.tipZh}</span><span data-i18n-en>${p.tipEn}</span></p>
      </div>
    </article>`;
  }).join('\n');

  return `<div class="dl-header">
      <div class="mono" style="font-size:13px;color:var(--ink-dim)">
        <span data-i18n-zh>最新版本：</span><span data-i18n-en>Latest release: </span>
        <span class="ver-pill">rsille-playground v${version}</span>
      </div>
      <div class="mono" style="font-size:12px;color:var(--ink-muted)">
        <span data-i18n-zh>所有文件均已在构建时通过 SHA-256 校验</span>
        <span data-i18n-en>All binaries SHA-256 verified at build time</span>
      </div>
    </div>
    <div class="dl-grid">${cards}</div>
    <div class="dl-footer">
      <span style="color:var(--ink-dim)">
        <span data-i18n-zh>校验与许可文件（校验命令：<code>sha256sum -c ${manifest}</code>）</span>
        <span data-i18n-en>Verification &amp; license (<code>sha256sum -c ${manifest}</code>)</span>
      </span>
      <div class="dl-footer-links">
        <a href="/files/${version}/${manifest}" download="${manifest}">SHA-256 ↓</a>
        <a href="/files/${version}/LICENCE" download="LICENCE">MIT LICENCE ↓</a>
        <a href="https://github.com/${repo}/releases/tag/rsille-playground-v${version}" target="_blank" rel="noopener">Release Notes ↗</a>
      </div>
    </div>`;
}

function renderTemplate(html, vars) {
  return html
    .replaceAll('{{coreVersion}}', vars.coreVersion)
    .replaceAll('{{playgroundBadge}}', vars.playgroundBadge)
    .replaceAll('{{downloads}}', vars.downloads);
}

const coreVersion = await readCargoVersion('Cargo.toml');
const playgroundVersion = offline
  ? await readCargoVersion('packages/rsille-playground/Cargo.toml')
  : await latestBinaryRelease();

await mkdir(join(root, 'target'), { recursive: true });
const stage = await mkdtemp(join(root, 'target/site-stage-'));
try {
  if (playgroundVersion && !offline) {
    await mirror(playgroundVersion, stage);
  }

  // Copy static site assets
  await Promise.all([
    copyFile(join(source, 'styles.css'), join(stage, 'styles.css')),
    copyFile(join(source, 'app.js'), join(stage, 'app.js')),
    copyFile(join(source, 'favicon.svg'), join(stage, 'favicon.svg')),
  ]);

  // Copy showcase & sprite media from the repository
  const mediaDir = join(stage, 'media');
  await mkdir(mediaDir, { recursive: true });
  await Promise.all(
    mediaAssets.map(([srcRel, destName]) =>
      copyFile(join(root, srcRel), join(mediaDir, destName)),
    ),
  );

  const vars = {
    coreVersion,
    playgroundBadge: playgroundVersion
      ? `rsille-playground v${playgroundVersion} · Direct CDN Mirror`
      : 'rsille-playground · Standalone CLI',
    downloads: downloadSection(playgroundVersion),
  };

  const [homeTpl, dlTpl] = await Promise.all([
    readFile(join(source, 'index.html'), 'utf8'),
    readFile(join(source, 'download.html'), 'utf8'),
  ]);

  await writeFile(join(stage, 'index.html'), renderTemplate(homeTpl, vars));
  await mkdir(join(stage, 'download'), { recursive: true });
  await writeFile(join(stage, 'download/index.html'), renderTemplate(dlTpl, vars));

  await rm(output, { recursive: true, force: true });
  await rename(stage, output);
  console.log(
    `Built ${output} (rsille v${coreVersion}, playground ${playgroundVersion ? `v${playgroundVersion}` : 'awaiting release'}${offline ? ', offline preview' : ''})`,
  );
} finally {
  await rm(stage, { recursive: true, force: true });
}

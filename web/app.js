(() => {
  const doc = document.documentElement;

  /* 1. Bilingual language switcher */
  function setLang(lang, persist) {
    const normalized = lang === 'en' ? 'en' : 'zh';
    doc.dataset.lang = normalized;
    doc.lang = normalized === 'en' ? 'en' : 'zh-CN';
    const title = doc.dataset[normalized === 'en' ? 'titleEn' : 'titleZh'];
    if (title) document.title = title;

    document.querySelectorAll('[data-lang-btn]').forEach((btn) => {
      const active = btn.dataset.langBtn === normalized;
      btn.classList.toggle('is-active', active);
      btn.setAttribute('aria-pressed', String(active));
    });

    if (persist) {
      try { localStorage.setItem('rsille-lang', normalized); } catch {}
      try {
        const url = new URL(location.href);
        url.searchParams.set('lang', normalized);
        history.replaceState(null, '', url);
      } catch {}
    }
  }

  setLang(doc.dataset.lang || 'zh', false);

  document.querySelectorAll('[data-lang-btn]').forEach((btn) => {
    btn.addEventListener('click', () => setLang(btn.dataset.langBtn, true));
  });

  /* 2. Platform auto-detection for download cards & Hero CTA */
  const ua = navigator.userAgent || '';
  const platform = navigator.userAgentData?.platform || navigator.platform || '';
  const isMobile = /Android|iPhone|iPad|iPod/i.test(ua) ||
    (/Mac/i.test(platform) && navigator.maxTouchPoints > 1);
  const system = /Win/i.test(platform)
    ? 'windows'
    : /Mac/i.test(platform)
      ? 'macos'
      : /Linux/i.test(platform)
        ? 'linux'
        : '';

  if (system && !isMobile) {
    document.querySelectorAll('.dl-grid').forEach((grid) => {
      const card = grid.querySelector(`[data-os="${system}"]`);
      if (card) {
        grid.prepend(card);
        card.classList.add('current');
      }
    });

    const labels = {
      windows: { zh: 'Windows', en: 'Windows' },
      macos: { zh: 'macOS', en: 'macOS' },
      linux: { zh: 'Linux', en: 'Linux' },
    };
    const osBadge = document.getElementById('hero-os-label');
    if (osBadge && labels[system]) {
      osBadge.innerHTML =
        `<span data-i18n-zh>（${labels[system].zh}）</span>` +
        `<span data-i18n-en> (${labels[system].en})</span>`;
    }
  }

  /* 3. Update run snippet when clicking architecture download links */
  document.querySelectorAll('.dl-card').forEach((card) => {
    const codeEl = card.querySelector('.dl-run code');
    const copyBtn = card.querySelector('.dl-run .copy-btn');
    card.querySelectorAll('.dl-btn[data-run-cmd]').forEach((link) => {
      const syncCmd = () => {
        const cmd = link.dataset.runCmd;
        if (codeEl && cmd) codeEl.textContent = cmd;
        if (copyBtn && cmd) copyBtn.dataset.copy = cmd;
        card.querySelectorAll('.dl-btn').forEach((b) => b.classList.remove('is-primary-arch'));
        link.classList.add('is-primary-arch');
      };
      link.addEventListener('mouseenter', syncCmd);
      link.addEventListener('focus', syncCmd);
      link.addEventListener('click', syncCmd);
    });
  });

  /* 4. Copy-to-clipboard buttons */
  document.querySelectorAll('.copy-btn').forEach((btn) => {
    btn.addEventListener('click', async () => {
      const text = btn.dataset.copy || btn.closest('.cmd-bar, .code-box, .dl-run')?.querySelector('code')?.textContent?.trim();
      if (!text) return;
      try {
        await navigator.clipboard.writeText(text);
        btn.classList.add('copied');
        const orig = btn.innerHTML;
        btn.innerHTML = '<span data-i18n-zh>已复制 ✓</span><span data-i18n-en>Copied ✓</span>';
        setTimeout(() => {
          btn.classList.remove('copied');
          btn.innerHTML = orig;
        }, 1600);
      } catch {}
    });
  });

  /* 5. Hero interactive terminal tabs (with lazy-loading for heavier GIFs) */
  document.querySelectorAll('.term-tab').forEach((tab) => {
    tab.addEventListener('click', () => {
      const target = tab.dataset.termTab;
      document.querySelectorAll('.term-tab').forEach((t) => {
        const active = t === tab;
        t.classList.toggle('is-active', active);
        t.setAttribute('aria-selected', String(active));
      });
      document.querySelectorAll('.term-pane').forEach((pane) => {
        const active = pane.dataset.termPane === target;
        pane.classList.toggle('is-active', active);
        if (active) {
          pane.querySelectorAll('img[data-src]').forEach((img) => {
            img.src = img.dataset.src;
            img.removeAttribute('data-src');
          });
        }
      });
    });
  });

  /* 6. Interactive shell pet reactions inside Hero Tab 1 */
  const bubble = document.getElementById('pet-bubble');
  const shellOutput = document.getElementById('shell-dynamic');
  const reactions = {
    git: {
      cmd: '$ git push origin main',
      out: 'Enumerating objects: 14, done. main -> main',
      zh: '推送成功！先别关终端，陪我吹两个泡泡？',
      en: 'Clean push! Stay a second and blow some bubbles?',
    },
    cargo: {
      cmd: '$ cargo test --workspace',
      out: 'test result: ok. 142 passed; 0 failed; finished in 1.18s',
      zh: '全绿通过！奖励自己一条小鱼干。',
      en: 'All green! That earns a fresh fish treat.',
    },
    vim: {
      cmd: '$ rsille pet -- vim README.md',
      out: 'overlay active · Kitty graphics stays interactive over vim',
      zh: '就算进了 vim，我也能浮在上面接着游。',
      en: 'Even inside vim, I keep swimming right on top.',
    },
    pet: {
      cmd: '$ # clicked the whale pet',
      out: 'o o O  *happy splash*  (press Ctrl+] anytime to exit)',
      zh: '被点到了！饿了可是会啃你终端字符的哦。',
      en: 'Boop! Careful, I nibble terminal text when hungry.',
    },
  };

  function triggerReaction(key) {
    const item = reactions[key];
    if (!item || !bubble) return;
    bubble.innerHTML =
      `<span data-i18n-zh>${item.zh}</span>` +
      `<span data-i18n-en>${item.en}</span>`;
    if (shellOutput) {
      shellOutput.innerHTML =
        `<div class="shell-line"><span class="s-dir">~/rsille</span> <span class="s-git">main</span> ${item.cmd}</div>` +
        `<div class="shell-line"><span class="s-ok">✓</span> <span class="s-dim">${item.out}</span></div>`;
    }
  }

  document.querySelectorAll('[data-sim-action]').forEach((el) => {
    el.addEventListener('click', () => triggerReaction(el.dataset.simAction));
  });

  /* 7. Quickstart code tabs */
  document.querySelectorAll('.qs-tab').forEach((tab) => {
    tab.addEventListener('click', () => {
      const target = tab.dataset.qsTab;
      document.querySelectorAll('.qs-tab').forEach((t) => {
        const active = t === tab;
        t.classList.toggle('is-active', active);
        t.setAttribute('aria-selected', String(active));
      });
      document.querySelectorAll('.qs-panel').forEach((panel) => {
        panel.classList.toggle('is-active', panel.dataset.qsPanel === target);
      });
    });
  });
})();

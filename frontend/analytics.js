/**
 * Analytics & Billing — Proxy usage, errors, cost breakdown, and API keys.
 * Uses mock data; replace with real API calls when backend is ready.
 */

(function () {
  // ----- Mock data (replace with API) -----
  const MOCK_METRICS = {
    totalRequests: 124700,
    successRate: 98.4,
    avgLatencyMs: 342,
    estimatedSavingsUsd: 1247,
  };

  const MOCK_ERRORS = [
    { timestamp: '2026-02-05T14:32:01Z', function_id: 'chat_completion_abc', error: 'Rate limit exceeded on OpenAI. Retry after 60s.' },
    { timestamp: '2026-02-05T14:28:44Z', function_id: 'embed_xyz', error: 'Invalid API key for Anthropic.' },
    { timestamp: '2026-02-05T13:15:22Z', function_id: 'chat_completion_abc', error: 'Context length exceeded. Max tokens: 128000.' },
    { timestamp: '2026-02-05T12:01:09Z', function_id: 'llama_worker', error: 'Model temporarily unavailable (Llama).' },
    { timestamp: '2026-02-04T18:45:33Z', function_id: 'chat_completion_abc', error: 'Rate limit exceeded on OpenAI. Retry after 60s.' },
    { timestamp: '2026-02-04T11:22:17Z', function_id: 'embed_xyz', error: 'Timeout connecting to provider.' },
  ];

  const MOCK_TOKEN_USAGE = [
    { model: 'GPT-4', tokens: 2840000 },
    { model: 'Claude', tokens: 1920000 },
    { model: 'Llama', tokens: 910000 },
  ];

  const STORAGE_KEYS = 'promptkeeper_proxy_keys';

  // ----- State -----
  let allErrors = [...MOCK_ERRORS];
  let proxyKeys = loadKeys();

  function loadKeys() {
    try {
      const raw = localStorage.getItem(STORAGE_KEYS);
      return raw ? JSON.parse(raw) : [];
    } catch {
      return [];
    }
  }

  function saveKeys() {
    localStorage.setItem(STORAGE_KEYS, JSON.stringify(proxyKeys));
  }

  // ----- Global metrics -----
  function renderMetrics() {
    document.getElementById('statTotalRequests').textContent = formatNumber(MOCK_METRICS.totalRequests);
    document.getElementById('statSuccessRate').textContent = MOCK_METRICS.successRate + '%';
    document.getElementById('statAvgLatency').textContent = MOCK_METRICS.avgLatencyMs + ' ms';
    document.getElementById('statSavings').textContent = '$' + formatNumber(MOCK_METRICS.estimatedSavingsUsd);
  }

  function formatNumber(n) {
    if (n >= 1e6) return (n / 1e6).toFixed(1) + 'M';
    if (n >= 1e3) return (n / 1e3).toFixed(1) + 'K';
    return String(n);
  }

  // ----- Error log (searchable table) -----
  function formatTimestamp(iso) {
    const d = new Date(iso);
    return d.toLocaleString(undefined, {
      dateStyle: 'short',
      timeStyle: 'medium',
    });
  }

  function renderErrorTable(filtered) {
    const tbody = document.getElementById('errorTableBody');
    const emptyEl = document.getElementById('errorTableEmpty');
    tbody.innerHTML = '';

    if (filtered.length === 0) {
      emptyEl.hidden = false;
      return;
    }
    emptyEl.hidden = true;

    filtered.forEach((row) => {
      const tr = document.createElement('tr');
      tr.innerHTML =
        '<td class="col-timestamp">' +
        escapeHtml(formatTimestamp(row.timestamp)) +
        '</td><td class="col-function-id">' +
        escapeHtml(row.function_id) +
        '</td><td class="col-error">' +
        escapeHtml(row.error) +
        '</td>';
      tbody.appendChild(tr);
    });
  }

  function escapeHtml(s) {
    const div = document.createElement('div');
    div.textContent = s;
    return div.innerHTML;
  }

  function filterErrors(query) {
    const q = (query || '').trim().toLowerCase();
    if (!q) return allErrors;
    return allErrors.filter(
      (e) =>
        e.function_id.toLowerCase().includes(q) ||
        e.error.toLowerCase().includes(q) ||
        e.timestamp.toLowerCase().includes(q)
    );
  }

  function setupErrorSearch() {
    const input = document.getElementById('errorSearch');
    input.addEventListener('input', () => {
      renderErrorTable(filterErrors(input.value));
    });
  }

  // ----- Cost breakdown bar chart -----
  const CHART_COLORS = ['#8B7BA8', '#B19CD9', '#D4C5E8'];

  function renderTokenChart() {
    const data = MOCK_TOKEN_USAGE;
    const maxTokens = Math.max(...data.map((d) => d.tokens), 1);
    const chartEl = document.getElementById('tokenChart');
    const legendEl = document.getElementById('chartLegend');

    chartEl.innerHTML = '';
    legendEl.innerHTML = '';

    data.forEach((d, i) => {
      const pct = (d.tokens / maxTokens) * 100;
      const group = document.createElement('div');
      group.className = 'bar-group';
      group.innerHTML =
        '<span class="bar-label">' +
        escapeHtml(d.model) +
        '</span>' +
        '<div class="bar-wrapper">' +
        '<div class="bar" style="height:' +
        pct +
        '%;background:' +
        CHART_COLORS[i % CHART_COLORS.length] +
        '"></div>' +
        '</div>' +
        '<span class="bar-value">' +
        formatNumber(d.tokens) +
        ' tokens</span>';
      chartEl.appendChild(group);
    });

    data.forEach((d, i) => {
      const span = document.createElement('span');
      span.innerHTML =
        '<span class="dot" style="background:' +
        CHART_COLORS[i % CHART_COLORS.length] +
        '"></span> ' +
        escapeHtml(d.model);
      legendEl.appendChild(span);
    });
  }

  // ----- API keys -----
  function generateKey() {
    const prefix = 'pk_proxy_';
    const bytes = new Uint8Array(24);
    crypto.getRandomValues(bytes);
    const key =
      prefix +
      Array.from(bytes, (b) => b.toString(16).padStart(2, '0')).join('');
    const item = {
      id: 'key_' + Date.now(),
      key,
      created: new Date().toISOString(),
      revoked: false,
    };
    proxyKeys.unshift(item);
    saveKeys();
    return item;
  }

  function maskKey(key) {
    if (!key || key.length < 12) return '••••••••';
    return key.slice(0, 8) + '••••••••' + key.slice(-4);
  }

  function renderKeys() {
    const list = document.getElementById('keysList');
    const template = document.getElementById('keyItemTemplate');
    const emptyEl = document.getElementById('keysEmpty');

    const items = list.querySelectorAll('.key-item:not(.template)');
    items.forEach((el) => el.remove());

    if (proxyKeys.length === 0) {
      emptyEl.hidden = false;
      return;
    }
    emptyEl.hidden = true;

    proxyKeys.forEach((item) => {
      const clone = template.cloneNode(true);
      clone.hidden = false;
      clone.removeAttribute('id');
      clone.classList.remove('template');
      if (item.revoked) clone.classList.add('revoked');

      clone.querySelector('.key-preview').textContent = item.revoked
        ? maskKey(item.key) + ' (revoked)'
        : maskKey(item.key);
      clone.querySelector('.key-created').textContent =
        'Created ' + formatTimestamp(item.created);

      const copyBtn = clone.querySelector('[data-action="copy"]');
      const revokeBtn = clone.querySelector('[data-action="revoke"]');

      if (item.revoked) {
        copyBtn.disabled = true;
        revokeBtn.disabled = true;
        revokeBtn.textContent = 'Revoked';
      } else {
        copyBtn.addEventListener('click', () => copyToClipboard(item.key));
        revokeBtn.addEventListener('click', () => revokeKey(item.id));
      }

      list.appendChild(clone);
    });
  }

  function copyToClipboard(text) {
    navigator.clipboard.writeText(text).then(
      () => showToast('Copied to clipboard'),
      () => showToast('Copy failed', true)
    );
  }

  function revokeKey(id) {
    const item = proxyKeys.find((k) => k.id === id);
    if (!item || item.revoked) return;
    if (!confirm('Revoke this proxy key? Apps using it will stop working.')) return;
    item.revoked = true;
    saveKeys();
    renderKeys();
    showToast('Key revoked');
  }

  function showToast(message, isError) {
    const toast = document.createElement('div');
    toast.setAttribute('role', 'status');
    toast.style.cssText =
      'position:fixed;bottom:24px;right:24px;padding:12px 20px;border-radius:10px;font-size:0.9rem;font-weight:500;z-index:9999;' +
      (isError
        ? 'background:#c62828;color:white;'
        : 'background:var(--midnight-blue);color:var(--lavender-light);');
    toast.textContent = message;
    document.body.appendChild(toast);
    setTimeout(() => toast.remove(), 3000);
  }

  function openNewKeyModal(keyItem) {
    const modal = document.getElementById('newKeyModal');
    document.getElementById('newKeyValue').textContent = keyItem.key;
    modal.showModal();
  }

  function setupKeyModal() {
    const modal = document.getElementById('newKeyModal');
    document.getElementById('copyNewKey').addEventListener('click', () => {
      const key = document.getElementById('newKeyValue').textContent;
      copyToClipboard(key);
    });
    document.getElementById('closeNewKeyModal').addEventListener('click', () => {
      modal.close();
    });
  }

  function setupGenerateKey() {
    document.getElementById('btnGenerateKey').addEventListener('click', () => {
      const keyItem = generateKey();
      renderKeys();
      openNewKeyModal(keyItem);
    });
  }

  // ----- Init -----
  function init() {
    renderMetrics();
    renderErrorTable(allErrors);
    setupErrorSearch();
    renderTokenChart();
    renderKeys();
    setupKeyModal();
    setupGenerateKey();
  }

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }
})();

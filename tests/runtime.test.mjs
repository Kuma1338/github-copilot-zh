import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';
import { JSDOM } from 'jsdom';

const dictionary = {
  version: 1,
  testedAppVersions: ['1.1.15'],
  exact: {
    Home: '主页',
    Settings: '设置',
    'Search skills...': '搜索技能...'
  },
  patterns: [
    { source: '{count} sessions', target: '{count} 个会话' },
    { source: 'Search {count} skills', target: '搜索 {count} 个技能' },
    { source: '{count}% quota used', target: '已使用 {count}% 配额' },
    { source: 'Provided by your GitHub account {name}.', target: '由你的 GitHub 账户 {name} 提供。' }
  ],
  excludedSelectors: [
    'code',
    'pre',
    'textarea',
    '[contenteditable="true"]',
    '[role="textbox"]',
    '[data-copilot-zh-skip]',
    '[data-message-author-role]'
  ]
};

async function createRuntime(html = '') {
  const source = await readFile(new URL('../localization/runtime.js', import.meta.url), 'utf8');
  const dom = new JSDOM(`<body>${html}</body>`, { runScripts: 'outside-only' });
  dom.window.__COPILOT_ZH_DICTIONARY__ = dictionary;
  dom.window.eval(source);
  return dom;
}

test('translates exact UI text while preserving whitespace', async () => {
  const dom = await createRuntime('<nav>  Home\n</nav>');
  const count = dom.window.__COPILOT_ZH__.translateRoot(dom.window.document.body);
  assert.equal(dom.window.document.querySelector('nav').textContent, '  主页\n');
  assert.equal(count, 1);
});

test('translates allowlisted accessibility attributes', async () => {
  const dom = await createRuntime('<button aria-label="Settings" title="Settings">x</button><input placeholder="Search skills...">');
  dom.window.__COPILOT_ZH__.translateRoot(dom.window.document.body);
  const button = dom.window.document.querySelector('button');
  assert.equal(button.getAttribute('aria-label'), '设置');
  assert.equal(button.getAttribute('title'), '设置');
  assert.equal(dom.window.document.querySelector('input').placeholder, '搜索技能...');
});

test('translates controlled parameterized labels', async () => {
  const dom = await createRuntime('<span>3 sessions</span>');
  dom.window.__COPILOT_ZH__.translateRoot(dom.window.document.body);
  assert.equal(dom.window.document.querySelector('span').textContent, '3 个会话');
});

test('does not treat arbitrary words as numeric counts', async () => {
  const dom = await createRuntime('<span>Manage sessions</span>');
  dom.window.__COPILOT_ZH__.translateRoot(dom.window.document.body);
  assert.equal(dom.window.document.querySelector('span').textContent, 'Manage sessions');
});

test('translates numeric counts inside controlled labels', async () => {
  const dom = await createRuntime('<input aria-label="Search 12 skills">');
  dom.window.__COPILOT_ZH__.translateRoot(dom.window.document.body);
  assert.equal(dom.window.document.querySelector('input').getAttribute('aria-label'), '搜索 12 个技能');
});

test('translates dynamic status while preserving account names', async () => {
  const dom = await createRuntime('<p>25% quota used</p><p>Provided by your GitHub account @Kuma1338.</p>');
  dom.window.__COPILOT_ZH__.translateRoot(dom.window.document.body);
  const paragraphs = [...dom.window.document.querySelectorAll('p')].map(element => element.textContent);
  assert.deepEqual(paragraphs, ['已使用 25% 配额', '由你的 GitHub 账户 @Kuma1338 提供。']);
});

test('skips code editable and conversation content', async () => {
  const dom = await createRuntime('<nav>Home</nav><pre>Home</pre><code>Home</code><textarea>Home</textarea><section contenteditable="true">Home</section><article data-message-author-role="user">Home</article>');
  dom.window.__COPILOT_ZH__.translateRoot(dom.window.document.body);
  assert.equal(dom.window.document.querySelector('nav').textContent, '主页');
  assert.equal(dom.window.document.querySelector('pre').textContent, 'Home');
  assert.equal(dom.window.document.querySelector('code').textContent, 'Home');
  assert.equal(dom.window.document.querySelector('textarea').value, 'Home');
  assert.equal(dom.window.document.querySelector('[contenteditable]').textContent, 'Home');
  assert.equal(dom.window.document.querySelector('article').textContent, 'Home');
});

test('translates dynamically inserted UI nodes', async () => {
  const dom = await createRuntime('<main></main>');
  dom.window.__COPILOT_ZH__.start();
  const button = dom.window.document.createElement('button');
  button.textContent = 'Settings';
  dom.window.document.querySelector('main').append(button);
  await new Promise(resolve => dom.window.setTimeout(resolve, 0));
  assert.equal(button.textContent, '设置');
});

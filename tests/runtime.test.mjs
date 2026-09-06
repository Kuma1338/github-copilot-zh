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

async function createRuntime(html = '', activeDictionary = dictionary) {
  const source = await readFile(new URL('../localization/runtime.js', import.meta.url), 'utf8');
  const dom = new JSDOM(`<body>${html}</body>`, { runScripts: 'outside-only' });
  dom.window.__COPILOT_ZH_DICTIONARY__ = activeDictionary;
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
  const dom = await createRuntime('<p>25% quota used</p><p>Provided by your GitHub account @example-user.</p>');
  dom.window.__COPILOT_ZH__.translateRoot(dom.window.document.body);
  const paragraphs = [...dom.window.document.querySelectorAll('p')].map(element => element.textContent);
  assert.deepEqual(paragraphs, ['已使用 25% 配额', '由你的 GitHub 账户 @example-user 提供。']);
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

test('translates the secondary start-session menu from the shipped dictionary', async () => {
  const shippedDictionary = JSON.parse(
    await readFile(new URL('../localization/zh-CN.json', import.meta.url), 'utf8')
  );
  const dom = await createRuntime(`
    <section>
      <h2>Start session in</h2>
      <p>No repositories yet</p>
      <p>Add a project from</p>
      <button>Local folder or repository...</button>
      <button>GitHub repository...</button>
      <button>Repository URL...</button>
      <button>Resume session by ID...</button>
      <button>Resume remote session...</button>
    </section>
  `, shippedDictionary);

  dom.window.__COPILOT_ZH__.translateRoot(dom.window.document.body);

  assert.deepEqual(
    [...dom.window.document.querySelectorAll('h2, p, button')].map(node => node.textContent),
    [
      '开始会话',
      '暂无仓库',
      '添加项目来源',
      '本地文件夹或仓库...',
      'GitHub 仓库...',
      '仓库 URL...',
      '通过 ID 恢复会话...',
      '恢复远程会话...'
    ]
  );
});

test('translates fixed home, customize, and sound labels from the shipped dictionary', async () => {
  const shippedDictionary = JSON.parse(
    await readFile(new URL('../localization/zh-CN.json', import.meta.url), 'utf8')
  );
  const dom = await createRuntime(`
    <main>
      <p>Create a space exploration quiz with animated feedback. Creates a sample project.</p>
      <button aria-label="Dismiss extend with MCP servers card">Dismiss extend with MCP servers card</button>
      <p>Connect tools like Figma, Playwright, or Azure so agents can take actions beyond your codebase.</p>
      <h2>Extend with MCP servers</h2>
      <p>Plan and manage Azure DevOps work in a visual, interactive workspace.</p>
      <p>Build, test, and deploy hosted agents with Microsoft Foundry.</p>
      <p>Design, critique, and polish product interfaces with expert guidance.</p>
      <p>Inspect and implement Figma designs faster.</p>
      <p>Pressure-test your product plan.</p>
      <p>Review Jira work items from active sprints in a live dashboard.</p>
      <p>Test and automate web apps in browsers.</p>
      <p>Bring end-to-end Power BI semantic model authoring to your AI agents.</p>
      <p>Build secure, accessible, reliable software.</p>
      <p>Review UI for usability and accessible design.</p>
      <p>Find answers across Microsoft 365 emails, meetings, documents, and Teams messages.</p>
      <span>Copilot Success</span>
      <span>Copilot Notification</span>
    </main>
  `, shippedDictionary);

  dom.window.__COPILOT_ZH__.translateRoot(dom.window.document.body);

  assert.deepEqual(
    [...dom.window.document.querySelectorAll('p, h2, span')].map(node => node.textContent),
    [
      '创建一个带动画反馈的太空探索问答游戏。将创建示例项目。',
      '连接 Figma、Playwright 或 Azure 等工具，让代理能够执行超出代码库范围的操作。',
      '通过 MCP 服务器扩展',
      '在可视化交互式工作区中规划和管理 Azure DevOps 工作。',
      '使用 Microsoft Foundry 构建、测试和部署托管代理。',
      '在专家指导下设计、评审和完善产品界面。',
      '更快地检查并实现 Figma 设计。',
      '对你的产品计划进行压力测试。',
      '在实时仪表板中查看当前迭代的 Jira 工作项。',
      '在浏览器中测试和自动化 Web 应用。',
      '为你的 AI 代理提供端到端的 Power BI 语义模型创作能力。',
      '构建安全、无障碍且可靠的软件。',
      '检查界面的易用性和无障碍设计。',
      '从 Microsoft 365 电子邮件、会议、文档和 Teams 消息中查找答案。',
      'Copilot 成功',
      'Copilot 通知'
    ]
  );
  assert.equal(
    dom.window.document.querySelector('button').getAttribute('aria-label'),
    '关闭“通过 MCP 服务器扩展”卡片'
  );
});

test('translates shortcut editor labels while preserving shortcut text and unknown commands', async () => {
  const shippedDictionary = JSON.parse(
    await readFile(new URL('../localization/zh-CN.json', import.meta.url), 'utf8')
  );
  const dom = await createRuntime(`
    <button aria-label="Ctrl + K, edit shortcut for Command palette">Ctrl + K, edit shortcut for Command palette</button>
    <button aria-label="Control + G, edit shortcut for Find next match">Control + G, edit shortcut for Find next match</button>
    <button aria-label="Ctrl + P, edit shortcut for Go to file">Ctrl + P, edit shortcut for Go to file</button>
    <button aria-label="Ctrl + X, edit shortcut for A user command">Ctrl + X, edit shortcut for A user command</button>
  `, shippedDictionary);

  dom.window.__COPILOT_ZH__.translateRoot(dom.window.document.body);

  assert.deepEqual(
    [...dom.window.document.querySelectorAll('button')].map(node => node.getAttribute('aria-label')),
    [
      'Ctrl + K，编辑“命令面板”的快捷键',
      'Control + G，编辑“查找下一个匹配项”的快捷键',
      'Ctrl + P，编辑“转到文件”的快捷键',
      'Ctrl + X, edit shortcut for A user command'
    ]
  );
  assert.equal(
    dom.window.document.querySelectorAll('button')[0].textContent,
    'Ctrl + K，编辑“命令面板”的快捷键'
  );
});

test('translates protected input labels while preserving user-entered values and content', async () => {
  const shippedDictionary = JSON.parse(
    await readFile(new URL('../localization/zh-CN.json', import.meta.url), 'utf8')
  );
  const dom = await createRuntime(`
    <input role="textbox" aria-label="Search settings…" placeholder="Search settings…" value="Home">
    <textarea aria-label="Message" placeholder="Message">Home</textarea>
    <div contenteditable="true" aria-label="Message" title="Message">Home</div>
  `, shippedDictionary);

  dom.window.__COPILOT_ZH__.translateRoot(dom.window.document.body);

  const input = dom.window.document.querySelector('input');
  const textarea = dom.window.document.querySelector('textarea');
  const editor = dom.window.document.querySelector('[contenteditable]');
  assert.equal(input.getAttribute('aria-label'), '搜索设置…');
  assert.equal(input.getAttribute('placeholder'), '搜索设置…');
  assert.equal(input.value, 'Home');
  assert.equal(textarea.getAttribute('aria-label'), '消息');
  assert.equal(textarea.getAttribute('placeholder'), '消息');
  assert.equal(textarea.value, 'Home');
  assert.equal(editor.getAttribute('aria-label'), '消息');
  assert.equal(editor.getAttribute('title'), '消息');
  assert.equal(editor.textContent, 'Home');
});

test('covers the observed secondary chrome inventory from Copilot 1.1.15', async () => {
  const shippedDictionary = JSON.parse(
    await readFile(new URL('../localization/zh-CN.json', import.meta.url), 'utf8')
  );
  const dom = await createRuntime('', shippedDictionary);
  const observedChrome = [
    'Utility', 'Add', 'Interactive', 'Autopilot', 'Typeahead menu', 'Step-by-step collaboration',
    'Plan first, execute when ready', 'End-to-end execution without interruption',
    'Create a space exploration quiz with animated feedback.',
    'Build a personal landing page with dark mode.',
    'Build a to-do web app with smooth animations.',
    'Build a snake game with score tracking and increasing speed.',
    'Build a weather app that shows live forecasts.',
    'Added a /goal command to set a persistent objective for autopilot to work towards in local sessions.',
    'Editing and saving files, including whole-file diff changes, now works in WSL, Direct, and Mission Control workspaces, with conflict detection.',
    'You can now attach images and videos to pull request descriptions by pasting, dragging, or selecting files.',
    'e.g., Daily code review', 'Trigger', 'Hours', 'Minute', 'Run in the cloud',
    'Run even when your computer is off, triggered by events or on a schedule.',
    'Prompt', 'Describe what this automation should do. Type / for skills…',
    'Chat messages quota: 0% used', 'Project: No projects yet', 'Create and run',
    'Create automation options', 'Search MCP servers', 'Search plugins', 'Search skills',
    'Search extensions', 'Search canvas', 'Search installed',
    'Popular MCP servers to help you get started.', 'Available',
    'Browse MCP servers by category.', 'Filter available MCP servers by category',
    'Development', 'Infrastructure', 'Databases', 'Intelligence', 'Productivity',
    'Observability', 'Commerce', 'Previous featured cards', 'Next featured cards',
    'MCP Servers', 'Connect Copilot to external tools, APIs, and data.',
    'Learn more. Opens in a new browser tab.', 'You don’t have any MCP servers installed.',
    'Add tools, skills, and workflows to Copilot.', 'You don’t have any plugins installed.',
    'Give Copilot instructions for specialized tasks.', 'Browse skills', 'Browse extensions',
    'Browse canvas', 'No extensions installed',
    'Extensions added directly or through plugins will appear here.', 'Install from gist/URL…',
    'Editor', 'New session: Editor', 'New session: Browser', 'New session: Terminal',
    'Chat messages used: 0%', 'Manage accounts', 'Keyboard shortcuts, Question mark',
    'Health check', 'What’s new. Opens in a new browser tab.',
    'Help center. Opens in a new browser tab.',
    'Search sessions, repos, PRs, issues, or paste a URL…', 'Chat', 'Change theme mode',
    'Reset zoom level', 'Go to Home', 'Go to My work', 'Go to Automations', 'Go to Settings',
    'Debug', 'Copy virtualization traces', 'Auto model selection',
    'Picks the best model for each task.', 'No manual models are available to switch to.',
    'Use Auto mode', 'Optimized for', 'Quality, speed, and cost',
    'How Auto model selection works', 'How Auto model selection works. Opens in a new browser tab.',
    'Report a bug or suggest a feature', 'What feedback would you like to share?',
    'This will be posted as a public GitHub issue.', 'Share feedback, Control + Enter',
    'Ctrl + K, edit shortcut for Command palette',
    'Control + G, edit shortcut for Find next match',
    'Ctrl + P, edit shortcut for Go to file'
  ];
  const untranslated = observedChrome.filter(value => dom.window.__COPILOT_ZH__.translateValue(value) === value);
  assert.deepEqual(untranslated, []);
});

test('covers dynamic automation and search menu labels from Copilot 1.1.15', async () => {
  const shippedDictionary = JSON.parse(
    await readFile(new URL('../localization/zh-CN.json', import.meta.url), 'utf8')
  );
  const dom = await createRuntime('', shippedDictionary);
  const observedChrome = [
    'Mode: Plan, Ctrl + Shift + M',
    'Mode: Autopilot, Ctrl + Shift + M',
    'No projects yet',
    'Use Create and run from the menu to test your prompt right away.',
    'Without a project, this automation will run as a chat.',
    'Change view…',
    'Dismiss',
    'Filter by…',
    'Resume session by ID',
    'Resume remote session'
  ];

  const untranslated = observedChrome.filter(
    value => dom.window.__COPILOT_ZH__.translateValue(value) === value
  );
  assert.deepEqual(untranslated, []);
});

test('translates secondary customize chrome and preserves dynamic names', async () => {
  const shippedDictionary = JSON.parse(
    await readFile(new URL('../localization/zh-CN.json', import.meta.url), 'utf8')
  );
  const dom = await createRuntime(`
    <section>
      <p>Built-in</p>
      <p>Personal</p>
      <button aria-label="Actions for telegram-bot">Actions for telegram-bot</button>
      <span>Disable telegram-bot</span>
      <button aria-label="Add server, Chrome DevTools MCP">Add server, Chrome DevTools MCP</button>
      <button>Add custom server</button>
      <p>Add executable tools and context to Copilot.</p>
      <p>Interactive surfaces for creating and editing rich content.</p>
      <p>Interactive surfaces for focused workflows.</p>
      <p>Plugins that add useful tools and expertise.</p>
      <p>Skills that expand how Copilot works.</p>
      <button>Browse plugins</button>
      <p>Browse plugins from your marketplaces.</p>
      <span>All marketplaces</span>
      <span>Available plugins</span>
      <button>Manage marketplaces</button>
      <button aria-label="Filter by marketplace">Filter by marketplace</button>
      <button aria-label="Filter skills by source">Filter skills by source</button>
      <span>Web</span>
      <button>MCP Server…</button>
      <button>Plugin…</button>
      <button>Skill…</button>
      <button>Canvas from URL…</button>
      <span>Reveal in Explorer</span>
      <input placeholder="Search MCP servers…">
      <input placeholder="Search plugins…">
      <input placeholder="Search skills…">
      <input placeholder="Search extensions…">
      <input placeholder="Search canvas…">
      <input placeholder="Search installed…">
      <span>Chrome DevTools MCP</span>
    </section>
  `, shippedDictionary);

  dom.window.__COPILOT_ZH__.translateRoot(dom.window.document.body);

  assert.deepEqual(
    [...dom.window.document.querySelectorAll('p, span, button')].map(node => node.textContent),
    [
      '内置',
      '个人',
      'telegram-bot 的操作',
      '禁用 telegram-bot',
      '添加服务器：Chrome DevTools MCP',
      '添加自定义服务器',
      '将可执行工具和上下文添加到 Copilot。',
      '用于创建和编辑丰富内容的交互式界面。',
      '用于专注工作流的交互式界面。',
      '添加实用工具和专业能力的插件。',
      '扩展 Copilot 工作方式的技能。',
      '浏览插件',
      '从你的市场浏览插件。',
      '全部市场',
      '可用插件',
      '管理市场',
      '按市场筛选',
      '按来源筛选技能',
      '网页',
      'MCP 服务器…',
      '插件…',
      '技能…',
      '从 URL 添加画布…',
      '在资源管理器中显示',
      'Chrome DevTools MCP'
    ]
  );

  assert.deepEqual(
    [...dom.window.document.querySelectorAll('input')].map(node => node.placeholder),
    [
      '搜索 MCP 服务器…',
      '搜索插件…',
      '搜索技能…',
      '搜索扩展…',
      '搜索画布…',
      '搜索已安装项…'
    ]
  );
  assert.equal(
    dom.window.document.querySelector('button[aria-label]').getAttribute('aria-label'),
    'telegram-bot 的操作'
  );
  assert.equal(
    dom.window.document.querySelectorAll('button[aria-label]')[1].getAttribute('aria-label'),
    '添加服务器：Chrome DevTools MCP'
  );
});

test('translates dynamic disable and add-server labels without broad substring replacement', async () => {
  const shippedDictionary = JSON.parse(
    await readFile(new URL('../localization/zh-CN.json', import.meta.url), 'utf8')
  );
  const dom = await createRuntime(`
    <p>Disable frontend-fullchain-optimization</p>
    <p>Add server, Serena</p>
    <p>Unrelated Disable and Add server, text</p>
    <p data-copilot-zh-skip="true">Disable frontend-fullchain-optimization</p>
  `, shippedDictionary);

  dom.window.__COPILOT_ZH__.translateRoot(dom.window.document.body);

  assert.deepEqual(
    [...dom.window.document.querySelectorAll('p')].map(node => node.textContent),
    [
      '禁用 frontend-fullchain-optimization',
      '添加服务器：Serena',
      'Unrelated Disable and Add server, text',
      'Disable frontend-fullchain-optimization'
    ]
  );
});

test('translates complete project suggestion labels and changelog errors', async () => {
  const shippedDictionary = JSON.parse(
    await readFile(new URL('../localization/zh-CN.json', import.meta.url), 'utf8')
  );
  const dom = await createRuntime(`
    <button aria-label="Build a personal landing page with dark mode. Creates a sample project.">
      Build a personal landing page with dark mode. Creates a sample project.
    </button>
    <p>Unable to load changelog.</p>
  `, shippedDictionary);

  dom.window.__COPILOT_ZH__.translateRoot(dom.window.document.body);

  assert.equal(
    dom.window.document.querySelector('button').textContent.trim(),
    '构建一个支持深色模式的个人落地页。将创建示例项目。'
  );
  assert.equal(
    dom.window.document.querySelector('button').getAttribute('aria-label'),
    '构建一个支持深色模式的个人落地页。将创建示例项目。'
  );
  assert.equal(dom.window.document.querySelector('p').textContent, '无法加载更新日志。');
});

test('translates split empty states and the marketplace empty state', async () => {
  const shippedDictionary = JSON.parse(
    await readFile(new URL('../localization/zh-CN.json', import.meta.url), 'utf8')
  );
  const dom = await createRuntime('', shippedDictionary);
  const emptyStates = [
    ['MCP servers', '你尚未安装任何 MCP 服务器。'],
    ['plugins', '你尚未安装任何插件。'],
    ['extensions', '你尚未安装任何扩展。']
  ];

  for (const [category] of emptyStates) {
    const paragraph = dom.window.document.createElement('p');
    paragraph.append(
      dom.window.document.createTextNode('You don’t have any '),
      dom.window.document.createTextNode(category),
      dom.window.document.createTextNode(' installed.')
    );
    dom.window.document.body.append(paragraph);
  }
  const marketplaceEmptyState = dom.window.document.createElement('p');
  marketplaceEmptyState.textContent = 'No plugins available from your marketplaces.';
  dom.window.document.body.append(marketplaceEmptyState);

  dom.window.__COPILOT_ZH__.translateRoot(dom.window.document.body);

  assert.deepEqual(
    [...dom.window.document.querySelectorAll('p')].map(node => node.textContent),
    [
      ...emptyStates.map(([, translated]) => translated),
      '你的市场中没有可用插件。'
    ]
  );
});

test('translates split empty states while preserving nested inline elements', async () => {
  const shippedDictionary = JSON.parse(
    await readFile(new URL('../localization/zh-CN.json', import.meta.url), 'utf8')
  );
  const dom = await createRuntime(
    '<p class="empty-state">You don’t have any <span>MCP servers</span> installed.</p>',
    shippedDictionary
  );

  dom.window.__COPILOT_ZH__.translateRoot(dom.window.document.body);

  const paragraph = dom.window.document.querySelector('.empty-state');
  assert.equal(paragraph.textContent, '你尚未安装任何 MCP 服务器。');
  assert.equal(paragraph.querySelector('span').textContent, 'MCP 服务器');
});

test('translates the hidden drag-and-drop accessibility instructions', async () => {
  const shippedDictionary = JSON.parse(
    await readFile(new URL('../localization/zh-CN.json', import.meta.url), 'utf8')
  );
  const dom = await createRuntime('', shippedDictionary);
  const instructions = dom.window.document.createElement('div');
  instructions.id = 'DndDescribedBy-0';
  instructions.setAttribute('aria-hidden', 'true');
  instructions.textContent = `
    To pick up a draggable item, press the space bar.
    While dragging, use the arrow keys to move the item.
    Press space again to drop the item in its new position, or press escape to cancel.
  `;
  dom.window.document.body.append(instructions);

  dom.window.__COPILOT_ZH__.translateRoot(dom.window.document.body);

  assert.equal(
    instructions.textContent.trim(),
    '按空格键拾取可拖动项目。 拖动时，使用箭头键移动项目。 再次按空格键将项目放置到新位置，或按 Esc 键取消。'
  );
});

test('refreshes the dictionary when the runtime is injected again', async () => {
  const source = await readFile(new URL('../localization/runtime.js', import.meta.url), 'utf8');
  const firstDictionary = {
    ...dictionary,
    exact: { ...dictionary.exact, Home: '主页' }
  };
  const secondDictionary = {
    ...dictionary,
    exact: { ...dictionary.exact, Home: '首页' }
  };
  const dom = await createRuntime('<nav>Home</nav>', firstDictionary);
  dom.window.__COPILOT_ZH__.start();
  assert.equal(dom.window.document.querySelector('nav').textContent, '主页');

  dom.window.__COPILOT_ZH_DICTIONARY__ = secondDictionary;
  dom.window.eval(source);

  assert.equal(dom.window.document.querySelector('nav').textContent, '首页');
});

test('covers the remaining settings, menu, health, account, and chat chrome', async () => {
  const shippedDictionary = JSON.parse(
    await readFile(new URL('../localization/zh-CN.json', import.meta.url), 'utf8')
  );
  const dom = await createRuntime('', shippedDictionary);
  const observedChrome = [
    'SCORE',
    'Minimal',
    'Detailed',
    'Collapse completed turn details behind a summary panel',
    'Group consecutive tool calls into a single panel',
    'Run tools without asking.',
    'Ask before running tools.',
    'Sessions are only available on this device',
    'View session activity from another device',
    'Remote control',
    'Control sessions from another device',
    'Inside repository',
    'Next to repository',
    'Custom',
    'Last 3 days',
    'Last 7 days',
    'Last 14 days',
    'Last 30 days',
    'Last 90 days',
    'Last 180 days',
    'Last 360 days',
    '1 day',
    '7 days',
    '15 days',
    '30 days',
    '60 days',
    '90 days',
    'Top left',
    'Top center',
    'Bottom left',
    'Bottom center',
    'Bottom right',
    'Color blind',
    'High contrast',
    'Dim',
    'Applies only in dark appearance with the GitHub palette.',
    'Sort',
    'Name',
    'Hue',
    'Family',
    'Gray',
    'Blue',
    'Cyan',
    'Green',
    'Orange',
    'Yellow',
    'Red',
    'Purple',
    '30 seconds',
    '1 minute',
    '5 minutes',
    '10 minutes',
    'Indefinite',
    'Show',
    'Modified',
    'Category',
    'or',
    '1 action',
    '2 actions',
    'Search providers…',
    'Custom endpoint',
    'Open audio file…',
    'None',
    'Help center',
    'Update authorization',
    'Manage plan',
    'Remove account',
    'Add project from',
    'Files...',
    'Folder...',
    'Project file or folder...',
    'Session...',
    'Use local folder or repository',
    'Clone from URL',
    'Grouping',
    'Ordering',
    'Updated',
    'Filters',
    'Environment',
    'Source',
    'Reset filters',
    'Collapse all',
    'Expand all',
    'Mark all as read',
    'Last updated',
    'Created',
    'Name (A-Z)',
    'Name (Z-A)',
    'Needs attention',
    'Working',
    'Done',
    'Draft',
    'Open',
    'Merged',
    'Closed',
    'No PR',
    'Desktop',
    'CLI',
    'Share as secret gist',
    'Autopilot runs end-to-end without stopping to check in.',
    'Ask anything. Use / for commands or & for sessions…',
    'New chat. Status: Idle.',
    'New chat, chat options',
    'Scroll to bottom',
    'Discard draft chat',
    'Search models',
    'No models match',
    'Sessions table',
    'Sessions you open will appear here.',
    'Check again',
    'Copy all',
    'Export',
    'Share as gist',
    'Fetching diagnostics',
    'Overview',
    'System health checks and status.',
    'Healthy',
    'Authentication',
    'Application',
    'Found',
    'Bundled',
    'Storage',
    'Database',
    'Logs',
    'Raw data',
    'About GitHub Copilot',
    'All rights reserved.',
    'License and Open Source Notices',
    'Make it yours'
  ];

  const untranslated = observedChrome.filter(
    value => dom.window.__COPILOT_ZH__.translateValue(value) === value
  );
  assert.deepEqual(untranslated, []);
});

test('translates a split empty state using the shipped dictionary', async () => {
  const shippedDictionary = JSON.parse(
    await readFile(new URL('../localization/zh-CN.json', import.meta.url), 'utf8')
  );
  const dom = await createRuntime(
    '<p>You don’t have any <span>MCP servers</span> installed.</p>',
    shippedDictionary
  );

  dom.window.__COPILOT_ZH__.translateRoot(dom.window.document.body);

  assert.equal(
    dom.window.document.querySelector('p').textContent,
    '你尚未安装任何 MCP 服务器。'
  );
});

test('translates a settings panel inserted after the runtime starts', async () => {
  const shippedDictionary = JSON.parse(
    await readFile(new URL('../localization/zh-CN.json', import.meta.url), 'utf8')
  );
  const dom = await createRuntime('<main></main>', shippedDictionary);
  dom.window.__COPILOT_ZH__.start();

  const panel = dom.window.document.createElement('section');
  panel.innerHTML = '<h2>Verbosity</h2><button>Minimal</button><p>Remote control</p>';
  dom.window.document.querySelector('main').append(panel);
  await new Promise(resolve => dom.window.setTimeout(resolve, 0));

  assert.deepEqual(
    [...panel.querySelectorAll('h2, button, p')].map(node => node.textContent),
    ['详细程度', '简洁', '远程控制']
  );
});
